//! Eén poll-actor vervangt de drie concurrerende poll-loops en thread-per-taak.
//!
//! Een enkele thread draait het ritme (vault 5s, ops 15s), fan-out per endpoint
//! met een kort afnamebudget, en publiceert één gedeelde Snapshot. UI leest
//! onder een korte read-lock; mislukte secties behouden hun laatste goede waarde.

use crate::http::{ApiError, Client, HttpClient};
use crate::models::{
    build_agents, build_fleet, build_ops_snapshot, build_providers, day_score_from_agent_summary,
    load_day_score_file, parse_health, watch_dog_path, watcher_events, HealthInfo, OpsSnapshot,
    Snapshot, SUGGESTION_TTL_SECONDS,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const VAULT_POLL_MS: u64 = 5_000;
pub const OPS_POLL_MS: u64 = 15_000;
pub const FETCH_BUDGET_MS: u64 = 8_000;
/// P1: vaste worker-pool voor de vault fan-out (i.p.v. thread-per-endpoint).
pub const POOL_WORKERS: usize = 4;

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
    let poller = Poller::new(shared, vault, ops);
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

/// P1: vaste kleine worker-pool. N threads wachten op een werk-queue en leven
/// zolang de actor leeft — thread-churn → 0 (was 10 threads per poll). Elke
/// job draagt zijn eigen results-sender, zodat batches geïsoleerd blijven
/// (geen kruisende resultaten na een budget-timeout van de vorige poll).
struct Job<C: HttpClient> {
    key: String,
    path: String,
    client: C,
    results: Sender<(String, Result<Value, ApiError>)>,
}

struct WorkerPool<C: HttpClient> {
    queue: Mutex<VecDeque<Job<C>>>,
    cond: Condvar,
    stop: AtomicBool,
}

impl<C: HttpClient> WorkerPool<C> {
    fn spawn(workers: usize) -> Arc<Self> {
        let pool = Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
            stop: AtomicBool::new(false),
        });
        for _ in 0..workers.max(1) {
            let pool = pool.clone();
            std::thread::spawn(move || pool.worker_loop());
        }
        pool
    }

    fn submit(&self, job: Job<C>) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(job);
        self.cond.notify_one();
    }

    /// Stop-signaal: bevrijdt blokkerende workers (actor-shutdown).
    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.cond.notify_all();
    }

    fn worker_loop(&self) {
        loop {
            let job = {
                let mut queue = self.queue.lock().unwrap();
                loop {
                    if self.stop.load(Ordering::Relaxed) {
                        return;
                    }
                    if let Some(job) = queue.pop_front() {
                        break job;
                    }
                    queue = self.cond.wait(queue).unwrap();
                }
            };
            let result = job.client.get_json(&job.path);
            let _ = job.results.send((job.key, result));
        }
    }
}

struct Poller<C: HttpClient> {
    shared: Shared,
    vault: C,
    ops: C,
    pool: Arc<WorkerPool<C>>,
}

