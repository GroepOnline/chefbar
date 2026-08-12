//! Read-only Joep Brain digest.
//!
//! Dit is de operator-brain (vault-gedeelde digest), NIET UDO Project Brain.
//! Geen live Brain-mount, alleen een periodieke read-only kopie. Fail-closed:
//! elke fout (netwerk, parse, ontbrekend endpoint) levert een lege digest —
//! geen error-spam, geen retry-storm.

use crate::config::EndpointProfile;
use crate::http::Client;
use crate::models::{parse_brain_digest, BrainChunk, BrainDigest};
use crate::policy::EndpointPolicy;
use std::time::Duration;

const DIGEST_TIMEOUT: Duration = Duration::from_secs(4);

/// Lokaal vault-pad voor de digest (geen mount — gewoon een read-only bestand).
fn local_digest_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("chefbar").join("brain-digest.json"))
}

/// Bestaat er geen lokale digest-file? Poll kan dan overslaan zonder endpoint.
pub fn no_local_digest() -> bool {
    local_digest_path().map(|p| !p.exists()).unwrap_or(true)
}

/// Haal de digest op: eerst HTTP (profile.brain_api), anders lokaal bestand.
/// Fail-closed: alles wat misgaat wordt een lege digest.
pub fn fetch_digest(profile: &EndpointProfile) -> BrainDigest {
    if let Some(base) = profile.brain_api.as_deref() {
        let policy = EndpointPolicy::default().with_profile_hosts(&[base]);
        let client = Client::new(base, policy).with_timeout(DIGEST_TIMEOUT);
        if let Ok(value) = client.get_json("/digest") {
            return parse_brain_digest(&value);
        }
        return BrainDigest::default();
    }
    match local_digest_path().and_then(|path| std::fs::read_to_string(path).ok()) {
        Some(text) => serde_json::from_str(&text)
            .map(|value| parse_brain_digest(&value))
            .unwrap_or_default(),
        None => BrainDigest::default(),
    }
}

/// Lexicale zoekactie over chunks: elk token moet als woord-prefix of
/// substring in titel/excerpt/pad/url voorkomen. Lege needle = geen hits.
pub fn search<'a>(needle: &str, digest: &'a BrainDigest) -> Vec<&'a BrainChunk> {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let tokens: Vec<&str> = needle.split_whitespace().collect();
    digest
        .chunks
        .iter()
        .filter(|chunk| {
            let hay = format!(
                "{} {} {} {}",
                chunk.title,
                chunk.excerpt.as_deref().unwrap_or(""),
                chunk.path.as_deref().unwrap_or(""),
                chunk.url.as_deref().unwrap_or("")
            )
            .to_lowercase();
            tokens.iter().all(|token| {
                hay.split_whitespace().any(|word| word.starts_with(token)) || hay.contains(token)
            })
        })
        .collect()
}

/// Doel voor BrainOpen: url wint, anders pad.
pub fn open_target(chunk: &BrainChunk) -> String {
    chunk
        .url
        .clone()
        .or_else(|| chunk.path.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(title: &str, path: &str, url: Option<&str>, excerpt: Option<&str>) -> BrainChunk {
        BrainChunk {
            title: title.into(),
            path: Some(path.into()),
            url: url.map(String::from),
            excerpt: excerpt.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn search_prefix_and_substring_lexical() {
        let digest = BrainDigest {
            chunks: vec![
                chunk(
                    "hard constraints",
                    "/brain/reports/2026-08-10/hard.md",
                    None,
                    None,
                ),
                chunk("fleet compute ssot", "/brain/index/compute.md", None, None),
            ],
            ..Default::default()
        };
        let hits = search("hard", &digest);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "hard constraints");
        let hits = search("comp", &digest);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path.as_deref(), Some("/brain/index/compute.md"));
        // substring op path telt ook
        let hits = search("compute", &digest);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn search_empty_needle_is_no_hits() {
        let digest = BrainDigest {
            chunks: vec![chunk("x", "/x", None, None)],
            ..Default::default()
        };
        assert!(search("", &digest).is_empty());
        assert!(search("   ", &digest).is_empty());
    }

    #[test]
    fn open_target_prefers_url_over_path() {
        let c = chunk(
            "t",
            "/pad.md",
            Some("https://vault.chefgroep.online/t"),
            None,
        );
        assert_eq!(open_target(&c), "https://vault.chefgroep.online/t");
        let c = chunk("t", "/pad.md", None, None);
        assert_eq!(open_target(&c), "/pad.md");
    }

    #[test]
    fn fetch_unreachable_endpoint_fails_closed() {
        let mut profile = EndpointProfile::default();
        profile.brain_api = Some("http://127.0.0.1:1".into()); // gegarandeerd dicht
        let digest = fetch_digest(&profile);
        assert!(digest.chunks.is_empty());
        assert!(digest.source.is_none());
    }
}
