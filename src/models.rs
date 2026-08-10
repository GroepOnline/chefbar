//! Getypeerde modellen voor Cheffactory-oppervlakken (vault-API + joep-ops).
//!
//! Parsing blijft tolerant: losse API-payloads worden genormaliseerd naar deze
//! structs; elke misser degradeert naar een lege/neutrale waarde, nooit panic.

use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Health (watchdog-state bestand op ~/.local/share/chefgroep-os)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealthInfo {
    pub ok: usize,
    pub warn: usize,
    pub down: usize,
    pub skip: usize,
    pub total: usize,
    pub level: String, // ok | warn | down
    pub updated_at: Option<String>,
}

impl HealthInfo {
    pub fn line(&self) -> String {
        if self.total == 0 {
            "OS health · onbekend".to_string()
        } else {
            format!("OS health · {}/{} ok", self.ok, self.total)
        }
    }
}

pub fn parse_health(text: &str) -> HealthInfo {
    let Ok(data) = serde_json::from_str::<Value>(text) else {
        return HealthInfo::default();
    };
    let mut info = HealthInfo::default();
    let Some(comps) = data.get("components").and_then(|c| c.as_object()) else {
        return info;
    };
    info.total = comps.len();
    info.updated_at = data
        .get("updated_at")
        .and_then(|v| v.as_str())
        .map(String::from);
    for comp in comps.values() {
        let status = comp
            .get("last_status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        match status.as_str() {
            "ok" | "up" | "healthy" | "running" => info.ok += 1,
            "warn" | "warning" | "degraded" | "flapping" => info.warn += 1,
            "skip" | "skipped" | "disabled" => info.skip += 1,
            _ => info.down += 1,
        }
    }
    if info.down > 0 {
        info.level = "down".into();
    } else if info.warn > 0 {
        info.level = "warn".into();
    } else if info.total > 0 && info.ok + info.skip == info.total {
        info.level = "ok".into();
    } else {
        info.level = if info.total == 0 { "down" } else { "warn" }.into();
    }
    info
}

use std::path::PathBuf;

pub fn watch_dog_path() -> PathBuf {
    match std::env::var("CHEFBAR_WATCHDOG_STATE") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => crate::home_dir().join(".local/share/chefgroep-os/watchdog-state.json"),
    }
}

pub fn load_health_file() -> HealthInfo {
    std::fs::read_to_string(watch_dog_path())
        .ok()
        .map(|text| parse_health(&text))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Dagscore (markdown-eval-reports + JSON-samples + chef-eval agent)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayScore {
    pub letter: Option<String>,
    pub score: Option<i64>,
    pub source: Option<String>,
}

impl DayScore {
    pub fn line(&self) -> String {
        match (&self.letter, self.score) {
            (Some(letter), Some(score)) => format!("Dagscore {letter} ({score}/100)"),
            (None, Some(score)) => format!("Dagscore {score}/100"),
            (Some(letter), None) => format!("Dagscore {letter}"),
            (None, None) => "Dagscore · n.v.t.".to_string(),
        }
    }
}

fn score_regex_letter(text: &str) -> Option<(String, i64)> {
    // "Score: **A+** (87 / 100)" — case-insensitive, flexibel met spaties.
    let lower = text.to_lowercase();
    let marker = "score:";
    let idx = lower.find(marker)?;
    let tail = &lower[idx + marker.len()..];
    let letter_start = tail.find('*')? + 1;
    let after = &tail[letter_start..];
    let letter_end = after.find('*')?;
    let letter = after[..letter_end].trim().to_uppercase();
    if letter.is_empty() || letter.len() > 3 {
        return None;
    }
    let paren = tail.find('(')?;
    let score: i64 = tail[paren + 1..]
        .split(|c: char| !c.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok())?;
    Some((letter, score))
}

pub fn load_day_score_raw(md_texts: Vec<String>, json_texts: Vec<String>) -> DayScore {
    for text in md_texts {
        if let Some((letter, score)) = score_regex_letter(&text) {
            return DayScore {
                letter: Some(letter),
                score: Some(score),
                source: None,
            };
        }
    }
    for text in json_texts {
        if let Ok(data) = serde_json::from_str::<Value>(&text) {
            if let Some(raw) = data.get("score") {
                if let Some(score) = raw.as_i64() {
                    return DayScore {
                        letter: None,
                        score: Some(score),
                        source: None,
                    };
                }
                if let Some(frac) = raw.as_f64() {
                    let score = if frac <= 1.0 { frac * 100.0 } else { frac };
                    return DayScore {
                        letter: None,
                        score: Some(score as i64),
                        source: None,
                    };
                }
            }
        }
    }
    DayScore::default()
}

pub fn eval_dir() -> PathBuf {
    match std::env::var("CHEFBAR_EVAL_DIR") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => crate::home_dir().join(".local/share/chefgroep-os/reports"),
    }
}

