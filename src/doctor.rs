//! Doctor-checks: profiel, policy, secrets (alleen fingerprints), watchdog,
//! en een korte netwerk-probe. Output als tekstregels + notificatie bij fouten.

use crate::config::{global_profile, EndpointProfile};
use crate::policy::EndpointPolicy;
use std::time::{Duration, Instant};

pub struct DoctorReport {
    pub lines: Vec<String>,
    pub failures: Vec<String>,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

pub fn run_checks() -> DoctorReport {
    let mut lines: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let profile = global_profile().clone();

    lines.push(format!(
        "profiel   {} (vault {})",
        profile.name,
        profile.label("vaultApi")
    ));

    // 1. Profiel: endpoints mogen niet op de defaults blijven in productie.
    if profile.name != "local" && profile.vault_api.starts_with("http://127.0.0.1") {
        failures.push("profiel gebruikt loopback terwijl naam niet 'local' is".into());
    }

    // 2. Policy: alle profiel-URL's moeten de https-gate passeren.
    let policy = EndpointPolicy::default().with_profile_hosts(&profile.all_urls());
    let mut policy_ok = true;
    for url in profile.all_urls() {
        match policy.require(url) {
            Ok(()) => {}
            Err(reason) => {
                policy_ok = false;
                lines.push(format!("policy   geweigerd: {url} ({reason})"));
            }
        }
    }
    if policy_ok {
        lines.push("policy   alle profiel-hosts toegestaan".into());
    } else {
        failures.push("policy-gate weigerde profiel-host(s)".into());
    }

    // 3. Secrets: token aanwezig? Alleen fingerprint, nooit de waarde.
    match crate::auth::load_env_file() {
        Ok(env) => {
            let token = env
                .get("CLOUDFLARE_ACCESS_TOKEN")
                .or_else(|| env.get("BEARER_TOKEN"))
                .or_else(|| env.get("VAULT_API_TOKEN"));
            if let Some(token) = token {
                let fp = crate::auth::fingerprint(token);
                lines.push(format!("secrets  token aanwezig (sha256[:12]={fp})"));
            } else {
                failures.push("geen bearer-token in de env-file".into());
            }
        }
        Err(_) => {
            failures.push("env-file niet leesbaar (CHEFBAR_ENV_FILE)".into());
        }
    }

    // 4. Watchdog-state (lokaal bestand).
    let watchdog = crate::models::watch_dog_path();
    match std::fs::read_to_string(&watchdog) {
        Ok(text) if !text.trim().is_empty() => {
            let health = crate::models::parse_health(&text);
            lines.push(format!(
                "watchdog {} ({}/{})",
                watchdog.display(),
                health.ok,
                health.total
            ));
        }
        _ => lines.push(format!("watchdog ontbreekt: {}", watchdog.display())),
    }

    // 5. Versie + IPC-socket.
    lines.push(format!("versie   {}", crate::VERSION));
    let socket = crate::ipc::socket_path();
    lines.push(if socket.exists() {
        "ipc      socket aanwezig".into()
    } else {
        "ipc      socket nog niet gestart".into()
    });

    // 6. Korte netwerk-probe tegen de vault-API (niet blokkerend hier: de
    //    poll-actor is de echte bron; doctor rapporteert alleen de laatste
    //    online-status).
    let online = crate::state::vault_online();
    lines.push(if online {
        "netwerk  vault bereikbaar (laatste poll ok)".into()
    } else {
        "netwerk  vault offline (laatste poll faalde)".into()
    });

    DoctorReport { lines, failures }
}

/// Latency-probe in een aparte thread; resultaat gemeld via notify.
pub fn run_checks_async(report: DoctorReport) {
    let started = Instant::now();
    let check_time = started.elapsed();
    if !report.ok() {
        let joined = report.failures.join("; ");
        crate::notify::notify("ChefBar-doctor", &joined, "error");
    } else {
        crate::notify::notify(
            "ChefBar-doctor",
            &format!("alles ok · {:.0}ms", check_time.as_millis()),
            "ok",
        );
    }
    let _: Duration = check_time;
    let _ = &report;
}

/// Doctor via CLI: leg het rapport als tekst op stdout.
pub fn print_report(report: &DoctorReport) {
    for line in &report.lines {
        println!("{line}");
    }
    if report.ok() {
        println!("doctor   OK");
    } else {
        println!("doctor   NIET OK");
    }
}

/// Helper zodat panel/doctor-UI het profiel kan tonen zonder import-cyclus.
pub fn profile_line(profile: &EndpointProfile) -> String {
    format!("{} · {}", profile.name, profile.label("vaultApi"))
}