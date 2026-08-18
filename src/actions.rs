//! Declaratieve action-registry + één executor.
//!
//! Acties zijn data (RunSpec), gebouwd uit de laatste snapshot — geen closures
//! die UI-state vangen. Executie loopt via één Executor met policy-clients.

use crate::config::EndpointProfile;
use crate::http::Client;
use crate::models::{OpsSnapshot, Snapshot};
use crate::palette::Action;
use serde_json::json;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSpec {
    Noop,
    OpenUrl(String),
    /// Open brain-chunk doel: url eerst, anders lokaal pad.
    BrainOpen(String),
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
    // Lane B — nieuwe domein-varianten
    OpenLinearIssue(String),
    CopySecretMeta {
        id: String,
    },
    FleetDeploy {
        node: String,
    },
    FleetExec {
        node: String,
        template: String,
    },
    PrunePreview,
    FocusDomain(String),
    TogglePalette,
    ToggleMute(String),
    SendControlChat,
}

impl RunSpec {
    /// Stable suffix for local frecency keys. Clipboard payloads stay out of
    /// `~/.local/share/chefbar/frecency.json`; secret ids stay as ids only.
    pub fn frecency_key(&self) -> String {
        match self {
            RunSpec::Noop => "Noop".into(),
            RunSpec::OpenUrl(_) => "OpenUrl".into(),
            RunSpec::OpenOcx => "OpenOcx".into(),
            RunSpec::FocusAgent(id) => format!("FocusAgent:{id}"),
            RunSpec::SendPrompt {
                terminal_id,
                pane_id,
            } => format!(
                "SendPrompt:{terminal_id}:{}",
                pane_id.as_deref().unwrap_or("")
            ),
            RunSpec::CreateTask { .. } => "CreateTask".into(),
            RunSpec::SwitchAccount {
                account_id,
                source,
                driver,
            } => format!(
                "SwitchAccount:{account_id}:{source}:{}",
                driver.as_deref().unwrap_or("")
            ),
            RunSpec::CancelTask(id) => format!("CancelTask:{id}"),
            RunSpec::ClipboardAdd => "ClipboardAdd".into(),
            RunSpec::ClipboardDelete(index) => format!("ClipboardDelete:{index}"),
            RunSpec::CopyText(_) => "CopyText".into(),
            RunSpec::DesktopAction(verb) => format!("DesktopAction:{verb}"),
            RunSpec::ShareSync(id) => format!("ShareSync:{id}"),
            RunSpec::Refresh => "Refresh".into(),
            RunSpec::OpenLinearIssue(id) => format!("OpenLinearIssue:{id}"),
            RunSpec::CopySecretMeta { id } => format!("CopySecretMeta:{id}"),
            RunSpec::FleetDeploy { node } => format!("FleetDeploy:{node}"),
            RunSpec::FleetExec { node, template } => {
                format!("FleetExec:{node}:{template}")
            }
            RunSpec::PrunePreview => "PrunePreview".into(),
            RunSpec::FocusDomain(domain) => format!("FocusDomain:{domain}"),
            RunSpec::TogglePalette => "TogglePalette".into(),
            RunSpec::ToggleMute(key) => format!("ToggleMute:{key}"),
            RunSpec::BrainOpen(target) => format!("BrainOpen:{target}"),
            RunSpec::SendControlChat => "SendControlChat".into(),
        }
    }
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

// ---------------------------------------------------------------------------
// Per-domein builders — pure functies, geen I/O
// ---------------------------------------------------------------------------

/// Inbox: blocked/hulp/down items — unified D1.
/// Tolerant: werkt ook als snapshot leeg is (0 items).
pub fn build_inbox_actions(snap: &Snapshot, _profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    for suggestion in snap.suggestions.iter().take(6) {
        let kind_label = match suggestion.kind {
            crate::models::SuggestionKind::FocusAgent(_) => "focus",
            crate::models::SuggestionKind::OpenDashboard => "dashboard",
            crate::models::SuggestionKind::None_ => "melding",
        };
        out.push(action(
            suggestion.title.clone(),
            suggestion.meta.clone(),
            suggestion.stamp.clone(),
            format!(
                "inbox melding attention {} {}",
                suggestion.title, kind_label
            ),
            match &suggestion.kind {
                crate::models::SuggestionKind::FocusAgent(id) => RunSpec::FocusAgent(id.clone()),
                crate::models::SuggestionKind::OpenDashboard => {
                    RunSpec::FocusDomain("control".into())
                }
                crate::models::SuggestionKind::None_ => RunSpec::Noop,
            },
        ));
    }
    // Als er geen suggesties zijn maar health down → toch een inbox-actie
    if out.is_empty() && snap.health.level == "down" && snap.health.total > 0 {
        out.push(action(
            format!("Health · {} down", snap.health.down),
            snap.health.line(),
            "FOUT",
            "inbox health fout down melding",
            RunSpec::FocusDomain("health".into()),
        ));
    }
    out
}

/// Fleet: nodes + herdr agents — D2.
/// Tolerant: toont 0 als snapshot leeg is.
pub fn build_fleet_actions(
    snap: &Snapshot,
    ops: &OpsSnapshot,
    _profile: &EndpointProfile,
) -> Vec<Action> {
    let mut out = Vec::new();
    // Bestaand: fleet-info als basis
    if snap.fleet.total > 0 {
        let label = snap
            .fleet
            .host
            .clone()
            .unwrap_or_else(|| "fleet".to_string());
        out.push(action(
            format!("Fleet · {}/{} online", snap.fleet.online, snap.fleet.total),
            label.clone(),
            if snap.fleet.stale { "FOUT" } else { "STIL" },
            "fleet herdr nodes status",
            RunSpec::FleetDeploy { node: label },
        ));
    }
    // Per agent een deploy/exec hint (read-only in 4.0 — template exec)
    for agent in ops.agents.iter().take(8) {
        if agent.name.trim().is_empty() && agent.workspace.trim().is_empty() {
            continue;
        }
        let node = agent.workspace.clone();
        out.push(action(
            format!("Deploy naar {} · {}", agent.name, node),
            agent.cwd.clone(),
            agent_stamp(&agent.status),
            format!("fleet deploy herdr {} {}", agent.name, node),
            RunSpec::FleetDeploy { node: node.clone() },
        ));
        out.push(action(
            format!("Run op {} · {}", agent.name, node),
            "kies template en voer uit",
            agent_stamp(&agent.status),
            format!("fleet exec herdr {} {}", agent.name, node),
            RunSpec::FleetExec {
                node,
                template: "status".into(),
            },
        ));
    }
    for node in snap.fleet_nodes.iter().take(8) {
        if node.id.is_empty() && node.title.is_empty() {
            continue;
        }
        let label = if node.title.is_empty() {
            node.id.clone()
        } else {
            node.title.clone()
        };
        out.push(action(
            format!("Node · {label}"),
            node.host.clone().unwrap_or_else(|| node.meta.clone()),
            if node.online { "BEZIG" } else { "STIL" },
            format!("fleet nodes {} {}", label, node.id),
            RunSpec::FocusDomain("fleet".into()),
        ));
    }
    out
}

/// Vault: accounts/providers/CRM — D3.
pub fn build_vault_actions(
    snap: &Snapshot,
    _ops: &OpsSnapshot,
    _profile: &EndpointProfile,
) -> Vec<Action> {
    let mut out = Vec::new();
    for row in &snap.providers {
        for acc in &row.accounts {
            let acc_id = acc.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if acc_id.is_empty() || Some(acc_id) == row.active_id.as_deref() {
                continue;
            }
            let label = acc
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(acc_id)
                .to_string();
            out.push(action(
                format!("Werk als {label}"),
                format!("{} · account wisselen", row.label),
                "STIL",
                format!("vault account switch wissel {} {}", row.label, label),
                RunSpec::SwitchAccount {
                    account_id: acc_id.to_string(),
                    source: row.source.clone(),
                    driver: row.driver.clone(),
                },
            ));
        }
    }
    for account in snap.vault_accounts.iter().take(8) {
        if account.id.is_empty() && account.title.is_empty() {
            continue;
        }
        out.push(action(
            format!("Account · {}", account.title),
            format!("{} · {}", account.provider, account.meta),
            "STIL",
            format!("vault account {} {}", account.title, account.id),
            RunSpec::FocusDomain("vault".into()),
        ));
    }
    for deal in snap.crm_deals.iter().take(8) {
        if deal.id.is_empty() && deal.title.is_empty() {
            continue;
        }
        let amount = deal.amount.clone().unwrap_or_default();
        out.push(action(
            format!("Deal · {}", deal.title),
            format!("{} {}", deal.status, amount).trim().to_string(),
            "STIL",
            format!("crm deals neon {} {}", deal.title, deal.id),
            RunSpec::FocusDomain("crm".into()),
        ));
    }
    out
}

/// Containers: observed vs desired diff — D4.
pub fn build_container_actions(snap: &Snapshot, _profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    // Generieke prune-preview (read-only diff) — altijd beschikbaar
    out.push(action(
        "Toon container drift",
        "observed vs desired — read-only",
        "STIL",
        "containers docker drift prune preview",
        RunSpec::PrunePreview,
    ));
    // Als snapshot.raw een containers-diff bevat, toon per-item (tolerant)
    if let Some(containers) = snap.raw.get("containers") {
        if let Some(items) = containers.get("observed").and_then(|v| v.as_array()) {
            for item in items.iter().take(6) {
                let Some(name) = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|name| !name.is_empty())
                else {
                    continue;
                };
                let host = item
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                out.push(action(
                    format!("Container · {name}"),
                    format!("{host} · image"),
                    "STIL",
                    format!("containers docker {name} {host}"),
                    RunSpec::CopyText(name.to_string()),
                ));
            }
        }
    }
    for name in snap.containers.drift.iter().take(6) {
        if name.trim().is_empty() {
            continue;
        }
        out.push(action(
            format!("Drift · {name}"),
            "observed vs desired",
            "HULP",
            format!("containers docker drift {name}"),
            RunSpec::CopyText(name.clone()),
        ));
    }
    out
}

