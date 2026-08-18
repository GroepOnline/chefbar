//! Per-agent meldings-demping (E5-staart: "rustige meldingen uitbreiden").
//!
//! Kleine JSON-lijst van agent-keys (`cursor::commerce`-vorm). De watcher
//! slaat gedempte agents over bij het sturen van toasts, en het paneel/tray
//! tonen de demp-status per agent. Zelfde tolerantiepatroon als
//! `panel_state.rs`: kapot bestand degradeert naar een lege set, atomic
//! schrijven via tmp + rename.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Pad naar de demp-lijst; `CHEFBAR_MUTED_AGENTS` wint (tests, warden-laag).
pub fn mutes_path() -> PathBuf {
    match std::env::var("CHEFBAR_MUTED_AGENTS") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chefbar/muted-agents.json"),
    }
}

/// Tolerant laden: missend of kapot bestand degradeert naar een lege set.
pub fn load() -> HashSet<String> {
    load_from(&mutes_path())
}

/// Pad-expliciete kern van `load` — testbaar zonder env.
pub fn load_from(path: &Path) -> HashSet<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .map(|keys| keys.into_iter().collect())
        .unwrap_or_default()
}

/// Atomair schrijven (tmp + rename): nooit een half bestand bij een crash.
pub fn save(keys: &HashSet<String>) -> bool {
    save_to(&mutes_path(), keys)
}

/// Pad-expliciete kern van `save` — testbaar zonder env.
pub fn save_to(path: &Path, keys: &HashSet<String>) -> bool {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let mut sorted: Vec<&String> = keys.iter().collect();
    sorted.sort();
    let Ok(json) = serde_json::to_string_pretty(&sorted) else {
        return false;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_err() {
        return false;
    }
    std::fs::rename(&tmp, path).is_ok()
}

/// Is deze agent-key gedempt? (leest het bestand, per poll-call — klein.)
pub fn is_muted(key: &str) -> bool {
    load().contains(key)
}

/// Toggle de demp voor één agent-key op een expliciet pad; geeft
/// `(nu_gedempt, opgeslagen)` terug zodat de caller een schrijffout kan melden.
pub fn toggle_at(path: &Path, key: &str) -> (bool, bool) {
    let mut keys = load_from(path);
    let now_muted = if keys.contains(key) {
        keys.remove(key);
        false
    } else {
        keys.insert(key.to_string());
        true
    };
    let persisted = save_to(path, &keys);
    (now_muted, persisted)
}

/// Toggle de demp voor één agent-key; geeft `(nu_gedempt, opgeslagen)` terug.
pub fn toggle(key: &str) -> (bool, bool) {
    toggle_at(&mutes_path(), key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("chefbar-mutes-{tag}-{}", std::process::id()))
            .join("muted-agents.json")
    }

    #[test]
    fn save_en_load_rondtrip() {
        let path = temp_path("rondtrip");
        let keys: HashSet<String> = ["cursor::commerce".into(), "kater::eval".into()]
            .into_iter()
            .collect();
        assert!(save_to(&path, &keys));
        assert_eq!(load_from(&path), keys);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missend_bestand_is_leeg() {
        let path = temp_path("leeg");
        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn kapot_json_is_leeg() {
        let path = temp_path("kapot");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{kapot").unwrap();
        assert!(load_from(&path).is_empty());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn toggle_zet_en_verwijdert() {
        let path = temp_path("toggle");
        let mut keys = HashSet::new();
        keys.insert("a::ws".into());
        assert!(save_to(&path, &keys));
        assert!(load_from(&path).contains("a::ws"));
        // toggle verwijdert
        keys.remove("a::ws");
        assert!(save_to(&path, &keys));
        assert!(!load_from(&path).contains("a::ws"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn toggle_at_persisteert_en_geeft_toestand() {
        let path = temp_path("toggle_at");
        // Eerste toggle zet + slaat op → (gedempt, opgeslagen).
        let (muted, ok) = toggle_at(&path, "cursor::commerce");
        assert!(muted && ok);
        assert!(load_from(&path).contains("cursor::commerce"));
        // Tweede toggle verwijdert + slaat op → (niet gedempt, opgeslagen).
        let (muted, ok) = toggle_at(&path, "cursor::commerce");
        assert!(!muted && ok);
        assert!(!load_from(&path).contains("cursor::commerce"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn toggle_at_meldt_schrijffout() {
        // Schrijven naar een ongeldig pad (directory als bestand) faalt en
        // `ok` is false, zodat de caller een foutmelding kan tonen.
        let path = PathBuf::from("/this/path/is/not/creatable/as-a-file/muted-agents.json");
        let (_muted, ok) = toggle_at(&path, "x::y");
        assert!(!ok);
    }
}
