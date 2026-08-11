//! Panel-state die overleeft wat de sessie overleeft.
//!
//! Het panel onthoudt het laatst gekozen harnas en de laatste zoekterm, ook
//! over een herstart van de app heen. Eén klein JSON-bestand, atomair
//! geschreven, nooit een verrassing bij heropenen. Geen secrets — alleen
//! UI-voorkeur.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelState {
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

/// Pad naar het state-bestand; `CHEFBAR_PANEL_STATE` wint (tests, warden-laag).
pub fn state_path() -> PathBuf {
    match std::env::var("CHEFBAR_PANEL_STATE") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chefbar/panel-state.json"),
    }
}

/// Tolerant laden: missend of kapot bestand degradeert naar defaults.
pub fn load() -> PanelState {
    load_from(&state_path())
}

/// Atomair schrijven (tmp + rename): nooit een half bestand bij een crash.
pub fn save(state: &PanelState) -> std::io::Result<()> {
    save_to(&state_path(), state)
}

/// Pad-expliciete kern van `load` — testbaar zonder env.
pub fn load_from(path: &std::path::Path) -> PanelState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Pad-expliciete kern van `save` — testbaar zonder env.
pub fn save_to(path: &std::path::Path, state: &PanelState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("chefbar-state-{tag}-{}", std::process::id()))
            .join("panel-state.json")
    }

    #[test]
    fn save_en_load_rondtrip() {
        let path = temp_path("rondtrip");
        let state = PanelState {
            harness: Some("sync".into()),
            query: Some("fleet".into()),
        };
        save_to(&path, &state);
        assert_eq!(load_from(&path), state);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missend_bestand_geeft_defaults() {
        let path = temp_path("leeg");
        assert_eq!(load_from(&path), PanelState::default());
    }

    #[test]
    fn kapot_json_geeft_defaults() {
        let path = temp_path("kapot");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{kapot").unwrap();
        assert_eq!(load_from(&path), PanelState::default());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn tmp_bestand_blijft_niet_staan() {
        let path = temp_path("tmp");
        save_to(&path, &PanelState::default());
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
