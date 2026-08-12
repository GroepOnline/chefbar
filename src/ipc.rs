//! IPC: Unix-socket voor externe commando's (hotkey-launchers, scripts).
//!
//! Luistert op $XDG_RUNTIME_DIR/chefbar.sock; elke regel is een UiCommand.
//! De luister-thread kan de GTK-thread niet direct aanraken: commando's gaan
//! via hetzelfde mpsc-kanaal als de tray (start_command_bridge).

use crate::tray::UiCommand;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub fn socket_path() -> PathBuf {
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir).join("chefbar.sock"),
        _ => PathBuf::from("/tmp/chefbar.sock"),
    }
}

pub fn parse_command(line: &str) -> Option<UiCommand> {
    let trimmed = line.trim();
    match trimmed {
        "bar" | "panel" | "open" | "show" | "dashboard" => Some(UiCommand::ShowPanel),
        "toggle-panel" => Some(UiCommand::TogglePanel),
        "refresh" | "reload" => Some(UiCommand::Refresh),
        "doctor" | "check" => Some(UiCommand::Doctor),
        "quit" | "exit" | "stop" => Some(UiCommand::Quit),
        "pause-notify" => Some(UiCommand::PauseNotifications),
        "toggle-autostart" => Some(UiCommand::ToggleAutostart),
        _ if trimmed.starts_with("state ") => {
            let state = trimmed.trim_start_matches("state ").trim();
            if matches!(state, "stil" | "bezig" | "hulp" | "fout" | "offline") {
                Some(UiCommand::ForceState(state.to_string()))
            } else {
                None
            }
        }
        _ => {
            let mut parts = trimmed.splitn(2, ' ');
            match parts.next() {
                Some("open-url") => parts
                    .next()
                    .map(|url| UiCommand::OpenUrl(url.trim().to_string())),
                Some("focus") => parts
                    .next()
                    .map(|id| UiCommand::FocusAgent(id.trim().to_string())),
                Some("desktop") => parts
                    .next()
                    .map(|verb| UiCommand::DesktopAction(verb.trim().to_string())),
                Some("mute") => parts
                    .next()
                    .map(|key| UiCommand::ToggleMute(key.trim().to_string())),
                Some("switch-account") => {
                    let rest = parts.next().unwrap_or("").trim();
                    let mut fields = rest.split_whitespace();
                    let account_id = fields.next().unwrap_or("").to_string();
                    let source = fields.next().unwrap_or("").to_string();
                    let driver = fields.next().map(String::from);
                    if account_id.is_empty() || source.is_empty() {
                        None
                    } else {
                        Some(UiCommand::SwitchAccount {
                            account_id,
                            source,
                            driver,
                        })
                    }
                }
                _ => None,
            }
        }
    }
}

pub fn send_command(command: UiCommand) -> Result<(), String> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).map_err(|e| e.to_string())?;
    let line = match command {
        UiCommand::ShowPanel => "show\n".to_string(),
        UiCommand::TogglePanel => "toggle-panel\n".to_string(),
        UiCommand::Refresh => "refresh\n".to_string(),
        UiCommand::Doctor => "doctor\n".to_string(),
        UiCommand::Quit => "quit\n".to_string(),
        UiCommand::OpenUrl(url) => format!("open-url {url}\n"),
        UiCommand::FocusAgent(id) => format!("focus {id}\n"),
        UiCommand::SwitchAccount {
            account_id,
            source,
            driver,
        } => format!(
            "switch-account {} {} {}\n",
            account_id,
            source,
            driver.unwrap_or_default()
        ),
        UiCommand::PauseNotifications => "pause-notify\n".to_string(),
        UiCommand::ToggleAutostart => "toggle-autostart\n".to_string(),
        UiCommand::DesktopAction(verb) => format!("desktop {verb}\n"),
        UiCommand::ForceState(state) => format!("state {state}\n"),
        UiCommand::ToggleMute(key) => format!("mute {key}\n"),
    };
    use std::io::Write;
    let mut stream = stream;
    stream
        .write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    // Zonodig flush zodat de listener het direct ziet (geen buffering-race).
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Wie de socket bezit: Owner bindt (en is dè instantie), Occupied betekent
/// dat een andere instantie hem al houdt — ook als die nog midden in de
/// GTK-init zit. Zo kan een gelijktijdige start nooit twee panels maken.
pub enum Acquire {
    Owner(UnixListener),
    Occupied,
}

/// Bind de IPC-socket (met stale-cleanup) zodra de app start, vóór GTK-init.
pub fn acquire() -> Acquire {
    acquire_at(&socket_path())
}