/// Secrets: alleen meta, copy via vault-api — D5.
pub fn build_secret_actions(snap: &Snapshot, _profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    for item in snap.secrets_meta.iter().take(8) {
        if item.id.is_empty() {
            continue;
        }
        out.push(action(
            format!("Kopieer secret · {}", item.title),
            "kopieert via vault — zichtbaar in audit-log, auto-clear",
            "STIL",
            format!("secrets vaultwarden wachtwoord {} {}", item.title, item.id),
            RunSpec::CopySecretMeta {
                id: item.id.clone(),
            },
        ));
    }
    let mut seen: HashSet<String> = out
        .iter()
        .filter_map(|a| match &a.run {
            RunSpec::CopySecretMeta { id } => Some(id.clone()),
            _ => None,
        })
        .collect();
    if let Some(secrets) = snap.raw.get("secrets_meta").and_then(|v| v.as_array()) {
        for item in secrets.iter().take(8) {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || !seen.insert(id.to_string()) {
                continue;
            }
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or(id);
            out.push(action(
                format!("Kopieer secret · {title}"),
                "kopieert via vault — zichtbaar in audit-log, auto-clear",
                "STIL",
                format!("secrets vaultwarden wachtwoord {title} {id}"),
                RunSpec::CopySecretMeta { id: id.to_string() },
            ));
        }
    }
    if out.is_empty() {
        out.push(action(
            "Secrets · geen items",
            "vaultwarden — kopieert via vault met audit-log",
            "STIL",
            "secrets vaultwarden wachtwoord",
            RunSpec::Noop,
        ));
    }
    out
}

