//! Endpoint profile — één configuratieobject bezit elk netwerkvlak.
//!
//! Local development blijft loopback; productie gebruikt HTTPS *.chefgroep.online
//! (Cloudflare). Tailnet-profielen zijn optioneel, nooit verplicht.

use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;
use url::Url;

pub const DEFAULT_PROFILE_NAME: &str = "local";
pub const DEFAULT_VAULT_API: &str = "http://127.0.0.1:8321/api";
pub const DEFAULT_OPS_API: &str = "http://127.0.0.1:10101";
pub const DEFAULT_DASHBOARD: &str = "http://127.0.0.1:8080";
pub const DEFAULT_DESKTOP: &str = "http://127.0.0.1:3000";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawProfile {
    pub name: Option<String>,
    pub vault_api: Option<String>,
    pub ops_api: Option<String>,
    pub dashboard: Option<String>,
    pub desktop: Option<String>,
    pub opencodex_dashboard: Option<String>,
    pub kater_workspace: Option<String>,
    pub linear_api: Option<String>,
    pub vaultwarden_url: Option<String>,
    pub brain_api: Option<String>,
    pub agents_api: Option<String>,
    pub flows_api: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointProfile {
    pub name: String,
    pub vault_api: String,
    pub ops_api: String,
    pub dashboard: String,
    pub desktop: String,
    pub opencodex_dashboard: Option<String>,
    pub kater_workspace: Option<String>,
    pub linear_api: Option<String>,
    pub vaultwarden_url: Option<String>,
    pub brain_api: Option<String>,
    pub agents_api: Option<String>,
    pub flows_api: Option<String>,
}

impl Default for EndpointProfile {
    fn default() -> Self {
        Self {
            name: DEFAULT_PROFILE_NAME.into(),
            vault_api: DEFAULT_VAULT_API.into(),
            ops_api: DEFAULT_OPS_API.into(),
            dashboard: DEFAULT_DASHBOARD.into(),
            desktop: DEFAULT_DESKTOP.into(),
            opencodex_dashboard: None,
            kater_workspace: None,
            linear_api: None,
            vaultwarden_url: None,
            brain_api: None,
            agents_api: None,
            flows_api: None,
        }
    }
}

impl EndpointProfile {
    pub fn endpoint(&self, key: &str) -> Option<&str> {
        match key {
            "vaultApi" => Some(&self.vault_api),
            "opsApi" => Some(&self.ops_api),
            "dashboard" => Some(&self.dashboard),
            "desktop" => Some(&self.desktop),
            "opencodexDashboard" => self.opencodex_dashboard.as_deref(),
            "katerWorkspace" => self.kater_workspace.as_deref(),
            "linearApi" => self.linear_api.as_deref(),
            "vaultwardenUrl" => self.vaultwarden_url.as_deref(),
            "brainApi" => self.brain_api.as_deref(),
            "agentsApi" => self.agents_api.as_deref(),
            "flowsApi" => self.flows_api.as_deref(),
            _ => None,
        }
    }

    /// Compacte mens-leesbare host:poort-vorm voor labels en doctor-uitvoer.
    pub fn label(&self, key: &str) -> String {
        let Some(value) = self.endpoint(key) else {
            return "niet ingesteld".into();
        };
        match Url::parse(value) {
            Ok(parsed) => {
                let host = parsed.host_str().unwrap_or(value).to_string();
                match parsed.port() {
                    Some(port) if port != 80 && port != 443 => format!("{host}:{port}"),
                    _ => host,
                }
            }
            Err(_) => value.to_string(),
        }
    }

    pub fn all_urls(&self) -> Vec<&str> {
        let mut urls = vec![
            self.vault_api.as_str(),
            self.ops_api.as_str(),
            self.dashboard.as_str(),
            self.desktop.as_str(),
        ];
        if let Some(url) = &self.opencodex_dashboard {
            urls.push(url);
        }
        if let Some(url) = &self.kater_workspace {
            urls.push(url);
        }
        if let Some(url) = &self.linear_api {
            urls.push(url);
        }
        if let Some(url) = &self.vaultwarden_url {
            urls.push(url);
        }
        if let Some(url) = &self.brain_api {
            urls.push(url);
        }
        if let Some(url) = &self.agents_api {
            urls.push(url);
        }
        if let Some(url) = &self.flows_api {
            urls.push(url);
        }
        urls
    }
}

fn clean_url(value: Option<String>, fallback: &str) -> String {
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let parsed = Url::parse(raw.trim());
            match parsed {
                Ok(parsed) if parsed.scheme() == "http" || parsed.scheme() == "https" => {
                    raw.trim().trim_end_matches('/').to_string()
                }
                _ => fallback.to_string(),
            }
        }
        _ => fallback.to_string(),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn default_profile_path() -> PathBuf {
    match env::var("CHEFBAR_ENDPOINT_PROFILE") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chefbar/endpoints.json"),
    }
}