fn acquire_at(path: &std::path::Path) -> Acquire {
    use std::os::unix::fs::FileTypeExt;
    if path.exists() {
        let is_socket = std::fs::metadata(path)
            .map(|m| m.file_type().is_socket())
            .unwrap_or(false);
        match UnixStream::connect(path) {
            // Levende luisteraar: netjes Occupied, nooit aan de file komen.
            Ok(_) => return Acquire::Occupied,
            // ECONNREFUSED op een socket = niemand luistert (stale): opruimen.
            Err(e) if is_socket && e.kind() == std::io::ErrorKind::ConnectionRefused => {
                let _ = std::fs::remove_file(path);
            }
            // Elke andere fout (permissies, transiënt): niet stelen — liever
            // geen nieuwe instantie dan een tweede eigenaar.
            Err(_) => return Acquire::Occupied,
        }
    }
    match UnixListener::bind(path) {
        Ok(listener) => {
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
            Acquire::Owner(listener)
        }
        Err(_) => Acquire::Occupied,
    }
}

/// Start de listener op een eerder geacquirede socket; commando's worden op
/// de UI-thread afgehandeld.
pub fn spawn_listener_on(listener: UnixListener, tx: Sender<UiCommand>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let tx = tx.clone();
                    handle_connection(stream, tx);
                }
                Err(_) => break,
            }
        }
        let _ = std::fs::remove_file(socket_path());
    });
}

fn handle_connection(stream: UnixStream, tx: Sender<UiCommand>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(command) = parse_command(&line) {
            let _ = tx.send(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_is_exclusief() {
        // Twee gelijktijdige starts: precies één Owner, de rest Occupied.
        let dir = std::env::temp_dir().join(format!("chefbar-test-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.sock");
        let _ = std::fs::remove_file(&path);

        let eerste = acquire_at(&path);
        assert!(
            matches!(eerste, Acquire::Owner(_)),
            "eerste bind moet lukken"
        );
        assert!(
            matches!(acquire_at(&path), Acquire::Occupied),
            "tweede bind is Occupied"
        );

        // Stale-cleanup: na drop blijft de socket-file liggen zonder
        // luisteraar; acquire moet die opruimen (ECONNREFUSED-branch).
        drop(eerste);
        assert!(path.exists(), "socket-file blijft liggen na drop");
        assert!(
            matches!(acquire_at(&path), Acquire::Owner(_)),
            "stale socket wordt opgeruimd en gebonden"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn parses_known_commands() {
        assert_eq!(parse_command("panel"), Some(UiCommand::ShowPanel));
        assert_eq!(parse_command("open"), Some(UiCommand::ShowPanel));
        assert_eq!(parse_command("toggle-panel"), Some(UiCommand::TogglePanel));
        assert_eq!(parse_command("refresh"), Some(UiCommand::Refresh));
        assert_eq!(parse_command("quit"), Some(UiCommand::Quit));
        assert_eq!(parse_command("onzin"), None);
    }

    #[test]
    fn parses_statusline_commands() {
        assert_eq!(
            parse_command("pause-notify"),
            Some(UiCommand::PauseNotifications)
        );
        assert_eq!(
            parse_command("toggle-autostart"),
            Some(UiCommand::ToggleAutostart)
        );
        assert_eq!(
            parse_command("desktop stop"),
            Some(UiCommand::DesktopAction("stop".into()))
        );
        assert_eq!(
            parse_command("open-url http://127.0.0.1:10101"),
            Some(UiCommand::OpenUrl("http://127.0.0.1:10101".into()))
        );
        assert_eq!(
            parse_command("focus pane-7"),
            Some(UiCommand::FocusAgent("pane-7".into()))
        );
        assert_eq!(
            parse_command("switch-account acc-1 vault cpm:opencodex"),
            Some(UiCommand::SwitchAccount {
                account_id: "acc-1".into(),
                source: "vault".into(),
                driver: Some("cpm:opencodex".into()),
            })
        );
        assert_eq!(
            parse_command("switch-account acc-1 vault"),
            Some(UiCommand::SwitchAccount {
                account_id: "acc-1".into(),
                source: "vault".into(),
                driver: None,
            })
        );
        assert_eq!(
            parse_command("mute cursor::commerce"),
            Some(UiCommand::ToggleMute("cursor::commerce".into()))
        );
        assert_eq!(
            parse_command("state bezig"),
            Some(UiCommand::ForceState("bezig".into()))
        );
        assert_eq!(
            parse_command("state fout"),
            Some(UiCommand::ForceState("fout".into()))
        );
        assert_eq!(parse_command("state onzin"), None);
    }

    #[test]
    fn socket_path_respects_xdg_runtime() {
        // XDG_RUNTIME_DIR set → socket daar.
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-xdg-123");
        assert_eq!(
            socket_path(),
            PathBuf::from("/tmp/test-xdg-123/chefbar.sock")
        );
        std::env::remove_var("XDG_RUNTIME_DIR");
        assert_eq!(socket_path(), PathBuf::from("/tmp/chefbar.sock"));
    }
}
