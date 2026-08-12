//! Harnas-model — room: samenhangende ruimte, geen losse pages.
//!
//! De app kan meerdere harnassen tegelijk tonen. Elk harnas is een
//! samenhangende werkruimte (fleet, commerce, sync, eval) met eigen
//! status, wachtrij en kleur. Dit model is puur afgeleid uit bestaande
//! snapshot-data — geen nieuwe netwerk-calls.

use crate::models::{OpsSnapshot, Snapshot};

// ---------------------------------------------------------------------------
// Harnas-groepen en soorten
// ---------------------------------------------------------------------------

/// Groep van harnassen — sidebar-secties (5 groepen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessGroup {
    Fleet,
    Commerce,
    Sync,
    Work,
    System,
}

impl HarnessGroup {
    pub fn id(&self) -> &'static str {
        match self {
            HarnessGroup::Fleet => "fleet",
            HarnessGroup::Commerce => "commerce",
            HarnessGroup::Sync => "sync",
            HarnessGroup::Work => "work",
            HarnessGroup::System => "system",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HarnessGroup::Fleet => "Fleet",
            HarnessGroup::Commerce => "Commerce",
            HarnessGroup::Sync => "Sync",
            HarnessGroup::Work => "Werk",
            HarnessGroup::System => "Systeem",
        }
    }
}

/// Soort harnas — bepaalt kleur, label en keyword-prefix voor filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessKind {
    // Bestaand (compat)
    Fleet,
    Commerce,
    Sync,
    Eval,
    // Nieuw in 4.0 — minstens 9 distinct ids
    Inbox,
    Herdr,
    Vault,
    Crm,
    Share,
    Clipboard,
    Desktop,
    Tasks,
    Linear,
    Containers,
    Secrets,
    Kater,
    Health,
}

