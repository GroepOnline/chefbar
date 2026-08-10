//! Notificaties + browser-openen, policy-gecheckt.

use crate::config;
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