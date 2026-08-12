//! Zoek-synoniemen — pure functies, geen I/O.
//!
//! Kleine alias-map zodat `fleet` ook `herdr`/`nodes` vindt, `vault` ook
//! `accounts`/`commerce`, etc. Geen LLM, alleen een deterministische expand.

use std::collections::{HashMap, HashSet};

/// Canoniek: elke key → lijst synoniemen (lager-case, zonder duplicaten).
/// Minstens 20 synoniem-regels; beide richtingen expliciet waar gewenst.
const ALIAS_ENTRIES: &[(&str, &[&str])] = &[
    ("fleet", &["herdr", "nodes"]),
    ("herdr", &["fleet", "nodes"]),
    ("nodes", &["fleet", "herdr"]),
    ("vault", &["accounts", "commerce"]),
    ("accounts", &["vault", "commerce"]),
    ("commerce", &["vault", "accounts"]),
    ("share", &["sync", "desktop"]),
    ("sync", &["share", "desktop"]),
    ("desktop", &["share", "sync", "webtop"]),
    ("linear", &["taken", "issues", "tickets"]),
    ("taken", &["linear", "issues", "tasks"]),
    ("issues", &["linear", "taken", "tickets"]),
    ("tickets", &["linear", "issues"]),
    ("secrets", &["vaultwarden", "wachtwoorden", "passwords"]),
    ("vaultwarden", &["secrets", "wachtwoorden"]),
    ("wachtwoorden", &["secrets", "vaultwarden"]),
    ("kater", &["gateway"]),
    ("gateway", &["kater"]),
    ("health", &["status", "eval", "dagscore"]),
    ("status", &["health", "eval", "dagscore"]),
    ("eval", &["health", "status", "dagscore"]),
    ("dagscore", &["health", "eval"]),
    ("tasks", &["taken", "commander", "jobs"]),
    ("containers", &["docker", "images", "prune"]),
    ("docker", &["containers", "images"]),
    ("clipboard", &["klembord", "copy", "plak"]),
    ("klembord", &["clipboard"]),
    ("inbox", &["notifs", "meldingen", "attention"]),
    ("crm", &["deals", "neon", "commerce"]),
    ("deals", &["crm", "neon"]),
];

fn alias_map() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
    for (k, vals) in ALIAS_ENTRIES {
        m.insert(*k, vals.to_vec());
    }
    m
}

/// Geef synoniemen voor één term (lowercase, zonder de term zelf).
/// Lege term → lege lijst.
pub fn aliases_for(term: &str) -> Vec<String> {
    let key = term.trim().to_lowercase();
    if key.is_empty() {
        return Vec::new();
    }
    alias_map()
        .get(key.as_str())
        .map(|vals| vals.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Breid een query uit met synoniemen: elk token krijgt zijn aliasen erbij,
/// gedupliceerd en lowercased, originele volgorde behouden waar mogelijk.
/// Lege query → lege string.
pub fn expand_query(query: &str) -> String {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return String::new();
    }
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for tok in tokens {
        if seen.insert(tok.clone()) {
            out.push(tok.clone());
        }
        for alias in aliases_for(&tok) {
            if seen.insert(alias.clone()) {
                out.push(alias);
            }
        }
    }
    out.join(" ")
}

/// Handig voor ranking: geef alle alias-varianten van een query als set.
pub fn expanded_terms(query: &str) -> Vec<String> {
    expand_query(query)
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_for_fleet_geeft_herdr_en_nodes() {
        let mut a = aliases_for("fleet");
        a.sort();
        assert!(a.contains(&"herdr".to_string()));
        assert!(a.contains(&"nodes".to_string()));
    }

    #[test]
    fn aliases_for_is_case_insensitive() {
        assert_eq!(aliases_for("Fleet"), aliases_for("fleet"));
        assert_eq!(aliases_for("VAULT"), aliases_for("vault"));
    }

    #[test]
    fn aliases_for_onbekend_is_leeg() {
        assert!(aliases_for("onbekend_xyz").is_empty());
        assert!(aliases_for("").is_empty());
    }

    #[test]
    fn expand_query_voegt_synoniemen_toe() {
        let expanded = expand_query("fleet");
        assert!(expanded.contains("fleet"));
        assert!(expanded.contains("herdr"));
        assert!(expanded.contains("nodes"));
    }

    #[test]
    fn expand_query_meerdere_tokens() {
        let expanded = expand_query("vault status");
        // vault → accounts, commerce ; status → health, eval
        assert!(expanded.contains("vault"));
        assert!(expanded.contains("accounts"));
        assert!(expanded.contains("status"));
        assert!(expanded.contains("health"));
    }

    #[test]
    fn expand_query_dedupliceert() {
        // "fleet herdr" → fleet(herdr,nodes) + herdr(fleet,nodes) → fleet,herdr,nodes zonder dubbel
        let expanded = expand_query("fleet herdr");
        let terms: Vec<&str> = expanded.split_whitespace().collect();
        let mut uniq = std::collections::HashSet::new();
        for t in &terms {
            assert!(uniq.insert(*t), "dubbele term {t} in {expanded}");
        }
        assert!(terms.contains(&"fleet"));
        assert!(terms.contains(&"herdr"));
        assert!(terms.contains(&"nodes"));
    }

    #[test]
    fn expand_query_leeg_blijft_leeg() {
        assert_eq!(expand_query(""), "");
        assert_eq!(expand_query("   "), "");
    }

    #[test]
    fn alias_map_heeft_minstens_20_entries() {
        assert!(
            ALIAS_ENTRIES.len() >= 20,
            "verwacht >=20 alias-regels, nu {}",
            ALIAS_ENTRIES.len()
        );
    }

    #[test]
    fn alle_kern_synoniemen_aanwezig() {
        // Eisen uit het lane-contract:
        for term in [
            "fleet", "vault", "share", "linear", "secrets", "kater", "health",
        ] {
            assert!(!aliases_for(term).is_empty(), "alias voor {term} ontbreekt");
        }
        // Specifieke mapping checks
        assert!(aliases_for("secrets").contains(&"vaultwarden".to_string()));
        assert!(aliases_for("kater").contains(&"gateway".to_string()));
        assert!(aliases_for("health").contains(&"status".to_string()));
        assert!(aliases_for("linear").contains(&"taken".to_string()));
    }

    #[test]
    fn expand_query_determinisme() {
        let a = expand_query("Fleet status");
        let b = expand_query("Fleet status");
        assert_eq!(a, b);
    }

    #[test]
    fn linear_alias_bevat_taken_en_issues() {
        let a = aliases_for("linear");
        assert!(a.contains(&"taken".to_string()));
        assert!(a.contains(&"issues".to_string()));
    }

    #[test]
    fn vault_alias_bevat_accounts_en_commerce() {
        let a = aliases_for("vault");
        assert!(a.contains(&"accounts".to_string()));
        assert!(a.contains(&"commerce".to_string()));
    }

    #[test]
    fn share_alias_bevat_sync_en_desktop() {
        let a = aliases_for("share");
        assert!(a.contains(&"sync".to_string()));
        assert!(a.contains(&"desktop".to_string()));
    }
}
