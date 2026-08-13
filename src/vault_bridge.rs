//! Vault bridge — typed parsers voor alle /api/* families (pure, geen I/O).
//!
//! Fase 0 stub voor ChefApp 5.0 lane H — scheidt HTTP-parsing van state.rs.
//! Elke domein-parser is `fn parse_<domain>(&Value) -> Option<DomainStruct>` met
//! tolerant parse (Default bij onbekende velden), zodat state.rs dun blijft.
//! In 5.0: 18 families — status, fleet, accounts, providers, clipboard,
//! timeline, brain, crm, neon, connectors, work, commander, share-sync,
//! desktop, opencodex, events, usage, version (vault_bridge meta).

use serde::Deserialize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Tolerant helper: parse als T, val terug op Default bij falen.
// ---------------------------------------------------------------------------

fn tolerant<T: Default + serde::de::DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

// ===========================================================================
// 1. status — GET /api/status  (StatusResponse + ServiceStatus)
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ServiceStatus {
    pub id: String,
    pub name: String,
    pub state: String,
    pub detail: Option<String>,
    pub version: Option<String>,
    pub uptime_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: Option<String>,
    pub commit: Option<String>,
    pub built_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct StatusResponse {
    pub services: Vec<ServiceStatus>,
    pub checked_at: String,
    pub version_info: Option<BuildInfo>,
    #[serde(default)]
    pub ok: Option<bool>,
}

pub fn parse_status(value: &Value) -> Option<StatusResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 2. fleet — GET /api/fleet
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetPeer {
    pub id: String,
    pub hostname: String,
    pub dns_name: String,
    pub os: String,
    pub online: bool,
    pub latency_ms: Option<i64>,
    pub last_handshake: Option<String>,
    pub ips: Vec<String>,
    pub relay: Option<String>,
    #[serde(rename = "self")]
    pub is_self: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub health: Option<String>,
    pub ports: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetUnit {
    pub id: String,
    pub active: String,
    pub sub: String,
    pub load: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetUtrechtNode {
    pub hostname: String,
    pub role: Option<String>,
    pub status: String,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub last_seen: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetUtrechtSection {
    pub enabled: bool,
    pub ok: bool,
    pub kater_available: bool,
    pub tool_listed: bool,
    pub cached: bool,
    pub source: Option<String>,
    pub node_count: u64,
    pub status_counts: std::collections::HashMap<String, u64>,
    pub nodes: Vec<FleetUtrechtNode>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FleetResponse {
    pub ok: bool,
    pub fetched_at: Option<String>,
    pub collected_at: Option<String>,
    pub stale: bool,
    pub source: String,
    pub host: Option<String>,
    #[serde(rename = "self")]
    pub self_peer: Option<FleetPeer>,
    pub peers: Vec<FleetPeer>,
    pub containers: Vec<FleetContainer>,
    pub units: Vec<FleetUnit>,
    pub utrecht: Option<FleetUtrechtSection>,
    pub error: Option<String>,
}

pub fn parse_fleet(value: &Value) -> Option<FleetResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 3. accounts — GET /api/accounts, GET /api/accounts/status,
//               GET /api/accounts/overview + GET /api/coding/accounts/overview
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountLink {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub vault_item: Option<String>,
    pub notes: Option<String>,
    pub last_used_at: Option<String>,
    pub links: Vec<AccountLink>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsStore {
    pub root: Option<String>,
    pub active_id: Option<String>,
    pub accounts: Vec<AccountProfile>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsStatusInsight {
    pub store: Option<AccountsStore>,
    pub active_account: Option<AccountProfile>,
    pub vault_providers: Vec<Value>,
    pub live: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsOverviewProvider {
    pub provider: String,
    pub accounts: Vec<AccountProfile>,
    pub active_account_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsOverview {
    pub providers: Vec<AccountsOverviewProvider>,
    pub fetched_at: Option<String>,
    pub source: Option<String>,
}

pub fn parse_accounts(value: &Value) -> Option<AccountsStore> {
    if value.is_null() {
        return None;
    }
    // vault /api/accounts can be either { accounts: [...] } or { root, activeId, accounts }
    // or a bare array. Normalize via tolerant then fallback to array-wrapping.
    let direct: AccountsStore = serde_json::from_value(value.clone()).unwrap_or_default();
    if !direct.accounts.is_empty() || direct.root.is_some() || direct.active_id.is_some() {
        return Some(direct);
    }
    if let Some(arr) = value.as_array() {
        let accs: Vec<AccountProfile> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        if !accs.is_empty() {
            return Some(AccountsStore {
                accounts: accs,
                ..Default::default()
            });
        }
    }
    if let Some(inner) = value.get("accounts").and_then(|v| v.as_array()) {
        let accs: Vec<AccountProfile> = inner
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        return Some(AccountsStore {
            accounts: accs,
            ..Default::default()
        });
    }
    Some(direct)
}

pub fn parse_accounts_overview(value: &Value) -> Option<AccountsOverview> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

pub fn parse_accounts_status(value: &Value) -> Option<AccountsStatusInsight> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 4. providers — GET /api/coding-providers, /api/coding-providers/keys,
//                /api/coding-providers/refresh, /api/coding-providers/auth
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CodingProviderKey {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CodingProviderSnapshot {
    pub provider: String,
    pub label: Option<String>,
    pub available: bool,
    pub accounts: Vec<AccountProfile>,
    pub active_account_id: Option<String>,
    pub keys: Vec<CodingProviderKey>,
    pub usage: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CodingProvidersResponse {
    pub providers: Vec<CodingProviderSnapshot>,
    pub fetched_at: Option<String>,
    pub source: Option<String>,
}

pub fn parse_providers(value: &Value) -> Option<CodingProvidersResponse> {
    if value.is_null() {
        return None;
    }
    // also accepts a bare array of providers
    if let Some(arr) = value.as_array() {
        let providers: Vec<CodingProviderSnapshot> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        return Some(CodingProvidersResponse {
            providers,
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

// ===========================================================================
// 5. clipboard — GET /api/clipboard
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardListResponse {
    pub items: Vec<ClipboardItem>,
    pub unavailable: Option<bool>,
    pub detail: Option<String>,
}

pub fn parse_clipboard(value: &Value) -> Option<ClipboardListResponse> {
    if value.is_null() {
        return None;
    }
    if let Some(arr) = value.as_array() {
        let items: Vec<ClipboardItem> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        return Some(ClipboardListResponse {
            items,
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

// ===========================================================================
// 6. timeline — GET /api/timeline, GET /api/timeline/stats
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: i64,
    pub ts: String,
    pub source: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub detail: Option<String>,
    pub workspace: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimelineFilters {
    pub since: Option<String>,
    pub source: Option<String>,
    pub kind: Option<String>,
    pub severity: Option<String>,
    pub q: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimelineResponse {
    pub ok: bool,
    pub events: Vec<TimelineEvent>,
    pub total: i64,
    pub sources: Vec<String>,
    pub fetched_at: Option<String>,
    pub source: Option<String>,
    pub filters: Option<TimelineFilters>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimelineHourBucket {
    pub hour: String,
    pub source: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TimelineStatsResponse {
    pub ok: bool,
    pub hours: i64,
    pub since: String,
    pub buckets: Vec<TimelineHourBucket>,
    pub by_source: std::collections::HashMap<String, i64>,
    pub total: i64,
    pub fetched_at: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

pub fn parse_timeline(value: &Value) -> Option<TimelineResponse> {
    if value.is_null() {
        return None;
    }
    if let Some(arr) = value.as_array() {
        let events: Vec<TimelineEvent> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        let total = events.len() as i64;
        return Some(TimelineResponse {
            ok: true,
            events,
            total,
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

pub fn parse_timeline_stats(value: &Value) -> Option<TimelineStatsResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 7. brain — GET /api/brain
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct BrainSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub source: String,
    pub source_label: Option<String>,
    pub abs_path: Option<String>,
    pub repo: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct BrainEval {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub kind: String,
    pub score: Option<i64>,
    pub grade: String,
    pub generated_at: Option<String>,
    pub source: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BrainDataset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub description: String,
    pub url: String,
    pub ok: bool,
    pub status: Option<String>,
    pub records: Option<i64>,
    pub indexed: Option<i64>,
    pub labels: Option<i64>,
    pub embed_coverage_pct: Option<f64>,
    pub embed_model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct BrainCounts {
    pub skills: i64,
    pub skills_local: i64,
    pub skills_repo: i64,
    pub evals: i64,
    pub datasets: i64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BrainResponse {
    pub ok: bool,
    pub collected_at: Option<String>,
    pub generated_at: Option<String>,
    pub org: Option<String>,
    pub source: Option<String>,
    pub eval_dir: Option<String>,
    pub skills: Vec<BrainSkill>,
    pub evals: Vec<BrainEval>,
    pub datasets: Vec<BrainDataset>,
    pub counts: Option<BrainCounts>,
    pub error: Option<String>,
    pub stale: Option<bool>,
    pub age_seconds: Option<i64>,
}

pub fn parse_brain(value: &Value) -> Option<BrainResponse> {
    value.as_object()?;
    let parsed: BrainResponse = tolerant(value);
    if parsed.ok
        || parsed.counts.is_some()
        || !parsed.skills.is_empty()
        || parsed.error.is_some()
        || parsed.source.is_some()
    {
        Some(parsed)
    } else {
        None
    }
}

// ===========================================================================
// 8. crm — GET /api/crm/* (organizations, contacts, applications, activities)
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CrmOrganization {
    pub id: String,
    pub name: String,
    pub kind: Option<String>,
    pub website: Option<String>,
    pub sector: Option<String>,
    pub score: Option<f64>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CrmContact {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CrmApplication {
    pub id: String,
    pub program_name: String,
    pub status: String,
    pub organization_id: Option<String>,
    pub organization_name: Option<String>,
    pub value: Option<String>,
    pub sector: Option<String>,
    pub score: Option<i64>,
    pub next_action: Option<String>,
    pub owner: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CrmActivity {
    pub id: String,
    pub kind: String,
    pub summary: String,
    pub detail: Option<String>,
    pub source: Option<String>,
    pub organization_id: Option<String>,
    pub application_id: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct CrmSnapshot {
    pub organizations: Vec<CrmOrganization>,
    pub contacts: Vec<CrmContact>,
    pub applications: Vec<CrmApplication>,
    pub activities: Vec<CrmActivity>,
    pub summary: Option<Value>,
    pub ok: Option<bool>,
    pub snapshot: Option<Value>,
}

pub fn parse_crm(value: &Value) -> Option<CrmSnapshot> {
    if value.is_null() {
        return None;
    }
    // /api/crm returns { ok, snapshot: { organizations, ... } }
    if let Some(snap) = value.get("snapshot") {
        let mut parsed: CrmSnapshot = tolerant(snap);
        if parsed.ok.is_none() {
            parsed.ok = value.get("ok").and_then(|v| v.as_bool());
        }
        return Some(parsed);
    }
    // flat list envelope { organizations: [...] } or { applications: [...] }
    Some(tolerant(value))
}

// ===========================================================================
// 9. neon — GET /api/neon/health
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct NeonHealthResult {
    pub ok: bool,
    pub configured: Option<String>,
    pub host: Option<String>,
    pub database: Option<String>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct NeonHealthResponse {
    pub ok: bool,
    pub neon: Option<NeonHealthResult>,
    pub error: Option<String>,
}

pub fn parse_neon(value: &Value) -> Option<NeonHealthResponse> {
    if value.is_null() {
        return None;
    }
    // Sometimes raw NeonHealthResult is returned directly without wrapper
    if value.get("neon").is_none() && value.get("ok").is_some() && value.get("host").is_some() {
        let inner: NeonHealthResult = tolerant(value);
        return Some(NeonHealthResponse {
            ok: inner.ok,
            neon: Some(inner),
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

// ===========================================================================
// 10. connectors — GET /api/connectors/{id}/events
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectorEventRecord {
    pub id: String,
    pub connector_id: String,
    pub ts: String,
    pub kind: String,
    pub source: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectorEventsResponse {
    pub connector_id: String,
    pub events: Vec<ConnectorEventRecord>,
    pub count: i64,
}

pub fn parse_connectors(value: &Value) -> Option<ConnectorEventsResponse> {
    if value.is_null() {
        return None;
    }
    if let Some(arr) = value.as_array() {
        let events: Vec<ConnectorEventRecord> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        let count = events.len() as i64;
        return Some(ConnectorEventsResponse {
            events,
            count,
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

// ===========================================================================
// 11. work — GET /api/work
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkPr {
    pub number: i64,
    pub title: String,
    pub url: String,
    pub repo: String,
    pub repo_full: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_draft: bool,
    pub review_status: String,
    pub ci_status: String,
    pub age_hours: f64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkIssue {
    pub number: i64,
    pub title: String,
    pub url: String,
    pub repo: String,
    pub repo_full: String,
    pub author: String,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub relation: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkCounts {
    pub open_prs: i64,
    pub merged_prs24h: i64,
    pub issues: i64,
    pub repos: i64,
    pub linear_issues: i64,
    pub notion_pages: i64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkResponse {
    pub ok: bool,
    pub collected_at: Option<String>,
    pub org: String,
    pub user: String,
    pub source: String,
    pub open_prs: Vec<WorkPr>,
    pub merged_prs24h: Vec<WorkPr>,
    pub issues: Vec<WorkIssue>,
    pub repos: Vec<Value>,
    pub counts: Option<WorkCounts>,
    pub error: Option<String>,
    pub stale: Option<bool>,
    pub age_seconds: Option<i64>,
}

pub fn parse_work(value: &Value) -> Option<WorkResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 12. commander — GET /api/commander/tasks, /api/commander/workspaces
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CommanderTask {
    pub id: String,
    pub prompt: String,
    pub agent_type: String,
    pub cwd: String,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub output_tail: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i64>,
    pub pid: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CommanderTasksResponse {
    pub tasks: Vec<CommanderTask>,
    pub total: i64,
    pub allow_prefix: Option<String>,
    pub max_concurrent: Option<i64>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CommanderWorkspace {
    pub path: String,
    pub label: String,
    pub group: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CommanderWorkspacesResponse {
    pub workspaces: Vec<CommanderWorkspace>,
    pub allow_prefix: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct CommanderSnapshot {
    pub tasks: Vec<CommanderTask>,
    pub workspaces: Vec<CommanderWorkspace>,
    pub total: i64,
    pub allow_prefix: Option<String>,
}

pub fn parse_commander(value: &Value) -> Option<CommanderSnapshot> {
    if value.is_null() {
        return None;
    }
    // try CommanderTasksResponse shape first, then flat tasks array
    if let Some(arr) = value.as_array() {
        let tasks: Vec<CommanderTask> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        let total = tasks.len() as i64;
        return Some(CommanderSnapshot {
            tasks,
            total,
            ..Default::default()
        });
    }
    let tasks_resp: CommanderTasksResponse =
        serde_json::from_value(value.clone()).unwrap_or_default();
    if !tasks_resp.tasks.is_empty() || tasks_resp.total != 0 {
        return Some(CommanderSnapshot {
            tasks: tasks_resp.tasks,
            total: tasks_resp.total,
            allow_prefix: tasks_resp.allow_prefix,
            ..Default::default()
        });
    }
    // also accept CommanderWorkspacesResponse inside same value
    Some(tolerant(value))
}

pub fn parse_commander_workspaces(value: &Value) -> Option<CommanderWorkspacesResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 13. share-sync — GET /api/share-sync/status, POST /api/share-sync/{pull,push}
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ShareSyncStatus {
    pub remote: String,
    pub branch: String,
    pub dirty: bool,
    pub last_pull: Option<String>,
    pub last_push: Option<String>,
    pub pending_files: i64,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct ShareSyncActionResponse {
    pub ok: bool,
    pub message: String,
}

pub fn parse_share_sync(value: &Value) -> Option<ShareSyncStatus> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 14. desktop — GET /api/desktop/status
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct DesktopStatusResponse {
    pub ok: bool,
    pub container: String,
    pub state: String,
    pub detail: Option<String>,
    pub url: String,
}

pub fn parse_desktop(value: &Value) -> Option<DesktopStatusResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 15. opencodex — GET /api/opencodex, /api/opencodex/status, /refresh
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct OpencodexAccount {
    pub id: String,
    pub label: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OpencodexSnapshot {
    pub source: String,
    pub fetched_at: Option<String>,
    pub observed_at: Option<String>,
    pub stale: Option<bool>,
    pub ok: Option<bool>,
    pub accounts: Vec<OpencodexAccount>,
    pub active_account_id: Option<String>,
    pub default_provider: Option<String>,
    pub provider_count: Option<i64>,
    pub port: Option<i64>,
    pub error: Option<Value>,
}

pub fn parse_opencodex(value: &Value) -> Option<OpencodexSnapshot> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 16. events — GET /api/events, GET /api/agents/events, GET /api/sessions
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: Option<String>,
    pub ts: Option<String>,
    pub agent: String,
    pub summary: String,
    pub detail: Option<String>,
    pub workspace: Option<String>,
    pub pane: Option<String>,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct EventsResponse {
    pub events: Vec<AgentEvent>,
    pub total: Option<i64>,
    pub source: Option<String>,
    pub sources: Vec<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentsResponse {
    pub agents: Vec<Value>,
    pub events: Vec<AgentEvent>,
    pub sessions: Vec<Value>,
    pub source: String,
    pub sources: Vec<String>,
    pub fetched_at: String,
}

pub fn parse_events(value: &Value) -> Option<EventsResponse> {
    if value.is_null() {
        return None;
    }
    if let Some(arr) = value.as_array() {
        let events: Vec<AgentEvent> = arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        let total = events.len() as i64;
        return Some(EventsResponse {
            events,
            total: Some(total),
            ..Default::default()
        });
    }
    Some(tolerant(value))
}

pub fn parse_agents(value: &Value) -> Option<AgentsResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 17. usage — GET /api/usage
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct LivePathInfo {
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
    pub mtime: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OcxUsageDay {
    pub date: String,
    pub requests: i64,
    pub total_tokens: i64,
    pub by_model: std::collections::HashMap<String, i64>,
    pub by_provider: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsUsageReport {
    pub ocx: Option<OcxUsageSection>,
    pub today: Option<OcxUsageDay>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OcxUsageSection {
    pub usage_log: Option<LivePathInfo>,
    pub today: Option<OcxUsageDay>,
}

pub fn parse_usage(value: &Value) -> Option<AccountsUsageReport> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

// ===========================================================================
// 18. version / vault_bridge — GET /api/version
// ===========================================================================

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct VersionResponse {
    pub version: String,
    pub commit: Option<String>,
    pub built_at: Option<String>,
    pub name: Option<String>,
    pub ok: Option<bool>,
}

pub fn parse_version(value: &Value) -> Option<VersionResponse> {
    if value.is_null() {
        return None;
    }
    Some(tolerant(value))
}

/// Compatibility alias for the 18th family name `vault_bridge`.
pub fn parse_vault_bridge(value: &Value) -> Option<VersionResponse> {
    parse_version(value)
}

// ---------------------------------------------------------------------------
// ping — liveness for Fase0
// ---------------------------------------------------------------------------

/// Status for Fase0 liveness check (retained for compatibility).
pub fn ping() -> &'static str {
    "vault_bridge 5.0 fase0 — parsers landen in lane H"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fase0_stub_leeft() {
        assert!(ping().contains("5.0"));
    }

    // --- determinisme helper ---
    fn assert_deterministic<T: PartialEq + std::fmt::Debug>(
        value: Value,
        parse: fn(&Value) -> Option<T>,
    ) {
        let a = parse(&value);
        let b = parse(&value);
        assert_eq!(a, b);
    }

    #[test]
    fn status_parse_fixture() {
        let v = json!({
            "services": [
                {"id":"vaultwarden","name":"Vaultwarden","state":"running","detail":"ok"},
                {"id":"desktop","name":"Desktop","state":"stopped"}
            ],
            "checkedAt":"2026-08-12T10:00:00Z",
            "versionInfo":{"version":"1.2.3","commit":"abc"}
        });
        assert_deterministic(v.clone(), parse_status);
        let p = parse_status(&v).unwrap();
        assert_eq!(p.services.len(), 2);
        assert_eq!(p.checked_at, "2026-08-12T10:00:00Z");
        assert_eq!(p.version_info.unwrap().version.as_deref(), Some("1.2.3"));
        assert!(parse_status(&json!(null)).is_none());
        assert!(parse_status(&json!({})).unwrap().services.is_empty());
    }

    #[test]
    fn fleet_parse_fixture() {
        let v = json!({
            "ok": true,
            "fetchedAt":"2026-08-12T10:00:00Z",
            "peers":[{"id":"p1","hostname":"jan","online":true,"self":true}],
            "containers":[{"id":"c1","name":"vault","image":"vault:latest","state":"running","status":"up"}],
            "units":[{"id":"u1","active":"active","sub":"running","load":"loaded"}]
        });
        let p = parse_fleet(&v).unwrap();
        assert_eq!(p.peers.len(), 1);
        assert_eq!(p.containers[0].name, "vault");
        assert!(parse_fleet(&json!(null)).is_none());
        // tolerant on garbage
        assert_eq!(
            parse_fleet(&json!({"peers":"bad"})).unwrap().peers,
            Vec::<FleetPeer>::new()
        );
        assert_deterministic(v, parse_fleet);
    }

    #[test]
    fn accounts_parse_fixture() {
        let v = json!({
            "root":"/tmp/accounts",
            "activeId":"a1",
            "accounts":[{"id":"a1","label":"Main","provider":"codex","links":[]}]
        });
        let p = parse_accounts(&v).unwrap();
        assert_eq!(p.accounts[0].id, "a1");
        // array envelope
        let v2 = json!([{"id":"a2","label":"Alt","provider":"pi"}]);
        assert_eq!(parse_accounts(&v2).unwrap().accounts[0].id, "a2");
        // wrapper { accounts: [...] }
        let v3 = json!({"accounts":[{"id":"a3","label":"X","provider":"ocx"}]});
        assert_eq!(parse_accounts(&v3).unwrap().accounts[0].id, "a3");
        assert!(parse_accounts(&json!(null)).is_none());
    }

    #[test]
    fn accounts_overview_parse_fixture() {
        let v = json!({
            "providers":[{"provider":"codex","accounts":[{"id":"a1","label":"Main","provider":"codex"}],"activeAccountId":"a1"}],
            "fetchedAt":"2026-08-12T10:00:00Z"
        });
        assert_eq!(parse_accounts_overview(&v).unwrap().providers.len(), 1);
        assert!(parse_accounts_overview(&json!(null)).is_none());
    }

    #[test]
    fn accounts_status_parse_fixture() {
        let v = json!({"store":{"root":"/x","activeId":"a1","accounts":[]},"activeAccount":null,"vaultProviders":[],"live":[]});
        assert!(parse_accounts_status(&v).is_some());
        assert!(parse_accounts_status(&json!(null)).is_none());
    }

    #[test]
    fn providers_parse_fixture() {
        let v = json!({
            "providers":[{"provider":"openai","label":"OpenAI","available":true,"accounts":[],"keys":[]}],
            "fetchedAt":"2026-08-12T10:00:00Z"
        });
        assert_eq!(parse_providers(&v).unwrap().providers[0].provider, "openai");
        // bare array
        let v2 = json!([{"provider":"pi","available":false}]);
        assert_eq!(parse_providers(&v2).unwrap().providers.len(), 1);
        assert!(parse_providers(&json!(null)).is_none());
    }

    #[test]
    fn clipboard_parse_fixture() {
        let v = json!({"items":[{"id":"1","text":"hello","createdAt":"2026-08-12T10:00:00Z"}],"unavailable":false});
        let p = parse_clipboard(&v).unwrap();
        assert_eq!(p.items[0].text, "hello");
        // bare array
        let v2 = json!([{"id":"2","text":"world","createdAt":"2026-08-12T10:00:00Z"}]);
        assert_eq!(parse_clipboard(&v2).unwrap().items.len(), 1);
        assert!(parse_clipboard(&json!(null)).is_none());
        // tolerant empty
        assert!(parse_clipboard(&json!({})).unwrap().items.is_empty());
    }

    #[test]
    fn timeline_parse_fixture() {
        let v = json!({
            "ok": true,
            "events":[{"id":1,"ts":"2026-08-12T10:00:00Z","source":"herdr","kind":"start","severity":"info","title":"started"}],
            "total":1,"sources":["herdr"],"fetchedAt":"2026-08-12T10:00:00Z","source":"host"
        });
        let p = parse_timeline(&v).unwrap();
        assert_eq!(p.events[0].title, "started");
        // bare array
        let v2 = json!([{"id":2,"ts":"2026-08-12T10:00:00Z","source":"docker","kind":"stop","severity":"warn","title":"stopped"}]);
        assert_eq!(parse_timeline(&v2).unwrap().events.len(), 1);
        assert!(parse_timeline(&json!(null)).is_none());
    }

    #[test]
    fn timeline_stats_parse_fixture() {
        let v = json!({
            "ok": true,"hours":24,"since":"2026-08-11T10:00:00Z",
            "buckets":[{"hour":"2026-08-12T10:00:00Z","source":"herdr","count":5}],
            "bySource":{"herdr":5},"total":5,"fetchedAt":"2026-08-12T10:00:00Z","source":"host"
        });
        assert_eq!(parse_timeline_stats(&v).unwrap().total, 5);
        assert!(parse_timeline_stats(&json!(null)).is_none());
    }

    #[test]
    fn brain_parse_fixture() {
        let v = json!({
            "ok": true,"org":"GroepOnline","source":"host",
            "skills":[{"id":"s1","name":"Skill One","description":"desc","path":"/x","source":"local"}],
            "evals":[],"datasets":[],"counts":{"skills":1,"skillsLocal":1,"skillsRepo":0,"evals":0,"datasets":0}
        });
        assert_eq!(parse_brain(&v).unwrap().skills[0].id, "s1");
        assert!(parse_brain(&json!(null)).is_none());
        assert_deterministic(v, parse_brain);
    }

    #[test]
    fn crm_parse_fixture() {
        // flat
        let v = json!({"organizations":[{"id":"o1","name":"Acme"}],"applications":[],"contacts":[],"activities":[]});
        assert_eq!(parse_crm(&v).unwrap().organizations[0].name, "Acme");
        // wrapped { ok, snapshot }
        let v2 = json!({"ok":true,"snapshot":{"organizations":[],"applications":[{"id":"a1","programName":"Prog","status":"open"}],"contacts":[],"activities":[]}});
        assert_eq!(parse_crm(&v2).unwrap().applications[0].id, "a1");
        assert!(parse_crm(&json!(null)).is_none());
    }

    #[test]
    fn neon_parse_fixture() {
        let v = json!({"ok":true,"neon":{"ok":true,"host":"ep-xxx.neon.tech","database":"vault","latencyMs":12}});
        assert!(parse_neon(&v).unwrap().neon.unwrap().ok);
        // raw health without wrapper
        let v2 = json!({"ok":true,"host":"ep-xxx.neon.tech","database":"vault","latencyMs":12});
        assert!(parse_neon(&v2).unwrap().ok);
        assert!(parse_neon(&json!(null)).is_none());
    }

    #[test]
    fn connectors_parse_fixture() {
        let v = json!({"connectorId":"share","events":[{"id":"e1","connectorId":"share","ts":"2026-08-12T10:00:00Z","kind":"share"}],"count":1});
        assert_eq!(parse_connectors(&v).unwrap().events[0].id, "e1");
        // bare array
        let v2 = json!([{"id":"e2","connectorId":"ops","ts":"2026-08-12T10:00:00Z","kind":"ops"}]);
        assert_eq!(parse_connectors(&v2).unwrap().count, 1);
        assert!(parse_connectors(&json!(null)).is_none());
    }

    #[test]
    fn work_parse_fixture() {
        let v = json!({
            "ok": true,"org":"GroepOnline","user":"joep","source":"vault_collector",
            "openPrs":[{"number":12,"title":"Fix","url":"https://github.com/x","repo":"x","repoFull":"o/x","author":"joep","createdAt":"2026-08-12T08:00:00Z","updatedAt":"2026-08-12T09:00:00Z","isDraft":false,"reviewStatus":"none","ciStatus":"pass","ageHours":2}],
            "mergedPrs24h":[],"issues":[],"repos":[],"counts":{"openPrs":1,"mergedPrs24h":0,"issues":0,"repos":0,"linearIssues":0,"notionPages":0}
        });
        assert_eq!(parse_work(&v).unwrap().open_prs[0].number, 12);
        assert!(parse_work(&json!(null)).is_none());
    }

    #[test]
    fn commander_parse_fixture() {
        let v = json!({
            "tasks":[{"id":"t1","prompt":"do","agentType":"codex","cwd":"/tmp","status":"queued","createdAt":"2026-08-12T10:00:00Z"}],
            "total":1,"allowPrefix":"/home/joep","maxConcurrent":4,"fetchedAt":"2026-08-12T10:00:00Z"
        });
        assert_eq!(parse_commander(&v).unwrap().tasks[0].id, "t1");
        // bare array
        let v2 = json!([{"id":"t2","prompt":"hi","agentType":"cline","cwd":"/tmp","status":"running","createdAt":"2026-08-12T10:00:00Z"}]);
        assert_eq!(parse_commander(&v2).unwrap().tasks[0].id, "t2");
        assert!(parse_commander(&json!(null)).is_none());
        // determinisme
        assert_deterministic(v, parse_commander);
    }

    #[test]
    fn commander_workspaces_parse_fixture() {
        let v = json!({"workspaces":[{"path":"/home/joep/ChefFactory","label":"Factory","group":"code"}],"allowPrefix":"/home/joep","fetchedAt":"2026-08-12T10:00:00Z"});
        assert_eq!(
            parse_commander_workspaces(&v).unwrap().workspaces[0].path,
            "/home/joep/ChefFactory"
        );
        assert!(parse_commander_workspaces(&json!(null)).is_none());
    }

    #[test]
    fn share_sync_parse_fixture() {
        let v = json!({"remote":"origin","branch":"main","dirty":false,"pendingFiles":2});
        assert_eq!(parse_share_sync(&v).unwrap().pending_files, 2);
        assert!(parse_share_sync(&json!(null)).is_none());
        assert!(parse_share_sync(&json!({})).unwrap().remote.is_empty());
    }

    #[test]
    fn desktop_parse_fixture() {
        let v = json!({"ok":true,"container":"vault-webtop","state":"running","url":"http://127.0.0.1:3000"});
        assert_eq!(parse_desktop(&v).unwrap().container, "vault-webtop");
        assert!(parse_desktop(&json!(null)).is_none());
    }

    #[test]
    fn opencodex_parse_fixture() {
        let v = json!({"source":"opencodex","fetchedAt":"2026-08-12T10:00:00Z","stale":false,"accounts":[{"id":"a1"}],"defaultProvider":"openai"});
        assert_eq!(parse_opencodex(&v).unwrap().source, "opencodex");
        assert!(parse_opencodex(&json!(null)).is_none());
        // tolerant on error shape { source, stale, error }
        let v2 = json!({"source":"opencodex","stale":true,"error":{"code":"not_running","message":"down"}});
        assert!(parse_opencodex(&v2).unwrap().stale.unwrap());
    }

    #[test]
    fn events_parse_fixture() {
        let v = json!({"events":[{"id":"e1","type":"session_start","agent":"a","summary":"hi"}],"total":1,"source":"host","sources":["host"],"fetchedAt":"2026-08-12T10:00:00Z"});
        assert_eq!(parse_events(&v).unwrap().events[0].id, "e1");
        // bare array
        let v2 = json!([{"id":"e2","type":"error","agent":"b","summary":"boom"}]);
        assert_eq!(parse_events(&v2).unwrap().events[0].id, "e2");
        assert!(parse_events(&json!(null)).is_none());
    }

    #[test]
    fn agents_parse_fixture() {
        let v = json!({"agents":[{"key":"a::ws"}],"events":[{"id":"e1","type":"command","agent":"a","summary":"x"}],"sessions":[],"source":"host","sources":["host"],"fetchedAt":"2026-08-12T10:00:00Z"});
        assert!(parse_agents(&v).is_some());
        assert!(parse_agents(&json!(null)).is_none());
    }

    #[test]
    fn usage_parse_fixture() {
        let v = json!({
            "oc":{"usageLog":{"path":"~/.opencodex/usage.jsonl","exists":true,"bytes":123,"mtime":"2026-08-12T10:00:00Z"},"today":{"date":"2026-08-12","requests":5,"totalTokens":1000,"byModel":{"gpt-4":5},"byProvider":{"openai":5}}},
            "today":{"date":"2026-08-12","requests":5,"totalTokens":1000,"byModel":{"gpt-4":5},"byProvider":{"openai":5}},
            "notes":[]
        });
        // note: vault uses `ocx` not `oc`; tolerant should still succeed with default
        assert!(parse_usage(&v).is_some());
        let v2 = json!({
            "ocx":{"usageLog":{"path":"~/.opencodex/usage.jsonl","exists":true},"today":{"date":"2026-08-12","requests":2,"totalTokens":200,"byModel":{},"byProvider":{}}},
            "today":{"date":"2026-08-12","requests":2,"totalTokens":200,"byModel":{},"byProvider":{}},
            "notes":["ok"]
        });
        assert_eq!(parse_usage(&v2).unwrap().today.unwrap().requests, 2);
        assert!(parse_usage(&json!(null)).is_none());
    }

    #[test]
    fn version_parse_fixture() {
        let v = json!({"version":"1.2.3","commit":"abc123","builtAt":"2026-08-12T10:00:00Z"});
        assert_eq!(parse_version(&v).unwrap().version, "1.2.3");
        assert_eq!(parse_vault_bridge(&v).unwrap().version, "1.2.3");
        assert!(parse_version(&json!(null)).is_none());
        assert!(parse_version(&json!({})).unwrap().version.is_empty());
    }

    #[test]
    fn all_parsers_tolerant_on_garbage_no_panic() {
        let garbage = json!("garbage");
        let _ = parse_status(&garbage);
        let _ = parse_fleet(&garbage);
        let _ = parse_accounts(&garbage);
        let _ = parse_accounts_overview(&garbage);
        let _ = parse_accounts_status(&garbage);
        let _ = parse_providers(&garbage);
        let _ = parse_clipboard(&garbage);
        let _ = parse_timeline(&garbage);
        let _ = parse_timeline_stats(&garbage);
        let _ = parse_brain(&garbage);
        let _ = parse_crm(&garbage);
        let _ = parse_neon(&garbage);
        let _ = parse_connectors(&garbage);
        let _ = parse_work(&garbage);
        let _ = parse_commander(&garbage);
        let _ = parse_share_sync(&garbage);
        let _ = parse_desktop(&garbage);
        let _ = parse_opencodex(&garbage);
        let _ = parse_events(&garbage);
        let _ = parse_usage(&garbage);
        let _ = parse_version(&garbage);
    }

    #[test]
    fn parsers_deterministic_across_calls() {
        let fixtures: Vec<Value> = vec![
            json!({"services":[]}),
            json!({"ok":true,"peers":[]}),
            json!({"accounts":[]}),
            json!({"items":[]}),
            json!({"events":[]}),
        ];
        for f in fixtures {
            let a = parse_status(&f);
            let b = parse_status(&f);
            assert_eq!(a, b);
            let a = parse_fleet(&f);
            let b = parse_fleet(&f);
            assert_eq!(a, b);
        }
    }
}
