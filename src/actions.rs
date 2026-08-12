//! Declaratieve action-registry + één executor.
//!
//! Acties zijn data (RunSpec), gebouwd uit de laatste snapshot — geen closures
//! die UI-state vangen. Executie loopt via één Executor met policy-clients.

use crate::config::EndpointProfile;
use crate::http::Client;
use crate::models::{OpsSnapshot, Snapshot};
use crate::palette::Action;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSpec {
    Noop,
    OpenUrl(String),
    OpenOcx,
    FocusAgent(String),
    SendPrompt {
        terminal_id: String,
        pane_id: Option<String>,
    },
    CreateTask {
        cwd: String,
    },
    SwitchAccount {
        account_id: String,
        source: String,
        driver: Option<String>,
    },
    CancelTask(String),
    ClipboardAdd,
    ClipboardDelete(usize),
    CopyText(String),
    DesktopAction(String),
    ShareSync(String),
    Refresh,
}

fn action(
    title: impl Into<String>,
    meta: impl Into<String>,
    stamp: impl Into<String>,
    keywords: impl Into<String>,
    run: RunSpec,
) -> Action {
    Action {
        title: title.into(),
        meta: meta.into(),
        stamp: stamp.into(),
        keywords: keywords.into(),
        section: "Acties".into(),
        shortcut: "↵".into(),
        needs_text: false,
        destructive: false,
        pinned: false,
        run,
    }
}

fn task_action(
    title: impl Into<String>,
    meta: impl Into<String>,
    keywords: impl Into<String>,
    run: RunSpec,
) -> Action {
    let mut a = action(title, meta, "TAAK", keywords, run);
    a.needs_text = true;
    a
}

fn destructive_action(
    title: impl Into<String>,
    meta: impl Into<String>,
    stamp: impl Into<String>,
    keywords: impl Into<String>,
    run: RunSpec,
) -> Action {
    let mut a = action(title, meta, stamp, keywords, run);
    a.destructive = true;
    a
}

fn agent_stamp(status: &str) -> &'static str {
    match status {
        "working" => "BEZIG",
        "idle" => "KLAAR",
        "blocked" => "HULP",
        _ => "STIL",
    }
}

/// Sync-acties mogen niet draaien als share-sync in een foutstatus staat
/// (parity met het Sync-harnas in harness.rs).
pub fn sync_blocked(snap: &Snapshot) -> bool {
    snap.share_sync.contains_key("error")
        || matches!(
            snap.share_sync.get("status").and_then(|v| v.as_str()),
            Some("error") | Some("blocked")
        )
}