/// Globale profiel-singleton, eerst gezet door main.rs voor IPc/executor.
static GLOBAL_PROFILE: OnceLock<EndpointProfile> = OnceLock::new();

pub fn set_global_profile(profile: EndpointProfile) {
    let _ = GLOBAL_PROFILE.set(profile);
}

pub fn global_profile() -> &'static EndpointProfile {
    GLOBAL_PROFILE.get_or_init(EndpointProfile::default)
}

/// Load het profiel: env-overrides winnen van het JSON-bestand.
pub fn load_profile(path: Option<&std::path::Path>) -> EndpointProfile {
    let profile_path = path.map(PathBuf::from).unwrap_or_else(default_profile_path);
    let raw: RawProfile = std::fs::read_to_string(&profile_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(RawProfile {
            name: None,
            vault_api: None,
            ops_api: None,
            dashboard: None,
            desktop: None,
            opencodex_dashboard: None,
            kater_workspace: None,
            linear_api: None,
            vaultwarden_url: None,
            brain_api: None,
            agents_api: None,
            flows_api: None,
        });

    let env_or = |env_name: &str, raw: Option<String>, fallback: &str| -> String {
        match env::var(env_name) {
            Ok(value) if !value.trim().is_empty() => clean_url(Some(value), fallback),
            _ => clean_url(raw, fallback),
        }
    };

    EndpointProfile {
        name: env::var("CHEFBAR_PROFILE_NAME")
            .ok()
            .or(raw.name)
            .unwrap_or_else(|| DEFAULT_PROFILE_NAME.into()),
        vault_api: env_or("CHEFBAR_VAULT_API", raw.vault_api, DEFAULT_VAULT_API),
        ops_api: env_or("CHEFBAR_OPS_API", raw.ops_api, DEFAULT_OPS_API),
        dashboard: env_or("CHEFBAR_DASHBOARD", raw.dashboard, DEFAULT_DASHBOARD),
        desktop: env_or("CHEFBAR_DESKTOP", raw.desktop, DEFAULT_DESKTOP),
        opencodex_dashboard: clean_optional(
            env::var("CHEFBAR_OPENCODEX_DASHBOARD")
                .ok()
                .or(raw.opencodex_dashboard),
        ),
        kater_workspace: clean_optional(
            env::var("CHEFBAR_KATER_WORKSPACE")
                .ok()
                .or(raw.kater_workspace),
        ),
        linear_api: clean_optional(env::var("CHEFBAR_LINEAR_API").ok().or(raw.linear_api)),
        vaultwarden_url: clean_optional(
            env::var("CHEFBAR_VAULTWARDEN_URL")
                .ok()
                .or(raw.vaultwarden_url),
        ),
        brain_api: clean_optional(env::var("CHEFBAR_BRAIN_API").ok().or(raw.brain_api)),
        agents_api: clean_optional(env::var("CHEFBAR_AGENTS_API").ok().or(raw.agents_api)),
        flows_api: clean_optional(env::var("CHEFBAR_FLOWS_API").ok().or(raw.flows_api)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_loads_remote_surfaces() {
        let dir = std::env::temp_dir().join(format!("chefbar-test-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("endpoints.json");
        std::fs::write(
            &path,
            r#"{"name":"online","vaultApi":"https://vault-api.chefgroep.online/api","opsApi":"https://ops.chefgroep.online","katerWorkspace":"https://kater.chefgroep.online/agents/","linearApi":"https://api.linear.app/graphql","vaultwardenUrl":"https://vault.bitwarden.example"}"#,
        )
        .unwrap();
        let profile = load_profile(Some(&path));
        assert_eq!(profile.name, "online");
        assert_eq!(profile.label("vaultApi"), "vault-api.chefgroep.online");
        assert_eq!(profile.label("opsApi"), "ops.chefgroep.online");
        assert_eq!(
            profile.kater_workspace.as_deref(),
            Some("https://kater.chefgroep.online/agents/").map(|s| s.trim_end_matches('/'))
        );
        assert_eq!(
            profile.linear_api.as_deref(),
            Some("https://api.linear.app/graphql")
        );
        assert_eq!(
            profile.vaultwarden_url.as_deref(),
            Some("https://vault.bitwarden.example")
        );
        assert_eq!(profile.brain_api, None);
        assert_eq!(profile.agents_api, None);
        assert_eq!(profile.flows_api, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_profile_is_loopback() {
        let profile = EndpointProfile::default();
        assert_eq!(profile.vault_api, DEFAULT_VAULT_API);
        assert_eq!(profile.label("vaultApi"), "127.0.0.1:8321");
        assert_eq!(profile.brain_api, None);
        assert_eq!(profile.agents_api, None);
        assert_eq!(profile.flows_api, None);
        assert_eq!(profile.endpoint("brainApi"), None);
        assert_eq!(profile.endpoint("agentsApi"), None);
        assert_eq!(profile.endpoint("flowsApi"), None);
    }
}
