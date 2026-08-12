//! Control-chat: directe operator-praat voor devops en overzicht.
//!
//! Geen ACP, geen tweede poll-actor. Versturen loopt via `herdr agent prompt`;
//! het antwoord komt uit `herdr agent read`. De UI houdt een eigen transcript
//! bij op `Shared.chat`. Leeg target → system-regel, geen error-spam.

use crate::models::{OpsSnapshot, Snapshot};
use crate::ops_cli;
use std::time::{SystemTime, UNIX_EPOCH};

const CONTROL_NAME_HINTS: &[&str] = &["control", "devops", "sysadmin", "fleet-ops"];

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
    pub busy: bool,
}

impl ChatLog {
    pub fn target_label(&self) -> String {
        self.target
            .clone()
            .unwrap_or_else(|| "geen control-agent".into())
    }
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

fn looks_like_control(agent: &crate::models::HerdrAgent) -> bool {
    let hay = format!(
        "{} {} {} {}",
        agent.name, agent.workspace, agent.pane_id, agent.cwd
    )
    .to_lowercase();
    CONTROL_NAME_HINTS.iter().any(|hint| hay.contains(hint))
}

fn is_reserved_product_lane(agent: &crate::models::HerdrAgent) -> bool {
    let cwd = agent.cwd.to_lowercase();
    let title = agent.workspace.to_lowercase();
    if cwd.contains("/cheffactory/chefbar") && !cwd.contains("worktree") {
        return true;
    }
    if title.contains("chefbar") && agent.status == "working" {
        return true;
    }
    false
}

/// Kies een Herdr-doel. Env wint. Anders een pane/naam met control-hint.
/// Nooit stiekem de visual ChefApp-lane.
pub fn resolve_target(ops: &OpsSnapshot) -> Option<String> {
    if let Some(env) = env_target() {
        return Some(env);
    }
    let mut hinted: Vec<&crate::models::HerdrAgent> = ops
        .agents
        .iter()
        .filter(|a| looks_like_control(a) && !is_reserved_product_lane(a))
        .collect();
    hinted.sort_by_key(|a| match a.status.as_str() {
        "idle" | "done" | "klaar" => 0,
        "blocked" => 1,
        _ => 2,
    });
    hinted
        .first()
        .map(|a| {
            if !a.pane_id.is_empty() {
                a.pane_id.clone()
            } else {
                a.terminal_id.clone()
            }
        })
        .filter(|id| !id.is_empty())
}

/// Korte control-context, geen secrets, geen dumps.
pub fn wrap_prompt(snap: &Snapshot, text: &str) -> String {
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
        "ChefApp control. Overzicht: vault {vault}, jcode memory {jcode}, fleet {}/{}. Antwoord kort in het Nederlands, geen secrets. Vraag: {}",
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

/// Zet de vraag op het transcript en stuur op een achtergrond-thread.
pub fn submit(shared: &crate::state::Shared, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let ops = shared.ops.read().unwrap().clone();
    let snap = shared.snapshot.read().unwrap().clone();
    let target = resolve_target(&ops);
    {
        let mut log = shared.chat.write().unwrap();
        if log.busy {
            return;
        }
        log.busy = true;
        log.target = target.clone();
        log.messages.push(ChatMessage {
            role: ChatRole::Operator,
            text: text.to_string(),
            at_unix: now_unix(),
        });
        if target.is_none() {
            log.busy = false;
            log.messages.push(ChatMessage {
                role: ChatRole::System,
                text: "Geen control-agent. Zet CHEFBAR_CONTROL_AGENT of start een Herdr-pane voor control.".into(),
                at_unix: now_unix(),
            });
            shared
                .chat_revision
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    }
    shared
        .chat_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let shared = shared.clone();
    let prompt = wrap_prompt(&snap, text);
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
        assert_eq!(resolve_target(&ops).as_deref(), Some("w2R:p2"));
    }

    #[test]
    fn resolve_skips_visual_chefbar_checkout() {
        let ops = OpsSnapshot {
            ok: true,
            agents: vec![agent(
                "pi",
                "w2M:p1",
                "idle",
                "/home/joep/ChefFactory/chefbar",
            )],
        };
        assert_eq!(resolve_target(&ops), None);
    }

    #[test]
    fn wrap_prompt_is_dutch_and_has_no_secret_shape() {
        let snap = Snapshot::default();
        let wrapped = wrap_prompt(&snap, "status jan");
        assert!(wrapped.contains("Vraag: status jan"));
        assert!(wrapped.contains("geen secrets"));
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
}