pub fn load_day_score_file() -> DayScore {
    let dir = eval_dir();
    if !dir.is_dir() {
        return DayScore::default();
    }
    let mut md_files: Vec<(std::time::SystemTime, String)> = Vec::new();
    let mut json_files: Vec<(std::time::SystemTime, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                        let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                        if name.ends_with(".md") {
                            md_files.push((modified, text));
                        } else if name.ends_with(".json") {
                            json_files.push((modified, text));
                        }
                    }
                }
            }
        }
    }
    md_files.sort_by_key(|(time, _)| *time);
    json_files.sort_by_key(|(time, _)| *time);
    load_day_score_raw(
        md_files.into_iter().rev().map(|(_, text)| text).collect(),
        json_files.into_iter().rev().map(|(_, text)| text).collect(),
    )
}

// ---------------------------------------------------------------------------
// Providers / accounts / agents / fleet
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ProviderRow {
    pub provider: String,
    pub label: String,
    pub active_label: Option<String>,
    pub active_id: Option<String>,
    pub source: String,
    pub driver: Option<String>,
    pub accounts: Vec<Value>,
    pub requests: Option<i64>,
    pub tokens: Option<i64>,
    pub usage_frac: f64,
    pub usage_level: String,
    pub usage_text: String,
    pub available: bool,
}

pub const OCX_REQ_BUDGET: i64 = 500;
pub const OCX_TOK_BUDGET: i64 = 40_000_000;

fn usage_frac(requests: Option<i64>, tokens: Option<i64>) -> (f64, String, String) {
    match (requests, tokens) {
        (None, None) => (0.0, "ok".into(), String::new()),
        _ => {
            let req = requests.unwrap_or(0) as f64;
            let tok = tokens.unwrap_or(0) as f64;
            let frac = (req / OCX_REQ_BUDGET as f64).max(tok / OCX_TOK_BUDGET as f64);
            let frac = frac.clamp(0.0, 1.0);
            let level = if frac >= 0.9 {
                "down"
            } else if frac >= 0.7 {
                "warn"
            } else {
                "ok"
            };
            let text = format!(
                "{} req · {} tok",
                requests.unwrap_or(0),
                tokens.unwrap_or(0)
            );
            (frac, level.into(), text)
        }
    }
}

pub fn build_providers(overview: Option<&Value>) -> Vec<ProviderRow> {
    let mut rows: Vec<ProviderRow> = Vec::new();
    let Some(overview) = overview else {
        return rows;
    };
    let Some(providers) = overview.get("providers").and_then(|p| p.as_array()) else {
        return rows;
    };
    for provider in providers {
        let accounts: Vec<Value> = provider
            .get("accounts")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let active_id = provider
            .get("activeAccountId")
            .and_then(|v| v.as_str())
            .map(String::from);
        let active = accounts
            .iter()
            .find(|item| item.get("id").and_then(|v| v.as_str()) == active_id.as_deref());
        let usage = provider.get("usage");
        let requests = usage
            .and_then(|u| u.get("requests"))
            .and_then(|v| v.as_i64());
        let tokens = usage.and_then(|u| u.get("tokens")).and_then(|v| v.as_i64());
        let (frac, level, text) = usage_frac(requests, tokens);
        let source = provider
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("vault")
            .to_string();
        let provider_id = provider
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string();
        let driver = if source == "cpm" {
            provider_id.strip_prefix("cpm:").map(String::from)
        } else {
            None
        };
        let refresh = provider.get("refresh");
        let unavailable = provider.get("availability").and_then(|v| v.as_str()) == Some("unavailable")
            || provider.get("error").is_some();
        let stale = provider.get("stale").and_then(|v| v.as_bool()).unwrap_or(false)
            || refresh.and_then(|r| r.get("error")).is_some();
        let (level, text) = (|| {
            if unavailable {
                ("down".to_string(), text)
            } else if stale {
                ("warn".to_string(), text)
            } else {
                (level, text)
            }
        })();
        let label = provider
            .get("label")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| provider_id.clone());
        let active_label = active.and_then(|a| a.get("label")).and_then(|v| v.as_str()).map(String::from);
        let usage_text = if text.is_empty() {
            active_label
                .clone()
                .or_else(|| Some(format!("{} accounts", accounts.len())))
                .unwrap_or_default()
        } else {
            text
        };
        rows.push(ProviderRow {
            provider: provider_id,
            label,
            active_label,
            active_id,
            source,
            driver,
            accounts,
            requests,
            tokens,
            usage_frac: frac,
            usage_level: level,
            usage_text,
            available: !unavailable && !stale,
        });
    }
    rows
}