/// Clipboard: geschiedenis — D6.
pub fn build_clipboard_actions(snap: &Snapshot, _profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
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
        if full.trim().is_empty() {
            continue;
        }
        out.push(action(
            format!("Kopieer · {text}"),
            format!("clipboard-rij {index}"),
            "STIL",
            format!("clipboard klembord kopieer plak {text}"),
            RunSpec::CopyText(full),
        ));
        out.push(destructive_action(
            format!("Verwijder clipboard-rij {index}"),
            text.clone(),
            "HULP",
            format!("clipboard klembord verwijder delete {index}"),
            RunSpec::ClipboardDelete(index),
        ));
    }
    // Altijd ook toevoegen-actie
    out.push(task_action(
        "Voeg toe aan clipboard",
        "typ tekst en kies deze actie",
        "clipboard klembord toevoegen add tekst",
        RunSpec::ClipboardAdd,
    ));
    out
}

/// Linear: assigned-to-me — D7.
pub fn build_linear_actions(snap: &Snapshot, profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    for issue in snap.linear_issues.iter().take(10) {
        if issue.id.is_empty() {
            continue;
        }
        out.push(action(
            format!("Linear · {}", issue.title),
            issue.id.clone(),
            "STIL",
            format!("linear taken issues tickets {} {}", issue.title, issue.id),
            RunSpec::OpenLinearIssue(issue.id.clone()),
        ));
    }
    let mut seen: HashSet<String> = out
        .iter()
        .filter_map(|a| match &a.run {
            RunSpec::OpenLinearIssue(id) => Some(id.clone()),
            _ => None,
        })
        .collect();
    if let Some(issues) = snap.raw.get("linear_issues").and_then(|v| v.as_array()) {
        for issue in issues.iter().take(10) {
            let id = issue.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || !seen.insert(id.to_string()) {
                continue;
            }
            let title = issue.get("title").and_then(|v| v.as_str()).unwrap_or(id);
            out.push(action(
                format!("Linear · {title}"),
                id.to_string(),
                "STIL",
                format!("linear taken issues tickets {title} {id}"),
                RunSpec::OpenLinearIssue(id.to_string()),
            ));
        }
    }
    if out.is_empty() {
        // Fallback: open Linear via dashboard/workaround als er geen issues zijn
        out.push(action(
            "Open Linear",
            "taken · issues — read-only in 4.0",
            "STIL",
            "linear taken issues tickets open",
            RunSpec::OpenUrl(format!(
                "{}/linear",
                profile.dashboard.trim_end_matches('/')
            )),
        ));
    }
    out
}

