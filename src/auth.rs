//! Auth-headers voor ChefBar → private *.online / vault-api.
//!
//! Interim: Bearer API-token (+ optioneel Cloudflare Access service-token-paar).
//! Doel: Authentik OIDC via dezelfde get_headers()-seam, zonder clientherbouw.

use std::env;
use std::path::PathBuf;

fn read_bearer() -> Option<String> {
    for env_name in ["CHEF_VAULT_API_TOKEN", "CHEFBAR_VAULT_TOKEN"] {
        if let Ok(value) = env::var(env_name) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    if let Ok(token_file) = env::var("CHEFBAR_VAULT_TOKEN_FILE") {
        if let Ok(text) = std::fs::read_to_string(token_file.trim()) {
            let text = text.trim();
            let value = text
                .strip_prefix("CHEF_VAULT_API_TOKEN=")
                .or_else(|| text.strip_prefix("CHEFBAR_VAULT_TOKEN="))
                .map(|rest| rest.trim().trim_matches('"').trim_matches('\''));
            if let Some(value) = value {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn legacy_env_file() -> Option<PathBuf> {
    match env::var("CHEFBAR_ENV_FILE") {
        Ok(path) if !path.trim().is_empty() => Some(PathBuf::from(path)),
        _ => dirs::home_dir().map(|home| {
            // Canoniek (GroepOnline): ChefFactory-werkruimte; oud org pad als
            // laatste fallback zodat migratiewerkstations blijven werken.
            let canonical = home.join("ChefFactory/chefgroep-vault/docker/.env");
            if canonical.exists() {
                canonical
            } else {
                home.join("Documents/Github/OnlineChefGroep/chefgroep-vault/docker/.env")
            }
        }),
    }
}

fn read_legacy_file() -> Option<String> {
    let path = legacy_env_file()?;
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        for prefix in ["CHEF_VAULT_API_TOKEN=", "CHEFBAR_VAULT_TOKEN="] {
            if let Some(rest) = line.trim_start().strip_prefix(prefix) {
                let value = rest.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn cf_access() -> Option<(String, String)> {
    let id = env::var("CF_ACCESS_CLIENT_ID")
        .or_else(|_| env::var("CHEFBAR_CF_ACCESS_CLIENT_ID"))
        .ok()?
        .trim()
        .to_string();
    let secret = env::var("CF_ACCESS_CLIENT_SECRET")
        .or_else(|_| env::var("CHEFBAR_CF_ACCESS_CLIENT_SECRET"))
        .ok()?
        .trim()
        .to_string();
    if id.is_empty() || secret.is_empty() {
        None
    } else {
        Some((id, secret))
    }
}

/// Request-headers voor vault/ops HTTPS-calls.
pub fn get_headers(json_body: bool) -> Vec<(String, String)> {
    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
    let bearer = read_bearer().or_else(read_legacy_file);
    if let Some(token) = bearer {
        headers.push(("Authorization".to_string(), format!("Bearer {token}")));
    }
    if let Some((id, secret)) = cf_access() {
        headers.push(("CF-Access-Client-Id".to_string(), id));
        headers.push(("CF-Access-Client-Secret".to_string(), secret));
    }
    if json_body {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }
    headers
}

/// Compacte status voor --doctor (nooit secrets echoën).
pub fn auth_status() -> (bool, bool) {
    let bearer = read_bearer().is_some() || read_legacy_file().is_some();
    let cf = cf_access().is_some();
    (bearer, cf)
}

/// Parse de CHEFBAR_ENV_FILE als KEY=VALUE-map voor doctor/ops inzage.
/// Waarden worden alleen als fingerprint gerapporteerd, nooit geprint.
pub fn load_env_file() -> Result<std::collections::HashMap<String, String>, String> {
    let path = match env::var("CHEFBAR_ENV_FILE") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => {
            let home = dirs::home_dir().ok_or("geen home-map")?;
            let canonical = home.join("ChefFactory/chefgroep-vault/docker/.env");
            if canonical.exists() {
                canonical
            } else {
                home.join("Documents/Github/OnlineChefGroep/chefgroep-vault/docker/.env")
            }
        }
    };
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !key.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    Ok(map)
}

/// Korte fingerprint van een secret voor doctor/chat: sha256[:12].
pub fn fingerprint(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    hex_fingerprint(&digest[..6])
}

fn hex_fingerprint(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
