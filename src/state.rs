//! Eén poll-actor vervangt de drie concurrerende poll-loops en thread-per-taak.
//!
//! Een enkele thread draait het ritme (vault 5s, ops 15s, vault-extra 30s,
//! linear 60s, kater 30s), fan-out per endpoint met een kort afnamebudget,
//! en publiceert één gedeelde Snapshot. UI leest onder een korte read-lock;
//! mislukte secties behouden hun laatste goede `last_poll_at` en zetten
//! `last_poll.ok` op false. Sync toont fout, niet een verse attempt-tijd.

use crate::http::{ApiError, Client};
use crate::models::{
    build_agents, build_clipboard_entries, build_commander_tasks, build_container_diff,
    build_crm_deals, build_fleet, build_fleet_nodes, build_herdr_workspaces, build_inbox,
    build_kater_status, build_linear_issues, build_obs_summary, build_ops_snapshot,
    build_providers, build_secrets_meta, build_vault_accounts, day_score_from_agent_summary,
    iso_now, load_day_score_file, parse_health, watch_dog_path, watcher_events, BrainDigest,
    HealthInfo, OpsSnapshot, PollHealth, Snapshot, SUGGESTION_TTL_SECONDS,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const VAULT_POLL_MS: u64 = 5_000;
pub const OPS_POLL_MS: u64 = 15_000;
pub const VAULT_EXTRA_POLL_MS: u64 = 30_000;
pub const LINEAR_POLL_MS: u64 = 60_000;
pub const KATER_POLL_MS: u64 = 30_000;
pub const JCODE_POLL_MS: u64 = 30_000;
pub const BRAIN_POLL_MS: u64 = 120_000;
pub const FETCH_BUDGET_MS: u64 = 8_000;
const PER_ENDPOINT_TIMEOUT_MS: u64 = 2_000;

fn suggestion_allowed(suggestion: &crate::models::Suggestion, mutes: &HashSet<String>) -> bool {
    suggestion.agent.is_empty() || !mutes.contains(&suggestion.agent)
}

fn toast_allowed_during_quiet(quiet: bool, status: &str) -> bool {
    !quiet || status == "error"
}

/// All fan-out keys present and Some. Empty map is not success (no poll ran).
fn all_results_ok(results: &HashMap<String, Option<Value>>) -> bool {
    !results.is_empty() && results.values().all(Option::is_some)
}

/// Vault statuslijn chip. Decode beats HTTP/blocked/offline so JSON failures
/// are not "offline". All-failed HTTP fan-out keeps the HTTP/blocked chip.
fn vault_poll_chip(results: &HashMap<String, Option<Value>>, errors: &[ApiError]) -> String {
    if all_results_ok(results) {
        return "ok".into();
    }
    if errors.iter().any(|err| matches!(err, ApiError::Decode(_))) {
        return "decode".into();
    }
    if let Some(err) = errors
        .iter()
        .find(|err| matches!(err, ApiError::Http(_, _) | ApiError::Blocked(_)))
    {
        return err.statuslijn_chip();
    }
    if !results.is_empty() && results.values().all(Option::is_none) {
        return "offline".into();
    }
    "gedeeltelijk".into()
}

/// Success: verse timestamp + ok. Failure: behoud laatste goede tijd, ok=false.
/// Eerste fail zonder prior success zet de attempt-tijd zodat Sync een rij toont.
fn mark_poll(snap: &mut Snapshot, source: &str, ok: bool, chip: &str) {
    if ok {
        snap.last_poll_at.insert(source.into(), iso_now());
    } else {
        snap.last_poll_at
            .entry(source.into())
            .or_insert_with(iso_now);
    }
    snap.last_poll.insert(
        source.into(),
        if ok {
            PollHealth::ok()
        } else {
            PollHealth::fail(chip)
        },
    );
}

/// Commandokanaal naar de actor (RefreshNow → directe poll, Shutdown).
pub static REFRESH_TX: Mutex<Option<Sender<ActorCommand>>> = Mutex::new(None);

/// Globale laatst-geziene vault-online status (voor doctor zonder actor).
pub static VAULT_ONLINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn vault_online() -> bool {
    VAULT_ONLINE.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorCommand {
    RefreshNow,
    Shutdown,
}

/// Gedeelde staat die alle oppervlakken (tray, bar, panel, ipc) lezen.
#[derive(Clone)]
pub struct Shared {
    pub snapshot: Arc<RwLock<Snapshot>>,
    pub ops: Arc<RwLock<OpsSnapshot>>,
    pub revision: Arc<AtomicI64>,
    pub vault_online: Arc<RwLock<bool>>,
    pub last_error: Arc<RwLock<Option<String>>>,
    pub chat: Arc<RwLock<crate::chat::ChatLog>>,
    pub chat_revision: Arc<AtomicI64>,
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

impl Shared {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(Snapshot::default())),
            ops: Arc::new(RwLock::new(OpsSnapshot::default())),
            revision: Arc::new(AtomicI64::new(0)),
            vault_online: Arc::new(RwLock::new(false)),
            last_error: Arc::new(RwLock::new(None)),
            chat: Arc::new(RwLock::new(crate::chat::chat_log_from_panel(
                &crate::panel_state::load(),
            ))),
            chat_revision: Arc::new(AtomicI64::new(0)),
        }
    }
}

