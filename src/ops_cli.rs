//! joep-ops / herdr CLI-seams voor acties (focus, prompt sturen).
//!
//! Eerst joep-ops REST, dan live `herdr` 0.8 CLI. Nooit verzonnen
//! subcommands (`agent send`, `fleet deploy`) — die bestaan niet.

use crate::http::{ApiError, Client};
use serde_json::json;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// `herdr agent read` is a snapshot, not `--wait`. Bound it so a hung binary
/// cannot leave Control-chat `busy` forever.
const HERDR_READ_TIMEOUT: Duration = Duration::from_secs(8);
/// herdr `--timeout 45000` plus slack if the child ignores it.
const HERDR_PROMPT_WAIT_TIMEOUT: Duration = Duration::from_secs(50);

/// Herdr pane/agent ids from env or panel-state must not be parsed as CLI flags.
pub fn valid_herdr_target(target: &str) -> bool {
    let target = target.trim();
    if target.is_empty() || target.starts_with('-') {
        return false;
    }
    target
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
}

fn herdr_target_or_none(target: &str) -> Option<&str> {
    let target = target.trim();
    valid_herdr_target(target).then_some(target)
}

fn run_herdr(args: &[String]) -> bool {
    let output = Command::new("herdr").args(args).output().ok();
    match output {
        Some(output) => output.status.success(),
        None => false,
    }
}

/// Resultaat van `herdr agent prompt --wait`. Onderscheidt "binary startte niet"
/// van "herdr draaide, wait eindigde zonder success" — anders wordt een
/// al-geaccepteerde devops-prompt een tweede keer ingestuurd.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaitPrompt {
    Success,
    RanUnsuccessfully,
    SpawnFailed,
}

fn classify_herdr_wait(result: Result<std::process::Output, std::io::Error>) -> WaitPrompt {
    match result {
        Ok(out) if out.status.success() => WaitPrompt::Success,
        Ok(_) => WaitPrompt::RanUnsuccessfully,
        Err(_) => WaitPrompt::SpawnFailed,
    }
}

fn allow_prompt_fallback(wait: WaitPrompt) -> bool {
    match wait {
        WaitPrompt::SpawnFailed => true,
        WaitPrompt::Success | WaitPrompt::RanUnsuccessfully => false,
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

fn command_output_bounded(
    program: &str,
    args: &[String],
    deadline: Duration,
) -> Result<std::process::Output, std::io::Error> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    bounded_wait_output(child, deadline)
}

fn bounded_wait_output(
    mut child: std::process::Child,
    deadline: Duration,
) -> Result<std::process::Output, std::io::Error> {
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None if started.elapsed() >= deadline => {
                let _ = child.kill();
                break child.wait()?;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    Ok(std::process::Output {
        status,
        stdout: stdout_h.join().unwrap_or_default(),
        stderr: stderr_h.join().unwrap_or_default(),
    })
}

fn run_herdr_output(args: &[String]) -> Option<String> {
    let output = command_output_bounded("herdr", args, HERDR_READ_TIMEOUT).ok()?;
    if output.stdout.is_empty() && !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Read-only snapshot of an agent pane. Never used by the poll actor.
pub fn read_agent(target: &str) -> Option<String> {
    let target = herdr_target_or_none(target)?;
    run_herdr_output(&herdr_read_args(target))
}

/// Control-chat send: `herdr agent prompt --wait`. Alleen zonder herdr
/// (spawn-fout) vallen we terug op een kale prompt; een timeout na acceptatie
/// mag dezelfde side-effecting vraag niet opnieuw insturen.
pub fn send_control_prompt(target: &str, text: &str) -> bool {
    let Some(target) = herdr_target_or_none(target) else {
        return false;
    };
    if text.trim().is_empty() {
        return false;
    }
    let wait = classify_herdr_wait(command_output_bounded(
        "herdr",
        &herdr_prompt_wait_args(target, text),
        HERDR_PROMPT_WAIT_TIMEOUT,
    ));
    if matches!(wait, WaitPrompt::Success) {
        return true;
    }
    if allow_prompt_fallback(wait) {
        return send_prompt(target, None, text);
    }
    false
}

/// Fleet-health plugin scan for one node. Untargeted scan is not a success.
pub fn herdr_scan_node_args(node: &str) -> Vec<String> {
    vec![
        "plugin".into(),
        "action".into(),
        "invoke".into(),
        "--plugin".into(),
        "com.chefgroep.fleet-health".into(),
        "scan-node".into(),
        "--arg".into(),
        format!("node={node}"),
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
    let Some(target) = herdr_target_or_none(target) else {
        return false;
    };
    match ops_client.post_json("/api/focus", &json!({"target": target})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&herdr_focus_args(target))
        }
        Err(ApiError::Blocked(_))
        | Err(ApiError::Http(_, _))
        | Err(ApiError::Transport(_))
        | Err(ApiError::Decode(_)) => run_herdr(&herdr_focus_args(target)),
    }
}

/// Typ een prompt in de TUI van een lopende agent en verstuur met Enter.
pub fn send_prompt(terminal_id: &str, pane_id: Option<&str>, text: &str) -> bool {
    let id = if terminal_id.is_empty() {
        pane_id.unwrap_or("")
    } else {
        terminal_id
    };
    let Some(id) = herdr_target_or_none(id) else {
        return false;
    };
    if !run_herdr(&herdr_prompt_args(id, text)) {
        return false;
    }
    if let Some(pane) = pane_id {
        let _ = run_herdr(&herdr_enter_args(pane));
    }
    true
}

/// Fleet deploy — ops API, then a node-targeted fleet-health scan.
pub fn fleet_deploy(ops_client: &Client, node: &str) -> bool {
    if node.trim().is_empty() {
        return false;
    }
    match ops_client.post_json("/api/fleet/deploy", &json!({"node": node})) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            run_herdr(&herdr_scan_node_args(node))
        }
        Err(_) => run_herdr(&herdr_scan_node_args(node)),
    }
}