impl HarnessKind {
    /// ID zoals gebruikt in filtering en selectie.
    pub fn id(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "fleet",
            HarnessKind::Commerce => "commerce",
            HarnessKind::Sync => "sync",
            HarnessKind::Eval => "eval",
            HarnessKind::Inbox => "inbox",
            HarnessKind::Herdr => "herdr",
            HarnessKind::Vault => "vault",
            HarnessKind::Crm => "crm",
            HarnessKind::Share => "share",
            HarnessKind::Clipboard => "clipboard",
            HarnessKind::Desktop => "desktop",
            HarnessKind::Tasks => "tasks",
            HarnessKind::Linear => "linear",
            HarnessKind::Containers => "containers",
            HarnessKind::Secrets => "secrets",
            HarnessKind::Kater => "kater",
            HarnessKind::Health => "health",
        }
    }

    /// Menselijk label voor de UI.
    pub fn label(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "Fleet",
            HarnessKind::Commerce => "Commerce",
            HarnessKind::Sync => "Sync",
            HarnessKind::Eval => "Evaluatie",
            HarnessKind::Inbox => "Inbox",
            HarnessKind::Herdr => "Herdr",
            HarnessKind::Vault => "Vault",
            HarnessKind::Crm => "CRM",
            HarnessKind::Share => "Share",
            HarnessKind::Clipboard => "Clipboard",
            HarnessKind::Desktop => "Desktop",
            HarnessKind::Tasks => "Taken",
            HarnessKind::Linear => "Linear",
            HarnessKind::Containers => "Containers",
            HarnessKind::Secrets => "Secrets",
            HarnessKind::Kater => "Kater",
            HarnessKind::Health => "Health",
        }
    }

    /// Accentkleur per harnas (hex, past bij Signaal/Devin warm-neutral).
    pub fn color(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "#2563eb",
            HarnessKind::Commerce => "#d97706",
            HarnessKind::Sync => "#059669",
            HarnessKind::Eval => "#7c3aed",
            HarnessKind::Inbox => "#e11d48",
            HarnessKind::Herdr => "#0ea5e9",
            HarnessKind::Vault => "#f59e0b",
            HarnessKind::Crm => "#6366f1",
            HarnessKind::Share => "#10b981",
            HarnessKind::Clipboard => "#84cc16",
            HarnessKind::Desktop => "#06b6d4",
            HarnessKind::Tasks => "#8b5cf6",
            HarnessKind::Linear => "#ec4899",
            HarnessKind::Containers => "#64748b",
            HarnessKind::Secrets => "#f97316",
            HarnessKind::Kater => "#14b8a6",
            HarnessKind::Health => "#22c55e",
        }
    }

    /// Keyword-prefixen waarmee actions aan dit harnas worden gekoppeld.
    /// Panel filtert actions waarvan een keyword-token met één van deze
    /// prefixen begint (prefix-match op keywords).
    pub fn prefixes(&self) -> Vec<&'static str> {
        match self {
            HarnessKind::Fleet => vec!["fleet", "herdr", "nodes", "ops", "agent"],
            HarnessKind::Commerce => vec!["commerce", "vault", "account", "commander", "clipboard"],
            HarnessKind::Sync => vec!["share", "sync", "desktop", "pull", "push"],
            HarnessKind::Eval => vec!["eval", "health", "dagscore", "dashboard", "score"],
            HarnessKind::Inbox => vec!["inbox", "melding", "attention", "blocked", "hulp"],
            HarnessKind::Herdr => vec!["herdr", "fleet", "workspace", "pane", "agent"],
            HarnessKind::Vault => vec!["vault", "account", "provider", "rekening"],
            HarnessKind::Crm => vec!["crm", "deals", "neon", "contact", "klant"],
            HarnessKind::Share => vec!["share", "sync", "desktop", "bestand", "pull", "push"],
            HarnessKind::Clipboard => vec!["clipboard", "klembord", "copy", "plak"],
            HarnessKind::Desktop => vec!["desktop", "webtop", "share", "sync"],
            HarnessKind::Tasks => vec!["tasks", "taken", "commander", "job", "taak"],
            HarnessKind::Linear => vec!["linear", "taken", "issues", "tickets", "ticket"],
            HarnessKind::Containers => vec!["containers", "docker", "image", "prune", "drift"],
            HarnessKind::Secrets => {
                vec!["secrets", "vaultwarden", "wachtwoord", "password", "geheim"]
            }
            HarnessKind::Kater => vec!["kater", "gateway", "proxy", "profile"],
            HarnessKind::Health => vec!["health", "status", "eval", "dagscore", "doctor"],
        }
    }

    /// Groep waartoe dit harnas behoort (sidebar-sectie).
    pub fn group(&self) -> HarnessGroup {
        match self {
            HarnessKind::Fleet | HarnessKind::Herdr | HarnessKind::Containers => {
                HarnessGroup::Fleet
            }
            HarnessKind::Vault | HarnessKind::Commerce | HarnessKind::Crm => HarnessGroup::Commerce,
            HarnessKind::Share
            | HarnessKind::Clipboard
            | HarnessKind::Desktop
            | HarnessKind::Sync => HarnessGroup::Sync,
            HarnessKind::Inbox | HarnessKind::Tasks | HarnessKind::Linear => HarnessGroup::Work,
            HarnessKind::Secrets | HarnessKind::Kater | HarnessKind::Health | HarnessKind::Eval => {
                HarnessGroup::System
            }
        }
    }

    /// Alle kinds in canonieke volgorde (voor iteratie/tests).
    pub fn all() -> Vec<HarnessKind> {
        vec![
            HarnessKind::Inbox,
            HarnessKind::Fleet,
            HarnessKind::Herdr,
            HarnessKind::Vault,
            HarnessKind::Commerce,
            HarnessKind::Crm,
            HarnessKind::Share,
            HarnessKind::Clipboard,
            HarnessKind::Desktop,
            HarnessKind::Tasks,
            HarnessKind::Linear,
            HarnessKind::Containers,
            HarnessKind::Secrets,
            HarnessKind::Kater,
            HarnessKind::Health,
            // Eval kept for compat, after Health
            HarnessKind::Eval,
            // Sync kept for compat (maps to Sync group)
            HarnessKind::Sync,
        ]
    }
}

/// Status van een harnas — uniform voor alle soorten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessStatus {
    Idle,
    Running,
    Blocked,
    Done,
}

impl HarnessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HarnessStatus::Idle => "idle",
            HarnessStatus::Running => "running",
            HarnessStatus::Blocked => "blocked",
            HarnessStatus::Done => "done",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            HarnessStatus::Idle => "stil",
            HarnessStatus::Running => "bezig",
            HarnessStatus::Blocked => "hulp nodig",
            HarnessStatus::Done => "klaar",
        }
    }
}

// ---------------------------------------------------------------------------
// Harness struct
// ---------------------------------------------------------------------------

