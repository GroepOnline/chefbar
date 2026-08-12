//! Notificaties + browser-openen, policy-gecheckt.
//! Inbox-coalescing + per-domein ernst: blocked > hulp > warn > ok.

use crate::config;
use crate::models::Suggestion;
use crate::policy::EndpointPolicy;
use std::process::Command;

/// Notificatie via joep-notify (ChefGroep), fallback notify-send.
pub fn notify(title: &str, body: &str, status: &str) {
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
    let candidates: Vec<Vec<String>> = vec![
        home.as_ref()
            .map(|home| {
                vec![
                    format!("{home}/.local/bin/joep-notify"),
                    "-s".to_string(),
                    "ops".to_string(),
                    "-S".to_string(),
                    status.to_string(),
                    "--".to_string(),
                    title.to_string(),
                    body.to_string(),
                ]
            })
            .unwrap_or_default(),
        vec![
            "notify-send".to_string(),
            "--app-name=ChefBar".to_string(),
            "--".to_string(),
            title.to_string(),
            body.to_string(),
        ],
    ];
    for cmd in candidates {
        if cmd.is_empty() {
            continue;
        }
        let Ok(mut child) = Command::new(&cmd[0]).args(&cmd[1..]).spawn() else {
            continue;
        };
        let _ = child.wait();
        return;
    }
}

/// Bepaal ernst per stempel (voor inbox-sortering / coalescing).
/// Contract Lane E: blocked > hulp/warn > rest.
pub fn severity_rank(stamp: &str) -> u8 {
    match stamp {
        "FOUT" | "blocked" | "BLOCKED" => 3,
        "HULP" | "warn" | "WARN" | "LIMIET" => 2,
        "KLAAR" | "ok" | "OK" => 1,
        _ => 0,
    }
}

/// Sorteer suggesties op ernst (hoogste eerst) — behoud binnen gelijk niveau
/// de originele volgorde (stable sort).
pub fn sort_by_severity(suggestions: &mut [Suggestion]) {
    suggestions.sort_by_key(|b| std::cmp::Reverse(severity_rank(&b.stamp)));
}

/// Coalesce een slice suggesties tot één toast-titel + body (gebruikt
/// severity_rank i.p.v. alleen HULP/FOUT literal).
pub fn coalesced_severity(suggestions: &[Suggestion]) -> &'static str {
    let max = suggestions
        .iter()
        .map(|s| severity_rank(&s.stamp))
        .max()
        .unwrap_or(0);
    match max {
        3 => "error",
        2 => "warn",
        _ => "ok",
    }
}

/// Open een URL alleen als die de policy passeert (https naar toegestane hosts).
pub fn open_url_with_policy(url: &str, policy: &EndpointPolicy) {
    if policy.require(url).is_err() {
        eprintln!("[warn] URL openen geweigerd: {url}");
        return;
    }
    match url::Url::parse(url) {
        Ok(parsed) if parsed.scheme() == "http" || parsed.scheme() == "https" => {
            let _ = Command::new("xdg-open").arg(url).spawn();
        }
        _ => eprintln!("[warn] URL-schema niet toegestaan: {url}"),
    }
}

/// Default-policy variant (profiel-hosts geïnjecteerd).
pub fn open_url(url: &str) {
    let urls = config::global_profile().all_urls();
    let policy = EndpointPolicy::default().with_profile_hosts(&urls);
    open_url_with_policy(url, &policy);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_blocked_beats_hulp() {
        assert!(severity_rank("FOUT") > severity_rank("HULP"));
        assert!(severity_rank("blocked") > severity_rank("HULP"));
        assert!(severity_rank("HULP") > severity_rank("KLAAR"));
    }

    #[test]
    fn coalesced_picks_worst() {
        let mk = |stamp: &str| Suggestion {
            key: "k".into(),
            title: "t".into(),
            meta: "m".into(),
            stamp: stamp.into(),
            action_label: "Open".into(),
            kind: crate::models::SuggestionKind::None_,
            created_unix: 0,
        };
        assert_eq!(coalesced_severity(&[mk("KLAAR")]), "ok");
        assert_eq!(coalesced_severity(&[mk("KLAAR"), mk("HULP")]), "warn");
        assert_eq!(coalesced_severity(&[mk("HULP"), mk("FOUT")]), "error");
    }
}
