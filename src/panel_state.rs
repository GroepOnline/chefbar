//! Panel-state die overleeft wat de sessie overleeft.
//!
//! Het panel onthoudt het laatst gekozen harnas en de laatste zoekterm, ook
//! over een herstart van de app heen. Eén klein JSON-bestand, atomair
//! geschreven, nooit een verrassing bij heropenen. Geen secrets — alleen
//! UI-voorkeur.
//!
//! 4.0-uitbreiding: active_group (was harness/alias), drawer_open, density,
//! recent_domains (capped 20). Tolerant laden met backwards compat en
//! 2 s debounce (regie in panel::Panel, hier alleen persist).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Dichtheid-token voor spacing in de UI.
pub const DENSITY_COMFORTABLE: &str = "comfortable";
pub const DENSITY_COMPACT: &str = "compact";

fn default_density() -> String {
    DENSITY_COMFORTABLE.to_string()
}

fn is_compact(d: &str) -> bool {
    d == DENSITY_COMPACT
}

/// Normaliseert density naar één van de twee toegestane waarden.
pub fn normalize_density(raw: &str) -> String {
    if is_compact(raw.trim()) {
        DENSITY_COMPACT.to_string()
    } else {
        DENSITY_COMFORTABLE.to_string()
    }
}

/// Cap voor recent_domains.
pub const RECENT_DOMAINS_CAP: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PanelState {
    /// Nieuwe canonieke naam; serialiseert als `active_group`. Deserialisatie
    /// accepteert ook `active_harness` en het oude `harness`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_group: Option<String>,

    /// Deprecated: alleen voor backwards compat bij lezen. Bij schrijven
    /// normaliseren we alles naar `active_group`; dit veld wordt daarom niet
    /// geserialiseerd tenzij expliciet nodig. We bewaren het hier zodat oude
    /// JSON (`{"harness":"fleet"}`) niet verloren gaat bij tolerant load.
    #[serde(default, skip_serializing)]
    pub harness: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Of de detail-drawer open stond.
    #[serde(default)]
    pub drawer_open: bool,

    /// `comfortable` | `compact`.
    #[serde(default = "default_density")]
    pub density: String,

    /// Recent bezochte domeinen/groepen, MRU, capped 20.
    #[serde(default)]
    pub recent_domains: Vec<String>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            active_group: None,
            harness: None,
            query: None,
            drawer_open: false,
            density: default_density(),
            recent_domains: Vec::new(),
        }
    }
}

// Custom Deserialize zodat oude JSON met `harness` of `active_harness`
// automatisch naar `active_group` mapt, tolerant en zonder panic.
impl<'de> Deserialize<'de> for PanelState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default, alias = "active_harness")]
            active_group: Option<String>,
            #[serde(default, alias = "active_harness")]
            harness: Option<String>,
            #[serde(default)]
            query: Option<String>,
            #[serde(default)]
            drawer_open: Option<bool>,
            #[serde(default)]
            density: Option<String>,
            #[serde(default)]
            recent_domains: Option<Vec<String>>,
            // vang extra velden tolerant op zonder te falen
            #[serde(default)]
            active_harness: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;
        // Prioriteit: active_group > active_harness > harness
        let active_group = raw.active_group.or(raw.active_harness).or(raw.harness);

        let density_raw = raw.density.unwrap_or_else(default_density);
        let density = normalize_density(&density_raw);

        let mut recent = raw.recent_domains.unwrap_or_default();
        // deduplicate behoudend volgorde, cap 20
        let mut seen = std::collections::HashSet::new();
        recent.retain(|d| {
            let k = d.trim().to_lowercase();
            if k.is_empty() {
                return false;
            }
            seen.insert(k)
        });
        if recent.len() > RECENT_DOMAINS_CAP {
            recent.truncate(RECENT_DOMAINS_CAP);
        }

        Ok(PanelState {
            active_group,
            harness: None,
            query: raw.query.filter(|q| !q.trim().is_empty()),
            drawer_open: raw.drawer_open.unwrap_or(false),
            density,
            recent_domains: recent,
        })
    }
}