impl<C: HttpClient> Poller<C> {
    fn new(shared: Shared, vault: C, ops: C) -> Self {
        Self {
            shared,
            vault,
            ops,
            pool: WorkerPool::spawn(POOL_WORKERS),
        }
    }

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
                Ok(ActorCommand::Shutdown) => {
                    self.pool.stop();
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.pool.stop();
                    break;
                }
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
        if !errors.is_empty() {
            crate::log::log(&format!("vault-poll onvolledig: {}", errors.join(", ")));
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
        // P1: jobs naar de vaste worker-pool (geen thread-per-endpoint);
        // resultaten per batch via een eigen kanaal, alles binnen budget.
        let (tx, rx): (Sender<(String, Result<Value, ApiError>)>, _) = channel();
        let started = Instant::now();
        for (key, path) in paths {
            self.pool.submit(Job {
                key: key.to_string(),
                path: path.to_string(),
                client: self.vault.clone(),
                results: tx.clone(),
            });
        }
        drop(tx);
        let mut results: HashMap<String, Option<Value>> = paths
            .iter()
            .map(|(key, _)| (key.to_string(), None))
            .collect();
        let total = paths.len();
        let mut received = 0usize;
        loop {
            if started.elapsed() > Duration::from_millis(FETCH_BUDGET_MS) {
                break;
            }
            if received >= total {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok((key, Ok(value))) => {
                    results.insert(key, Some(value));
                    received += 1;
                }
                Ok((_, Err(_))) => received += 1,
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
            Err(ApiError::Http(code, _)) => {
                crate::log::log(&format!("ops-poll mislukt (HTTP {code})"));
                (Some(code.to_string()), false)
            }
            Err(ApiError::Transport(reason)) => {
                crate::log::log(&format!("ops-poll offline: {reason}"));
                (Some("offline".to_string()), false)
            }
            Err(ApiError::Blocked(reason)) => {
                crate::log::log(&format!("ops-poll geblokkeerd: {reason}"));
                (Some("geblokkeerd".to_string()), false)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::ApiError;
    use serde_json::json;

    /// De tien vault-paden die fetch_all ophaalt (parity met fetch_all).
    const ALL_PATHS: [&str; 10] = [
        "/status",
        "/accounts/overview",
        "/agents",
        "/agents/events?limit=8",
        "/fleet",
        "/commander/tasks?limit=12",
        "/clipboard",
        "/desktop/status",
        "/share-sync/status",
        "/sessions",
    ];

    /// Q3: mock-HTTP-client zonder netwerk; responses zijn per pad stubbaar
    /// (Arc<Mutex> zodat een test tussen polls de responses kan wisselen).
    #[derive(Clone)]
    struct MockClient {
        responses: Arc<Mutex<HashMap<String, Result<Value, ApiError>>>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }
        fn stub(&self, path: &str, result: Result<Value, ApiError>) {
            self.responses
                .lock()
                .unwrap()
                .insert(path.to_string(), result);
        }
        fn stub_ok(&self, path: &str, value: Value) {
            self.stub(path, Ok(value));
        }
        fn stub_err(&self, path: &str) {
            self.stub(path, Err(ApiError::Transport("offline".into())));
        }
        fn stub_all_ok(&self) {
            for path in ALL_PATHS {
                self.stub(path, Ok(json!({"ok": true})));
            }
        }
        fn stub_all_err(&self) {
            for path in ALL_PATHS {
                self.stub_err(path);
            }
        }
    }

    impl HttpClient for MockClient {
        fn get_json(&self, path: &str) -> Result<Value, ApiError> {
            self.responses
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .unwrap_or_else(|| Err(ApiError::Transport(format!("niet gestubbed: {path}"))))
        }
    }

    fn poller_met() -> Poller<MockClient> {
        Poller::new(Shared::new(), MockClient::new(), MockClient::new())
    }

    fn agents_payload(status: &str) -> Value {
        json!({"agents": [{"key": "a1", "agent": "cursor", "workspace": "commerce", "status": status, "summary": "werkt"}]})
    }

    #[test]
    fn fetch_all_populeert_snapshot_bij_succes() {
        let poller = poller_met();
        poller.vault.stub_all_ok();
        poller.vault.stub_ok(
            "/accounts/overview",
            json!({"revision": 7, "providers": []}),
        );
        poller.vault.stub_ok("/agents", agents_payload("running"));
        poller.vault.stub_ok(
            "/fleet",
            json!({"online": 2, "total": 2, "host": "r1", "stale": false}),
        );
        poller.vault.stub_ok(
            "/commander/tasks?limit=12",
            json!({"tasks": [{"id": "t1", "prompt": "bouw", "status": "queued"}]}),
        );

        poller.poll_vault();

        let snap = poller.shared.snapshot.read().unwrap().clone();
        assert_eq!(snap.revision, 7);
        assert_eq!(snap.agents.len(), 1);
        assert_eq!(snap.tasks.len(), 1);
        assert_eq!(snap.error, None);
        assert!(snap.poll.vault_ok);
    }

    #[test]
    fn fetch_all_meldt_vault_offline_bij_alle_fouten() {
        let poller = poller_met();
        poller.vault.stub_all_err();
        poller.poll_vault();
        let snap = poller.shared.snapshot.read().unwrap().clone();
        assert_eq!(snap.error.as_deref(), Some("vault offline"));
        assert!(!snap.poll.vault_ok);
    }

    #[test]
    fn fetch_all_meldt_gedeeltelijk_bij_deels_fouten() {
        let poller = poller_met();
        poller.vault.stub_all_ok();
        poller.vault.stub_err("/agents");
        poller.poll_vault();
        let snap = poller.shared.snapshot.read().unwrap().clone();
        let err = snap.error.as_deref().unwrap_or("");
        assert!(err.starts_with("gedeeltelijk: "), "fout: {err}");
        assert!(err.contains("agents"));
        // /status ok → deels goed → poll-gezondheid vault ok.
        assert!(snap.poll.vault_ok);
    }

    #[test]
    fn watcher_meldt_statusovergang_eenmalig() {
        // Coalescing: transitie → één suggestie; zelfde status → geen nieuwe.
        let poller = poller_met();
        poller.vault.stub_all_ok();
        poller.vault.stub_ok("/agents", agents_payload("running"));
        poller.poll_vault();
        assert!(
            poller
                .shared
                .snapshot
                .read()
                .unwrap()
                .suggestions
                .is_empty(),
            "eerste poll (nieuwe agent) mag geen suggestie geven"
        );

        poller.vault.stub_ok("/agents", agents_payload("blocked"));
        poller.poll_vault();
        assert_eq!(
            poller.shared.snapshot.read().unwrap().suggestions.len(),
            1,
            "transitie running→blocked geeft één suggestie"
        );

        poller.vault.stub_ok("/agents", agents_payload("blocked"));
        poller.poll_vault();
        assert_eq!(
            poller.shared.snapshot.read().unwrap().suggestions.len(),
            1,
            "zelfde status geeft geen nieuwe suggestie"
        );
    }

    #[test]
    fn poll_ops_vult_snapshot_en_status() {
        let poller = poller_met();
        poller.ops.stub_ok(
            "/api/snapshot",
            json!({"ok": true, "agents": [{"terminal_id": "t1", "agent": "cursor", "agent_status": "working", "terminal_title_stripped": "commerce"}]}),
        );
        poller.poll_ops();
        let ops = poller.shared.ops.read().unwrap().clone();
        assert!(ops.ok);
        assert_eq!(ops.agents.len(), 1);
        assert_eq!(ops.agents[0].terminal_id, "t1");
        let snap = poller.shared.snapshot.read().unwrap().clone();
        assert!(snap.poll.ops_ok);
        assert_eq!(snap.poll.ops_status.as_deref(), Some("ok"));
    }

    #[test]
    fn poll_ops_meldt_http_fout_in_status() {
        let poller = poller_met();
        poller
            .ops
            .stub("/api/snapshot", Err(ApiError::Http(302, "redirect".into())));
        poller.poll_ops();
        let snap = poller.shared.snapshot.read().unwrap().clone();
        assert!(!snap.poll.ops_ok);
        assert_eq!(snap.poll.ops_status.as_deref(), Some("302"));
    }

    #[test]
    fn worker_pool_beperkt_gelijktijdigheid() {
        // P1: de pool draait nooit meer dan N jobs tegelijk (hier 2 workers).
        #[derive(Clone)]
        struct CountingMock {
            active: Arc<AtomicI64>,
            max_seen: Arc<AtomicI64>,
        }
        impl HttpClient for CountingMock {
            fn get_json(&self, _path: &str) -> Result<Value, ApiError> {
                let now = self.active.fetch_add(1, Ordering::SeqCst) + 1;
                self.max_seen.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(40));
                self.active.fetch_sub(1, Ordering::SeqCst);
                Ok(json!({}))
            }
        }

        // De atomics zijn al gedeeld (Arc); de mock zelf is goedkoop te klonen.
        let counters = CountingMock {
            active: Arc::new(AtomicI64::new(0)),
            max_seen: Arc::new(AtomicI64::new(0)),
        };
        let pool = WorkerPool::<CountingMock>::spawn(2);
        let (tx, rx) = channel();
        for i in 0..6 {
            pool.submit(Job {
                key: format!("k{i}"),
                path: "/x".into(),
                client: counters.clone(),
                results: tx.clone(),
            });
        }
        drop(tx);
        let mut done = 0;
        while done < 6 {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(_) => done += 1,
                Err(_) => break,
            }
        }
        assert_eq!(done, 6, "alle jobs moeten afkomen");
        let max = counters.max_seen.load(Ordering::SeqCst);
        assert!(max <= 2, "max gelijktijdigheid {max} > 2");
        pool.stop();
    }
}
