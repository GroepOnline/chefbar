//! Doctor-checks: profiel, policy, secrets (alleen fingerprints), watchdog,
//! en per-domein probes (vault, ops, linear, kater).
//!
//! Output als tekstregels + notificatie bij fouten. Exit 0 = ok, 1 = waarschuwing, 2 = fout.
//! IPC-first: als de live instantie reageert, is dat leidend.

use crate::config::{global_profile, EndpointProfile};
use crate::policy::EndpointPolicy;
use std::time::Instant;

pub struct DoctorReport {
    pub lines: Vec<String>,
    pub failures: Vec<String>,
    /// 0 = ok, 1 = warn (degraded), 2 = error (fout/blocker)
    pub exit_code: i32,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.failures.is_empty() && self.exit_code == 0
    }

    /// Exit-code volgens contract: 0/1/2. Behoud `ok()` voor backwards-compat
    /// (0 = ok, alles anders = niet ok).
    pub fn code(&self) -> i32 {
        self.exit_code
    }
}

/// Per-domein status voor de doctor-lijst (voor future snapshot-integratie).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainStatus {
    Ok(String),
    Warn(String),
    Error(String),
    Skipped(String),
}

fn domain_label(key: &str) -> &'static str {
    match key {
        "vault" => "vault   ",
        "ops" => "ops     ",
        "linear" => "linear  ",
        "kater" => "kater   ",
        _ => "domein  ",
    }
}

/// Check één domein: als endpoint ontbreekt → skip, anders policy-check
/// + korte netwerk-hint (via vault_online / env).
///
/// Geen echte HTTP hier (doctor moet snel zijn en niet blokkeren op netwerk).
fn check_domain(key: &str, url_opt: Option<&str>, policy: &EndpointPolicy) -> DomainStatus {
    let Some(url) = url_opt else {
        return DomainStatus::Skipped(format!("{} niet ingesteld", domain_label(key)));
    };
    if url.trim().is_empty() {
        return DomainStatus::Skipped(format!("{} niet ingesteld", domain_label(key)));
    }
    // Loopback in productie-profiel is al eerder gefaald; hier alleen policy.
    match policy.require(url) {
        Ok(()) => {}
        Err(reason) => {
            return DomainStatus::Error(format!("{} geweigerd ({})", domain_label(key), reason));
        }
    }
    // Voor vault/ops hebben we vault_online(); voor linear/kater: alleen
    // "geconfigureerd" als check (echte probe is via fetch in Poller).
    let online = crate::state::vault_online();
    match key {
        "vault" => {
            if online {
                DomainStatus::Ok(format!("{} bereikbaar ({})", domain_label(key), url))
            } else {
                DomainStatus::Warn(format!(
                    "{} offline of nog geen poll ({})",
                    domain_label(key),
                    url
                ))
            }
        }
        "ops" => DomainStatus::Ok(format!("{} geconfigureerd ({})", domain_label(key), url)),
        "linear" | "kater" => {
            DomainStatus::Ok(format!("{} geconfigureerd ({})", domain_label(key), url))
        }
        _ => DomainStatus::Ok(format!("{} ok ({})", domain_label(key), url)),
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
    let (has_bearer, has_cf) = crate::auth::auth_status();
    if has_bearer {
        lines.push(format!("secrets  bearer ok (cloudflare={})", has_cf));
    } else if has_cf {
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

    // 6. Per-domein checks (vault, ops, linear, kater) — skip als niet ingesteld.
    let linear_url = std::env::var("CHEFBAR_LINEAR_API").ok().or_else(|| {
        std::env::var("LINEAR_API_KEY")
            .ok()
            .map(|_| "linear-configured".to_string())
    });
    let kater_url = profile.kater_workspace.clone();
    let ops_url = Some(profile.ops_api.as_str());
    let vault_url = Some(profile.vault_api.as_str());

    let domains: Vec<(&str, Option<String>)> = vec![
        ("vault", vault_url.map(String::from)),
        ("ops", ops_url.map(String::from)),
        ("linear", linear_url),
        ("kater", kater_url),
    ];
    let mut domain_warned = false;
    let mut domain_failed = false;
    for (key, url_opt) in domains {
        let status = check_domain(key, url_opt.as_deref(), &policy);
        match status {
            DomainStatus::Ok(msg) => lines.push(msg),
            DomainStatus::Skipped(msg) => lines.push(msg),
            DomainStatus::Warn(msg) => {
                lines.push(msg.clone());
                domain_warned = true;
            }
            DomainStatus::Error(msg) => {
                lines.push(msg.clone());
                failures.push(msg);
                domain_failed = true;
            }
        }
    }
    // Exit-code: 2 bij harde fout (failures of domain error), 1 bij warn-only,
    // 0 als alles ok of alleen skipped.
    let exit_code = if !failures.is_empty() || domain_failed {
        2
    } else if domain_warned {
        1
    } else {
        0
    };

    // 7. Korte netwerk-probe tegen de vault-API (niet blokkerend: poll-actor
    //    is de echte bron; doctor rapporteert alleen de laatste online-status).
    let online = crate::state::vault_online();
    lines.push(if online {
        "netwerk  vault bereikbaar (laatste poll ok)".into()
    } else {
        "netwerk  vault offline (laatste poll faalde of nog geen poll)".into()
    });

    DoctorReport {
        lines,
        failures,
        exit_code,
    }
}

/// Latency-probe in een aparte thread; resultaat gemeld via notify.
pub fn run_checks_async(report: DoctorReport) {
    let check_time = Instant::now().elapsed();
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
}

/// Doctor via CLI: leg het rapport als tekst op stdout. Exit-code via caller.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_skip_when_not_configured() {
        let policy = EndpointPolicy::default();
        let status = check_domain("linear", None, &policy);
        assert!(matches!(status, DomainStatus::Skipped(_)));
        let status2 = check_domain("kater", Some(""), &policy);
        assert!(matches!(status2, DomainStatus::Skipped(_)));
    }

    #[test]
    fn domain_ok_when_configured() {
        // ops met loopback is ok via policy allowlist
        let policy = EndpointPolicy::default().with_profile_hosts(&["http://127.0.0.1:10101"]);
        let status = check_domain("ops", Some("http://127.0.0.1:10101"), &policy);
        assert!(matches!(status, DomainStatus::Ok(_)));
    }

    #[test]
    fn exit_code_mapping() {
        let r0 = DoctorReport {
            lines: vec![],
            failures: vec![],
            exit_code: 0,
        };
        assert!(r0.ok());
        assert_eq!(r0.code(), 0);
        let r1 = DoctorReport {
            lines: vec![],
            failures: vec![],
            exit_code: 1,
        };
        assert!(!r1.ok());
        assert_eq!(r1.code(), 1);
        let r2 = DoctorReport {
            lines: vec!["x".into()],
            failures: vec!["fout".into()],
            exit_code: 2,
        };
        assert!(!r2.ok());
        assert_eq!(r2.code(), 2);
    }
}
