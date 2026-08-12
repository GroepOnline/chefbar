//! Minimale bestandlogger (Q4): actor/executor-fouten naar
//! `~/.local/state/chefbar/chefbar.log` (of `CHEFBAR_LOG`). README en
//! notificaties verwezen al naar "chefbar.log" — dat bestand bestond niet.

use std::io::Write;
use std::path::PathBuf;

/// Pad naar het logbestand; `CHEFBAR_LOG` wint (tests en warden-laag).
pub fn log_path() -> PathBuf {
    match std::env::var("CHEFBAR_LOG") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => dirs::state_dir()
            .unwrap_or_else(|| crate::home_dir().join(".local/state"))
            .join("chefbar/chefbar.log"),
    }
}

/// Schrijf één regel (append, tolerant; nooit paniek). Gaat ook naar stderr
/// zodat systemd/journald het meekrijgt.
pub fn log(line: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "[{}] {line}", hhmmss());
    }
    eprintln!("[chefbar] {line}");
}

/// Korte UTC-tijd HH:MM:SS voor logregels (geen chrono-dep nodig).
fn hhmmss() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = (secs % 86400 / 3600, secs % 3600 / 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // De tests muteren dezelfde CHEFBAR_LOG-env-var; rust draait tests parallel,
    // dus één lock serialiseert ze (anders flaky: remove_var tijdens een andere test).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn pad_respecteert_chefbar_log() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("CHEFBAR_LOG", "/tmp/chefbar-test-log/x.log");
        assert_eq!(log_path(), PathBuf::from("/tmp/chefbar-test-log/x.log"));
        std::env::remove_var("CHEFBAR_LOG");
    }

    #[test]
    fn default_pad_is_onder_state_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("CHEFBAR_LOG");
        let path = log_path();
        assert!(path.ends_with("chefbar/chefbar.log"));
    }

    #[test]
    fn log_schrijft_een_regel_naar_bestand() {
        let _guard = ENV_LOCK.lock().unwrap();
        let path = "/tmp/chefbar-test-log/x.log";
        std::env::set_var("CHEFBAR_LOG", path);
        let _ = std::fs::remove_file(path);
        log("testregel-q4");
        let content = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            content.contains("testregel-q4"),
            "logregel moet in het bestand staan: {content:?}"
        );
        let _ = std::fs::remove_file(path);
        std::env::remove_var("CHEFBAR_LOG");
    }
}
