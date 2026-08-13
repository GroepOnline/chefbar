//! Control-chat: directe operator-praat voor devops en overzicht.
//!
//! Besluit 2026-08-12: **Pi is de default-harnas**. jcode is geheugen
//! (context in de prompt), nooit een chat-doel. Andere live Herdr-agents
//! (hermes/grok/claude, en cursor alleen met de hand) zijn kiesbaar; de app
//! start geen nieuwe kinds. Geen ACP, geen tweede poll-actor.

use crate::models::{HerdrAgent, OpsSnapshot, Snapshot};
use crate::ops_cli;
use std::time::{SystemTime, UNIX_EPOCH};

const CONTROL_NAME_HINTS: &[&str] = &["control", "devops", "sysadmin", "fleet-ops"];
/// Auto-pick order. Cursor zit er bewust niet in — dat is vaak de dirigent.
const AUTO_KINDS: &[&str] = &["pi", "hermes", "grok", "claude"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    Operator,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub text: String,
    pub at_unix: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ChatLog {
    pub messages: Vec<ChatMessage>,
    pub target: Option<String>,
    pub kind: Option<String>,
    pub busy: bool,
    /// Handmatig vastgezet via de harnas-kiezer; auto-resolve blijft daarna weg.
    pub pinned: bool,
}

impl ChatLog {
    pub fn target_label(&self) -> String {
        match (&self.kind, &self.target) {
            (Some(kind), Some(id)) => format!("{kind} · {id}"),
            (None, Some(id)) => id.clone(),
            _ => "geen Pi".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTarget {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub label: String,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn env_target() -> Option<String> {
    for key in ["CHEFBAR_CONTROL_AGENT", "CHEFBAR_CONTROL_PANE"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn agent_id(agent: &HerdrAgent) -> String {
    if !agent.pane_id.is_empty() {
        agent.pane_id.clone()
    } else {
        agent.terminal_id.clone()
    }
}

fn kind_of(agent: &HerdrAgent) -> String {
    let raw = agent.name.trim().to_lowercase();
    if raw.is_empty() {
        "agent".into()
    } else {
        raw
    }
}

fn is_jcode(agent: &HerdrAgent) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        agent.name, agent.workspace, agent.cwd, agent.alias, agent.pane_id
    )
    .to_lowercase();
    hay.contains("jcode")
}

fn looks_like_control(agent: &HerdrAgent) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        agent.name, agent.workspace, agent.pane_id, agent.cwd, agent.alias
    )
    .to_lowercase();
    CONTROL_NAME_HINTS.iter().any(|hint| hay.contains(hint))
}

/// Alleen de WORKENDE visual-lane is gereserveerd: een checkout zonder
/// "worktree" in de cwd én status working. Een idle Pi op die cwd blijft
/// gewoon kiesbaar als control-chat target.
fn is_reserved_product_lane(agent: &HerdrAgent) -> bool {
    let cwd = agent.cwd.to_lowercase();
    let title = agent.workspace.to_lowercase();
    let working = agent.status.eq_ignore_ascii_case("working");
    if cwd.contains("/cheffactory/chefbar") && !cwd.contains("worktree") {
        return working;
    }
    if title.contains("chefbar") && working {
        return true;
    }
    false
}

fn is_auto_kind(kind: &str) -> bool {
    AUTO_KINDS.contains(&kind)
}

/// Picker: Pi/Hermes/Grok/Claude, plus Cursor als die niet de visual-lane is.
/// jcode nooit.
pub fn is_picker_eligible(agent: &HerdrAgent) -> bool {
    if is_jcode(agent) || is_reserved_product_lane(agent) {
        return false;
    }
    let kind = kind_of(agent);
    is_auto_kind(&kind) || kind == "cursor"
}

fn auto_score(agent: &HerdrAgent) -> (u8, u8, u8) {
    let kind = kind_of(agent);
    let kind_rank = match kind.as_str() {
        "pi" => 0,
        "hermes" => 1,
        "grok" | "claude" => 2,
        _ => 9,
    };
    let idle = match agent.status.as_str() {
        "idle" | "done" | "klaar" => 0,
        "blocked" => 1,
        _ => 2,
    };
    let hint =
        if looks_like_control(agent) || agent.alias.trim().eq_ignore_ascii_case("chefapp-herdr") {
            0
        } else {
            1
        };
    (kind_rank, idle, hint)
}

pub fn list_targets(ops: &OpsSnapshot) -> Vec<ChatTarget> {
    let mut out: Vec<ChatTarget> = ops
        .agents
        .iter()
        .filter(|a| is_picker_eligible(a))
        .map(|a| {
            let id = agent_id(a);
            let kind = kind_of(a);
            ChatTarget {
                label: format!("{kind} · {id}"),
                id,
                kind,
                status: a.status.clone(),
            }
        })
        .filter(|t| !t.id.is_empty())
        .collect();
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

/// Kies een Herdr-doel. Pinned > env > beste live Pi (dan Hermes).
/// Nooit jcode, nooit stiekem de werkende visual ChefApp-lane, nooit auto-Cursor.
pub fn resolve_target(ops: &OpsSnapshot, pinned: Option<&str>) -> Option<String> {
    if let Some(pin) = pinned.map(str::trim).filter(|s| !s.is_empty()) {
        if ops
            .agents
            .iter()
            .any(|a| agent_id(a) == pin && is_picker_eligible(a))
            || env_target().as_deref() == Some(pin)
        {
            return Some(pin.to_string());
        }
        if ops.agents.iter().any(|a| agent_id(a) == pin) {
            return Some(pin.to_string());
        }
    }
    if let Some(env) = env_target() {
        return Some(env);
    }
    let mut auto: Vec<&HerdrAgent> = ops
        .agents
        .iter()
        .filter(|a| is_picker_eligible(a) && is_auto_kind(&kind_of(a)))
        .collect();
    auto.sort_by_key(|a| auto_score(a));
    auto.first()
        .map(|a| agent_id(a))
        .filter(|id| !id.is_empty())
}

pub fn kind_for(ops: &OpsSnapshot, target: &str) -> Option<String> {
    ops.agents
        .iter()
        .find(|a| agent_id(a) == target)
        .map(kind_of)
}

pub fn pin_target(shared: &crate::state::Shared, id: &str) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    let ops = shared.ops.read().unwrap().clone();
    let kind = kind_for(&ops, id);
    let mut log = shared.chat.write().unwrap();
    if log.busy {
        return;
    }
    log.target = Some(id.to_string());
    log.kind = kind;
    log.pinned = true;
    drop(log);
    let persist_id = id.to_string();
    std::thread::spawn(move || {
        if !crate::panel_state::persist_control_pin(Some(&persist_id), true) {
            crate::log::log("control-pin opslaan lukte niet");
        }
    });
    shared
        .chat_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Hydrateer ChatLog vanuit panel-state: pin overleeft een herstart.
/// Target zonder actieve pin wordt niet vastgezet.
pub fn chat_log_from_panel(panel: &crate::panel_state::PanelState) -> ChatLog {
    let mut log = ChatLog::default();
    if panel.control_pinned {
        if let Some(target) = panel
            .control_target
            .as_deref()
            .filter(|t| !t.trim().is_empty())
        {
            log.target = Some(target.to_string());
            log.pinned = true;
        }
    }
    log
}

/// Korte control-context, geen secrets, geen dumps.
pub fn wrap_prompt(snap: &Snapshot, text: &str, kind: &str) -> String {
    let vault = if crate::state::vault_online() {
        "online"
    } else {
        "offline"
    };
    let jcode = if snap.jcode_memory.online {
        "online"
    } else {
        "offline"
    };
    format!(
        "ChefApp control ({kind}). Overzicht: vault {vault}, jcode memory {jcode} (geheugen, geen chat), fleet {}/{}. Antwoord kort in het Nederlands, geen secrets. Vraag: {}",
        snap.fleet.online,
        snap.fleet.total,
        text.trim()
    )
}

pub fn parse_read_output(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let result = value.get("result").unwrap_or(&value);
            for key in ["text", "output", "content", "body"] {
                if let Some(text) = result.get(key).and_then(|v| v.as_str()) {
                    return text.trim().to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

/// Nieuw terminalstuk na een prompt: suffix van `after` t.o.v. `before`.
pub fn terminal_delta(before: &str, after: &str) -> String {
    if let Some(stripped) = after.strip_prefix(before) {
        return stripped.trim().to_string();
    }
    if let Some(idx) = after.find(before) {
        return after[idx + before.len()..].trim().to_string();
    }
    after.trim().to_string()
}

fn send_and_read(target: &str, prompt: &str) -> Result<String, String> {
    let before = ops_cli::read_agent(target).unwrap_or_default();
    if !ops_cli::send_control_prompt(target, prompt) {
        return Err("Sturen naar de control-agent lukte niet.".into());
    }
    let after = ops_cli::read_agent(target).unwrap_or_default();
    let delta = terminal_delta(&parse_read_output(&before), &parse_read_output(&after));
    if delta.is_empty() {
        return Err("Geen antwoord gelezen. De agent draait misschien nog.".into());
    }
    Ok(delta)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitStatus {
    Empty,
    Busy,
    NoTarget,
    Sent,
}

/// Zet de vraag op het transcript en stuur op een achtergrond-thread.
pub fn submit(shared: &crate::state::Shared, text: &str) -> SubmitStatus {
    let text = text.trim();
    if text.is_empty() {
        return SubmitStatus::Empty;
    }
    let ops = shared.ops.read().unwrap().clone();
    let snap = shared.snapshot.read().unwrap().clone();
    let pinned = {
        let log = shared.chat.read().unwrap();
        if log.pinned {
            log.target.clone()
        } else {
            None
        }
    };
    let target = resolve_target(&ops, pinned.as_deref());
    let kind = target
        .as_deref()
        .and_then(|id| kind_for(&ops, id))
        .unwrap_or_else(|| "pi".into());
    {
        let mut log = shared.chat.write().unwrap();
        if log.busy {
            return SubmitStatus::Busy;
        }
        log.busy = true;
        log.target = target.clone();
        log.kind = Some(kind.clone());
        log.messages.push(ChatMessage {
            role: ChatRole::Operator,
            text: text.to_string(),
            at_unix: now_unix(),
        });
        if target.is_none() {
            log.busy = false;
            log.messages.push(ChatMessage {
                role: ChatRole::System,
                text: "Geen Pi. Zet CHEFBAR_CONTROL_AGENT of start een Herdr-pane met Pi voor control.".into(),
                at_unix: now_unix(),
            });
            shared
                .chat_revision
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return SubmitStatus::NoTarget;
        }
    }
    shared
        .chat_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let shared = shared.clone();
    let prompt = wrap_prompt(&snap, text, &kind);
    std::thread::spawn(move || {
        let result = match shared.chat.read().unwrap().target.clone() {
            Some(target) => send_and_read(&target, &prompt),
            None => Err("Geen control-agent.".into()),
        };
        let mut log = shared.chat.write().unwrap();
        log.busy = false;
        match result {
            Ok(reply) => log.messages.push(ChatMessage {
                role: ChatRole::Agent,
                text: reply,
                at_unix: now_unix(),
            }),
            Err(err) => log.messages.push(ChatMessage {
                role: ChatRole::System,
                text: err,
                at_unix: now_unix(),
            }),
        }
        drop(log);
        shared
            .chat_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    SubmitStatus::Sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HerdrAgent;

    fn agent(name: &str, pane: &str, status: &str, cwd: &str) -> HerdrAgent {
        HerdrAgent {
            name: name.into(),
            pane_id: pane.into(),
            terminal_id: format!("term-{pane}"),
            status: status.into(),
            cwd: cwd.into(),
            workspace: name.into(),
            ..Default::default()
        }
    }

    fn agent_met_alias(alias: &str, pane: &str, cwd: &str) -> HerdrAgent {
        HerdrAgent {
            alias: alias.into(),
            ..agent("pi", pane, "idle", cwd)
        }
    }

    #[test]
    fn resolve_prefers_control_hint_over_visual_lane() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w2M:p1", "working", "/home/joep/ChefFactory/chefbar"),
                agent(
                    "pi",
                    "w2R:p2",
                    "idle",
                    "/home/joep/.herdr/worktrees/chefbar/control",
                ),
            ],
        };
        // tweede pane-cwd bevat "control"
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2R:p2"));
    }

    #[test]
    fn werkende_visual_lane_gereserveerd_idle_pi_blijft_kiesbaar() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        // working visual-lane: niet kiesbaar
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent(
                "pi",
                "w2M:p1",
                "working",
                "/home/joep/ChefFactory/chefbar",
            )],
        };
        assert_eq!(resolve_target(&ops, None), None);
        // zelfde cwd, maar idle: wél kiesbaar als control-target
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent(
                "pi",
                "w2M:p1",
                "idle",
                "/home/joep/ChefFactory/chefbar",
            )],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2M:p1"));
    }

    #[test]
    fn alias_chefapp_herdr_wint_van_andere_idle_pi() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w2S:p2", "idle", "/tmp/ops-lane"),
                agent_met_alias(
                    "chefapp-herdr",
                    "w2R:p2",
                    "/home/joep/.herdr/worktrees/chefbar/worktree-pi-bind",
                ),
            ],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2R:p2"));
    }

    #[test]
    fn resolve_prefers_idle_pi_over_cursor() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("cursor", "w2R:p1", "idle", "/home/joep/ChefFactory"),
                agent("pi", "w2S:p2", "idle", "/tmp/control-ops"),
            ],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2S:p2"));
        let listed: Vec<String> = list_targets(&ops).into_iter().map(|t| t.kind).collect();
        assert!(listed.contains(&"pi".into()));
        assert!(listed.contains(&"cursor".into()));
    }

    #[test]
    fn jcode_is_never_a_chat_target() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent(
                "pi",
                "w9:p1",
                "idle",
                "/var/lib/chef-jcode-memory/home",
            )],
        };
        assert!(list_targets(&ops).is_empty());
        assert_eq!(resolve_target(&ops, None), None);

        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent_met_alias("jcode", "w9:p2", "/tmp/ops-lane")],
        };
        assert!(list_targets(&ops).is_empty());
        assert_eq!(resolve_target(&ops, None), None);
    }

    #[test]
    fn pinned_target_wins_over_auto_pi() {
        std::env::remove_var("CHEFBAR_CONTROL_AGENT");
        std::env::remove_var("CHEFBAR_CONTROL_PANE");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w2S:p2", "idle", "/tmp/a"),
                agent("hermes", "w2S:p3", "idle", "/tmp/b"),
            ],
        };
        assert_eq!(
            resolve_target(&ops, Some("w2S:p3")).as_deref(),
            Some("w2S:p3")
        );
    }

    #[test]
    fn wrap_prompt_is_dutch_and_has_no_secret_shape() {
        let snap = Snapshot::default();
        let wrapped = wrap_prompt(&snap, "status jan", "pi");
        assert!(wrapped.contains("Vraag: status jan"));
        assert!(wrapped.contains("geen secrets"));
        assert!(wrapped.contains("geheugen, geen chat"));
        assert!(wrapped.contains("(pi)"));
        assert!(!wrapped.contains("ghp_"));
        assert!(!wrapped.contains("Bearer "));
    }

    #[test]
    fn parse_read_output_json_and_raw() {
        let json = r#"{"id":"cli:agent:read","result":{"text":"fleet ok"}}"#;
        assert_eq!(parse_read_output(json), "fleet ok");
        assert_eq!(parse_read_output("  raw line  "), "raw line");
    }

    #[test]
    fn terminal_delta_takes_suffix() {
        assert_eq!(terminal_delta("abc", "abcdef"), "def");
        assert_eq!(terminal_delta("nope", "hello"), "hello");
    }

    #[test]
    fn submit_reports_empty_busy_and_no_target() {
        let shared = crate::state::Shared::new();
        assert_eq!(submit(&shared, "  "), SubmitStatus::Empty);
        assert_eq!(submit(&shared, "status jan"), SubmitStatus::NoTarget);
        shared.chat.write().unwrap().busy = true;
        assert_eq!(submit(&shared, "nog een"), SubmitStatus::Busy);
    }

    #[test]
    fn pin_overleeft_save_en_load_via_panel_state() {
        let path = std::env::temp_dir()
            .join(format!("chefbar-control-pin-{}", std::process::id()))
            .join("panel-state.json");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2R:p2"),
            true
        ));
        let panel = crate::panel_state::load_from(&path);
        let log = chat_log_from_panel(&panel);
        assert_eq!(log.target.as_deref(), Some("w2R:p2"));
        assert!(log.pinned);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn hydratie_zonder_pin_geeft_ongepinde_log() {
        let panel = crate::panel_state::PanelState::default();
        let log = chat_log_from_panel(&panel);
        assert_eq!(log.target, None);
        assert!(!log.pinned);
        // target zonder actieve pin wordt niet vastgezet
        let panel = crate::panel_state::PanelState {
            control_target: Some("w2S:p2".into()),
            ..Default::default()
        };
        let log = chat_log_from_panel(&panel);
        assert_eq!(log.target, None);
        assert!(!log.pinned);
    }
}
