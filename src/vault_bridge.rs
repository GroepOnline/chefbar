//! Vault bridge — typed parsers voor alle /api/* families (pure, geen I/O).
//!
//! Fase 0 stub voor ChefApp 5.0 lane H — scheidt HTTP-parsing van state.rs.
//! Elke domein-parser is `fn parse_<domain>(Value) -> Option<DomainStruct>` met
//! tolerant parse (Default bij onbekende velden), zodat state.rs dun blijft.
//! In 5.0 groeit dit naar 18 families: status, fleet, accounts, providers,
//! clipboard, timeline, brain, crm, neon, connectors, work, commander,
//! share-sync, desktop, opencodex, observability-events.

use serde_json::Value;

/// Generieke tolerant helper: parse als T, val terug op Default bij falen.
#[allow(dead_code)]
fn tolerant<T: Default + serde::de::DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// Status (GET /api/status) — al in models.rs, hier als bridge-ping (toekomst).
pub fn ping() -> &'static str {
    "vault_bridge 5.0 fase0 — parsers landen in lane H"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fase0_stub_leeft() {
        assert!(ping().contains("5.0"));
    }
}