/// Fleet exec — template via ops API; herdr fallback is a node-targeted plugin scan.
pub fn fleet_exec(ops_client: &Client, node: &str, template: &str) -> bool {
    if node.trim().is_empty() {
        return false;
    }
    match ops_client.post_json(
        "/api/fleet/exec",
        &json!({"node": node, "template": template}),
    ) {
        Ok(payload) => {
            if payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            if fleet_template_is_scan(template) {
                return run_herdr(&herdr_scan_node_args(node));
            }
            false
        }
        Err(_) => {
            if fleet_template_is_scan(template) {
                run_herdr(&herdr_scan_node_args(node))
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
    fn valid_herdr_target_rejects_flags_and_empty() {
        assert!(valid_herdr_target("w2M:p1"));
        assert!(valid_herdr_target("chefapp-herdr"));
        assert!(!valid_herdr_target(""));
        assert!(!valid_herdr_target("   "));
        assert!(!valid_herdr_target("--wait"));
        assert!(!valid_herdr_target("-p1"));
        assert!(!valid_herdr_target("pane id"));
    }

    #[test]
    fn read_agent_rejects_flag_like_target() {
        assert!(read_agent("--wait").is_none());
    }

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
        let node = herdr_scan_node_args("sofie");
        let fleet = herdr_scan_fleet_args();
        assert_eq!(node[0], "plugin");
        assert!(node.contains(&"com.chefgroep.fleet-health".into()));
        assert!(node.contains(&"scan-node".into()));
        assert!(node.contains(&"node=sofie".into()));
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

    #[test]
    fn wait_timeout_does_not_resend_prompt() {
        assert!(!allow_prompt_fallback(WaitPrompt::Success));
        assert!(!allow_prompt_fallback(WaitPrompt::RanUnsuccessfully));
        assert!(allow_prompt_fallback(WaitPrompt::SpawnFailed));
    }

    #[test]
    fn classify_wait_treats_spawn_error_as_fallback() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "herdr");
        assert_eq!(classify_herdr_wait(Err(err)), WaitPrompt::SpawnFailed);
    }

    #[test]
    fn bounded_wait_kills_a_hung_child() {
        let started = Instant::now();
        let output = command_output_bounded("sleep", &["2".into()], Duration::from_millis(80))
            .expect("sleep");
        assert!(!output.status.success());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn bounded_wait_keeps_stdout_from_a_fast_child() {
        let output =
            command_output_bounded("echo", &["control-read".into()], Duration::from_secs(2))
                .expect("echo");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("control-read"));
    }
}
