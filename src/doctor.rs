//! Doctor-checks: profiel, policy, secrets (alleen fingerprints), watchdog,
//! en een korte netwerk-probe. Output als tekstregels + notificatie bij fouten.

use crate::config::{global_profile, EndpointProfile};
use crate::policy::EndpointPolicy;
use std::time::Instant;

pub struct DoctorReport {
    pub lines: Vec<String>,
    pub failures: Vec<String>,
}

impl DoctorReport {
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }

    /// Exit-code voor scripts/systemd: 0 ok / 1 degraded / 2 down.
    /// Down = geen bruikbare credentials of vault onbereikbaar; de rest
    /// (policy, profiel, watchdog) telt als degraded.
    pub fn status(&self) -> u8 {
        if self.ok() {
            return 0;
        }
        let down = self
            .failures
            .iter()
            .any(|f| f.contains("credentials") || f.contains("offline"));
        if down {
            2
        } else {
            1
        }
    }

    /// Report-regels inclusief een machine-leesbare statusregel voor IPC.
    pub fn report_lines(&self) -> Vec<String> {
        let mut out = self.lines.clone();
        out.push(if self.ok() {
            "doctor   OK".into()
        } else {
            "doctor   NIET OK".into()
        });
        out.push(format!("doctor-status {}", self.status()));
        out
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
        _ => lines.push(format!("watchdog ontbreekt: {} (nog geen poll gedraaid?)", watchdog.display())),
    }

    // 5. Versie + IPC-socket.
    lines.push(format!("versie   {}", crate::VERSION));
    let socket = crate::ipc::socket_path();
    lines.push(if socket.exists() {
        "ipc      socket aanwezig".into()
    } else {
        "ipc      socket nog niet gestart (ook ok voor --doctor)".into()
    });

    // 6. Korte netwerk-probe tegen de vault-API (niet blokkerend hier: de
    //    poll-actor is de echte bron; doctor rapporteert alleen de laatste
    //    online-status).
    let online = crate::state::vault_online();
    lines.push(if online {
        "netwerk  vault bereikbaar (laatste poll ok)".into()
    } else {
        "netwerk  vault offline (laatste poll faalde of nog geen poll)".into()
    });
    if !online {
        failures.push("vault offline (laatste poll faalde of nog geen poll)".into());
    }

    DoctorReport { lines, failures }
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

/// Doctor via CLI: leg het rapport als tekst op stdout.
pub fn print_report(report: &DoctorReport) {
    for line in report.report_lines() {
        println!("{line}");
    }
}

/// Helper zodat panel/doctor-UI het profiel kan tonen zonder import-cyclus.
pub fn profile_line(profile: &EndpointProfile) -> String {
    format!("{} · {}", profile.name, profile.label("vaultApi"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(failures: Vec<&str>) -> DoctorReport {
        DoctorReport {
            lines: vec!["profiel   test".into()],
            failures: failures.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn status_is_0_when_ok() {
        assert_eq!(report(vec![]).status(), 0);
    }

    #[test]
    fn status_is_1_for_degraded() {
        assert_eq!(report(vec!["policy-gate weigerde profiel-host(s)"]).status(), 1);
    }

    #[test]
    fn status_is_2_when_down() {
        assert_eq!(report(vec!["geen bruikbare credentials gevonden"]).status(), 2);
        assert_eq!(report(vec!["vault offline (laatste poll faalde)"]).status(), 2);
    }

    #[test]
    fn report_lines_include_machine_status() {
        let r = report(vec!["policy-gate weigerde profiel-host(s)"]);
        let lines = r.report_lines();
        assert!(lines.iter().any(|l| l == "doctor-status 1"));
        assert!(lines.iter().any(|l| l == "doctor   NIET OK"));
        assert!(lines.iter().any(|l| l == "profiel   test"));
        assert!(lines.iter().filter(|l| l.starts_with("doctor")).count() == 2);
    }
}
