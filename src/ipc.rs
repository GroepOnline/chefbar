//! IPC: Unix-socket voor externe commando's (hotkey-launchers, scripts).
//!
//! Luistert op $XDG_RUNTIME_DIR/chefbar.sock; elke regel is een UiCommand.
//! De luister-thread kan de GTK-thread niet direct aanraken: commando's gaan
//! via hetzelfde mpsc-kanaal als de tray (start_command_bridge).

use crate::tray::UiCommand;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub fn socket_path() -> PathBuf {
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir).join("chefbar.sock"),
        _ => PathBuf::from("/tmp/chefbar.sock"),
    }
}

pub fn parse_command(line: &str) -> Option<UiCommand> {
    match line.trim() {
        "bar" | "panel" | "toggle-panel" | "dashboard" | "open" | "show" => {
            Some(UiCommand::TogglePanel)
        }
        "refresh" | "reload" => Some(UiCommand::Refresh),
        "doctor" | "check" => Some(UiCommand::Doctor),
        "quit" | "exit" | "stop" => Some(UiCommand::Quit),
        _ => None,
    }
}

pub fn send_command(command: UiCommand) -> Result<(), String> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).map_err(|e| e.to_string())?;
    let line = match command {
        UiCommand::TogglePanel => "panel\n",
        UiCommand::Refresh => "refresh\n",
        UiCommand::Doctor => "doctor\n",
        UiCommand::Quit => "quit\n",
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

/// Start de listener; commando's worden op de UI-thread afgehandeld.
pub fn spawn_listener(tx: Sender<UiCommand>) {
    std::thread::spawn(move || {
        let path = socket_path();
        // Stale socket opruimen: als er een oude socket ligt maar niemand
        // luistert, is connect_err → veilig verwijderen. Als connect_ok, draait
        // er al een instantie (en is bind EADDRINUSE correct).
        if path.exists() {
            if UnixStream::connect(&path).is_err() {
                let _ = std::fs::remove_file(&path);
            }
        }
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return,
            Err(_) => return,
        };
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let tx = tx.clone();
                    handle_connection(stream, tx);
                }
                Err(_) => break,
            }
        }
        let _ = std::fs::remove_file(&path);
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
    fn parses_known_commands() {
        assert_eq!(parse_command("panel"), Some(UiCommand::TogglePanel));
        assert_eq!(parse_command("open"), Some(UiCommand::TogglePanel));
        assert_eq!(parse_command("refresh"), Some(UiCommand::Refresh));
        assert_eq!(parse_command("quit"), Some(UiCommand::Quit));
        assert_eq!(parse_command("onzin"), None);
    }

    #[test]
    fn socket_path_respects_xdg_runtime() {
        // XDG_RUNTIME_DIR set → socket daar.
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-xdg-123");
        assert_eq!(socket_path(), PathBuf::from("/tmp/test-xdg-123/chefbar.sock"));
        std::env::remove_var("XDG_RUNTIME_DIR");
        assert_eq!(socket_path(), PathBuf::from("/tmp/chefbar.sock"));
    }
}
