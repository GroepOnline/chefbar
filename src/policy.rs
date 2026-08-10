//! Netwerk safety policy voor ChefBar-clients.
//!
//! Hoofdpad: HTTPS naar profiel-hosts / *.chefgroep.online / expliciete
//! allowlist. Optioneel: loopback, Tailscale CGNAT, *.ts.net (nooit verplicht).
//! Bearer-tokens volgen nooit redirects. Same-origin join alleen.

use std::collections::HashSet;
use std::env;
use std::net::IpAddr;
use url::Url;

pub const TAILNET_CGNAT: &str = "100.64.0.0/10";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointPolicy {
    https_allowlist: HashSet<String>,
    http_allowlist: HashSet<String>,
    online_suffixes: Vec<String>,
    allow_tsnet_https: bool,
    allow_tailnet_http: bool,
    profile_https_hosts: HashSet<String>,
}

fn host_set(env_name: &str) -> HashSet<String> {
    env::var(env_name)
        .unwrap_or_default()
        .split(',')
        .map(|item| item.trim().to_lowercase().trim_end_matches('.').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn online_suffixes() -> Vec<String> {
    let raw = env::var("CHEFBAR_ONLINE_SUFFIXES").unwrap_or_default();
    if raw.trim().is_empty() {
        return vec![".chefgroep.online".to_string()];
    }
    raw.split(',')
        .map(|item| {
            let item = item.trim().to_lowercase();
            if item.starts_with('.') {
                item
            } else {
                format!(".{item}")
            }
        })
        .filter(|item| !item.is_empty())
        .collect()
}

fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => value.trim() != "0" && value.trim().to_lowercase() != "false",
        Err(_) => default,
    }
}

impl Default for EndpointPolicy {
    fn default() -> Self {
        Self {
            https_allowlist: host_set("CHEFBAR_HTTPS_ALLOWLIST"),
            http_allowlist: host_set("CHEFBAR_HTTP_ALLOWLIST"),
            online_suffixes: online_suffixes(),
            allow_tsnet_https: env_flag("CHEFBAR_ALLOW_TSNET_HTTPS", true),
            allow_tailnet_http: env_flag("CHEFBAR_ALLOW_TAILNET_HTTP", true),
            profile_https_hosts: HashSet::new(),
        }
    }
}

impl EndpointPolicy {
    pub fn with_profile_hosts(&self, urls: &[&str]) -> Self {
        let mut hosts = self.profile_https_hosts.clone();
        for url in urls {
            if let Ok(parsed) = Url::parse(url) {
                if let Some(host) = parsed.host_str() {
                    hosts.insert(host.to_lowercase());
                }
            }
        }
        Self {
            profile_https_hosts: hosts,
            ..self.clone()
        }
    }

    fn is_private_online(&self, host: &str) -> bool {
        let host = host.trim_start_matches('.');
        self.online_suffixes
            .iter()
            .any(|suffix| host == suffix.trim_start_matches('.') || host.ends_with(suffix))
    }

    fn is_loopback(&self, host: &str) -> bool {
        matches!(host.to_lowercase().as_str(), "localhost" | "localhost.localdomain")
            || host
                .parse::<IpAddr>()
                .map(|addr| addr.is_loopback())
                .unwrap_or(false)
    }

    pub fn allows(&self, url: &str) -> bool {
        let Ok(parsed) = Url::parse(url) else {
            return false;
        };
        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return false;
        }
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let host = host.to_lowercase();

