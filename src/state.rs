//! Eén poll-actor vervangt de drie concurrerende poll-loops en thread-per-taak.
//!
//! Een enkele thread draait het ritme (vault 5s, ops 15s), fan-out per endpoint
//! met een kort afnamebudget, en publiceert één gedeelde Snapshot. UI leest
//! onder een korte read-lock; mislukte secties behouden hun laatste goede waarde.

use crate::http::{ApiError, Client};
use crate::models::{
    build_agents, build_fleet, build_ops_snapshot, build_providers, day_score_from_agent_summary,
    load_day_score_file, parse_health, watch_dog_path, watcher_events, HealthInfo, OpsSnapshot,
    Snapshot, SUGGESTION_TTL_SECONDS,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const VAULT_POLL_MS: u64 = 5_000;
pub const OPS_POLL_MS: u64 = 15_000;
pub const FETCH_BUDGET_MS: u64 = 8_000;

/// Commandokanaal naar de actor (RefreshNow → directe poll, Shutdown).
pub static REFRESH_TX: Mutex<Option<Sender<ActorCommand>>> = Mutex::new(None);

/// Globale laatst-geziene vault-online status (voor doctor zonder actor).
pub static VAULT_ONLINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn vault_online() -> bool {
    VAULT_ONLINE.load(Ordering::Relaxed)
}

/// Laatste poll-tijdstip van de actor (Unix; doctor-observability, E1).
pub static LAST_POLL_UNIX: AtomicI64 = AtomicI64::new(0);

/// Laatste ops-poll: online-flag + statusregel (ok / HTTP-code / offline).
pub static OPS_ONLINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static OPS_STATUS: Mutex<Option<String>> = Mutex::new(None);

/// Poll-gezondheid als compacte regel — voor doctor en panel zonder
/// Shared-handle (parity met `VAULT_ONLINE`):
/// "poll 4s · vault ok · ops 302".
pub fn last_poll_label() -> String {
    crate::models::PollHealth {
        last_poll_unix: LAST_POLL_UNIX.load(Ordering::Relaxed),
        vault_ok: vault_online(),
        ops_ok: OPS_ONLINE.load(Ordering::Relaxed),
        ops_status: OPS_STATUS.lock().unwrap().clone(),
    }
    .label()
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
        let mut next_local = Instant::now();
        self.poll_watchdog_into_shared();
        loop {
            // Begin met onmiddellijke eerste polls.
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
            let deadline = next_vault.min(next_ops);
            let timeout = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(1));
            match rx.recv_timeout(timeout) {
                Ok(ActorCommand::RefreshNow) => {
                    self.poll_vault();
                    next_vault = Instant::now() + Duration::from_millis(VAULT_POLL_MS);
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
        let results = self.fetch_all();
        let prev_snapshot = self.shared.snapshot.read().unwrap().clone();
        let (mut snap, mut any_ok) = (prev_snapshot.clone(), false);

        // Laatste-goede-waarde per sectie; de rest van het beeld blijft staan.
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
            any_ok = true;
        }
        if let Some(clipboard) = results.get("clipboard").cloned().flatten() {
            snap.clipboard = clipboard
                .get("items")
                .and_then(|i| i.as_array())
                .cloned()
                .unwrap_or_default();
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

        // Dagscore: bestand eerst; valt terug op de chef-eval agent summary
        // uit /agents (parity met de Python load_day_score).
        if snap.day_score.score.is_none() {
            snap.day_score =
                day_score_from_agent_summary(results.get("agents").and_then(|v| v.as_ref()))
                    .unwrap_or_else(load_day_score_file);
        }

        // Cloudflare Access sessies uit de connector/API-feed.
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
                // sessies worden elders gerankt op basis van events; hier alleen cache.
                snap.raw["sessions"] = Value::Array(events);
            }
            any_ok = true;
        }

        // Watcher-suggesties (parity): transities → één rustige toast + snapshot-feed.
        let fresh: Vec<_> = watcher_events(&prev_snapshot, &snap);
        if !fresh.is_empty() {
            if let Some((title, body, status)) = crate::models::coalesce_toasts(&fresh) {
                crate::notify::notify(&title, &body, status);
            }
            snap.suggestions.retain(|s| {
                s.fresh(SUGGESTION_TTL_SECONDS) && !fresh.iter().any(|n| n.key == s.key)
            });
            snap.suggestions.extend(fresh.clone());
            snap.suggestions.truncate(6);
        }

        snap.fetched_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        // Poll-gezondheid (E1): vault-secties ok? Laatste poll = dit moment.
        snap.poll.last_poll_unix = snap.fetched_at_unix;
        snap.poll.vault_ok = any_ok;

        let mut errors: Vec<String> = Vec::new();
        for (key, value) in &results {
            if value.is_none() {
                errors.push(key.clone());
            }
        }
        snap.error = if errors.is_empty() {
            None
        } else if errors.len() == results.len() {
            Some("vault offline".to_string())
        } else {
            Some(format!("gedeeltelijk: {}", errors.join(", ")))
        };

        {
            let mut vault_online = self.shared.vault_online.write().unwrap();
            *vault_online = any_ok;
        }
        VAULT_ONLINE.store(any_ok, Ordering::Relaxed);
        LAST_POLL_UNIX.store(snap.fetched_at_unix, Ordering::Relaxed);
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

    fn fetch_all(&self) -> HashMap<String, Option<Value>> {
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
        // Fan-out over een klein threaddeel: per endpoint kort, alles binnen budget.
        let (tx, rx): (Sender<(String, Result<Value, ApiError>)>, _) = channel();
        let client = self.vault.clone();
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
        let result = self.ops.get_json("/api/snapshot");
        // Status + snapshot; bij fout behouden we de laatste goede ops-snapshot,
        // maar de poll-gezondheid wordt wél bijgewerkt (E1: "ops 302").
        let (ops_status, ops_ok) = match &result {
            Ok(payload) => {
                let ops = build_ops_snapshot(Some(payload));
                *self.shared.ops.write().unwrap() = ops;
                (Some("ok".to_string()), true)
            }
            Err(ApiError::Http(code, _)) => (Some(code.to_string()), false),
            Err(ApiError::Transport(_)) => (Some("offline".to_string()), false),
            Err(ApiError::Blocked(_)) => (Some("geblokkeerd".to_string()), false),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        {
            let mut snap = self.shared.snapshot.write().unwrap();
            snap.poll.last_poll_unix = now;
            snap.poll.ops_ok = ops_ok;
            snap.poll.ops_status = ops_status.clone();
        }
        OPS_ONLINE.store(ops_ok, Ordering::Relaxed);
        *OPS_STATUS.lock().unwrap() = ops_status;
    }
}