#[derive(Debug, Clone)]
pub struct AgentRow {
    pub key: String,
    pub agent: String,
    pub workspace: String,
    pub status: String,
    pub summary: String,
    pub last_activity: Option<String>,
    pub running: bool,
}

pub fn parse_ts(value: Option<&str>) -> i64 {
    let Some(value) = value else {
        return 0;
    };
    let value = value.trim();
    // Normaliseer "+0200" → "+02:00" voor tijdzones zonder dubbele punt.
    let bytes = value.as_bytes();
    let normalized = if bytes.len() >= 5
        && (bytes[bytes.len() - 5] == b'+' || bytes[bytes.len() - 5] == b'-')
        && bytes[bytes.len() - 3] != b':'
    {
        let mut fixed = value.to_string();
        fixed.insert(value.len() - 2, ':');
        fixed
    } else {
        value.to_string()
    };
    // Simpele ISO-parse: we doen alleen de "YYYY-MM-DDTHH:MM:SS" prefix.
    // Heuristiek: splits op 'T', parse datum + tijd als unix-achtige score.
    // Volledige chrono-parse zou een dep toevoegen; volstaat voor sortering.
    let mut ts: i64 = 0;
    let date_part = normalized
        .split(['T', ' '])
        .next()
        .unwrap_or_default()
        .replace('-', "");
    if let Ok(d) = date_part.parse::<i64>() {
        ts = d.saturating_mul(86_400);
    }
    if let Some(time_part) = normalized.split(['T', ' ']).nth(1) {
        let cleaned: String = time_part
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ':')
            .collect();
        let parts: Vec<&str> = cleaned.split(':').collect();
        let (h, m, s) = (
            parts.first().and_then(|p| p.parse::<i64>().ok()).unwrap_or(0),
            parts.get(1).and_then(|p| p.parse::<i64>().ok()).unwrap_or(0),
            parts.get(2).and_then(|p| p.parse::<i64>().ok()).unwrap_or(0),
        );
        ts += h * 3600 + m * 60 + s;
    }
    ts
}

pub fn build_agents(agents_payload: Option<&Value>) -> Vec<AgentRow> {
    let mut rows = Vec::new();
    let Some(agents_payload) = agents_payload else {
        return rows;
    };
    let Some(items) = agents_payload.get("agents").and_then(|a| a.as_array()) else {
        return rows;
    };
    let mut parsed: Vec<(bool, i64, String, AgentRow)> = Vec::new();
    for item in items {
        let status = item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_lowercase();
        let running = status == "running";
        let agent = item
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let workspace = item
            .get("workspace")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();
        let key = item
            .get("key")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("{agent}::{workspace}"));
        let summary: String = item
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();
        let last_activity = item
            .get("lastActivity")
            .and_then(|v| v.as_str())
            .map(String::from);
        let ts = parse_ts(last_activity.as_deref());
        parsed.push((
            running,
            ts,
            agent.clone(),
            AgentRow {
                key,
                agent,
                workspace,
                status,
                summary,
                last_activity,
                running,
            },
        ));
    }
    parsed.sort_by(|a, b| {
        // running eerst, dan lastActivity desc, dan naam
        b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2))
    });
    rows.extend(parsed.into_iter().take(8).map(|(_, _, _, row)| row));
    rows
}

#[derive(Debug, Clone, Default)]
pub struct FleetInfo {
    pub online: usize,
    pub total: usize,
    pub host: Option<String>,
    pub stale: bool,
}

pub fn build_fleet(fleet_payload: Option<&Value>) -> FleetInfo {
    let mut info = FleetInfo::default();
    let Some(fleet_payload) = fleet_payload else {
        return info;
    };
    info.stale = fleet_payload.get("stale").and_then(|v| v.as_bool()).unwrap_or(false);
    info.host = fleet_payload.get("host").and_then(|v| v.as_str()).map(String::from);
    let mut nodes: Vec<&Value> = Vec::new();
    if let Some(self_node) = fleet_payload.get("self") {
        nodes.push(self_node);
    }
    if let Some(peers) = fleet_payload.get("peers").and_then(|p| p.as_array()) {
        nodes.extend(peers);
    }
    info.total = nodes.len();
    info.online = nodes
        .iter()
        .filter(|n| n.get("online").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    info
}

// ---------------------------------------------------------------------------
// Ops snapshot (joep-ops / herdr)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct HerdrAgent {
    pub terminal_id: String,
    pub name: String,
    pub status: String, // working | idle | blocked | unknown
    pub workspace: String,
    pub workspace_id: String,
    pub cwd: String,
    pub pane_id: String,
    pub focused: bool,
}