/// Bouw de catalogus uit de laatste snapshots (pure functie, geen I/O).
pub fn build_actions(
    ops: &OpsSnapshot,
    snap: &Snapshot,
    profile: &EndpointProfile,
    sessions: Vec<crate::sessions::Session>,
) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();
    let home = crate::home_dir();
    let home_str = home.to_string_lossy().to_string();

    for agent in &ops.agents {
        let stamp = agent_stamp(&agent.status);
        let cwd_label = agent.cwd.replace(&home_str, "~");
        actions.push(action(
            format!("Focus {} · {}", agent.name, agent.workspace),
            cwd_label,
            stamp,
            format!("focus herdr spring {} {}", agent.name, agent.workspace),
            RunSpec::FocusAgent(agent.terminal_id.clone()),
        ));
        actions.push(task_action(
            format!("Stuur naar {} · {}", agent.name, agent.workspace),
            "typ je opdracht en kies deze regel",
            format!(
                "stuur send prompt opdracht {} {}",
                agent.name, agent.workspace
            ),
            RunSpec::SendPrompt {
                terminal_id: agent.terminal_id.clone(),
                pane_id: if agent.pane_id.is_empty() {
                    None
                } else {
                    Some(agent.pane_id.clone())
                },
            },
        ));
    }

    let mut seen_ws: Vec<String> = Vec::new();
    for agent in &ops.agents {
        if agent.workspace_id.is_empty() || seen_ws.contains(&agent.workspace_id) {
            continue;
        }
        seen_ws.push(agent.workspace_id.clone());
        actions.push(task_action(
            format!("Nieuwe agent in {}", agent.workspace),
            "start een cursor-agent met jouw opdracht",
            format!("nieuwe start agent workspace {}", agent.workspace),
            RunSpec::CreateTask {
                cwd: if agent.cwd.is_empty() {
                    home_str.clone()
                } else {
                    agent.cwd.clone()
                },
            },
        ));
    }

    for row in &snap.providers {
        for acc in &row.accounts {
            let acc_id = acc.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if Some(acc_id) == row.active_id.as_deref() {
                continue;
            }
            let label = acc
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(acc_id)
                .to_string();
            actions.push(action(
                format!("Werk als {label}"),
                format!("{} · account wisselen", row.label),
                "STIL",
                format!("account switch wissel {} {}", row.label, label),
                RunSpec::SwitchAccount {
                    account_id: acc_id.to_string(),
                    source: row.source.clone(),
                    driver: row.driver.clone(),
                },
            ));
        }
    }

    for task in &snap.tasks {
        let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let status = task
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("queued");
        let prompt: String = task
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("Taak zonder omschrijving")
            .chars()
            .take(52)
            .collect();
        if matches!(status, "queued" | "running") {
            actions.push(destructive_action(
                format!("Stop taak · {prompt}"),
                format!("{task_id} · {status}"),
                "HULP",
                format!("commander taak stop annuleer cancel {task_id}"),
                RunSpec::CancelTask(task_id.to_string()),
            ));
        }
    }

    for (index, item) in snap.clipboard.iter().take(6).enumerate() {
        let text: String = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .map(|c| if c == '\n' { ' ' } else { c })
            .take(56)
            .collect();
        let full = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        actions.push(action(
            format!("Kopieer · {text}"),
            format!("clipboard-rij {index}"),
            "STIL",
            format!("clipboard klembord kopieer plak {text}"),
            RunSpec::CopyText(full),
        ));
        actions.push(destructive_action(
            format!("Verwijder clipboard-rij {index}"),
            text.clone(),
            "HULP",
            format!("clipboard klembord verwijder delete {index}"),
            RunSpec::ClipboardDelete(index),
        ));
    }

    for event in snap.events.iter().take(5) {
        let agent = event
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("Agent");
        let workspace = event
            .get("workspace")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let summary: String = event
            .get("summary")
            .or_else(|| event.get("kind"))
            .and_then(|v| v.as_str())
            .unwrap_or("update")
            .chars()
            .take(54)
            .collect();
        let stamp = if event.get("kind").and_then(|v| v.as_str()) == Some("done") {
            "KLAAR"
        } else {
            "BEZIG"
        };
        actions.push(action(
            format!("{agent} · {summary}"),
            workspace,
            stamp,
            format!("recent event agent feed {agent} {workspace} {summary}"),
            RunSpec::OpenUrl(format!(
                "{}/#agents",
                profile.dashboard.trim_end_matches('/')
            )),
        ));
    }

    for session in sessions {
        if let Some((label, spec)) = session_open_spec(&session, profile) {
            let stamp = if session.needs_attention() {
                "HULP"
            } else {
                "BEZIG"
            };
            let title: String = session.title.chars().take(48).collect();
            actions.push(action(
                format!("{label} · {title}"),
                session.summary.clone(),
                stamp,
                format!(
                    "sessie session {} {} {}",
                    session.source, session.id, session.title
                ),
                spec,
            ));
        }
    }

    let desktop_running = snap.desktop.get("state").and_then(|v| v.as_str()) == Some("running");
    let pending = snap
        .share_sync
        .get("pendingFiles")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    actions.extend([
        task_action(
            "Stuur taak naar Commander",
            "typ je opdracht en druk op Enter",
            "commander agent opdracht taak start",
            RunSpec::CreateTask {
                cwd: home_str.clone(),
            },
        ),
        task_action(
            "Voeg toe aan clipboard",
            "typ tekst en kies deze actie",
            "clipboard klembord toevoegen add tekst",
            RunSpec::ClipboardAdd,
        ),
        action(
            if desktop_running {
                "Stop desktop"
            } else {
                "Start desktop"
            },
            "webtop · remote desktop",
            if desktop_running { "BEZIG" } else { "STIL" },
            "desktop webtop start stop",
            RunSpec::DesktopAction(if desktop_running { "stop" } else { "start" }.into()),
        ),
        action(
            "Open ops",
            format!("joep-ops · {}", profile.label("opsApi")),
            "STIL",
            "open ops joep-ops herdr overzicht",
            RunSpec::OpenUrl(profile.ops_api.clone()),
        ),
        action(
            "Open dashboard (Thuis)",
            "vault dashboard · alles in één oogopslag",
            "STIL",
            "open dashboard thuis vault",
            RunSpec::OpenUrl(profile.dashboard.clone()),
        ),
        action(
            "Open desktop",
            "webtop · remote desktop",
            "STIL",
            "open desktop webtop",
            RunSpec::OpenUrl(profile.desktop.clone()),
        ),
        action(
            "Open OpenCodex",
            "dashboard en providerstatus",
            "STIL",
            "opencodex ocx codex dashboard",
            RunSpec::OpenOcx,
        ),
        action(
            "Ververs status",
            "haal de nieuwste status op",
            "STIL",
            "ververs refresh status",
            RunSpec::Refresh,
        ),
    ]);

    // Sync-acties alleen als share-sync gezond is; bij fout één uitleg-actie
    // (Noop) zodat pull/push nooit tegen een kapotte sync lopen.
    if sync_blocked(snap) {
        actions.push(action(
            "Sync hapert",
            "los de sync-fout op voordat je bestanden ophaalt of deelt",
            "FOUT",
            "share sync error hapert fout",
            RunSpec::Noop,
        ));
    } else {
        actions.extend([
            action(
                "Haal gedeelde bestanden op",
                format!("{pending} wijzigingen wachten"),
                "STIL",
                "share sync pull ophalen bestanden",
                RunSpec::ShareSync("pull".into()),
            ),
            action(
                "Deel lokale bestanden",
                "push naar de gedeelde map",
                "STIL",
                "share sync push delen bestanden",
                RunSpec::ShareSync("push".into()),
            ),
        ]);
    }
    actions
}

