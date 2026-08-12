//! joep-ops / herdr CLI-seams voor acties (focus, prompt sturen).
//!
//! Eerst joep-ops REST, dan live `herdr` 0.8 CLI. Nooit verzonnen
//! subcommands (`agent send`, `fleet deploy`) — die bestaan niet.

use crate::http::{ApiError, Client};
use serde_json::json;
use std::process::Command;

fn run_herdr(args: &[String]) -> bool {
    let output = Command::new("herdr").args(args).output().ok();
    match output {
        Some(output) => output.status.success(),
        None => false,
    }
}

pub fn herdr_focus_args(target: &str) -> Vec<String> {
    vec!["agent".into(), "focus".into(), target.into()]
}

pub fn herdr_prompt_args(target: &str, text: &str) -> Vec<String> {
    vec!["agent".into(), "prompt".into(), target.into(), text.into()]
}

pub fn herdr_enter_args(pane: &str) -> Vec<String> {
    vec![
        "pane".into(),
        "send-keys".into(),
        pane.into(),
        "Enter".into(),
    ]
}

pub fn herdr_read_args(target: &str) -> Vec<String> {
    vec![
        "agent".into(),
        "read".into(),
        target.into(),
        "--source".into(),
        "recent".into(),
        "--lines".into(),
        "80".into(),
        "--format".into(),
        "text".into(),
    ]
}

pub fn herdr_prompt_wait_args(target: &str, text: &str) -> Vec<String> {
    vec![
        "agent".into(),
        "prompt".into(),
        target.into(),
        text.into(),
        "--wait".into(),
        "--until".into(),
        "idle".into(),
        "--until".into(),
        "done".into(),
        "--until".into(),
        "blocked".into(),
        "--timeout".into(),
        "45000".into(),
    ]
}

fn run_herdr_output(args: &[String]) -> Option<String> {
    let output = Command::new("herdr").args(args).output().ok()?;
    if output.stdout.is_empty() && !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Read-only snapshot of an agent pane. Never used by the poll actor.
pub fn read_agent(target: &str) -> Option<String> {
    if target.trim().is_empty() {
        return None;
    }
    run_herdr_output(&herdr_read_args(target))
}

/// Control-chat send: prompt + wait for idle/done/blocked, then Enter as fallback.
pub fn send_control_prompt(target: &str, text: &str) -> bool {
    if target.trim().is_empty() || text.trim().is_empty() {
        return false;
    }
    if run_herdr(&herdr_prompt_wait_args(target, text)) {
        return true;
    }
    send_prompt(target, None, text)
}

/// Fleet-health plugin scan. `herdr fleet …` is not a real command.
pub fn herdr_scan_node_args() -> Vec<String> {
    vec![
        "plugin".into(),
        "action".into(),
        "invoke".into(),
        "--plugin".into(),
        "com.chefgroep.fleet-health".into(),
        "scan-node".into(),
    ]
}

pub fn herdr_scan_fleet_args() -> Vec<String> {
    vec![
        "plugin".into(),
        "action".into(),
        "invoke".into(),
        "--plugin".into(),
        "com.chefgroep.fleet-health".into(),
        "scan-fleet".into(),
    ]
}

fn fleet_template_is_scan(template: &str) -> bool {
    let t = template.to_ascii_lowercase();
    t.contains("scan") || t.contains("health") || t.contains("status") || t == "exec"
}

/// Focus een herdr agent/terminal; eerst joep-ops, dan CLI.
pub fn ops_focus(ops_client: &Client, target: &str) -> bool {
    match ops_client.post_json("/api/focus", &json!({"target": target})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&herdr_focus_args(target))
        }
        Err(ApiError::Blocked(_)) | Err(ApiError::Http(_, _)) | Err(ApiError::Transport(_)) => {
            run_herdr(&herdr_focus_args(target))
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
    if !run_herdr(&herdr_prompt_args(id, text)) {
        return false;
    }
    if let Some(pane) = pane_id {
        let _ = run_herdr(&herdr_enter_args(pane));
    }
    true
}

/// Fleet deploy — ops API, then fleet-health scan as the only herdr fallback.
pub fn fleet_deploy(ops_client: &Client, node: &str) -> bool {
    match ops_client.post_json("/api/fleet/deploy", &json!({"node": node})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&herdr_scan_node_args())
        }
        Err(_) => run_herdr(&herdr_scan_node_args()),
    }
}

/// Fleet exec — template via ops API; herdr fallback is plugin scan, not `herdr fleet`.
pub fn fleet_exec(ops_client: &Client, node: &str, template: &str) -> bool {
    match ops_client.post_json(
        "/api/fleet/exec",
        &json!({"node": node, "template": template}),
    ) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            if fleet_template_is_scan(template) {
                return run_herdr(&herdr_scan_node_args());
            }
            false
        }
        Err(_) => {
            if fleet_template_is_scan(template) {
                run_herdr(&herdr_scan_node_args())
            } else {
                false
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_args_use_agent_prompt_not_send() {
        let args = herdr_prompt_args("w2M:p1", "hello");
        assert_eq!(args, ["agent", "prompt", "w2M:p1", "hello"]);
        assert!(!args.iter().any(|a| a == "send"));
    }

    #[test]
    fn focus_args_are_agent_focus() {
        assert_eq!(
            herdr_focus_args("chefapp-herdr"),
            ["agent", "focus", "chefapp-herdr"]
        );
    }

    #[test]
    fn enter_args_send_return_to_pane() {
        assert_eq!(
            herdr_enter_args("w2M:p1"),
            ["pane", "send-keys", "w2M:p1", "Enter"]
        );
    }

    #[test]
    fn fleet_fallback_is_plugin_not_herdr_fleet() {
        let node = herdr_scan_node_args();
        let fleet = herdr_scan_fleet_args();
        assert_eq!(node[0], "plugin");
        assert!(node.contains(&"com.chefgroep.fleet-health".into()));
        assert!(node.contains(&"scan-node".into()));
        assert!(fleet.contains(&"scan-fleet".into()));
        assert!(!node.iter().any(|a| a == "fleet"));
        assert!(!fleet.iter().any(|a| a == "deploy"));
    }

    #[test]
    fn scan_templates_are_recognized() {
        assert!(fleet_template_is_scan("scan-node"));
        assert!(fleet_template_is_scan("health"));
        assert!(fleet_template_is_scan("status"));
        assert!(!fleet_template_is_scan("deploy-prod"));
    }

    #[test]
    fn read_args_are_agent_read_not_poll() {
        let args = herdr_read_args("w2R:p2");
        assert_eq!(args[0], "agent");
        assert_eq!(args[1], "read");
        assert!(args.contains(&"recent".into()));
        assert!(!args.iter().any(|a| a == "watch"));
    }

    #[test]
    fn prompt_wait_stays_on_agent_prompt() {
        let args = herdr_prompt_wait_args("control", "status");
        assert_eq!(args[0], "agent");
        assert_eq!(args[1], "prompt");
        assert!(args.contains(&"--wait".into()));
        assert!(!args.iter().any(|a| a == "send"));
    }
}
