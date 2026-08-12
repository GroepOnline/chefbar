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
        "bar" | "panel" | "open" | "show" | "dashboard" => Some(UiCommand::ShowPanel),
        "toggle-panel" => Some(UiCommand::TogglePanel),
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
        UiCommand::ShowPanel => "show\n",
        UiCommand::TogglePanel => "toggle-panel\n",
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

/// Doctor via een draaiende instantie: stuurt `doctor`, leest alle
/// report-regels terug (EOF gemarkeerd door de luisteraar) en parst de
/// machine-leesbare statusregel. Gebruikt door `--doctor` zodat die de echte
/// runtime-env van de service ziet, niet de kale shell-profiel-default.
pub struct DoctorReply {
    pub lines: Vec<String>,
    pub status: u8,
}

pub fn send_doctor() -> Result<DoctorReply, String> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).map_err(|e| e.to_string())?;
    use std::io::{BufRead, Write};
    let mut stream = stream;
    stream
        .write_all(b"doctor\n")
        .and_then(|_| stream.flush())
        .map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();
    let mut reader = BufReader::new(&stream);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => lines.push(line.trim_end().to_string()),
            Err(_) => break,
        }
    }
    let status = lines
        .iter()
        .rev()
        .find_map(|l| l.strip_prefix("doctor-status ").map(|s| s.trim().parse::<u8>().unwrap_or(1)))
        .unwrap_or(1);
    Ok(DoctorReply { lines, status })
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
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(command) = parse_command(&line) {
            // Doctor: afhandelen in de luisteraar-thread zodat de caller het
            // report terugkrijgt (doctor-checks raken geen GTK: config
            // OnceLock, atomics, bestanden). Report terugschrijven + socket
            // sluiten markeert EOF voor send_doctor(). De tray-doctor (via
            // start_command_bridge) blijft op de UI-thread lopen.
            if command == UiCommand::Doctor {
                let report = crate::doctor::run_checks();
                use std::io::Write;
                let stream = reader.get_mut();
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(3)));
                for l in report.report_lines() {
                    let _ = writeln!(stream, "{l}");
                }
                break;
            }
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
        assert!(matches!(eerste, Acquire::Owner(_)), "eerste bind moet lukken");
        assert!(matches!(acquire_at(&path), Acquire::Occupied), "tweede bind is Occupied");

        // Stale-cleanup: na drop blijft de socket-file liggen zonder
        // luisteraar; acquire moet die opruimen (ECONNREFUSED-branch).
        drop(eerste);
        assert!(path.exists(), "socket-file blijft liggen na drop");
        assert!(matches!(acquire_at(&path), Acquire::Owner(_)), "stale socket wordt opgeruimd en gebonden");

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
    fn socket_path_respects_xdg_runtime() {
        // XDG_RUNTIME_DIR set → socket daar.
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-xdg-123");
        assert_eq!(socket_path(), PathBuf::from("/tmp/test-xdg-123/chefbar.sock"));
        std::env::remove_var("XDG_RUNTIME_DIR");
        assert_eq!(socket_path(), PathBuf::from("/tmp/chefbar.sock"));
    }
}