fn session_open_spec(
    session: &crate::sessions::Session,
    profile: &EndpointProfile,
) -> Option<(String, RunSpec)> {
    use crate::sessions::SessionActionKind;
    match session.primary_action() {
        SessionActionKind::None_ => None,
        SessionActionKind::Kater => {
            let base = profile.kater_workspace.as_deref()?;
            let kid = session.attach.kater_session_id.as_deref()?;
            Some((
                "Open sessie".into(),
                RunSpec::OpenUrl(format!("{}/{}", base.trim_end_matches('/'), kid)),
            ))
        }
        SessionActionKind::Focus => session
            .attach
            .focus
            .clone()
            .map(|focus| ("Neem over".into(), RunSpec::FocusAgent(focus))),
        SessionActionKind::Workspace => session
            .attach
            .workspace_url
            .clone()
            .map(|url| ("Open workspace".into(), RunSpec::OpenUrl(url))),
        SessionActionKind::Browser => session
            .attach
            .browser
            .clone()
            .map(|url| ("Open browser".into(), RunSpec::OpenUrl(url))),
        SessionActionKind::Evidence => session
            .attach
            .evidence_url
            .clone()
            .map(|url| ("Bekijk evidence".into(), RunSpec::OpenUrl(url))),
    }
}

// ---------------------------------------------------------------------------
// Executor — één plek die RunSpec uitvoert (altijd in een achtergrond-thread
// behalve UI-only zaken zoals CopyText/OpenUrl).
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Executor {
    pub vault: Client,
    pub ops: Client,
    pub profile: EndpointProfile,
    /// Laatste bekende vault-revision (expectedRevision bij accountswitch).
    pub revision: std::sync::Arc<std::sync::atomic::AtomicI64>,
}

