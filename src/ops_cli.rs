//! joep-ops / herdr CLI-seams voor acties (focus, prompt sturen).
//!
//! Eerst joep-ops REST, dan herdr CLI als fallback — dezelfde volgorde als de
//! Python-implementatie, maar met één gedocumenteerde contract-per-actie.

use crate::http::{ApiError, Client};
use serde_json::json;
use std::process::Command;

fn run_herdr(args: &[&str]) -> bool {
    let output = Command::new("herdr")
        .args(args)
        .output()
        .ok();
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
            false
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