/// Handmatige refresh (bijv. uit de UI): vraagt een directe poll aan de actor.
pub fn refresh_global() {
    if let Some(tx) = REFRESH_TX.lock().unwrap().as_ref() {
        let _ = tx.send(ActorCommand::RefreshNow);
    }
}

/// Start de actor-thread; geeft een Executor-vriendelijke handle terug.
pub fn spawn_actor(shared: Shared, vault: Client, ops: Client) -> ActorHandle {
    let (tx, rx): (Sender<ActorCommand>, Receiver<ActorCommand>) = channel();
    *REFRESH_TX.lock().unwrap() = Some(tx.clone());
    let poller = Poller { shared, vault, ops };
    let handle = std::thread::spawn(move || poller.run(rx));
    ActorHandle {
        tx,
        thread: Some(handle),
    }
}

pub struct ActorHandle {
    tx: Sender<ActorCommand>,
    #[allow(dead_code)]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ActorHandle {
    pub fn refresh(&self) {
        let _ = self.tx.send(ActorCommand::RefreshNow);
    }
}

struct Poller {
    shared: Shared,
    vault: Client,
    ops: Client,
}

impl Poller {
    fn run(self, rx: Receiver<ActorCommand>) {
        let mut next_vault = Instant::now();
        let mut next_ops = Instant::now();
        let mut next_vault_extra = Instant::now();
        let mut next_linear = Instant::now();
        let mut next_kater = Instant::now();
        let mut next_jcode = Instant::now();
        let mut next_brain = Instant::now();
        let mut next_local = Instant::now();
        self.poll_watchdog_into_shared();
        loop {
            let now = Instant::now();
            if now >= next_local {
                self.poll_watchdog_into_shared();
                next_local = now + Duration::from_secs(5);
            }
            if now >= next_vault {
                self.poll_vault();
                next_vault = Instant::now() + Duration::from_millis(VAULT_POLL_MS);
            }
            if now >= next_ops {
                self.poll_ops();
                next_ops = Instant::now() + Duration::from_millis(OPS_POLL_MS);
            }
            if now >= next_vault_extra {
                self.poll_vault_extra();
                next_vault_extra = Instant::now() + Duration::from_millis(VAULT_EXTRA_POLL_MS);
            }
            if now >= next_linear {
                self.poll_linear();
                next_linear = Instant::now() + Duration::from_millis(LINEAR_POLL_MS);
            }
            if now >= next_kater {
                self.poll_kater();
                next_kater = Instant::now() + Duration::from_millis(KATER_POLL_MS);
            }
            if now >= next_jcode {
                self.poll_jcode_memory();
                next_jcode = Instant::now() + Duration::from_millis(JCODE_POLL_MS);
            }
            if now >= next_brain {
                self.poll_brain_digest();
                next_brain = Instant::now() + Duration::from_millis(BRAIN_POLL_MS);
            }
            let deadline = next_vault
                .min(next_ops)
                .min(next_vault_extra)
                .min(next_linear)
                .min(next_kater)
                .min(next_jcode)
                .min(next_brain);
            let timeout = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(1));
            match rx.recv_timeout(timeout) {
                Ok(ActorCommand::RefreshNow) => {
                    self.poll_vault();
                    self.poll_vault_extra();
                    next_vault = Instant::now() + Duration::from_millis(VAULT_POLL_MS);
                    next_vault_extra = Instant::now() + Duration::from_millis(VAULT_EXTRA_POLL_MS);
                }
                Ok(ActorCommand::Shutdown) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// Lokale health + dagscore (bestanden) alvast doorgeven: geen netwerk nodig.
    fn poll_watchdog_into_shared(&self) {
        let text = std::fs::read_to_string(watch_dog_path()).unwrap_or_default();
        let health = if text.is_empty() {
            HealthInfo::default()
        } else {
            parse_health(&text)
        };
        let day_score = load_day_score_file();
        {
            let mut snap = self.shared.snapshot.write().unwrap();
            snap.health = health;
            snap.day_score = day_score;
        }
    }

    fn poll_vault(&self) {
        let (results, fetch_errors) = self.fetch_all();
        let prev_snapshot = self.shared.snapshot.read().unwrap().clone();
        let (mut snap, mut any_ok) = (prev_snapshot.clone(), false);

        if !snap.raw.is_object() {
            snap.raw = Value::Object(Default::default());
        }
        if let Some(status) = results.get("status").cloned().flatten() {
            snap.raw["status"] = status.clone();
            any_ok = true;
        }
        if let Some(providers) = results.get("accounts/overview").cloned().flatten() {
            snap.revision = providers
                .get("revision")
                .and_then(|v| v.as_i64())
                .unwrap_or(snap.revision);
            snap.providers = build_providers(Some(&providers));
            snap.raw["providers"] = providers;
            any_ok = true;
        }
        if let Some(agents) = results.get("agents").cloned().flatten() {
            snap.agents = build_agents(Some(&agents));
            snap.raw["agents"] = agents;
            any_ok = true;
        }
        if let Some(events) = results.get("agent_events").cloned().flatten() {
            snap.events = events
                .get("events")
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            any_ok = true;
        }
        if let Some(fleet) = results.get("fleet").cloned().flatten() {
            snap.fleet = build_fleet(Some(&fleet));
            snap.raw["fleet"] = fleet;
            any_ok = true;
        }
        if let Some(tasks) = results.get("tasks").cloned().flatten() {
            snap.tasks = tasks
                .get("tasks")
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default();
            // commander_tasks mirror
            snap.commander_tasks = build_commander_tasks(Some(&tasks));
            any_ok = true;
        }
        if let Some(clipboard) = results.get("clipboard").cloned().flatten() {
            snap.clipboard = build_clipboard_entries(Some(&clipboard));
            any_ok = true;
        }
        if let Some(desktop) = results.get("desktop/status").cloned().flatten() {
            snap.desktop = desktop
                .as_object()
                .cloned()
                .map(|m| m.into_iter().collect())
                .unwrap_or_default();
            any_ok = true;
        }
        if let Some(share_sync) = results.get("share-sync/status").cloned().flatten() {
            snap.share_sync = share_sync
                .as_object()
                .cloned()
                .map(|m| m.into_iter().collect())
                .unwrap_or_default();
            any_ok = true;
        }

        if snap.day_score.score.is_none() {
            snap.day_score =
                day_score_from_agent_summary(results.get("agents").and_then(|v| v.as_ref()))
                    .unwrap_or_else(load_day_score_file);
        }

        let sessions_payload = results
            .get("sessions")
            .cloned()
            .flatten()
            .or_else(|| results.get("connector_events").cloned().flatten());
        if let Some(payload) = sessions_payload {
            let events: Vec<Value> = payload
                .get("events")
                .and_then(|e| e.as_array())
                .cloned()
                .or_else(|| payload.get("sessions").and_then(|s| s.as_array()).cloned())
                .unwrap_or_default();
            if !events.is_empty() {
                snap.events.extend(events.clone());
                snap.raw["sessions"] = Value::Array(events);
            }
            any_ok = true;
        }

        // Per-agent mute: gedempte agents leveren geen toast en hun oude
        // inbox-suggesties worden direct uit de snapshot verwijderd.
        let mutes = crate::mutes::load();
        let fresh: Vec<_> = watcher_events(&prev_snapshot, &snap)
            .into_iter()
            .filter(|suggestion| suggestion_allowed(suggestion, &mutes))
            .collect();
        if !fresh.is_empty() {
            if let Some((title, body, status)) = crate::models::coalesce_toasts(&fresh) {
                // Rustige uren dempen alleen niet-kritieke meldingen; FOUT
                // blijft altijd zichtbaar. De inbox blijft gevuld.
                let quiet = crate::quiet::quiet_window()
                    .map(|window| crate::quiet::in_quiet_hours(&window))
                    .unwrap_or(false);
                if toast_allowed_during_quiet(quiet, status) {
                    crate::notify::notify(&title, &body, status);
                }
            }
            snap.suggestions.retain(|suggestion| {
                suggestion.fresh(SUGGESTION_TTL_SECONDS)
                    && !mutes.contains(&suggestion.agent)
                    && !fresh.iter().any(|new| new.key == suggestion.key)
            });
            snap.suggestions.extend(fresh.clone());
            snap.suggestions.truncate(6);
        } else {
            snap.suggestions.retain(|suggestion| {
                suggestion.fresh(SUGGESTION_TTL_SECONDS) && suggestion_allowed(suggestion, &mutes)
            });
        }

        snap.fetched_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut errors: Vec<String> = Vec::new();
        for (key, value) in &results {
            if value.is_none() {
                errors.push(key.clone());
            }
        }
        // Availability stays any_ok (at least one endpoint). Sync freshness
        // requires every expected vault response — partial is fout, not verse.
        let vault_ok = all_results_ok(&results);
        let vault_chip = vault_poll_chip(&results, &fetch_errors);
        snap.error = match vault_chip.as_str() {
            "ok" => None,
            "offline" => Some("vault offline".to_string()),
            "decode" => Some("vault decode".to_string()),
            "geblokkeerd" => Some("vault geblokkeerd".to_string()),
            _ => Some(format!("gedeeltelijk: {}", errors.join(", "))),
        };
        mark_poll(&mut snap, "vault", vault_ok, &vault_chip);

        {
            let mut vault_online = self.shared.vault_online.write().unwrap();
            *vault_online = any_ok;
        }
        VAULT_ONLINE.store(any_ok, Ordering::Relaxed);
        {
            let mut last_error = self.shared.last_error.write().unwrap();
            *last_error = if errors.is_empty() {
                None
            } else {
                Some(errors.join(", "))
            };
        }
        {
            let mut current = self.shared.snapshot.write().unwrap();
            *current = snap;
        }
        crate::tray::update_from(&self.shared.snapshot);
        {
            let actual = self.shared.snapshot.read().unwrap().revision;
            let loaded = self.shared.revision.load(Ordering::Relaxed);
            self.shared
                .revision
                .store(actual.max(loaded), Ordering::Relaxed);
        }
    }

    fn poll_vault_extra(&self) {
        let results = self.fetch_vault_extra();
        let mut snap = self.shared.snapshot.write().unwrap();

        if let Some(val) = results.get("vault_accounts").cloned().flatten() {
            snap.vault_accounts = build_vault_accounts(Some(&val));
        }
        if let Some(val) = results.get("crm_deals").cloned().flatten() {
            snap.crm_deals = build_crm_deals(Some(&val));
        }
        if let Some(val) = results.get("secrets_meta").cloned().flatten() {
            snap.secrets_meta = build_secrets_meta(Some(&val));
        }
        if let Some(val) = results.get("containers").cloned().flatten() {
            snap.containers = build_container_diff(Some(&val));
        }
        if let Some(val) = results.get("inbox").cloned().flatten() {
            snap.inbox = build_inbox(Some(&val));
        }
        if let Some(val) = results.get("fleet_nodes").cloned().flatten() {
            snap.fleet_nodes = build_fleet_nodes(Some(&val));
        }
        if let Some(val) = results.get("herdr_workspaces").cloned().flatten() {
            snap.herdr_workspaces = build_herdr_workspaces(Some(&val));
        }
        if let Some(val) = results.get("commander_tasks").cloned().flatten() {
            snap.commander_tasks = build_commander_tasks(Some(&val));
            // sync legacy tasks
            if let Some(arr) = val.get("tasks").and_then(|v| v.as_array()) {
                snap.tasks = arr.clone();
            }
        }
        if let Some(val) = results.get("clipboard_extra").cloned().flatten() {
            snap.clipboard = build_clipboard_entries(Some(&val));
        }
        if let Some(val) = results.get("observability").cloned().flatten() {
            snap.observability = build_obs_summary(Some(&val));
        }
        if let Some(val) = results.get("brain").cloned().flatten() {
            if let Some(parsed) = crate::vault_bridge::parse_brain(&val) {
                snap.brain = parsed;
            }
        }

        mark_poll(
            &mut snap,
            "vault_extra",
            all_results_ok(&results),
            if all_results_ok(&results) {
                "ok"
            } else {
                "gedeeltelijk"
            },
        );
    }

    fn poll_linear(&self) {
        let results = self.fetch_linear();
        let mut snap = self.shared.snapshot.write().unwrap();
        if let Some(val) = results.get("linear").cloned().flatten() {
            snap.linear_issues = build_linear_issues(Some(&val));
            mark_poll(&mut snap, "linear", true, "ok");
        } else {
            let linear_api = std::env::var("LINEAR_API")
                .or_else(|_| std::env::var("CHEFBAR_LINEAR_API"))
                .ok();
            if linear_api
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                mark_poll(&mut snap, "linear", false, "offline");
            }
        }
    }

    fn poll_kater(&self) {
        let results = self.fetch_kater();
        let mut snap = self.shared.snapshot.write().unwrap();
        if let Some(val) = results.get("kater").cloned().flatten() {
            snap.kater_status = build_kater_status(Some(&val));
            mark_poll(&mut snap, "kater", true, "ok");
        } else {
            let has_kater = crate::config::global_profile()
                .kater_workspace
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if has_kater {
                mark_poll(&mut snap, "kater", false, "offline");
            }
        }
    }

    fn poll_jcode_memory(&self) {
        let bind = std::env::var("CHEFBAR_JCODE_MEMORY_BIND")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "100.111.187.17:7643".into());
        let host = bind
            .rsplit_once(':')
            .map(|(h, _)| h.to_string())
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| bind.clone());
        let shared = self.shared.clone();
        std::thread::spawn(move || {
            let addr = bind.parse::<SocketAddr>().ok().or_else(|| {
                bind.to_socket_addrs()
                    .ok()
                    .and_then(|mut addrs| addrs.next())
            });
            let online = match addr {
                Some(addr) => {
                    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(400)).is_ok()
                }
                None => false,
            };
            let mut snap = shared.snapshot.write().unwrap();
            snap.jcode_memory = crate::models::JcodeMemoryStatus {
                online,
                host,
                bind,
                status: if online {
                    "online".into()
                } else {
                    "offline".into()
                },
            };
            mark_poll(
                &mut snap,
                "jcode_memory",
                online,
                if online { "ok" } else { "offline" },
            );
        });
    }

    /// Read-only brain-digest ophalen (HTTP of lokaal vault-pad). Geen
    /// endpoint ingesteld → niets doen, geen error-spam. Fail-closed in brain.rs.
    fn poll_brain_digest(&self) {
        let profile = crate::config::global_profile();
        if profile.brain_api.is_none() && crate::brain::no_local_digest() {
            let mut snap = self.shared.snapshot.write().unwrap();
            snap.brain_digest = BrainDigest::default();
            snap.last_poll_at.insert("brain_digest".into(), iso_now());
            return;
        }
        let digest = crate::brain::fetch_digest(profile);
        let mut snap = self.shared.snapshot.write().unwrap();
        snap.brain_digest = digest;
        snap.last_poll_at.insert("brain_digest".into(), iso_now());
    }

    fn fetch_all(&self) -> (HashMap<String, Option<Value>>, Vec<ApiError>) {
        let paths: &[(&str, &str)] = &[
            ("status", "/status"),
            ("accounts/overview", "/accounts/overview"),
            ("agents", "/agents"),
            ("agent_events", "/agents/events?limit=8"),
            ("fleet", "/fleet"),
            ("tasks", "/commander/tasks?limit=12"),
            ("clipboard", "/clipboard"),
            ("desktop/status", "/desktop/status"),
            ("share-sync/status", "/share-sync/status"),
            ("sessions", "/sessions"),
        ];
        let (tx, rx): (Sender<(String, Result<Value, ApiError>)>, _) = channel();
        let client = self
            .vault
            .clone()
            .with_timeout(Duration::from_millis(PER_ENDPOINT_TIMEOUT_MS));
        let started = Instant::now();
        for (key, path) in paths {
            let (key, path, tx, client) = (
                key.to_string(),
                path.to_string(),
                tx.clone(),
                client.clone(),
            );
            std::thread::spawn(move || {
                let result = client.get_json(&path);
                let _ = tx.send((key, result));
            });
        }
        drop(tx);
        let mut results: HashMap<String, Option<Value>> = paths
            .iter()
            .map(|(key, _)| (key.to_string(), None))
            .collect();
        let mut errors: Vec<ApiError> = Vec::new();
        loop {
            if started.elapsed() > Duration::from_millis(FETCH_BUDGET_MS) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok((key, Ok(value))) => {
                    results.insert(key, Some(value));
                }
                Ok((_, Err(err))) => {
                    errors.push(err);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        (results, errors)
    }

    fn fetch_vault_extra(&self) -> HashMap<String, Option<Value>> {
        let paths: &[(&str, &str)] = &[
            ("vault_accounts", "/accounts"),
            ("crm_deals", "/crm/deals"),
            ("secrets_meta", "/secrets/meta"),
            ("containers", "/containers"),
            ("inbox", "/inbox"),
            ("fleet_nodes", "/fleet/nodes"),
            ("herdr_workspaces", "/herdr/workspaces"),
            ("commander_tasks", "/commander/tasks?limit=20"),
            ("clipboard_extra", "/clipboard"),
            ("observability", "/observability/summary"),
            ("brain", "/brain"),
        ];
        Self::fanout(&self.vault, paths)
    }

    fn fetch_linear(&self) -> HashMap<String, Option<Value>> {
        let base = std::env::var("LINEAR_API")
            .or_else(|_| std::env::var("CHEFBAR_LINEAR_API"))
            .or_else(|_| std::env::var("LINEAR_API_URL"))
            .unwrap_or_default();
        let base = base.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return HashMap::new();
        }
        let policy = crate::policy::EndpointPolicy::default();
        let client =
            Client::new(&base, policy).with_timeout(Duration::from_millis(PER_ENDPOINT_TIMEOUT_MS));
        let paths: &[(&str, &str)] = &[("linear", "/issues?limit=20")];
        Self::fanout(&client, paths)
    }

    fn fetch_kater(&self) -> HashMap<String, Option<Value>> {
        let base = crate::config::global_profile()
            .kater_workspace
            .clone()
            .unwrap_or_default();
        let base = base.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return HashMap::new();
        }
        // katerWorkspace is vaak https://kater.../agents/ — strip trailing path voor status
        let policy = crate::policy::EndpointPolicy::default();
        let client =
            Client::new(&base, policy).with_timeout(Duration::from_millis(PER_ENDPOINT_TIMEOUT_MS));
        let paths: &[(&str, &str)] = &[("kater", "/api/status"), ("kater_alt", "/status")];
        let mut res = Self::fanout(&client, paths);
        // normaliseer: kater_alt -> kater fallback
        if res.get("kater").and_then(|v| v.as_ref()).is_none() {
            if let Some(val) = res.remove("kater_alt").flatten() {
                res.insert("kater".into(), Some(val));
            }
        } else {
            res.remove("kater_alt");
        }
        res
    }

    fn fanout(client: &Client, paths: &[(&str, &str)]) -> HashMap<String, Option<Value>> {
        let (tx, rx): (Sender<(String, Result<Value, ApiError>)>, _) = channel();
        let client = client
            .clone()
            .with_timeout(Duration::from_millis(PER_ENDPOINT_TIMEOUT_MS));
        let started = Instant::now();
        for (key, path) in paths {
            let (key, path, tx, client) = (
                key.to_string(),
                path.to_string(),
                tx.clone(),
                client.clone(),
            );
            std::thread::spawn(move || {
                let result = client.get_json(&path);
                let _ = tx.send((key, result));
            });
        }
        drop(tx);
        let mut results: HashMap<String, Option<Value>> = paths
            .iter()
            .map(|(key, _)| (key.to_string(), None))
            .collect();
        loop {
            if started.elapsed() > Duration::from_millis(FETCH_BUDGET_MS) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok((key, Ok(value))) => {
                    results.insert(key, Some(value));
                }
                Ok((_, Err(_))) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        results
    }

    fn poll_ops(&self) {
        let payload = match self
            .ops
            .clone()
            .with_timeout(Duration::from_millis(PER_ENDPOINT_TIMEOUT_MS))
            .get_json("/api/snapshot")
        {
            Ok(payload) => payload,
            Err(err) => {
                let mut snap = self.shared.snapshot.write().unwrap();
                mark_poll(&mut snap, "ops", false, &err.statuslijn_chip());
                return;
            }
        };
        let ops = build_ops_snapshot(Some(&payload));
        {
            let mut current = self.shared.ops.write().unwrap();
            *current = ops;
        }
        crate::chat::refresh_persisted_pin(&self.shared);
        {
            let mut snap = self.shared.snapshot.write().unwrap();
            mark_poll(&mut snap, "ops", true, "ok");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Suggestion, SuggestionKind};

    fn suggestion(agent: &str) -> Suggestion {
        Suggestion {
            key: "agent-status".into(),
            agent: agent.into(),
            title: "status".into(),
            meta: String::new(),
            stamp: "HULP".into(),
            action_label: "Open".into(),
            kind: SuggestionKind::FocusAgent(agent.into()),
            created_unix: 0,
        }
    }

    #[test]
    fn mute_filter_laat_niet_agent_meldingen_door() {
        let mutes = HashSet::from(["cursor::commerce".to_string()]);
        assert!(suggestion_allowed(&suggestion(""), &mutes));
        assert!(suggestion_allowed(&suggestion("codex::vault"), &mutes));
        assert!(!suggestion_allowed(&suggestion("cursor::commerce"), &mutes));
    }

    #[test]
    fn quiet_hours_laten_alleen_fouten_door() {
        assert!(toast_allowed_during_quiet(false, "ok"));
        assert!(toast_allowed_during_quiet(true, "error"));
        assert!(!toast_allowed_during_quiet(true, "warn"));
        assert!(!toast_allowed_during_quiet(true, "ok"));
    }

    #[test]
    fn mark_poll_keeps_last_good_time_on_failure() {
        let mut snap = Snapshot::default();
        mark_poll(&mut snap, "linear", true, "ok");
        let good = snap.last_poll_at.get("linear").cloned().unwrap();
        assert_eq!(snap.last_poll.get("linear"), Some(&PollHealth::ok()));
        mark_poll(&mut snap, "linear", false, "offline");
        assert_eq!(snap.last_poll_at.get("linear"), Some(&good));
        assert_eq!(
            snap.last_poll.get("linear"),
            Some(&PollHealth::fail("offline"))
        );
    }

    #[test]
    fn mark_poll_first_failure_still_records_a_row() {
        let mut snap = Snapshot::default();
        mark_poll(&mut snap, "ops", false, "offline");
        assert!(snap.last_poll_at.contains_key("ops"));
        assert_eq!(
            snap.last_poll.get("ops"),
            Some(&PollHealth::fail("offline"))
        );
    }

    #[test]
    fn vault_partial_failure_is_not_a_fresh_poll() {
        let mut results = HashMap::new();
        results.insert("health".into(), Some(Value::Null));
        results.insert("agents".into(), None);
        assert!(!all_results_ok(&results));
        let mut snap = Snapshot::default();
        mark_poll(&mut snap, "vault", all_results_ok(&results), "gedeeltelijk");
        assert_eq!(
            snap.last_poll.get("vault"),
            Some(&PollHealth::fail("gedeeltelijk"))
        );
    }

    #[test]
    fn vault_poll_chip_decode_beats_offline_and_partial() {
        let mut mixed = HashMap::new();
        mixed.insert("status".into(), Some(Value::Null));
        mixed.insert("agents".into(), None);
        let decode = vec![ApiError::Decode("JSON-parse faalde".into())];
        assert_eq!(vault_poll_chip(&mixed, &decode), "decode");

        let mut all_failed = HashMap::new();
        all_failed.insert("status".into(), None);
        all_failed.insert("agents".into(), None);
        assert_eq!(vault_poll_chip(&all_failed, &decode), "decode");
        assert_eq!(
            vault_poll_chip(&all_failed, &[ApiError::Transport("down".into())]),
            "offline"
        );
        assert_eq!(vault_poll_chip(&mixed, &[]), "gedeeltelijk");
        assert_eq!(
            vault_poll_chip(&all_failed, &[ApiError::Http(302, "access".into())]),
            "302"
        );
    }

    #[test]
    fn vault_extra_partial_failure_is_not_a_fresh_poll() {
        let mut results = HashMap::new();
        results.insert("inbox".into(), Some(Value::Null));
        results.insert("fleet_nodes".into(), None);
        results.insert("brain".into(), Some(Value::Null));
        assert!(!all_results_ok(&results));
        let mut snap = Snapshot::default();
        mark_poll(
            &mut snap,
            "vault_extra",
            all_results_ok(&results),
            "gedeeltelijk",
        );
        assert_eq!(
            snap.last_poll.get("vault_extra"),
            Some(&PollHealth::fail("gedeeltelijk"))
        );
    }

    #[test]
    fn all_results_ok_requires_every_expected_key() {
        let mut results = HashMap::new();
        results.insert("health".into(), Some(Value::Null));
        results.insert("agents".into(), Some(Value::Null));
        assert!(all_results_ok(&results));
        assert!(!all_results_ok(&HashMap::new()));
    }
}
