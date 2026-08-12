//! joep-ops / herdr CLI-seams voor acties (focus, prompt sturen).
//!
//! Eerst joep-ops REST, dan herdr CLI als fallback — dezelfde volgorde als de
//! Python-implementatie, maar met één gedocumenteerde contract-per-actie.

use crate::http::{ApiError, Client};
use serde_json::json;
use std::process::Command;

fn run_herdr(args: &[&str]) -> bool {
    let output = Command::new("herdr").args(args).output().ok();
    match output {
        Some(output) => output.status.success(),
        None => false,
    }
}

/// Focus een herdr agent/terminal; eerst joep-ops, dan CLI.
pub fn ops_focus(ops_client: &Client, target: &str) -> bool {
    match ops_client.post_json("/api/focus", &json!({"target": target})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&["agent", "focus", target])
        }
        Err(ApiError::Blocked(_)) | Err(ApiError::Http(_, _)) | Err(ApiError::Transport(_)) => {
            run_herdr(&["agent", "focus", target])
        }
    }
}

/// Typ een prompt in de TUI van een lopende agent en verstuur met Enter.
pub fn send_prompt(terminal_id: &str, pane_id: Option<&str>, text: &str) -> bool {
    let id = if terminal_id.is_empty() {
        pane_id.unwrap_or("")
    } else {
        terminal_id
    };
    if id.is_empty() {
        return false;
    }
    if !run_herdr(&["agent", "send", id, text]) {
        return false;
    }
    if let Some(pane) = pane_id {
        let _ = run_herdr(&["pane", "send-keys", pane, "Enter"]);
    }
    true
}

/// Fleet deploy — eerst via ops API, dan CLI fallback.
pub fn fleet_deploy(ops_client: &Client, node: &str) -> bool {
    match ops_client.post_json("/api/fleet/deploy", &json!({"node": node})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&["fleet", "deploy", node])
        }
        Err(_) => run_herdr(&["fleet", "deploy", node]),
    }
}

/// Fleet exec — template-commando op node.
pub fn fleet_exec(ops_client: &Client, node: &str, template: &str) -> bool {
    match ops_client.post_json(
        "/api/fleet/exec",
        &json!({"node": node, "template": template}),
    ) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&["fleet", "exec", node, template])
        }
        Err(_) => run_herdr(&["fleet", "exec", node, template]),
    }
}

/// Prune preview — via vault of ops; geeft Ok(summary) of Err.
pub fn prune_preview(vault_client: &Client) -> Result<String, String> {
    match vault_client.get_json("/containers/prune-preview") {
        Ok(val) => Ok(val
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("prune preview klaar")
            .to_string()),
        Err(e) => Err(e.to_string()),
    }
}