impl Executor {
    pub fn run(&self, spec: &RunSpec, query: &str) {
        match spec {
            RunSpec::Noop => {}
            RunSpec::CopyText(_text) => {
                // GTK-clipboard pad wordt in de UI afgehandeld (panel.rs); hier
                // alleen toast — zonder inhoud (privacy, niks in notificaties).
                crate::notify::notify("Gekopieerd", "Tekst staat op het klembord.", "ok");
            }
            RunSpec::OpenUrl(url) => crate::notify::open_url(url),
            RunSpec::Refresh => self.request_refresh(),
            RunSpec::OpenOcx => {
                let url = self.profile.opencodex_dashboard.clone().unwrap_or_else(|| {
                    format!(
                        "{}/#opencodex",
                        self.profile.dashboard.trim_end_matches('/')
                    )
                });
                crate::notify::open_url(&url);
            }
            RunSpec::FocusAgent(terminal_id) => {
                let target = terminal_id.clone();
                let ops = self.ops.clone();
                self.spawn_bg(move || {
                    let _ = ops_focus(&ops, &target);
                });
            }
            RunSpec::SendPrompt {
                terminal_id,
                pane_id,
            } => {
                let text = query.to_string();
                let terminal = terminal_id.clone();
                let pane = pane_id.clone();
                self.spawn_bg(move || {
                    let ok = crate::ops_cli::send_prompt(&terminal, pane.as_deref(), &text);
                    if ok {
                        crate::notify::notify("Opdracht verstuurd", &text, "ok");
                    } else {
                        crate::notify::notify("Sturen lukte niet", "zie chefbar.log", "error");
                    }
                });
            }
            RunSpec::CreateTask { cwd } => {
                let prompt = query.to_string();
                let cwd = cwd.clone();
                let vault = self.vault.clone();
                self.spawn_bg(move || {
                    let body = json!({"prompt": prompt, "agentType": "cursor", "cwd": cwd});
                    match vault.post_json("/commander/tasks", &body) {
                        Ok(result) => {
                            let tid = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            crate::notify::notify("Agent aan de slag", tid, "ok");
                        }
                        Err(_) => crate::notify::notify(
                            "Taak starten lukte niet",
                            "zie chefbar.log",
                            "error",
                        ),
                    }
                });
            }
            RunSpec::SwitchAccount {
                account_id,
                source,
                driver,
            } => {
                let account_id = account_id.clone();
                let source = source.clone();
                let driver = driver.clone();
                let revision = self.revision.load(std::sync::atomic::Ordering::Relaxed);
                let vault = self.vault.clone();
                self.spawn_bg(move || {
                    let mut body = json!({
                        "source": source,
                        "accountId": account_id,
                        "expectedRevision": revision,
                    });
                    if let Some(driver) = driver {
                        body["driver"] = json!(driver);
                    }
                    let headers = vec![(
                        "Idempotency-Key".to_string(),
                        format!(
                            "chefbar-{}-{}",
                            std::process::id(),
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_nanos())
                                .unwrap_or(0)
                        ),
                    )];
                    match vault.post_json_headers("/coding/accounts/switch", &body, &headers) {
                        Ok(_) => crate::notify::notify("Account gewisseld", "", "ok"),
                        Err(_) => crate::notify::notify("Wisselen lukte niet", "", "error"),
                    }
                });
            }
            RunSpec::CancelTask(task_id) => {
                let path = format!("/commander/tasks/{}/cancel", urlencoding(task_id));
                let vault = self.vault.clone();
                self.spawn_bg(move || match vault.post_json(&path, &json!({})) {
                    Ok(_) => crate::notify::notify("Taak gestopt", "", "ok"),
                    Err(_) => crate::notify::notify("Stoppen lukte niet", "", "error"),
                });
            }
            RunSpec::ClipboardAdd => {
                let text = query.to_string();
                let vault = self.vault.clone();
                self.spawn_bg(move || {
                    match vault.post_json("/clipboard", &json!({"text": text})) {
                        Ok(_) => crate::notify::notify("Toegevoegd aan clipboard", "", "ok"),
                        Err(_) => crate::notify::notify("Toevoegen lukte niet", "", "error"),
                    }
                });
            }
            RunSpec::ClipboardDelete(row) => {
                let row = *row;
                let vault = self.vault.clone();
                self.spawn_bg(
                    move || match vault.delete_json(&format!("/clipboard/{row}")) {
                        Ok(_) => crate::notify::notify("Clipboard-rij verwijderd", "", "ok"),
                        Err(_) => crate::notify::notify("Verwijderen lukte niet", "", "error"),
                    },
                );
            }
            RunSpec::DesktopAction(verb) => {
                let verb = verb.clone();
                let vault = self.vault.clone();
                self.spawn_bg(move || {
                    match vault.post_json(&format!("/desktop/{verb}"), &json!({})) {
                        Ok(_) => crate::notify::notify(
                            if verb == "start" {
                                "Desktop gestart"
                            } else {
                                "Desktop gestopt"
                            },
                            "",
                            "ok",
                        ),
                        Err(_) => crate::notify::notify("Desktop-actie lukte niet", "", "error"),
                    }
                });
            }
            RunSpec::ShareSync(kind) => {
                let kind = kind.clone();
                let vault = self.vault.clone();
                self.spawn_bg(move || {
                    // Live check: nooit pull/push tegen een sync in foutstatus.
                    let blocked = vault
                        .get_json("/share-sync/status")
                        .ok()
                        .map(|status| {
                            status.get("error").is_some()
                                || matches!(
                                    status.get("status").and_then(|v| v.as_str()),
                                    Some("error") | Some("blocked")
                                )
                        })
                        .unwrap_or(false);
                    if blocked {
                        crate::notify::notify(
                            "Sync hapert",
                            "Los de sync-fout op voordat je bestanden ophaalt of deelt.",
                            "error",
                        );
                        return;
                    }
                    match vault.post_json(&format!("/share-sync/{kind}"), &json!({})) {
                        Ok(_) => {
                            crate::notify::notify("Gedeelde bestanden gesynchroniseerd", "", "ok")
                        }
                        Err(_) => crate::notify::notify("Sync lukte niet", "", "error"),
                    }
                });
            }
        }
    }

    /// UI-knop-variant (geen query).
    pub fn run_for_ui(&self, spec: &RunSpec) {
        self.run(spec, "");
    }

    fn spawn_bg<F: FnOnce() + Send + 'static>(&self, f: F) {
        std::thread::spawn(f);
    }

    fn request_refresh(&self) {
        if let Some(tx) = crate::state::REFRESH_TX.lock().unwrap().as_ref() {
            let _ = tx.send(crate::state::ActorCommand::RefreshNow);
        }
    }
}

