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

/// Fallback: chef-eval agent summary uit /agents (parity met Python api.py
/// load_day_score: geen reports-bestand, dan de chef-eval summary-score).
pub fn day_score_from_agent_summary(agents_payload: Option<&Value>) -> Option<DayScore> {
    let items = agents_payload?.get("agents")?.as_array()?;
    let item = items
        .iter()
        .find(|a| a.get("agent").and_then(|v| v.as_str()) == Some("chef-eval"))?;
    let summary = item.get("summary").and_then(|v| v.as_str())?;
    let (letter, score) = score_regex_letter(summary)?;
    Some(DayScore {
        letter: Some(letter),
        score: Some(score),
        source: Some("chef-eval Summary".into()),
    })
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
    /// Laatste refresh-tijdstip van de provider (uit `refresh.updatedAt`), bruikbaar
    /// om versheid in de UI te tonen. None als de backend geen timestamp gaf.
    pub refresh_at: Option<String>,
    /// Data is ouder dan de connector-refresh (stale/error) → UI-waarschuwing.
    pub stale: bool,
    /// Waarom de data stale/onbeschikbaar is (endpoint onbereikbaar / 401 /
    /// connector oud). None bij verse data. Toont in de badge-tooltip.
    pub stale_reason: Option<String>,
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
        let unavailable = provider.get("availability").and_then(|v| v.as_str())
            == Some("unavailable")
            || provider.get("error").is_some();
        let stale = provider
            .get("stale")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || refresh.and_then(|r| r.get("error")).is_some();
        // Freshness-contract: toon een concrete reden, nooit alleen "STALE".
        let stale_reason: Option<String> = if unavailable {
            let err = provider.get("error").and_then(|v| v.as_str()).unwrap_or("");
            if err.contains("401") || err.contains("unauthor") || err.contains("token") {
                Some("401 · auth verlopen".into())
            } else if !err.is_empty() {
                Some(format!("endpoint onbereikbaar · {err}"))
            } else {
                Some("endpoint onbereikbaar".into())
            }
        } else if stale {
            let err = refresh
                .and_then(|r| r.get("error"))
                .and_then(|v| v.as_str());
            match err {
                Some(e) if !e.is_empty() => Some(format!("connector oud · {e}")),
                _ => Some("connector-data oud".into()),
            }
        } else {
            None
        };
        let (level, text) = if unavailable {
            ("down".to_string(), text)
        } else if stale {
            ("warn".to_string(), text)
        } else {
            (level, text)
        };
        let refresh_at = refresh
            .and_then(|r| r.get("updatedAt"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let label = provider
            .get("label")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| provider_id.clone());
        let active_label = active
            .and_then(|a| a.get("label"))
            .and_then(|v| v.as_str())
            .map(String::from);
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
            refresh_at,
            stale,
            stale_reason,
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
            parts
                .first()
                .and_then(|p| p.parse::<i64>().ok())
                .unwrap_or(0),
            parts
                .get(1)
                .and_then(|p| p.parse::<i64>().ok())
                .unwrap_or(0),
            parts
                .get(2)
                .and_then(|p| p.parse::<i64>().ok())
                .unwrap_or(0),
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
    info.stale = fleet_payload
        .get("stale")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    info.host = fleet_payload
        .get("host")
        .and_then(|v| v.as_str())
        .map(String::from);
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
                terminal_id: a
                    .get("terminal_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: a
                    .get("agent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent")
                    .to_string(),
                status,
                workspace: a
                    .get("terminal_title_stripped")
                    .or_else(|| a.get("cwd"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                workspace_id: a
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                cwd: a
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                pane_id: a
                    .get("pane_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                focused: a.get("focused").and_then(|v| v.as_bool()).unwrap_or(false),
            });
        }
    }
    snap
}

// ---------------------------------------------------------------------------
// ChefApp 4.0 — nieuwe domein-types (alles Default + tolerant parse)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct InboxItem {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct FleetNode {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
    pub host: Option<String>,
    pub online: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HerdrWorkspace {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VaultAccount {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub provider: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct CommanderTask {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct CrmDeal {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerDiff {
    pub observed: Vec<Value>,
    pub desired: Vec<Value>,
    pub drift: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SecretMeta {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct ClipboardEntry {
    pub id: String,
    pub title: String,
    pub text: String,
    pub meta: String,
    pub status: String,
    pub created_at: Option<String>,
    pub raw: Value,
}

impl ClipboardEntry {
    /// Compat-get zodat bestaande actions.rs `item.get("text")` blijft werken.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.raw.get(key)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinearIssue {
    pub id: String,
    pub title: String,
    pub meta: String,
    pub status: String,
    pub url: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct KaterStatus {
    pub online: bool,
    pub status: String,
    pub profile: Option<String>,
    pub meta: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObsSummary {
    pub ok: bool,
    pub status: String,
    pub errors: Vec<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JcodeMemoryStatus {
    pub online: bool,
    pub host: String,
    pub bind: String,
    pub status: String,
}

// --- tolerant builders (Value -> structs, misser = Default) ---

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}
fn opt_str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(String::from)
}

pub fn build_inbox(data: Option<&Value>) -> Vec<InboxItem> {
    let Some(arr) = data
        .and_then(|v| v.get("items").or_else(|| v.get("inbox")))
        .and_then(|v| v.as_array())
    else {
        // ook direct array zonder wrapper
        if let Some(arr) = data.and_then(|v| v.as_array()) {
            return arr.iter().map(parse_inbox_item).collect();
        }
        return Vec::new();
    };
    arr.iter().map(parse_inbox_item).collect()
}
fn parse_inbox_item(v: &Value) -> InboxItem {
    let payload = v.get("payload").filter(|p| p.is_object()).unwrap_or(v);
    InboxItem {
        id: opt_str_field(payload, "id")
            .or_else(|| opt_str_field(v, "id"))
            .unwrap_or_default(),
        title: opt_str_field(payload, "title")
            .or_else(|| opt_str_field(v, "title"))
            .or_else(|| opt_str_field(payload, "summary"))
            .unwrap_or_default(),
        meta: opt_str_field(payload, "meta")
            .or_else(|| opt_str_field(payload, "summary"))
            .unwrap_or_default(),
        status: opt_str_field(payload, "status")
            .or_else(|| opt_str_field(v, "status"))
            .unwrap_or_else(|| "unknown".into()),
    }
}

pub fn build_fleet_nodes(data: Option<&Value>) -> Vec<FleetNode> {
    let Some(arr) = data
        .and_then(|v| v.get("nodes").or_else(|| v.get("fleet_nodes")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.get("peers")).and_then(|v| v.as_array()))
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_fleet_node).collect()
}
fn parse_fleet_node(v: &Value) -> FleetNode {
    FleetNode {
        id: str_field(v, "id"),
        title: opt_str_field(v, "name")
            .or_else(|| opt_str_field(v, "title"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "host").unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| {
            if v.get("online").and_then(|x| x.as_bool()).unwrap_or(false) {
                "online".into()
            } else {
                "offline".into()
            }
        }),
        host: opt_str_field(v, "host"),
        online: v.get("online").and_then(|x| x.as_bool()).unwrap_or(false),
    }
}

pub fn build_herdr_workspaces(data: Option<&Value>) -> Vec<HerdrWorkspace> {
    let Some(arr) = data
        .and_then(|v| v.get("workspaces").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_herdr_workspace).collect()
}
fn parse_herdr_workspace(v: &Value) -> HerdrWorkspace {
    HerdrWorkspace {
        id: str_field(v, "id"),
        title: opt_str_field(v, "name")
            .or_else(|| opt_str_field(v, "title"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "cwd").unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| "unknown".into()),
        cwd: opt_str_field(v, "cwd"),
    }
}

pub fn build_vault_accounts(data: Option<&Value>) -> Vec<VaultAccount> {
    let Some(arr) = data
        .and_then(|v| v.get("accounts").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_vault_account).collect()
}
fn parse_vault_account(v: &Value) -> VaultAccount {
    VaultAccount {
        id: str_field(v, "id"),
        title: opt_str_field(v, "label")
            .or_else(|| opt_str_field(v, "title"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "provider")
            .or_else(|| opt_str_field(v, "email"))
            .unwrap_or_default(),
        provider: opt_str_field(v, "provider").unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| "unknown".into()),
    }
}

pub fn build_commander_tasks(data: Option<&Value>) -> Vec<CommanderTask> {
    let Some(arr) = data
        .and_then(|v| v.get("tasks").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_commander_task).collect()
}
fn parse_commander_task(v: &Value) -> CommanderTask {
    CommanderTask {
        id: str_field(v, "id"),
        title: opt_str_field(v, "title")
            .or_else(|| opt_str_field(v, "name"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "cwd")
            .or_else(|| opt_str_field(v, "summary"))
            .unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| "unknown".into()),
    }
}

pub fn build_crm_deals(data: Option<&Value>) -> Vec<CrmDeal> {
    let Some(arr) = data
        .and_then(|v| v.get("deals").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_crm_deal).collect()
}
fn parse_crm_deal(v: &Value) -> CrmDeal {
    CrmDeal {
        id: str_field(v, "id"),
        title: opt_str_field(v, "title")
            .or_else(|| opt_str_field(v, "name"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "stage")
            .or_else(|| opt_str_field(v, "meta"))
            .unwrap_or_default(),
        status: opt_str_field(v, "status")
            .or_else(|| opt_str_field(v, "stage"))
            .unwrap_or_else(|| "unknown".into()),
        amount: opt_str_field(v, "amount").or_else(|| opt_str_field(v, "value")),
    }
}

pub fn build_container_diff(data: Option<&Value>) -> ContainerDiff {
    let Some(obj) = data.and_then(|v| v.as_object()) else {
        return ContainerDiff::default();
    };
    let observed = obj
        .get("observed")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let desired = obj
        .get("desired")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let drift = obj
        .get("drift")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    ContainerDiff {
        observed,
        desired,
        drift,
    }
}

pub fn build_secrets_meta(data: Option<&Value>) -> Vec<SecretMeta> {
    let Some(arr) = data
        .and_then(|v| v.get("secrets").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_secret_meta).collect()
}
fn parse_secret_meta(v: &Value) -> SecretMeta {
    SecretMeta {
        id: str_field(v, "id"),
        title: opt_str_field(v, "title")
            .or_else(|| opt_str_field(v, "name"))
            .unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "collection")
            .or_else(|| opt_str_field(v, "meta"))
            .unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| "unknown".into()),
    }
}

pub fn build_clipboard_entries(data: Option<&Value>) -> Vec<ClipboardEntry> {
    let Some(arr) = data
        .and_then(|v| v.get("items").or_else(|| v.get("clipboard")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter()
        .enumerate()
        .map(|(i, v)| parse_clipboard_entry(v, i))
        .collect()
}
fn parse_clipboard_entry(v: &Value, idx: usize) -> ClipboardEntry {
    let text = opt_str_field(v, "text").unwrap_or_default();
    let title = if text.is_empty() {
        opt_str_field(v, "title").unwrap_or_else(|| format!("clipboard-rij {idx}"))
    } else {
        text.chars().take(40).collect()
    };
    ClipboardEntry {
        id: opt_str_field(v, "id").unwrap_or_else(|| format!("cb-{idx}")),
        title: title.clone(),
        text,
        meta: opt_str_field(v, "meta").unwrap_or_default(),
        status: opt_str_field(v, "status").unwrap_or_else(|| "ok".into()),
        created_at: opt_str_field(v, "created_at").or_else(|| opt_str_field(v, "updatedAt")),
        raw: v.clone(),
    }
}

pub fn build_linear_issues(data: Option<&Value>) -> Vec<LinearIssue> {
    let Some(arr) = data
        .and_then(|v| v.get("issues").or_else(|| v.get("items")))
        .and_then(|v| v.as_array())
        .or_else(|| data.and_then(|v| v.as_array()))
    else {
        return Vec::new();
    };
    arr.iter().map(parse_linear_issue).collect()
}
fn parse_linear_issue(v: &Value) -> LinearIssue {
    LinearIssue {
        id: opt_str_field(v, "id")
            .or_else(|| opt_str_field(v, "identifier"))
            .unwrap_or_default(),
        title: opt_str_field(v, "title").unwrap_or_else(|| str_field(v, "id")),
        meta: opt_str_field(v, "project")
            .or_else(|| opt_str_field(v, "team"))
            .unwrap_or_default(),
        status: opt_str_field(v, "state")
            .or_else(|| opt_str_field(v, "status"))
            .unwrap_or_else(|| "unknown".into()),
        url: opt_str_field(v, "url"),
        project: opt_str_field(v, "project"),
    }
}

pub fn build_kater_status(data: Option<&Value>) -> KaterStatus {
    let Some(v) = data else {
        return KaterStatus::default();
    };
    KaterStatus {
        online: v
            .get("online")
            .and_then(|x| x.as_bool())
            .unwrap_or_else(|| v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false)),
        status: opt_str_field(v, "status").unwrap_or_else(|| {
            if v.get("online").and_then(|x| x.as_bool()).unwrap_or(false) {
                "online".into()
            } else {
                "offline".into()
            }
        }),
        profile: opt_str_field(v, "profile"),
        meta: opt_str_field(v, "meta")
            .or_else(|| opt_str_field(v, "message"))
            .unwrap_or_default(),
    }
}

pub fn build_obs_summary(data: Option<&Value>) -> ObsSummary {
    let Some(v) = data else {
        return ObsSummary::default();
    };
    let errors: Vec<String> = v
        .get("errors")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    ObsSummary {
        ok: v
            .get("ok")
            .and_then(|x| x.as_bool())
            .unwrap_or(errors.is_empty()),
        status: opt_str_field(v, "status").unwrap_or_else(|| {
            if errors.is_empty() {
                "ok".into()
            } else {
                "warn".into()
            }
        }),
        errors,
        updated_at: opt_str_field(v, "updatedAt").or_else(|| opt_str_field(v, "updated_at")),
    }
}

pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // simpele ISO8601 zonder externe chrono: YYYY-MM-DDTHH:MM:SSZ benadering via secs
    // We gebruiken gewoon secs als fallback string; tolerant voor UI.
    // Voor correct ISO gebruiken we chrono-achtige format handmatig: epoch -> string
    // Hier volstaat een stabiele ISO-achtige waarde voor stale-detectie.
    format!("{secs}")
}

pub fn iso_now() -> String {
    // Probeer chrono-achtig ISO8601 via time crate-less: gebruik SystemTime debug.
    // We doen een eenvoudige UTC format: gebruik std::time + handmatige calc is complex,
    // dus val terug op `now_iso8601` (secs string) — UI behandelt beide als stale-marker.
    // Voor testbaarheid geven we een vaste ISO prefix als `time` feature ontbreekt.
    now_iso8601()
}

/// Per-bron pollresultaat: ok-flag voor Sync + compacte chip voor de statuslijn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PollHealth {
    pub ok: bool,
    /// Compact chip: `"ok"`, HTTP-code (`"302"`), `"offline"`, `"gedeeltelijk"`, …
    pub chip: String,
}

impl PollHealth {
    pub fn ok() -> Self {
        Self {
            ok: true,
            chip: "ok".into(),
        }
    }

    pub fn fail(chip: impl Into<String>) -> Self {
        Self {
            ok: false,
            chip: chip.into(),
        }
    }
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
    pub clipboard: Vec<ClipboardEntry>,
    pub events: Vec<Value>,
    pub desktop: HashMap<String, Value>,
    pub share_sync: HashMap<String, Value>,
    pub error: Option<String>,
    pub raw: Value,
    pub suggestions: Vec<Suggestion>,
    // ChefApp 4.0 — nieuwe domeinen (Default + tolerant)
    pub inbox: Vec<InboxItem>,
    pub fleet_nodes: Vec<FleetNode>,
    pub herdr_workspaces: Vec<HerdrWorkspace>,
    pub vault_accounts: Vec<VaultAccount>,
    pub commander_tasks: Vec<CommanderTask>,
    pub crm_deals: Vec<CrmDeal>,
    pub containers: ContainerDiff,
    pub secrets_meta: Vec<SecretMeta>,
    pub linear_issues: Vec<LinearIssue>,
    pub kater_status: KaterStatus,
    pub observability: ObsSummary,
    pub last_poll_at: HashMap<String, String>,
    /// Per bron: ok + statuslijn-chip. Ontbreekt = nog nooit gepold.
    pub last_poll: HashMap<String, PollHealth>,
    pub brain: crate::vault_bridge::BrainResponse,
    pub jcode_memory: JcodeMemoryStatus,
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

    /// W4 statuslijn-chip: `"laatste poll 4s geleden · vault ok · ops 302"`.
    pub fn poll_statuslijn(&self) -> String {
        format!(
            "{} · {} · {}",
            self.poll_age_label(),
            self.endpoint_chip("vault"),
            self.endpoint_chip("ops")
        )
    }

    fn poll_age_label(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        if self.fetched_at_unix == 0 {
            return "laatste poll —".to_string();
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let elapsed = now.max(self.fetched_at_unix) - self.fetched_at_unix;
        if elapsed < 60 {
            return format!("laatste poll {elapsed}s geleden");
        }
        if elapsed < 3600 {
            return format!("laatste poll {}min geleden", elapsed / 60);
        }
        format!("laatste poll {}u geleden", elapsed / 3600)
    }

    fn endpoint_chip(&self, key: &str) -> String {
        if let Some(health) = self.last_poll.get(key) {
            return format!("{key} {}", health.chip);
        }
        if key == "vault" {
            return match self.error.as_deref() {
                None if self.fetched_at_unix > 0 => "vault ok".to_string(),
                Some("vault offline") => "vault offline".to_string(),
                Some(_) => "vault gedeeltelijk".to_string(),
                None => "vault —".to_string(),
            };
        }
        format!("{key} —")
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
        if self.agents.iter().any(|a| {
            matches!(
                a.status.as_str(),
                "blocked" | "waiting" | "needs_input" | "input" | "attention"
            )
        }) {
            return ("hulp".into(), "ChefGroep · even jou nodig".into());
        }
        let running = self.agents.iter().filter(|a| a.running).count();
        if running > 0 {
            return (
                "bezig".into(),
                format!("ChefGroep · {running} aan het werk"),
            );
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
    /// Agent-key voor per-agent demping; leeg voor niet-agent meldingen.
    pub agent: String,
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

    /// Toast-ernst per stempel (joep-notify -S).
    pub fn notify_status(&self) -> &'static str {
        match self.stamp.as_str() {
            "KLAAR" => "ok",
            "HULP" | "LIMIET" => "warn",
            _ => "error",
        }
    }
}

/// Vat verse watcher-suggesties samen tot hooguit één toast per poll-cyclus.
/// Eén suggestie krijgt haar eigen toast; meerdere smelten tot één rustige
/// melding met de ergste ernst. Geen ticker, geen toast-storm bij een
/// gelijktijdige agent-wissel.
pub fn coalesce_toasts(fresh: &[Suggestion]) -> Option<(String, String, &'static str)> {
    let first = fresh.first()?;
    if fresh.len() == 1 {
        return Some((
            first.title.clone(),
            first.meta.clone(),
            first.notify_status(),
        ));
    }
    let worst = if fresh.iter().any(|s| s.stamp == "FOUT") {
        "error"
    } else if fresh
        .iter()
        .any(|s| matches!(s.stamp.as_str(), "HULP" | "LIMIET"))
    {
        "warn"
    } else {
        "ok"
    };
    let mut names: Vec<&str> = fresh.iter().map(|s| s.title.as_str()).collect();
    names.truncate(3);
    let mut body = names.join(", ");
    if fresh.len() > 3 {
        body.push_str(&format!(" +{} meer", fresh.len() - 3));
    }
    Some((
        format!("ChefGroep · {} meldingen", fresh.len()),
        body,
        worst,
    ))
}

pub const SUGGESTION_TTL_SECONDS: i64 = 45;

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Watcher-overgangen tussen de vorige en nieuwe snapshot.
///
/// Pure functie (geen I/O, geen notificaties): geeft suggesties terug voor
/// statusveranderingen van agents — parity met de Python watcher. De actor
/// stuurt ze daarna als toast + slaat ze in de snapshot.
pub fn watcher_events(prev: &Snapshot, next: &Snapshot) -> Vec<Suggestion> {
    let mut out: Vec<Suggestion> = Vec::new();
    let prev_agents: HashMap<String, &AgentRow> =
        prev.agents.iter().map(|a| (a.key.clone(), a)).collect();
    for agent in &next.agents {
        // Alleen transities van agents die al bekend waren (geen startup-spam).
        let Some(was_agent) = prev_agents.get(&agent.key) else {
            continue;
        };
        let was = Some(was_agent.status.as_str());
        let is = agent.status.as_str();
        let key = format!("{}-{}", agent.key, is);
        // Alleen op transitie, niet elke poll opnieuw.
        if was == Some(is) {
            continue;
        }
        let (title, meta, stamp, kind) = match is {
            "blocked" | "waiting" | "needs_input" | "input" | "attention" => (
                format!("{} · even jou nodig", agent.agent),
                agent.summary.clone(),
                "HULP",
                SuggestionKind::FocusAgent(agent.key.clone()),
            ),
            "failed" | "error" | "crashed" => (
                format!("{} · hapert", agent.agent),
                agent.summary.clone(),
                "FOUT",
                SuggestionKind::FocusAgent(agent.key.clone()),
            ),
            "done" => (
                format!("{} · klaar", agent.agent),
                agent.summary.clone(),
                "KLAAR",
                SuggestionKind::OpenDashboard,
            ),
            _ => continue,
        };
        out.push(Suggestion {
            key,
            agent: agent.key.clone(),
            title,
            meta,
            stamp: stamp.into(),
            action_label: "Open".into(),
            kind,
            created_unix: now_unix(),
        });
    }
    out
}

#[cfg(test)]
mod watcher_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stale_reason_is_specific() {
        // 401 → auth verlopen; onbereikbaar zonder error → endpoint; connector
        // error → connector oud; verse data → None.
        let rows = build_providers(Some(&json!({
            "providers": [
                {
                    "id": "p1",
                    "source": "vault",
                    "label": "Auth",
                    "error": "401 Unauthorized",
                    "availability": "unavailable",
                    "accounts": []
                },
                {
                    "id": "p2",
                    "source": "vault",
                    "label": "Down",
                    "availability": "unavailable",
                    "accounts": []
                },
                {
                    "id": "p3",
                    "source": "cpm:ocx",
                    "label": "Oud",
                    "stale": true,
                    "refresh": { "error": "timeout", "updatedAt": "2026-08-12T00:00:00Z" },
                    "accounts": []
                },
                {
                    "id": "p4",
                    "source": "vault",
                    "label": "Vers",
                    "refresh": { "updatedAt": "2026-08-12T08:00:00Z" },
                    "accounts": []
                }
            ]
        })));
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].stale_reason.as_deref(), Some("401 · auth verlopen"));
        assert_eq!(
            rows[1].stale_reason.as_deref(),
            Some("endpoint onbereikbaar")
        );
        assert_eq!(
            rows[2].stale_reason.as_deref(),
            Some("connector oud · timeout")
        );
        assert_eq!(rows[3].stale_reason, None);
        assert!(rows[0].stale_reason.is_some());
    }

    fn snap_with(agents: Vec<(&str, &str)>) -> Snapshot {
        Snapshot {
            raw: json!({}),
            agents: agents
                .into_iter()
                .map(|(key, status)| AgentRow {
                    key: key.into(),
                    agent: key.into(),
                    workspace: "ws".into(),
                    status: status.into(),
                    summary: String::new(),
                    last_activity: None,
                    running: status == "running",
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn transition_to_blocked_yields_hulp() {
        let prev = snap_with(vec![("a::ws", "running")]);
        let next = snap_with(vec![("a::ws", "blocked")]);
        let events = watcher_events(&prev, &next);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stamp, "HULP");
        assert_eq!(events[0].kind, SuggestionKind::FocusAgent("a::ws".into()));
    }

    #[test]
    fn first_appearance_is_silent() {
        let prev = snap_with(vec![]);
        let next = snap_with(vec![("b::ws", "running")]);
        assert!(watcher_events(&prev, &next).is_empty());
    }

    #[test]
    fn no_status_change_yields_nothing() {
        let prev = snap_with(vec![("c::ws", "running")]);
        let next = snap_with(vec![("c::ws", "running")]);
        assert!(watcher_events(&prev, &next).is_empty());
    }

    #[test]
    fn done_transition_is_klaar() {
        let prev = snap_with(vec![("d::ws", "running")]);
        let next = snap_with(vec![("d::ws", "done")]);
        let events = watcher_events(&prev, &next);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stamp, "KLAAR");
    }

    #[test]
    fn coalesce_geeft_enkele_toast_door() {
        let prev = snap_with(vec![("a::ws", "running")]);
        let next = snap_with(vec![("a::ws", "blocked")]);
        let events = watcher_events(&prev, &next);
        let (title, _body, status) = coalesce_toasts(&events).unwrap();
        assert_eq!(title, "a::ws · even jou nodig");
        assert_eq!(status, "warn");
    }

    #[test]
    fn coalesce_bundelt_storm_tot_een_toast() {
        let prev = snap_with(vec![
            ("a::ws", "running"),
            ("b::ws", "running"),
            ("c::ws", "idle"),
            ("d::ws", "running"),
        ]);
        let next = snap_with(vec![
            ("a::ws", "blocked"),
            ("b::ws", "failed"),
            ("c::ws", "blocked"),
            ("d::ws", "done"),
        ]);
        let events = watcher_events(&prev, &next);
        assert_eq!(events.len(), 4);
        let (title, body, status) = coalesce_toasts(&events).unwrap();
        assert_eq!(title, "ChefGroep · 4 meldingen");
        // ergste ernst wint: failed → error
        assert_eq!(status, "error");
        // body toont de eerste drie + restteller
        assert!(body.contains("+1 meer"));
    }

    #[test]
    fn coalesce_leeg_is_stil() {
        assert!(coalesce_toasts(&[]).is_none());
    }
}

#[cfg(test)]
mod chefapp_tolerant_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inbox_tolerant_null_returns_empty() {
        assert!(build_inbox(None).is_empty());
        assert!(build_inbox(Some(&json!(null))).is_empty());
        assert!(build_inbox(Some(&json!({}))).is_empty());
    }

    #[test]
    fn inbox_parses_wrapper_and_direct_array() {
        let v = json!({"items":[{"id":"1","title":"Hi"}]});
        assert_eq!(build_inbox(Some(&v)).len(), 1);
        let v2 = json!([{"id":"2","title":"Yo"}]);
        assert_eq!(build_inbox(Some(&v2)).len(), 1);
    }

    #[test]
    fn fleet_nodes_tolerant_on_missing_and_parses() {
        assert!(build_fleet_nodes(None).is_empty());
        let v = json!({"nodes":[{"id":"n1","name":"ctrl","online":true}]});
        let nodes = build_fleet_nodes(Some(&v));
        assert_eq!(nodes[0].id, "n1");
        assert!(nodes[0].online);
    }

    #[test]
    fn vault_accounts_tolerant() {
        assert!(build_vault_accounts(Some(&json!({"accounts":null}))).is_empty());
        let v = json!([{"id":"a1","label":"Main"}]);
        assert_eq!(build_vault_accounts(Some(&v))[0].id, "a1");
    }

    #[test]
    fn clipboard_entries_tolerant_and_compat_get() {
        let v = json!({"items":[{"text":"hello"}]});
        let entries = build_clipboard_entries(Some(&v));
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].get("text").and_then(|x| x.as_str()),
            Some("hello")
        );
        assert!(build_clipboard_entries(None).is_empty());
        assert!(build_clipboard_entries(Some(&json!({}))).is_empty());
    }

    #[test]
    fn linear_issues_tolerant() {
        assert!(build_linear_issues(None).is_empty());
        let v = json!({"issues":[{"id":"CHE-1","title":"Fix bug"}]});
        assert_eq!(build_linear_issues(Some(&v))[0].id, "CHE-1");
    }

    #[test]
    fn kater_status_offline_default() {
        let s = build_kater_status(None);
        assert!(!s.online);
        let s2 = build_kater_status(Some(&json!({"online":true,"status":"online"})));
        assert!(s2.online);
    }

    #[test]
    fn obs_summary_default_ok_when_no_errors() {
        let s = build_obs_summary(Some(&json!({})));
        assert!(s.ok);
        let s2 = build_obs_summary(Some(&json!({"errors":["boom"]})));
        assert!(!s2.ok);
    }

    #[test]
    fn container_diff_tolerant() {
        let d = build_container_diff(None);
        assert!(d.observed.is_empty());
        let v = json!({"observed":[1],"desired":[2],"drift":["a"]});
        let d2 = build_container_diff(Some(&v));
        assert_eq!(d2.drift, vec!["a"]);
    }

    #[test]
    fn snapshot_default_all_empty_no_panic() {
        let snap = Snapshot::default();
        assert!(snap.inbox.is_empty());
        assert!(snap.fleet_nodes.is_empty());
        assert!(snap.vault_accounts.is_empty());
        assert!(snap.linear_issues.is_empty());
        assert!(snap.last_poll_at.is_empty());
        assert!(snap.last_poll.is_empty());
        // tolerant builders should not panic on garbage
        let _ = build_secrets_meta(Some(&json!("garbage")));
        let _ = build_crm_deals(Some(&json!(123)));
    }

    #[test]
    fn poll_statuslijn_vault_ok_ops_http() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut snap = Snapshot {
            fetched_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
                - 4,
            ..Default::default()
        };
        snap.last_poll.insert("vault".into(), PollHealth::ok());
        snap.last_poll.insert("ops".into(), PollHealth::fail("302"));
        let line = snap.poll_statuslijn();
        assert!(line.contains("vault ok"), "expected vault ok in {line}");
        assert!(line.contains("ops 302"), "expected ops 302 in {line}");
        assert!(line.contains("laatste poll"), "expected poll age in {line}");
        assert!(
            line.contains("s geleden"),
            "expected seconds chip in {line}"
        );
    }

    #[test]
    fn poll_statuslijn_never_polled() {
        let snap = Snapshot::default();
        assert_eq!(snap.poll_statuslijn(), "laatste poll — · vault — · ops —");
    }
}