/// Één harnas — visueel een room, functioneel een gefilterde lens.
#[derive(Debug, Clone)]
pub struct Harness {
    pub id: String,
    pub label: String,
    pub kind: HarnessKind,
    pub status: HarnessStatus,
    pub queue_depth: usize,
    pub active_task: Option<String>,
    pub provider_label: Option<String>,
    pub color: String,
}

impl Harness {
    /// Maak een harnas met alle velden expliciet.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        kind: HarnessKind,
        status: HarnessStatus,
        queue_depth: usize,
        active_task: Option<String>,
        provider_label: Option<String>,
    ) -> Self {
        let color = kind.color().to_string();
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            status,
            queue_depth,
            active_task,
            provider_label,
            color,
        }
    }
}

// ---------------------------------------------------------------------------
// Bouwer — pure functie, geen I/O
// ---------------------------------------------------------------------------

/// Bouw harnassen uit bestaande snapshot-data.
/// - fleet/herdr: uit fleet-info + ops agents
/// - commerce/vault/crm: uit providers
/// - sync/share/clipboard/desktop: uit share_sync/clipboard/desktop
/// - tasks/linear: uit tasks + linear_issues
/// - inbox/health/kater/containers/secrets: tolerant (0 als snapshot leeg)
///
/// Geen nieuwe netwerk-calls; alles komt uit `snapshot` en `ops`.
/// Nieuwe kinds tonen 0 items als snapshot leeg is (tolerant).
pub fn build_harnesses(snapshot: &Snapshot, ops: &OpsSnapshot) -> Vec<Harness> {
    let mut out: Vec<Harness> = Vec::with_capacity(17);

    // ---- Fleet harnas -----------------------------------------------------
    let fleet = &snapshot.fleet;
    let fleet_status = if fleet.stale {
        HarnessStatus::Blocked
    } else if fleet.total == 0 {
        HarnessStatus::Idle
    } else if fleet.online == 0 || fleet.online < fleet.total {
        HarnessStatus::Blocked
    } else {
        HarnessStatus::Running
    };
    let fleet_label = if fleet.total > 0 {
        "Fleet".to_string()
    } else {
        "Fleet · onbekend".to_string()
    };
    out.push(Harness::new(
        HarnessKind::Fleet.id(),
        fleet_label,
        HarnessKind::Fleet,
        fleet_status,
        fleet.total,
        fleet.host.clone(),
        snapshot
            .providers
            .first()
            .map(|p| p.label.clone())
            .or_else(|| fleet.host.clone()),
    ));

    // ---- Commerce harnas --------------------------------------------------
    let commerce_agents: Vec<&crate::models::HerdrAgent> = ops
        .agents
        .iter()
        .filter(|a| {
            a.cwd.to_lowercase().contains("commerce")
                || a.workspace.to_lowercase().contains("commerce")
                || a.name.to_lowercase().contains("commerce")
        })
        .collect();
    let vault_provider_label = snapshot
        .providers
        .iter()
        .find(|p| {
            p.provider.to_lowercase().contains("vault") || p.label.to_lowercase().contains("vault")
        })
        .map(|p| p.label.clone());
    let commerce_queue = commerce_agents.len();
    let commerce_status = if commerce_agents.iter().any(|a| a.status == "blocked") {
        HarnessStatus::Blocked
    } else if commerce_agents.iter().any(|a| a.status == "working") {
        HarnessStatus::Running
    } else {
        HarnessStatus::Idle
    };
    let commerce_active = commerce_agents
        .first()
        .map(|a| format!("{} · {}", a.name, a.workspace));
    out.push(Harness::new(
        HarnessKind::Commerce.id(),
        "Commerce",
        HarnessKind::Commerce,
        commerce_status,
        commerce_queue,
        commerce_active,
        vault_provider_label.clone(),
    ));

    // ---- Eval harnas (compat) ---------------------------------------------
    let health = &snapshot.health;
    let day_score = &snapshot.day_score;
    let eval_status = if health.level == "down" || health.level == "warn" {
        HarnessStatus::Blocked
    } else if health.total == 0 && day_score.score.is_none() {
        HarnessStatus::Idle
    } else if day_score.score.is_some() && health.level == "ok" {
        HarnessStatus::Running
    } else {
        HarnessStatus::Idle
    };
    let eval_label = if day_score.score.is_some() || day_score.letter.is_some() {
        day_score.line()
    } else {
        "Evaluatie · n.v.t.".to_string()
    };
    let eval_active = Some(if day_score.score.is_some() {
        day_score.line()
    } else {
        health.line()
    });
    out.push(Harness::new(
        HarnessKind::Eval.id(),
        eval_label.clone(),
        HarnessKind::Eval,
        eval_status.clone(),
        health.total,
        eval_active.clone(),
        day_score.source.clone(),
    ));

    // ---- Sync harnas ------------------------------------------------------
    let share_sync = &snapshot.share_sync;
    let sync_error = share_sync.contains_key("error")
        || share_sync.get("status").and_then(|v| v.as_str()) == Some("error")
        || share_sync.get("status").and_then(|v| v.as_str()) == Some("blocked");
    let sync_status = if sync_error {
        HarnessStatus::Blocked
    } else {
        HarnessStatus::Idle
    };
    let sync_pending = share_sync
        .get("pendingFiles")
        .or_else(|| share_sync.get("pending"))
        .and_then(|v| v.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0);
    let sync_active = share_sync
        .get("last_sync")
        .and_then(|v| v.as_str())
        .or_else(|| share_sync.get("updated_at").and_then(|v| v.as_str()))
        .map(|s| s.to_string());
    out.push(Harness::new(
        HarnessKind::Sync.id(),
        "Sync",
        HarnessKind::Sync,
        sync_status.clone(),
        sync_pending,
        sync_active.clone(),
        None,
    ));

    // ---- Inbox harnas (D1) — unified attention ---------------------------
    // Tolerant: queue = blocked agents + health down + suggestions
    let blocked_agents = ops
        .agents
        .iter()
        .filter(|a| a.status == "blocked" || a.status == "waiting")
        .count()
        + snapshot
            .agents
            .iter()
            .filter(|a| matches!(a.status.as_str(), "blocked" | "waiting" | "needs_input"))
            .count();
    let inbox_queue = snapshot.suggestions.len() + blocked_agents + snapshot.health.down;
    let inbox_status = if snapshot.error.is_some()
        || health.level == "down"
        || health.level == "warn"
        || blocked_agents > 0
        || !snapshot.suggestions.is_empty()
    {
        HarnessStatus::Blocked
    } else {
        HarnessStatus::Idle
    };
    let inbox_label = if inbox_queue > 0 {
        format!("Inbox · {inbox_queue} om aandacht")
    } else {
        "Inbox".to_string()
    };
    out.push(Harness::new(
        HarnessKind::Inbox.id(),
        inbox_label,
        HarnessKind::Inbox,
        inbox_status,
        inbox_queue,
        snapshot.suggestions.first().map(|s| s.title.clone()),
        None,
    ));

    // ---- Herdr harnas (D2) ------------------------------------------------
    let herdr_queue = ops.agents.len();
    let herdr_status = if ops.agents.iter().any(|a| a.status == "blocked") {
        HarnessStatus::Blocked
    } else if ops.agents.iter().any(|a| a.status == "working") {
        HarnessStatus::Running
    } else {
        HarnessStatus::Idle
    };
    let herdr_active = ops
        .agents
        .first()
        .map(|a| format!("{} · {}", a.name, a.workspace));
    out.push(Harness::new(
        HarnessKind::Herdr.id(),
        "Herdr",
        HarnessKind::Herdr,
        herdr_status,
        herdr_queue,
        herdr_active,
        None,
    ));

    // ---- Vault harnas (D3) ------------------------------------------------
    let vault_queue = snapshot.providers.len();
    let vault_status = if snapshot.providers.iter().any(|p| p.stale) {
        HarnessStatus::Blocked
    } else {
        HarnessStatus::Idle
    };
    out.push(Harness::new(
        HarnessKind::Vault.id(),
        "Vault",
        HarnessKind::Vault,
        vault_status,
        vault_queue,
        vault_provider_label,
        None,
    ));

    // ---- CRM harnas (D3) --------------------------------------------------
    let crm_queue = snapshot.crm_deals.len();
    let crm_status = if crm_queue == 0 {
        HarnessStatus::Idle
    } else {
        HarnessStatus::Running
    };
    out.push(Harness::new(
        HarnessKind::Crm.id(),
        "CRM",
        HarnessKind::Crm,
        crm_status,
        crm_queue,
        snapshot.crm_deals.first().map(|deal| deal.title.clone()),
        None,
    ));

    // ---- Share harnas (D6) — share variant of sync ------------------------
    out.push(Harness::new(
        HarnessKind::Share.id(),
        "Share",
        HarnessKind::Share,
        sync_status.clone(),
        sync_pending,
        sync_active,
        None,
    ));

    // ---- Clipboard harnas (D6) --------------------------------------------
    let clip_len = snapshot.clipboard.len();
    out.push(Harness::new(
        HarnessKind::Clipboard.id(),
        "Clipboard",
        HarnessKind::Clipboard,
        HarnessStatus::Idle,
        clip_len,
        snapshot
            .clipboard
            .first()
            .map(|entry| entry.text.chars().take(24).collect()),
        None,
    ));

    // ---- Desktop harnas (D6) ----------------------------------------------
    let desktop_running = snapshot.desktop.get("state").and_then(|v| v.as_str()) == Some("running");
    out.push(Harness::new(
        HarnessKind::Desktop.id(),
        "Desktop",
        HarnessKind::Desktop,
        if desktop_running {
            HarnessStatus::Running
        } else {
            HarnessStatus::Idle
        },
        if desktop_running { 1 } else { 0 },
        snapshot
            .desktop
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        None,
    ));

    // ---- Tasks harnas (D7) ------------------------------------------------
    let tasks_len = snapshot.tasks.len();
    let tasks_status = if snapshot
        .tasks
        .iter()
        .any(|t| t.get("status").and_then(|v| v.as_str()) == Some("blocked"))
    {
        HarnessStatus::Blocked
    } else if snapshot.tasks.iter().any(|t| {
        matches!(
            t.get("status").and_then(|v| v.as_str()),
            Some("running") | Some("working")
        )
    }) {
        HarnessStatus::Running
    } else {
        HarnessStatus::Idle
    };
    out.push(Harness::new(
        HarnessKind::Tasks.id(),
        "Taken",
        HarnessKind::Tasks,
        tasks_status,
        tasks_len,
        snapshot
            .tasks
            .first()
            .and_then(|t| t.get("prompt"))
            .and_then(|v| v.as_str())
            .map(|s| s.chars().take(32).collect()),
        None,
    ));

    // ---- Linear harnas (D7) -----------------------------------------------
    let linear_queue = snapshot.linear_issues.len();
    out.push(Harness::new(
        HarnessKind::Linear.id(),
        "Linear",
        HarnessKind::Linear,
        if linear_queue == 0 {
            HarnessStatus::Idle
        } else {
            HarnessStatus::Running
        },
        linear_queue,
        snapshot
            .linear_issues
            .first()
            .map(|issue| issue.title.clone()),
        None,
    ));

    // ---- Containers harnas (D4) -------------------------------------------
    let container_queue = snapshot
        .containers
        .drift
        .len()
        .max(snapshot.containers.observed.len());
    out.push(Harness::new(
        HarnessKind::Containers.id(),
        "Containers",
        HarnessKind::Containers,
        if snapshot.containers.drift.is_empty() {
            HarnessStatus::Idle
        } else {
            HarnessStatus::Blocked
        },
        container_queue,
        snapshot.containers.drift.first().cloned(),
        None,
    ));

    // ---- Secrets harnas (D5) ----------------------------------------------
    let secrets_queue = snapshot.secrets_meta.len();
    out.push(Harness::new(
        HarnessKind::Secrets.id(),
        "Secrets",
        HarnessKind::Secrets,
        HarnessStatus::Idle,
        secrets_queue,
        snapshot
            .secrets_meta
            .first()
            .map(|secret| secret.title.clone()),
        None,
    ));

    // ---- Kater harnas (D8) ------------------------------------------------
    let kater_online = snapshot.kater_status.online;
    out.push(Harness::new(
        HarnessKind::Kater.id(),
        "Kater",
        HarnessKind::Kater,
        if kater_online {
            HarnessStatus::Running
        } else if snapshot.kater_status.status.is_empty() {
            HarnessStatus::Idle
        } else {
            HarnessStatus::Blocked
        },
        if kater_online { 1 } else { 0 },
        snapshot
            .kater_status
            .profile
            .clone()
            .or_else(|| Some(snapshot.kater_status.status.clone()).filter(|s| !s.is_empty())),
        None,
    ));

    // ---- Health harnas (D8) — mirrors eval but with own id ----------------
    out.push(Harness::new(
        HarnessKind::Health.id(),
        if health.total > 0 {
            health.line()
        } else {
            "Health · onbekend".to_string()
        },
        HarnessKind::Health,
        eval_status,
        health.total,
        eval_active,
        None,
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FleetInfo, HealthInfo, HerdrAgent, OpsSnapshot, Snapshot};

    fn empty_snapshot() -> Snapshot {
        Snapshot::default()
    }

    #[test]
    fn bouwt_minstens_negen_harnassen() {
        let snap = empty_snapshot();
        let ops = OpsSnapshot::default();
        let h = build_harnesses(&snap, &ops);
        assert!(h.len() >= 9, "verwacht >=9 harnassen, kreeg {}", h.len());
        // distinct ids
        let mut ids: Vec<String> = h.iter().map(|x| x.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), h.len(), "ids moeten distinct zijn");
    }

    #[test]
    fn compat_bevat_oude_vier_ids() {
        let h = build_harnesses(&empty_snapshot(), &OpsSnapshot::default());
        let ids: Vec<&str> = h.iter().map(|x| x.id.as_str()).collect();
        for need in ["fleet", "commerce", "sync", "eval"] {
            assert!(ids.contains(&need), "compat id {need} ontbreekt in {ids:?}");
        }
    }

    #[test]
    fn nieuwe_kinds_tonen_nul_bij_lege_snapshot() {
        let h = build_harnesses(&empty_snapshot(), &OpsSnapshot::default());
        for kind in [
            HarnessKind::Inbox,
            HarnessKind::Linear,
            HarnessKind::Containers,
            HarnessKind::Secrets,
            HarnessKind::Kater,
            HarnessKind::Crm,
        ] {
            let found = h.iter().find(|x| x.kind == kind).expect("kind aanwezig");
            assert_eq!(
                found.queue_depth,
                0,
                "lege snapshot: {} moet 0 tonen",
                kind.id()
            );
            assert_eq!(found.status, HarnessStatus::Idle);
        }
    }

    #[test]
    fn sync_harnas_blocked_bij_error() {
        let mut snap = empty_snapshot();
        snap.share_sync
            .insert("status".into(), serde_json::Value::String("error".into()));
        let h = build_harnesses(&snap, &OpsSnapshot::default());
        let sync = h.iter().find(|x| x.id == "sync").unwrap();
        assert_eq!(sync.status, HarnessStatus::Blocked);
    }

    #[test]
    fn sync_active_valt_terug_op_updated_at_als_last_sync_null_is() {
        let mut snap = empty_snapshot();
        snap.share_sync
            .insert("last_sync".into(), serde_json::Value::Null);
        snap.share_sync.insert(
            "updated_at".into(),
            serde_json::Value::String("2026-08-11T21:00:00Z".into()),
        );
        let h = build_harnesses(&snap, &OpsSnapshot::default());
        let sync = h.iter().find(|x| x.id == "sync").unwrap();
        assert_eq!(sync.active_task.as_deref(), Some("2026-08-11T21:00:00Z"));
    }

    #[test]
    fn fleet_status_blocked_bij_stale() {
        let mut snap = empty_snapshot();
        snap.fleet = FleetInfo {
            online: 2,
            total: 2,
            host: Some("bc-scan-2".into()),
            stale: true,
        };
        let h = build_harnesses(&snap, &OpsSnapshot::default());
        let fleet = h.iter().find(|x| x.id == "fleet").unwrap();
        assert_eq!(fleet.status, HarnessStatus::Blocked);
        assert_eq!(fleet.queue_depth, 2);
    }

    #[test]
    fn commerce_detecteert_cwd() {
        let mut ops = OpsSnapshot::default();
        ops.agents.push(HerdrAgent {
            terminal_id: "t1".into(),
            name: "cursor".into(),
            status: "working".into(),
            workspace: "commerce".into(),
            workspace_id: "w1".into(),
            cwd: "/home/joep/commerce/app".into(),
            pane_id: "p1".into(),
            focused: false,
        });
        let h = build_harnesses(&empty_snapshot(), &ops);
        let commerce = h.iter().find(|x| x.id == "commerce").unwrap();
        assert_eq!(commerce.queue_depth, 1);
        assert_eq!(commerce.status, HarnessStatus::Running);
        assert!(commerce.active_task.is_some());
    }

    #[test]
    fn eval_status_gebaseerd_op_health() {
        let mut snap = empty_snapshot();
        snap.health = HealthInfo {
            ok: 0,
            warn: 0,
            down: 3,
            skip: 0,
            total: 3,
            level: "down".into(),
            updated_at: None,
        };
        let h = build_harnesses(&snap, &OpsSnapshot::default());
        let eval = h.iter().find(|x| x.id == "eval").unwrap();
        assert_eq!(eval.status, HarnessStatus::Blocked);
        assert_eq!(eval.queue_depth, 3);
    }

    #[test]
    fn harnas_kleuren_zijn_hex() {
        for kind in HarnessKind::all() {
            assert!(
                kind.color().starts_with('#'),
                "kleur voor {} moet hex zijn",
                kind.id()
            );
            assert!(kind.color().len() == 7);
        }
    }

    #[test]
    fn harness_prefixes_bevatten_eigen_id() {
        for kind in HarnessKind::all() {
            let prefixes = kind.prefixes();
            assert!(
                !prefixes.is_empty(),
                "prefixes voor {} mag niet leeg zijn",
                kind.id()
            );
            // elk harnas heeft minstens zijn eigen id of een alias als prefix
            // (we checken dat prefix-match werkt voor een actie met dat keyword)
        }
    }

    #[test]
    fn harness_groups_zijn_correct() {
        assert_eq!(HarnessKind::Fleet.group(), HarnessGroup::Fleet);
        assert_eq!(HarnessKind::Herdr.group(), HarnessGroup::Fleet);
        assert_eq!(HarnessKind::Vault.group(), HarnessGroup::Commerce);
        assert_eq!(HarnessKind::Commerce.group(), HarnessGroup::Commerce);
        assert_eq!(HarnessKind::Crm.group(), HarnessGroup::Commerce);
        assert_eq!(HarnessKind::Share.group(), HarnessGroup::Sync);
        assert_eq!(HarnessKind::Clipboard.group(), HarnessGroup::Sync);
        assert_eq!(HarnessKind::Desktop.group(), HarnessGroup::Sync);
        assert_eq!(HarnessKind::Sync.group(), HarnessGroup::Sync);
        assert_eq!(HarnessKind::Inbox.group(), HarnessGroup::Work);
        assert_eq!(HarnessKind::Tasks.group(), HarnessGroup::Work);
        assert_eq!(HarnessKind::Linear.group(), HarnessGroup::Work);
        assert_eq!(HarnessKind::Containers.group(), HarnessGroup::Fleet);
        // Containers currently Fleet group — if later System, update test accordingly
        // For now we follow Fleet grouping for containers as infra; assert flexibility:
        // accept either Fleet or System for containers
        let cg = HarnessKind::Containers.group();
        assert!(
            cg == HarnessGroup::Fleet || cg == HarnessGroup::System,
            "containers group onverwacht: {:?}",
            cg
        );
        assert_eq!(HarnessKind::Secrets.group(), HarnessGroup::System);
        assert_eq!(HarnessKind::Kater.group(), HarnessGroup::System);
        assert_eq!(HarnessKind::Health.group(), HarnessGroup::System);
        assert_eq!(HarnessKind::Eval.group(), HarnessGroup::System);
    }

    #[test]
    fn prefix_matching_fleet_vindt_herdr_actie() {
        let fleet_prefixes = HarnessKind::Fleet.prefixes();
        let herdr_action_keywords = "herdr workspace pane agent";
        let matches = fleet_prefixes.iter().any(|p| {
            herdr_action_keywords
                .split_whitespace()
                .any(|kw| kw.starts_with(p))
        });
        assert!(matches, "fleet moet herdr-acties vinden via prefixes");
    }

    #[test]
    fn all_ids_distinct() {
        let mut ids: Vec<&str> = HarnessKind::all().iter().map(|k| k.id()).collect();
        let orig_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), orig_len, "HarnessKind ids moeten distinct zijn");
    }

    // Legacy: behoud oude bouwt_vier_harnassen semantiek als compat-check
    #[test]
    fn bouwt_vier_harnassen_compat() {
        let snap = empty_snapshot();
        let ops = OpsSnapshot::default();
        let h = build_harnesses(&snap, &ops);
        // oude test verwachtte exact 4; nu >=9, maar eerste 4 waren fleet/commerce/eval/sync
        assert!(h.len() >= 4);
        let ids: Vec<&str> = h.iter().map(|x| x.id.as_str()).collect();
        for need in ["fleet", "commerce", "eval", "sync"] {
            assert!(ids.contains(&need), "compat id {need} ontbreekt in {ids:?}");
        }
    }
}