fn ops_focus(ops: &Client, target: &str) -> bool {
    crate::ops_cli::ops_focus(ops, target)
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EndpointProfile;
    use crate::models::OpsSnapshot;

    fn catalogus_met(snap: &Snapshot) -> Vec<Action> {
        build_actions(
            &OpsSnapshot::default(),
            snap,
            &EndpointProfile::default(),
            Vec::new(),
        )
    }

    #[test]
    fn sync_acties_beschikbaar_als_gezond() {
        let snap = Snapshot::default();
        let actions = catalogus_met(&snap);
        let sync_runs: Vec<&RunSpec> = actions
            .iter()
            .map(|a| &a.run)
            .filter(|r| matches!(r, RunSpec::ShareSync(_)))
            .collect();
        assert_eq!(sync_runs.len(), 2);
    }

    #[test]
    fn sync_acties_geblokkeerd_bij_error_status() {
        let mut snap = Snapshot::default();
        snap.share_sync
            .insert("status".into(), serde_json::Value::String("error".into()));
        let actions = catalogus_met(&snap);
        assert!(actions
            .iter()
            .all(|a| !matches!(a.run, RunSpec::ShareSync(_))));
        let uitleg = actions
            .iter()
            .find(|a| a.title == "Sync hapert")
            .expect("uitleg-actie aanwezig");
        assert_eq!(uitleg.run, RunSpec::Noop);
        assert_eq!(uitleg.stamp, "FOUT");
    }

    #[test]
    fn sync_blocked_detecteert_error_key_en_blocked_status() {
        let mut snap = Snapshot::default();
        snap.share_sync
            .insert("error".into(), serde_json::Value::String("disk vol".into()));
        assert!(sync_blocked(&snap));

        let mut snap = Snapshot::default();
        snap.share_sync
            .insert("status".into(), serde_json::Value::String("blocked".into()));
        assert!(sync_blocked(&snap));

        let mut snap = Snapshot::default();
        snap.share_sync
            .insert("status".into(), serde_json::Value::String("ok".into()));
        assert!(!sync_blocked(&snap));
    }
}