impl PanelState {
    /// Canonieke groep, met fallback op `harness` alias voor callers die nog
    /// het oude veld lezen.
    pub fn effective_group(&self) -> Option<&str> {
        self.active_group.as_deref().or(self.harness.as_deref())
    }

    /// Push een domein naar recent_domains (MRU, dedup, cap 20).
    pub fn push_recent_domain(&mut self, domain: &str) {
        let d = domain.trim().to_string();
        if d.is_empty() {
            return;
        }
        self.recent_domains.retain(|x| x != &d);
        self.recent_domains.insert(0, d);
        if self.recent_domains.len() > RECENT_DOMAINS_CAP {
            self.recent_domains.truncate(RECENT_DOMAINS_CAP);
        }
    }

    /// CSS-klas voor density-token.
    pub fn density_class(&self) -> &'static str {
        if self.density == DENSITY_COMPACT {
            "density-compact"
        } else {
            "density-comfortable"
        }
    }
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
pub fn save(state: &PanelState) -> bool {
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
/// Normaliseert verbose velden en schrijft alleen `active_group` (canoniek).
pub fn save_to(path: &std::path::Path, state: &PanelState) -> bool {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    // Zorg dat density genormaliseerd is en recent_domains gecapped.
    let mut normalized = state.clone();
    normalized.density = normalize_density(&normalized.density);
    if normalized.recent_domains.len() > RECENT_DOMAINS_CAP {
        normalized.recent_domains.truncate(RECENT_DOMAINS_CAP);
    }
    // Serialiseer via een compacte map die alleen `active_group` schrijft
    // (geen `harness` duplicatie). We hergebruiken de Serialize impl maar
    // die skipt het deprecated `harness` veld.
    let Ok(json) = serde_json::to_string_pretty(&normalized) else {
        return false;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_err() {
        return false;
    }
    std::fs::rename(&tmp, path).is_ok()
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
            active_group: Some("sync".into()),
            harness: None,
            query: Some("fleet".into()),
            drawer_open: false,
            density: DENSITY_COMFORTABLE.into(),
            recent_domains: vec!["fleet".into()],
        };
        assert!(save_to(&path, &state));
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
        assert!(save_to(&path, &PanelState::default()));
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn backwards_compat_harness_maps_to_active_group() {
        let path = temp_path("compat-harness");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"harness":"commerce","query":"x"}"#).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.active_group.as_deref(), Some("commerce"));
        assert_eq!(loaded.query.as_deref(), Some("x"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn backwards_compat_active_harness_maps() {
        let path = temp_path("compat-active-harness");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"active_harness":"eval"}"#).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.active_group.as_deref(), Some("eval"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn density_normalizes() {
        let path = temp_path("density");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"density":"compact"}"#).unwrap();
        assert_eq!(load_from(&path).density, DENSITY_COMPACT);
        std::fs::write(&path, r#"{"density":"onzin"}"#).unwrap();
        assert_eq!(load_from(&path).density, DENSITY_COMFORTABLE);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn recent_domains_capped() {
        let mut s = PanelState::default();
        for i in 0..30 {
            s.push_recent_domain(&format!("d{i}"));
        }
        assert_eq!(s.recent_domains.len(), RECENT_DOMAINS_CAP);
        assert_eq!(s.recent_domains[0], "d29");
        // dedup
        s.push_recent_domain("d5");
        assert_eq!(s.recent_domains[0], "d5");
        assert_eq!(s.recent_domains.iter().filter(|x| *x == "d5").count(), 1);
    }

    #[test]
    fn drawer_open_persists() {
        let path = temp_path("drawer");
        let state = PanelState {
            drawer_open: true,
            ..Default::default()
        };
        assert!(save_to(&path, &state));
        assert!(load_from(&path).drawer_open);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
