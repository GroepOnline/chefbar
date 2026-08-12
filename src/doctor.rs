//! Doctor-checks: profiel, policy, secrets (alleen fingerprints), watchdog,
//! en een korte netwerk-probe. Output als tekstregels + notificatie bij fouten.

use crate::config::{global_profile, EndpointProfile};
use crate::policy::EndpointPolicy;
use std::time::Instant;

pub struct DoctorReport {
    pub lines: Vec<String>,
    pub failures: Vec<String>,
    /// Doorlooptijd van de checks (ms) — voor de "alles ok"-melding.
    pub elapsed_ms: u128,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }
}

pub fn run_checks() -> DoctorReport {
    let started = Instant::now();
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
    //    Vault-bearer is nodig voor vault/; Cloudflare is optioneel (edge-auth
    //    als de deploy het gebruikt). Alleen bearer-afwezig telt als failure.
    let (has_bearer, has_cf) = crate::auth::auth_status();
    if has_bearer {
        lines.push(format!("secrets  bearer ok (cloudflare={})", has_cf));
    } else if has_cf {
        // CF zonder bearer is in sommige edge-deploys voldoende, maar waarschuw
        lines.push("secrets  cloudflare aanwezig, bearer ontbreekt (check vault-auth)".into());
        failures.push("geen bearer-token gevonden (alleen cloudflare)".into());
    } else {
        failures.push("geen bruikbare credentials gevonden".into());
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
        _ => lines.push(format!(
            "watchdog ontbreekt: {} (nog geen poll gedraaid?)",
            watchdog.display()
        )),
    }

    // 5. Versie + IPC-socket.
    lines.push(format!("versie   {}", crate::VERSION));
    let socket = crate::ipc::socket_path();
    lines.push(if socket.exists() {
        "ipc      socket aanwezig".into()
    } else {
        "ipc      socket nog niet gestart (ook ok voor --doctor)".into()
    });

    // 5b. chef-hud (E3): een tweede quick-command-overlay naast ChefBar leest
    //     als "tweede ChefBar" (Alt+Space). Alleen informeren, nooit een
    //     failure — de keuze (retireren/herbinden) is van de gebruiker.
    let hud = crate::home_dir().join(".local/bin/chef-hud");
    if hud.exists() {
        lines.push(
            "chef-hud aanwezig — kan als tweede ChefBar lezen (Alt+Space); retireer of herbind, zie README"
                .into(),
        );
    }

    // 6. Poll-gezondheid (E1): de actor meldt de laatste poll (vault + ops) —
    //    via dezelfde statics die hij elke cyclus bijwerkt. Zonder draaiende
    //    actor toont dit eerlijk "poll nooit".
    lines.push(format!("poll     {}", crate::state::last_poll_label()));

    DoctorReport {
        lines,
        failures,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

/// Latency-probe in een aparte thread; resultaat gemeld via notify.
pub fn run_checks_async(report: DoctorReport) {
    let check_time = report.elapsed_ms;
    if !report.ok() {
        let joined = report.failures.join("; ");
        crate::notify::notify("ChefBar-doctor", &joined, "error");
    } else {
        crate::notify::notify(
            "ChefBar-doctor",
            &format!("alles ok · {:.0}ms", check_time),
            "ok",
        );
    }
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
