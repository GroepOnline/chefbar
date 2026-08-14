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
    /// Agent-kind at send time. Historical replies keep this label when the
    /// live target later switches.
    pub kind: Option<String>,
}

impl ChatMessage {
    pub fn who_label(&self) -> &str {
        match self.role {
            ChatRole::Operator => "jij",
            ChatRole::Agent => self.kind.as_deref().unwrap_or("agent"),
            ChatRole::System => "app",
        }
    }
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

fn read_ops(shared: &crate::state::Shared) -> OpsSnapshot {
    shared
        .ops
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn read_snapshot(shared: &crate::state::Shared) -> Snapshot {
    shared
        .snapshot
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn with_chat_read<T, F>(shared: &crate::state::Shared, f: F) -> T
where
    F: FnOnce(&ChatLog) -> T,
{
    let log = shared
        .chat
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&log)
}

fn with_chat_write<T, F>(shared: &crate::state::Shared, f: F) -> T
where
    F: FnOnce(&mut ChatLog) -> T,
{
    let mut log = shared
        .chat
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut log)
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
    // Idle/done first, then kind (Pi before Hermes), then control-hint.
    (idle, kind_rank, hint)
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

fn live_eligible(ops: &OpsSnapshot, id: &str) -> bool {
    ops.agents
        .iter()
        .any(|a| agent_id(a) == id && is_picker_eligible(a))
}

fn configured_id_allowed(ops: &OpsSnapshot, id: &str) -> bool {
    if live_eligible(ops, id) {
        return true;
    }
    // Absent from this snapshot: honor the operator-configured id.
    // Live but ineligible (jcode / reserved working lane): never.
    !ops.agents.iter().any(|a| agent_id(a) == id)
}

/// Kies een Herdr-doel. Pinned > env > beste live Pi (dan Hermes).
/// Nooit jcode, nooit stiekem de werkende visual ChefApp-lane, nooit auto-Cursor.
pub fn resolve_target(ops: &OpsSnapshot, pinned: Option<&str>) -> Option<String> {
    if let Some(pin) = pinned.map(str::trim).filter(|s| !s.is_empty()) {
        if live_eligible(ops, pin) {
            return Some(pin.to_string());
        }
    }
    if let Some(env) = env_target() {
        if configured_id_allowed(ops, &env) {
            return Some(env);
        }
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
    let ops = read_ops(shared);
    let kind = kind_for(&ops, id);
    let alias = ops
        .agents
        .iter()
        .find(|a| agent_id(a) == id)
        .map(|a| a.alias.trim().to_string())
        .filter(|a| !a.is_empty());
    if with_chat_write(shared, |log| {
        if log.busy {
            return true;
        }
        log.target = Some(id.to_string());
        log.kind = kind;
        log.pinned = true;
        false
    }) {
        return;
    }
    let persist_id = id.to_string();
    std::thread::spawn(move || {
        if !crate::panel_state::persist_control_pin(Some(&persist_id), true, alias.as_deref()) {
            crate::log::log("control-pin opslaan lukte niet");
        }
    });
    shared
        .chat_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

fn persist_unpin_bg() {
    std::thread::spawn(|| {
        if !crate::panel_state::persist_control_pin(None, false, None) {
            crate::log::log("control-pin droppen lukte niet");
        }
    });
}

/// Na elke ops-poll: remap een gepinde pane-id via de alias (of het oude
/// pane-id) naar de live agent. Weg/jcode/gereserveerde working visual-lane
/// → unpin (persist) en daarna auto-pick. Geen pin → niets te doen.
pub fn refresh_persisted_pin(shared: &crate::state::Shared) {
    refresh_persisted_pin_at(shared, &crate::panel_state::state_path());
}

/// Pad-expliciete kern van `refresh_persisted_pin` — testbaar zonder env.
pub fn refresh_persisted_pin_at(shared: &crate::state::Shared, state_path: &std::path::Path) {
    let ops = read_ops(shared);
    let (target, busy, pinned) = with_chat_read(shared, |log| {
        (log.target.clone().unwrap_or_default(), log.busy, log.pinned)
    });
    if !pinned || busy {
        return;
    }
    if ops.agents.is_empty() {
        return;
    }
    let alias = crate::panel_state::load_from(state_path)
        .control_alias
        .unwrap_or_default();
    let matched = ops.agents.iter().find(|a| {
        (!alias.is_empty() && a.alias.eq_ignore_ascii_case(&alias)) || agent_id(a) == target
    });
    match matched {
        Some(agent) if is_picker_eligible(agent) => {
            let id = agent_id(agent);
            let kind = kind_of(agent);
            let alias_keep = if agent.alias.trim().is_empty() {
                None
            } else {
                Some(agent.alias.trim().to_string())
            };
            let changed = with_chat_write(shared, |log| {
                if log.busy {
                    return false;
                }
                let stored_alias = (!alias.is_empty()).then_some(alias.as_str());
                let changed = log.target.as_deref() != Some(id.as_str())
                    || log.kind.as_deref() != Some(kind.as_str())
                    || alias_keep.as_deref() != stored_alias;
                log.target = Some(id.clone());
                log.kind = Some(kind.clone());
                log.pinned = true;
                changed
            });
            if changed {
                let _ = crate::panel_state::persist_control_pin_to(
                    state_path,
                    Some(&id),
                    true,
                    alias_keep.as_deref(),
                );
                shared
                    .chat_revision
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        Some(_) => drop_ineligible_pin(shared, state_path, &ops),
        None => {}
    }
}

fn drop_ineligible_pin(
    shared: &crate::state::Shared,
    state_path: &std::path::Path,
    ops: &OpsSnapshot,
) {
    if with_chat_write(shared, |log| {
        if log.busy {
            return true;
        }
        log.target = None;
        log.kind = None;
        log.pinned = false;
        false
    }) {
        return;
    }
    let _ = crate::panel_state::persist_control_pin_to(state_path, None, false, None);
    if let Some(auto) = resolve_target(ops, None) {
        let kind = kind_for(ops, &auto);
        with_chat_write(shared, |log| {
            if !log.busy {
                log.target = Some(auto);
                log.kind = kind;
            }
        });
    }
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
/// Geen overlap (buffer geroteerd) → leeg, nooit de hele recente dump.
pub fn terminal_delta(before: &str, after: &str) -> String {
    if before.is_empty() {
        return String::new();
    }
    if let Some(stripped) = after.strip_prefix(before) {
        return stripped.trim().to_string();
    }
    if let Some(idx) = after.find(before) {
        return after[idx + before.len()..].trim().to_string();
    }
    String::new()
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
    let ops = read_ops(shared);
    let snap = read_snapshot(shared);
    let pinned = with_chat_read(
        shared,
        |log| {
            if log.pinned {
                log.target.clone()
            } else {
                None
            }
        },
    );
    let target = resolve_target(&ops, pinned.as_deref());
    let pin_kept = pinned.is_some() && target.as_deref() == pinned.as_deref();
    let kind = target
        .as_deref()
        .and_then(|id| kind_for(&ops, id))
        .unwrap_or_else(|| "pi".into());
    let early = with_chat_write(shared, |log| {
        if log.busy {
            return Some(SubmitStatus::Busy);
        }
        log.busy = true;
        log.target = target.clone();
        log.kind = Some(kind.clone());
        if pinned.is_some() && !pin_kept {
            log.pinned = false;
        }
        log.messages.push(ChatMessage {
            role: ChatRole::Operator,
            text: text.to_string(),
            at_unix: now_unix(),
            kind: None,
        });
        if target.is_none() {
            log.busy = false;
            log.messages.push(ChatMessage {
                role: ChatRole::System,
                text: "Geen Pi. Zet CHEFBAR_CONTROL_AGENT of start een Herdr-pane met Pi voor control.".into(),
                at_unix: now_unix(),
                kind: None,
            });
            None
        } else {
            Some(SubmitStatus::Sent)
        }
    });
    if let Some(SubmitStatus::Busy) = early {
        return SubmitStatus::Busy;
    }
    let Some(dispatch_target) = target else {
        shared
            .chat_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if pinned.is_some() && !pin_kept {
            persist_unpin_bg();
        }
        return SubmitStatus::NoTarget;
    };
    if pinned.is_some() && !pin_kept {
        persist_unpin_bg();
    }
    shared
        .chat_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let shared = shared.clone();
    let prompt = wrap_prompt(&snap, text, &kind);
    let reply_kind = kind;
    std::thread::spawn(move || {
        let result = send_and_read(&dispatch_target, &prompt);
        with_chat_write(&shared, |log| {
            log.busy = false;
            match result {
                Ok(reply) => log.messages.push(ChatMessage {
                    role: ChatRole::Agent,
                    text: reply,
                    at_unix: now_unix(),
                    kind: Some(reply_kind),
                }),
                Err(err) => log.messages.push(ChatMessage {
                    role: ChatRole::System,
                    text: err,
                    at_unix: now_unix(),
                    kind: None,
                }),
            }
        });
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
    use crate::test_env::EnvGuard;

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
        let _g = EnvGuard::acquire();
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
        let _g = EnvGuard::acquire();
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
        let _g = EnvGuard::acquire();
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
        let _g = EnvGuard::acquire();
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
        let _g = EnvGuard::acquire();
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
        let _g = EnvGuard::acquire();
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
        assert_eq!(terminal_delta("nope", "hello"), "");
        assert_eq!(terminal_delta("", "stale buffer"), "");
        assert_eq!(terminal_delta("abc", "abc"), "");
    }

    #[test]
    fn submit_reports_empty_busy_and_no_target() {
        let _g = EnvGuard::acquire();
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
            true,
            Some("chefapp-herdr")
        ));
        let panel = crate::panel_state::load_from(&path);
        let log = chat_log_from_panel(&panel);
        assert_eq!(log.target.as_deref(), Some("w2R:p2"));
        assert!(log.pinned);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    fn shared_met_pin(path: &std::path::Path) -> crate::state::Shared {
        let shared = crate::state::Shared::new();
        let panel = crate::panel_state::load_from(path);
        *shared.chat.write().unwrap() = chat_log_from_panel(&panel);
        shared
    }

    fn temp_state_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("chefbar-pin-resolve-{tag}-{}", std::process::id()))
            .join("panel-state.json")
    }

    fn set_ops(shared: &crate::state::Shared, ops: OpsSnapshot) {
        *shared.ops.write().unwrap() = ops;
    }

    #[test]
    fn alias_remap_naar_nieuw_pane_id() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("alias-remap");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2R:p2"),
            true,
            Some("chefapp-herdr")
        ));
        let shared = shared_met_pin(&path); // hydrateert pin uit panel-state
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![agent_met_alias(
                    "chefapp-herdr",
                    "w2R:p3",
                    "/home/joep/.herdr/worktrees/chefbar/worktree-pi-bind",
                )],
            },
        );
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert_eq!(log.target.as_deref(), Some("w2R:p3"));
        assert_eq!(log.kind.as_deref(), Some("pi"));
        assert!(log.pinned);
        let persisted = crate::panel_state::load_from(&path);
        assert_eq!(persisted.control_target.as_deref(), Some("w2R:p3"));
        assert_eq!(persisted.control_alias.as_deref(), Some("chefapp-herdr"));
        assert!(persisted.control_pinned);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn stale_pane_zonder_alias_behoudt_pin() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("stale-no-alias");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2Q:p9"),
            true,
            None
        ));
        let shared = shared_met_pin(&path);
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![agent("pi", "w2S:p2", "idle", "/tmp/ops-lane")],
            },
        );
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert!(log.pinned);
        assert_eq!(log.target.as_deref(), Some("w2Q:p9"));
        let persisted = crate::panel_state::load_from(&path);
        assert!(persisted.control_pinned);
        assert_eq!(persisted.control_target.as_deref(), Some("w2Q:p9"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn lege_ops_snapshot_behoudt_pin() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("empty-ops");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2R:p2"),
            true,
            Some("chefapp-herdr")
        ));
        let shared = shared_met_pin(&path);
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![],
            },
        );
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert!(log.pinned);
        assert_eq!(log.target.as_deref(), Some("w2R:p2"));
        let persisted = crate::panel_state::load_from(&path);
        assert!(persisted.control_pinned);
        assert_eq!(persisted.control_target.as_deref(), Some("w2R:p2"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn jcode_pin_van_disk_wordt_gedropt() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("jcode-pin");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w9:p1"),
            true,
            None
        ));
        let shared = shared_met_pin(&path);
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![agent(
                    "pi",
                    "w9:p1",
                    "idle",
                    "/var/lib/chef-jcode-memory/home",
                )],
            },
        );
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert!(!log.pinned);
        assert_eq!(log.target, None);
        let persisted = crate::panel_state::load_from(&path);
        assert!(!persisted.control_pinned);
        assert_eq!(persisted.control_target, None);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn working_visual_pin_wordt_gedropt() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("visual-pin");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2M:p1"),
            true,
            None
        ));
        let shared = shared_met_pin(&path);
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![
                    agent("pi", "w2M:p1", "working", "/home/joep/ChefFactory/chefbar"),
                    agent_met_alias(
                        "chefapp-herdr",
                        "w2R:p2",
                        "/home/joep/.herdr/worktrees/chefbar/worktree-pi-bind",
                    ),
                ],
            },
        );
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert!(!log.pinned);
        assert_eq!(log.target.as_deref(), Some("w2R:p2"));
        let persisted = crate::panel_state::load_from(&path);
        assert!(!persisted.control_pinned);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn ineligible_pin_valt_door_naar_auto() {
        let _g = EnvGuard::acquire();
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w9:p1", "idle", "/var/lib/chef-jcode-memory/home"),
                agent("pi", "w2S:p2", "idle", "/tmp/ops-lane"),
            ],
        };
        // jcode-pin wint niet; doorvallen naar auto-pick
        assert_eq!(
            resolve_target(&ops, Some("w9:p1")).as_deref(),
            Some("w2S:p2")
        );
        // bestaande pin weg → auto
        assert_eq!(
            resolve_target(&ops, Some("w9:p9")).as_deref(),
            Some("w2S:p2")
        );
    }

    #[test]
    fn refresh_laat_busy_target_met_rust() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("busy-refresh");
        assert!(crate::panel_state::persist_control_pin_to(
            &path,
            Some("w2R:p2"),
            true,
            Some("chefapp-herdr")
        ));
        let shared = shared_met_pin(&path);
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![agent_met_alias(
                    "chefapp-herdr",
                    "w2R:p3",
                    "/home/joep/.herdr/worktrees/chefbar/worktree-pi-bind",
                )],
            },
        );
        shared.chat.write().unwrap().busy = true;
        refresh_persisted_pin_at(&shared, &path);
        let log = shared.chat.read().unwrap().clone();
        assert_eq!(log.target.as_deref(), Some("w2R:p2"));
        assert!(log.pinned);
        assert!(log.busy);
        let persisted = crate::panel_state::load_from(&path);
        assert_eq!(persisted.control_target.as_deref(), Some("w2R:p2"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn submit_stale_pin_unpint_voor_auto_pick() {
        let _g = EnvGuard::acquire();
        let path = temp_state_path("submit-stale");
        std::env::set_var("CHEFBAR_PANEL_STATE", &path);
        let shared = crate::state::Shared::new();
        {
            let mut log = shared.chat.write().unwrap();
            log.target = Some("w2Q:p9".into());
            log.pinned = true;
        }
        set_ops(
            &shared,
            OpsSnapshot {
                ok: true,
                agents: vec![agent("pi", "w2S:p2", "idle", "/tmp/ops-lane")],
            },
        );
        assert_eq!(submit(&shared, "status jan"), SubmitStatus::Sent);
        let log = shared.chat.read().unwrap().clone();
        assert!(!log.pinned);
        assert_eq!(log.target.as_deref(), Some("w2S:p2"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn idle_hermes_beats_working_pi() {
        let _g = EnvGuard::acquire();
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w2S:p2", "working", "/tmp/pi-busy"),
                agent("hermes", "w2S:p3", "idle", "/tmp/hermes-idle"),
            ],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2S:p3"));
    }

    #[test]
    fn env_jcode_is_skipped_for_auto() {
        let _g = EnvGuard::acquire();
        std::env::set_var("CHEFBAR_CONTROL_AGENT", "w9:p1");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![
                agent("pi", "w9:p1", "idle", "/var/lib/chef-jcode-memory/home"),
                agent("pi", "w2S:p2", "idle", "/tmp/ops-lane"),
            ],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w2S:p2"));
    }

    #[test]
    fn env_unknown_id_is_honored() {
        let _g = EnvGuard::acquire();
        std::env::set_var("CHEFBAR_CONTROL_AGENT", "w8:p8");
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent("pi", "w2S:p2", "idle", "/tmp/ops-lane")],
        };
        assert_eq!(resolve_target(&ops, None).as_deref(), Some("w8:p8"));
    }

    #[test]
    fn agent_who_label_stays_on_the_message() {
        let msg = ChatMessage {
            role: ChatRole::Agent,
            text: "fleet ok".into(),
            at_unix: 1,
            kind: Some("pi".into()),
        };
        assert_eq!(msg.who_label(), "pi");
        let sys = ChatMessage {
            role: ChatRole::System,
            text: "fout".into(),
            at_unix: 1,
            kind: Some("hermes".into()),
        };
        assert_eq!(sys.who_label(), "app");
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