/// Kater: gateway/profielen — D8.
pub fn build_kater_actions(snap: &Snapshot, profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    if let Some(kater_url) = profile.kater_workspace.clone() {
        out.push(action(
            "Open Kater",
            profile.label("katerWorkspace"),
            "STIL",
            "kater gateway proxy profiel open",
            RunSpec::OpenUrl(kater_url),
        ));
    }
    if !snap.kater_status.status.is_empty() {
        let status = snap.kater_status.status.as_str();
        out.push(action(
            format!("Kater · {status}"),
            snap.kater_status
                .profile
                .clone()
                .unwrap_or_else(|| "gateway status".into()),
            if snap.kater_status.online {
                "STIL"
            } else {
                "FOUT"
            },
            "kater gateway status",
            RunSpec::FocusDomain("kater".into()),
        ));
    } else if let Some(kater) = snap.raw.get("kater_status") {
        let status = kater
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        out.push(action(
            format!("Kater · {status}"),
            "gateway status",
            if status == "ok" { "STIL" } else { "FOUT" },
            "kater gateway status",
            RunSpec::FocusDomain("kater".into()),
        ));
    }
    if out.is_empty() {
        out.push(action(
            "Kater · geen profiel",
            "voeg CHEFBAR_KATER_WORKSPACE toe",
            "STIL",
            "kater gateway",
            RunSpec::Noop,
        ));
    }
    out
}