#[derive(Debug, Clone, Default)]
pub struct OpsSnapshot {
    pub ok: bool,
    pub agents: Vec<HerdrAgent>,
}

pub fn build_ops_snapshot(data: Option<&Value>) -> OpsSnapshot {
    let mut snap = OpsSnapshot::default();
    let Some(data) = data else {
        return snap;
    };
    snap.ok = true;
    if let Some(agents) = data.get("agents").and_then(|a| a.as_array()) {
        for a in agents {
            let status = a
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_lowercase();
            snap.agents.push(HerdrAgent {
                terminal_id: a.get("terminal_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: a.get("agent").and_then(|v| v.as_str()).unwrap_or("agent").to_string(),
                status,
                workspace: a
                    .get("terminal_title_stripped")
                    .or_else(|| a.get("cwd"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                workspace_id: a.get("workspace_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                cwd: a.get("cwd").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                pane_id: a.get("pane_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                focused: a.get("focused").and_then(|v| v.as_bool()).unwrap_or(false),
            });
        }
    }
    snap
}

// ---------------------------------------------------------------------------
// Snapshot (één consistent beeld voor de hele app)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub fetched_at_unix: i64,
    pub health: HealthInfo,
    pub day_score: DayScore,
    pub providers: Vec<ProviderRow>,
    pub agents: Vec<AgentRow>,
    pub fleet: FleetInfo,
    pub revision: i64,
    pub tasks: Vec<Value>,
    pub clipboard: Vec<Value>,
    pub events: Vec<Value>,
    pub desktop: HashMap<String, Value>,
    pub share_sync: HashMap<String, Value>,
    pub error: Option<String>,
    pub raw: Value,
    pub ops: OpsSnapshot,
    pub suggestions: Vec<Value>,
}

impl Snapshot {
    pub fn fetched_label(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        if self.fetched_at_unix == 0 {
            return "—".to_string();
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let elapsed = now.max(self.fetched_at_unix) - self.fetched_at_unix;
        if elapsed < 60 {
            return "nu".to_string();
        }
        if elapsed < 3600 {
            return format!("{}min geleden", elapsed / 60);
        }
        format!("{}u geleden", elapsed / 3600)
    }

    /// Tray-state per de statuslijn-spec: offline > fout > hulp > bezig > stil.
    pub fn tray_state(&self) -> (String, String) {
        if self.error.is_some() {
            return ("offline".into(), "ChefGroep · alles offline".into());
        }
        if self.health.level == "down" && self.health.total > 0 {
            let services = self
                .raw
                .get("status")
                .and_then(|s| s.get("services"))
                .and_then(|s| s.as_array());
            let down = services.and_then(|svcs| {
                svcs.iter().find_map(|svc| {
                    let state = svc.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    if !matches!(state, "running" | "ok" | "healthy") {
                        svc.get("name").and_then(|v| v.as_str())
                    } else {
                        None
                    }
                })
            });
            return (
                "fout".into(),
                format!("ChefGroep · {} hapert", down.unwrap_or("een dienst")),
            );
        }
        let failed = self
            .agents
            .iter()
            .find(|a| matches!(a.status.as_str(), "failed" | "error" | "crashed"));
        if let Some(agent) = failed {
            return ("fout".into(), format!("ChefGroep · {} hapert", agent.agent));
        }
        if self
            .agents
            .iter()
            .any(|a| matches!(a.status.as_str(), "blocked" | "waiting" | "needs_input" | "input" | "attention"))
        {
            return ("hulp".into(), "ChefGroep · even jou nodig".into());
        }
        let running = self.agents.iter().filter(|a| a.running).count();
        if running > 0 {
            return ("bezig".into(), format!("ChefGroep · {running} aan het werk"));
        }
        if self.health.level == "warn" {
            return ("hulp".into(), "ChefGroep · even jou nodig".into());
        }
        ("stil".into(), "ChefGroep · nog niks gebeurd vandaag".into())
    }
}

// ---------------------------------------------------------------------------
// Suggesties (watcher-overgangen)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub key: String,
    pub title: String,
    pub meta: String,
    pub stamp: String, // KLAAR | HULP | FOUT | LIMIET
    pub action_label: String,
    pub kind: SuggestionKind,
    pub created_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionKind {
    FocusAgent(String),
    OpenDashboard,
    None_,
}

impl Suggestion {
    pub fn fresh(&self, ttl_seconds: i64) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now - self.created_unix < ttl_seconds
    }
}