        if self.is_loopback(&host) {
            return true;
        }
        // IP-adressen: loopback of Tailscale CGNAT (alleen http voor tailnet).
        if let Ok(addr) = host.parse::<IpAddr>() {
            if addr.is_loopback() {
                return true;
            }
            if scheme == "http" && self.allow_tailnet_http && tailet_addr(&addr) {
                return true;
            }
            return false;
        }
        if scheme == "https" {
            if self.https_allowlist.contains(&host)
                || self.profile_https_hosts.contains(&host)
                || self.is_private_online(&host)
                || (self.allow_tsnet_https && host.ends_with(".ts.net"))
            {
                return true;
            }
            return false;
        }
        self.http_allowlist.contains(&host)
    }

    /// Gooi als de URL niet is toegestaan.
    pub fn require(&self, url: &str) -> Result<(), String> {
        if self.allows(url) {
            Ok(())
        } else {
            Err(format!("ChefBar blokkeert niet-toegestaan endpoint: {url}"))
        }
    }

    /// Same-origin join van een pad op een toegestane base.
    ///
    /// De base is netwerkvlak-prefix (bijv. ".../api"): joins worden opgevat
    /// als padverlenging, niet als root-relatieve URL-join. Query-strings in
    /// het pad blijven intact.
    pub fn safe_join(&self, base: &str, path: &str) -> Result<String, String> {
        self.require(base)?;
        if path.trim_start().starts_with("http://") || path.trim_start().starts_with("https://") {
            return Err(format!("absolute URL-pad mag nooit de host vervangen: {path}"));
        }
        let base_url = Url::parse(base).map_err(|e| e.to_string())?;
        let base_str = if base.ends_with('/') {
            base.to_string()
        } else {
            format!("{base}/")
        };
        let url = format!("{base_str}{}", path.trim_start_matches('/'));
        let joined = Url::parse(&url).map_err(|e| e.to_string())?;
        // Same-origin: het pad mag de host nooit vervangen.
        if (base_url.scheme(), base_url.host_str(), base_url.port())
            != (joined.scheme(), joined.host_str(), joined.port())
        {
            return Err(format!("cross-origin endpoint join geblokkeerd: {path}"));
        }
        self.require(joined.as_str())?;
        Ok(joined.to_string())
    }
}

fn tailet_addr(addr: &IpAddr) -> bool {
    // Tailscale CGNAT 100.64.0.0/10
    match addr {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 100 && octets[1] >= 64 && octets[1] <= 127
        }
        IpAddr::V6(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_and_tailnet_http_are_allowed() {
        let policy = EndpointPolicy {
            https_allowlist: ["vault.chefgroep.online".into()].into_iter().collect(),
            http_allowlist: HashSet::new(),
            online_suffixes: vec![".chefgroep.online".into()],
            allow_tsnet_https: true,
            allow_tailnet_http: true,
            profile_https_hosts: HashSet::new(),
        };
        assert!(policy.allows("http://127.0.0.1:8321/api"));
        assert!(policy.allows("http://100.115.43.1:18321/api"));
    }

    #[test]
    fn private_online_https_allowed_without_explicit_allowlist() {
        let policy = EndpointPolicy::default();
        assert!(policy.allows("https://vault-api.chefgroep.online/api"));
        assert!(policy.allows("https://kater.chefgroep.online/agents/"));
    }

    #[test]
    fn public_http_and_unknown_https_are_blocked() {
        let policy = EndpointPolicy::default();
        assert!(!policy.allows("http://example.com/api"));
        assert!(!policy.allows("https://example.com/api"));
    }

    #[test]
    fn tsnet_and_explicit_https_are_allowed() {
        let policy = EndpointPolicy {
            https_allowlist: ["vault.chefgroep.online".into()].into_iter().collect(),
            http_allowlist: HashSet::new(),
            online_suffixes: vec![".chefgroep.online".into()],
            allow_tsnet_https: true,
            allow_tailnet_http: true,
            profile_https_hosts: HashSet::new(),
        };
        assert!(policy.allows("https://chef-control.example.ts.net/api"));
        assert!(policy.allows("https://vault.chefgroep.online/api"));
    }

    #[test]
    fn safe_join_rejects_absolute_cross_origin() {
        let policy = EndpointPolicy::default();
        assert!(
            policy
                .safe_join("https://vault-api.chefgroep.online/api", "https://evil.example/x")
                .is_err(),
            "absolute URL-pad mag nooit de host vervangen"
        );
    }

    #[test]
    fn safe_join_keeps_same_origin_and_policy() {
        let policy = EndpointPolicy::default();
        let joined = policy
            .safe_join("https://vault-api.chefgroep.online/api", "/status")
            .unwrap();
        assert_eq!(joined, "https://vault-api.chefgroep.online/api/status");
    }
}