/// Health: observability + day_score — D8.
pub fn build_health_actions(snap: &Snapshot, _profile: &EndpointProfile) -> Vec<Action> {
    let mut out = Vec::new();
    let health_line = snap.health.line();
    out.push(action(
        health_line.clone(),
        format!(
            "{}/{} ok · {}",
            snap.health.ok, snap.health.total, snap.health.level
        ),
        if snap.health.level == "down" {
            "FOUT"
        } else {
            "STIL"
        },
        "health status eval dagscore doctor",
        RunSpec::FocusDomain("health".into()),
    ));
    let ds_line = snap.day_score.line();
    out.push(action(
        ds_line,
        snap.day_score
            .source
            .clone()
            .unwrap_or_else(|| "dagscore".to_string()),
        if snap.day_score.score.is_some() {
            "BEZIG"
        } else {
            "STIL"
        },
        "health dagscore eval score",
        RunSpec::FocusDomain("health".into()),
    ));
    if !snap.observability.status.is_empty() {
        out.push(action(
            format!("Observability · {}", snap.observability.status),
            snap.observability
                .updated_at
                .clone()
                .unwrap_or_else(|| "samenvatting".into()),
            if snap.observability.ok {
                "STIL"
            } else {
                "FOUT"
            },
            "health observability events catalog",
            RunSpec::FocusDomain("health".into()),
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Brain search — palette-prefix `?` (D10)
// ---------------------------------------------------------------------------

/// `?term` zoekt lexical door de brain-digest; resultaat opent pad/url.
/// Zonder `?`-prefix: lege lijst (de normale catalogus blijft ongewijzigd).
pub fn build_brain_search_actions(snap: &Snapshot, query: &str) -> Vec<Action> {
    if !query.starts_with('?') {
        return Vec::new();
    }
    let needle = query.trim_start_matches('?');
    crate::brain::search(needle, &snap.brain_digest)
        .into_iter()
        .filter_map(|chunk| {
            let target = crate::brain::open_target(chunk);
            if target.is_empty() {
                return None;
            }
            Some(action(
                if chunk.title.is_empty() {
                    target.clone()
                } else {
                    chunk.title.clone()
                },
                chunk.excerpt.clone().unwrap_or_default(),
                "STIL",
                format!("brain digest zoek {}", chunk.title),
                RunSpec::BrainOpen(target),
            ))
        })
        .take(8)
        .collect()
}

// ---------------------------------------------------------------------------
// Hoofd-builder — concateneert alle domeinen (pure functie, geen I/O)
// ---------------------------------------------------------------------------

/// Bouw de catalogus uit de laatste snapshots (pure functie, geen I/O).
/// `mutes` is de gedempte agent-set, buiten meegegeven zodat deze functie
/// geen bestand leest (en deterministisch blijft voor tests/per-keystroke).
pub fn build_actions(
    ops: &OpsSnapshot,
    snap: &Snapshot,
    profile: &EndpointProfile,
    sessions: Vec<crate::sessions::Session>,
    mutes: &HashSet<String>,
) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();
    let home = crate::home_dir();
    let home_str = home.to_string_lossy().to_string();

    // Domein-builders — elk puur, tolerant
    actions.extend(build_inbox_actions(snap, profile));
    actions.extend(build_fleet_actions(snap, ops, profile));
    actions.extend(build_vault_actions(snap, ops, profile));
    actions.extend(build_container_actions(snap, profile));
    actions.extend(build_secret_actions(snap, profile));
    actions.extend(build_clipboard_actions(snap, profile));
    actions.extend(build_linear_actions(snap, profile));
    actions.extend(build_kater_actions(snap, profile));
    actions.extend(build_health_actions(snap, profile));
    // `mutes` wordt buiten meegegeven (eenmaal geladen per render/keystroke),
    // zodat de palette-rij de huidige demp-status toont zonder I/O hierbinnen.
    for agent in &snap.agents {
        let verb = if mutes.contains(&agent.key) {
            "Ontdemp"
        } else {
            "Demp"
        };
        actions.push(action(
            format!("{verb} {} · {}", agent.agent, agent.workspace),
            "tray- en inboxmeldingen aan/uit",
            "STIL",
            format!("demp mute agent {} {}", agent.agent, agent.workspace),
            RunSpec::ToggleMute(agent.key.clone()),
        ));
    }
    if snap.brain.ok || !snap.brain.skills.is_empty() {
        let counts = snap.brain.counts.clone().unwrap_or_default();
        actions.push(action(
            format!("Brain · {} skills · {} evals", counts.skills, counts.evals),
            snap.brain
                .source
                .clone()
                .unwrap_or_else(|| "vault /api/brain".into()),
            if snap.brain.ok { "STIL" } else { "HULP" },
            "brain memory wiki skills eval",
            RunSpec::FocusDomain("health".into()),
        ));
    }
    if !snap.jcode_memory.status.is_empty() {
        actions.push(action(
            format!("jcode memory · {}", snap.jcode_memory.status),
            format!("{} · {}", snap.jcode_memory.host, snap.jcode_memory.bind),
            if snap.jcode_memory.online {
                "BEZIG"
            } else {
                "FOUT"
            },
            "jcode memory session gateway runner",
            RunSpec::FocusDomain("kater".into()),
        ));
    }

    // Bestaand: herdr focus/send
    for agent in &ops.agents {
        if agent.terminal_id.trim().is_empty() {
            continue;
        }
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

    // Bestaand: tasks cancel (aanvullend op build_vault etc.)
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

    // Bestaand: events feed
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
        action(
            "Focus domein…",
            "spring naar Inbox, Fleet, Vault, Linear, …",
            "STIL",
            "focus domein domain inbox fleet vault",
            RunSpec::FocusDomain("inbox".into()),
        ),
        action(
            "Toggle palette",
            "snel zoeken — overlay",
            "STIL",
            "toggle palette zoek overlay",
            RunSpec::TogglePalette,
        ),
        action(
            "Open control-chat",
            "devops en overzicht — directe agent-praat",
            "STIL",
            "control chat devops overzicht agent prompt",
            RunSpec::FocusDomain("control".into()),
        ),
        task_action(
            "Vraag control",
            "typ je vraag en kies deze regel",
            "control chat stuur vraag devops fleet",
            RunSpec::SendControlChat,
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
    /// Zelfde snapshot/pin als het Control-canvas (palette "Vraag control").
    pub shared: crate::state::Shared,
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
            RunSpec::BrainOpen(target) => {
                let target = target.trim();
                if target.starts_with("http://") || target.starts_with("https://") {
                    crate::notify::open_url(target);
                } else if target.starts_with('/') && !target.contains('\0') {
                    let _ = std::process::Command::new("xdg-open")
                        .arg("--")
                        .arg(target)
                        .spawn();
                } else if !target.is_empty() {
                    eprintln!("[warn] BrainOpen geweigerd: {target}");
                }
            }
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
                    let _ = crate::ops_cli::ops_focus(&ops, &target);
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
                // Geen lokale webtop, geen Thuis/Ploeg. ChefBar is de surface.
                // IPC desktop * is a no-op (geen POST, geen error-toast).
                if let Some(url) = resolve_desktop_action(verb, &self.profile.desktop) {
                    crate::notify::open_url(&url);
                }
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
            RunSpec::OpenLinearIssue(issue_id) => {
                // Open Linear issue via dashboard fallback; policy-checked via open_url.
                let url = format!(
                    "{}/linear/{}",
                    self.profile.dashboard.trim_end_matches('/'),
                    urlencoding(issue_id)
                );
                crate::notify::open_url(&url);
            }
            RunSpec::CopySecretMeta { id } => {
                let id = id.clone();
                let vault = self.vault.clone();
                self.spawn_bg(
                    move || match vault.post_json("/secrets/copy", &json!({"id": id})) {
                        Ok(_) => crate::notify::notify(
                            "Secret gekopieerd",
                            "via vault — zichtbaar in audit-log, auto-clear",
                            "ok",
                        ),
                        Err(_) => crate::notify::notify("Kopiëren lukte niet", "", "error"),
                    },
                );
            }
            RunSpec::FleetDeploy { node } => {
                let node = node.clone();
                let ops = self.ops.clone();
                self.spawn_bg(move || {
                    if crate::ops_cli::fleet_deploy(&ops, &node) {
                        crate::notify::notify("Deploy gestart", &node, "ok");
                    } else {
                        crate::notify::notify("Deploy lukte niet", "", "error");
                    }
                });
            }
            RunSpec::FleetExec { node, template } => {
                let node = node.clone();
                let template = template.clone();
                let ops = self.ops.clone();
                self.spawn_bg(move || {
                    if crate::ops_cli::fleet_exec(&ops, &node, &template) {
                        crate::notify::notify(
                            "Fleet exec gestart",
                            &format!("{node}:{template}"),
                            "ok",
                        );
                    } else {
                        crate::notify::notify("Fleet exec lukte niet", "", "error");
                    }
                });
            }
            RunSpec::PrunePreview => {
                let vault = self.vault.clone();
                self.spawn_bg(move || match vault.get_json("/containers/prune-preview") {
                    Ok(val) => {
                        let summary = val
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("prune preview klaar");
                        crate::notify::notify("Prune preview", summary, "ok");
                    }
                    Err(_) => crate::notify::notify("Prune preview lukte niet", "", "error"),
                });
            }
            RunSpec::FocusDomain(domain) => {
                if crate::tray::send_ui(crate::tray::UiCommand::FocusDomain(domain.clone())) {
                    crate::notify::notify("Focus domein", domain, "ok");
                } else {
                    crate::notify::notify("Focus domein", "paneel nog niet klaar", "hulp");
                }
            }
            RunSpec::TogglePalette => {
                if !crate::tray::send_ui(crate::tray::UiCommand::TogglePalette) {
                    crate::notify::notify("Palette", "toggle — Super+Space", "ok");
                }
            }
            RunSpec::ToggleMute(key) => {
                let key = key.clone();
                self.spawn_bg(move || {
                    let (now_muted, ok) = crate::mutes::toggle(&key);
                    if !ok {
                        crate::notify::notify(
                            "Dempen lukte niet",
                            &format!("kon demp-lijst niet opslaan voor {key}"),
                            "error",
                        );
                        return;
                    }
                    crate::state::refresh_global();
                    let state = if now_muted { "gedempt" } else { "ontdempt" };
                    crate::notify::notify("Dempen", &format!("{key} {state}"), "ok");
                });
            }
            RunSpec::SendControlChat => {
                let text = query.trim();
                if text.is_empty() {
                    crate::notify::notify("Control", "typ eerst een vraag", "hulp");
                    return;
                }
                match crate::chat::submit(&self.shared, text) {
                    crate::chat::SubmitStatus::Sent => {
                        crate::notify::notify("Control", "vraag verstuurd", "ok");
                    }
                    crate::chat::SubmitStatus::Busy => {
                        crate::notify::notify("Control", "vorige vraag loopt nog", "hulp");
                    }
                    crate::chat::SubmitStatus::NoTarget => {
                        crate::notify::notify(
                            "Control",
                            "geen Pi — zet CHEFBAR_CONTROL_AGENT of kies een harnas",
                            "hulp",
                        );
                    }
                    crate::chat::SubmitStatus::Empty => {
                        crate::notify::notify("Control", "typ eerst een vraag", "hulp");
                    }
                }
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

/// Geen lokale webtop, geen Thuis/Ploeg-split. IPC desktop start/stop is een no-op.
pub fn resolve_desktop_action(_verb: &str, _desktop_url: &str) -> Option<String> {
    None
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
    use crate::models::{BrainChunk, BrainDigest, OpsSnapshot};

    fn catalogus_met(snap: &Snapshot) -> Vec<Action> {
        build_actions(
            &OpsSnapshot::default(),
            snap,
            &EndpointProfile::default(),
            Vec::new(),
            &HashSet::new(),
        )
    }

    fn snap_met_digest() -> Snapshot {
        Snapshot {
            brain_digest: BrainDigest {
                chunks: vec![
                    BrainChunk {
                        title: "hard constraints".into(),
                        path: Some("/brain/hard.md".into()),
                        ..Default::default()
                    },
                    BrainChunk {
                        title: "compute ssot".into(),
                        url: Some("https://vault.chefgroep.online/brain/compute".into()),
                        excerpt: Some("live compute latch".into()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn brain_search_vereist_vraagteken_prefix() {
        let snap = snap_met_digest();
        assert!(build_brain_search_actions(&snap, "hard").is_empty());
        assert!(build_brain_search_actions(&snap, "?").is_empty());
        assert_eq!(build_brain_search_actions(&snap, "?hard").len(), 1);
    }

    #[test]
    fn brain_search_opent_pad_of_url() {
        let snap = snap_met_digest();
        let actions = build_brain_search_actions(&snap, "?hard");
        assert_eq!(actions[0].run, RunSpec::BrainOpen("/brain/hard.md".into()));
        let actions = build_brain_search_actions(&snap, "?compute");
        assert_eq!(
            actions[0].run,
            RunSpec::BrainOpen("https://vault.chefgroep.online/brain/compute".into())
        );
    }

    #[test]
    fn brain_search_slaat_chunks_zonder_doel_over() {
        let snap = Snapshot {
            brain_digest: BrainDigest {
                chunks: vec![BrainChunk {
                    title: "geen doel".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(build_brain_search_actions(&snap, "?geen").is_empty());
    }

    #[test]
    fn brain_search_doelloze_hits_eten_limiet_niet_op() {
        let mut chunks: Vec<BrainChunk> = (0..8)
            .map(|i| BrainChunk {
                title: format!("hard leeg {i}"),
                ..Default::default()
            })
            .collect();
        chunks.push(BrainChunk {
            title: "hard constraints".into(),
            path: Some("/brain/hard.md".into()),
            ..Default::default()
        });
        let snap = Snapshot {
            brain_digest: BrainDigest {
                chunks,
                ..Default::default()
            },
            ..Default::default()
        };
        let actions = build_brain_search_actions(&snap, "?hard");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].run, RunSpec::BrainOpen("/brain/hard.md".into()));
    }

    #[test]
    fn vraag_control_zit_in_catalogus() {
        let actions = catalogus_met(&Snapshot::default());
        let vraag = actions
            .iter()
            .find(|a| a.title == "Vraag control")
            .expect("palette-actie Vraag control");
        assert!(matches!(vraag.run, RunSpec::SendControlChat));
    }

    #[test]
    fn geen_thuis_ploeg_desktop_split() {
        let actions = catalogus_met(&Snapshot::default());
        assert!(actions.iter().all(|a| {
            !a.title.contains("Thuis")
                && !a.title.contains("Ploeg")
                && a.title != "Open ops"
                && a.title != "Open desktop"
                && a.title != "Start desktop"
                && a.title != "Stop desktop"
        }));
        assert_eq!(
            resolve_desktop_action("start", "https://desktop.chefgroep.online"),
            None
        );
        assert_eq!(
            resolve_desktop_action("stop", "https://desktop.chefgroep.online"),
            None
        );
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

    #[test]
    fn per_domein_builders_zijn_puur_en_deterministisch() {
        let snap = Snapshot::default();
        let ops = OpsSnapshot::default();
        let profile = EndpointProfile::default();
        let a1 = build_inbox_actions(&snap, &profile);
        let a2 = build_inbox_actions(&snap, &profile);
        assert_eq!(a1.len(), a2.len());

        let b1 = build_fleet_actions(&snap, &ops, &profile);
        let b2 = build_fleet_actions(&snap, &ops, &profile);
        assert_eq!(b1.len(), b2.len());

        let c1 = build_clipboard_actions(&snap, &profile);
        let c2 = build_clipboard_actions(&snap, &profile);
        assert_eq!(c1.len(), c2.len());
    }

    #[test]
    fn runspec_determinisme() {
        let a = RunSpec::CopySecretMeta { id: "abc".into() };
        let b = RunSpec::CopySecretMeta { id: "abc".into() };
        let c = RunSpec::CopySecretMeta { id: "xyz".into() };
        assert_eq!(a, b);
        assert_ne!(a, c);

        let d = RunSpec::FleetDeploy {
            node: "sofie".into(),
        };
        let e = RunSpec::FleetDeploy {
            node: "sofie".into(),
        };
        assert_eq!(d, e);

        let f = RunSpec::FleetExec {
            node: "jan".into(),
            template: "status".into(),
        };
        let g = RunSpec::FleetExec {
            node: "jan".into(),
            template: "status".into(),
        };
        assert_eq!(f, g);

        let copy = RunSpec::CopyText("super-secret-token".into());
        assert_eq!(copy.frecency_key(), "CopyText");
        assert!(!copy.frecency_key().contains("super-secret"));
        assert_eq!(
            RunSpec::CopySecretMeta { id: "sec-1".into() }.frecency_key(),
            "CopySecretMeta:sec-1"
        );
    }

    #[test]
    fn secret_copy_is_niet_destructief_maar_met_waarschuwing() {
        let snap = Snapshot::default();
        let profile = EndpointProfile::default();
        // Bouw met placeholder raw secrets_meta
        let mut snap_with = snap.clone();
        snap_with.secrets_meta = vec![crate::models::SecretMeta {
            id: "sec-1".into(),
            title: "API key".into(),
            ..Default::default()
        }];
        snap_with.raw = serde_json::json!({
            "secrets_meta": [{"id": "sec-1", "title": "API key"}]
        });
        let actions = build_secret_actions(&snap_with, &profile);
        let copy = actions
            .iter()
            .find(|a| matches!(a.run, RunSpec::CopySecretMeta { .. }))
            .expect("copy-secret actie aanwezig");
        assert!(!copy.destructive, "secret copy mag niet destructive zijn");
        assert!(
            copy.meta.contains("audit-log"),
            "meta moet waarschuwing bevatten, kreeg: {}",
            copy.meta
        );
    }

    #[test]
    fn focus_domain_en_toggle_palette_bestaan() {
        let snap = Snapshot::default();
        let actions = catalogus_met(&snap);
        assert!(actions
            .iter()
            .any(|a| matches!(a.run, RunSpec::FocusDomain(_))));
        assert!(actions
            .iter()
            .any(|a| matches!(a.run, RunSpec::TogglePalette)));
        assert!(actions
            .iter()
            .any(|a| matches!(a.run, RunSpec::SendControlChat)));
        assert!(actions.iter().any(|a| matches!(
            a.run,
            RunSpec::FocusDomain(ref d) if d == "control"
        )));
        assert!(actions
            .iter()
            .any(|a| a.needs_text && matches!(a.run, RunSpec::SendControlChat)));
    }

    #[test]
    fn prune_preview_bestaat() {
        let snap = Snapshot::default();
        let profile = EndpointProfile::default();
        let actions = build_container_actions(&snap, &profile);
        assert!(actions
            .iter()
            .any(|a| matches!(a.run, RunSpec::PrunePreview)));
    }

    #[test]
    fn open_linear_issue_variant_bestaat() {
        let snap = Snapshot::default();
        let mut snap_with = snap.clone();
        snap_with.raw = serde_json::json!({
            "linear_issues": [{"id": "LIN-123", "title": "Fix bug"}]
        });
        let profile = EndpointProfile::default();
        let actions = build_linear_actions(&snap_with, &profile);
        assert!(actions
            .iter()
            .any(|a| matches!(a.run, RunSpec::OpenLinearIssue(_))));
    }

    #[test]
    fn per_domein_geen_io() {
        // Pure check: builders mogen geen I/O doen — ze mogen niet panicken
        // op lege snapshot en moeten binnen 100 ms klaar zijn.
        let snap = Snapshot::default();
        let ops = OpsSnapshot::default();
        let profile = EndpointProfile::default();
        let start = std::time::Instant::now();
        let _ = build_inbox_actions(&snap, &profile);
        let _ = build_fleet_actions(&snap, &ops, &profile);
        let _ = build_vault_actions(&snap, &ops, &profile);
        let _ = build_container_actions(&snap, &profile);
        let _ = build_secret_actions(&snap, &profile);
        let _ = build_clipboard_actions(&snap, &profile);
        let _ = build_linear_actions(&snap, &profile);
        let _ = build_kater_actions(&snap, &profile);
        let _ = build_health_actions(&snap, &profile);
        assert!(start.elapsed().as_millis() < 100);
    }
}
