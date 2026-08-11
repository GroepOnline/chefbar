//! Harnas-model — room: samenhangende ruimte, geen losse pages.
//!
//! De app kan meerdere harnassen tegelijk tonen. Elk harnas is een
//! samenhangende werkruimte (fleet, commerce, sync, eval) met eigen
//! status, wachtrij en kleur. Dit model is puur afgeleid uit bestaande
//! snapshot-data — geen nieuwe netwerk-calls.

use crate::models::{OpsSnapshot, Snapshot};

// ---------------------------------------------------------------------------
// Harnas-soorten en statussen
// ---------------------------------------------------------------------------

/// Soort harnas — bepaalt kleur, label en keyword-prefix voor filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessKind {
    Fleet,
    Commerce,
    Sync,
    Eval,
}

impl HarnessKind {
    /// ID zoals gebruikt in filtering en selectie.
    pub fn id(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "fleet",
            HarnessKind::Commerce => "commerce",
            HarnessKind::Sync => "sync",
            HarnessKind::Eval => "eval",
        }
    }

    /// Menselijk label voor de UI.
    pub fn label(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "Fleet",
            HarnessKind::Commerce => "Commerce",
            HarnessKind::Sync => "Sync",
            HarnessKind::Eval => "Evaluatie",
        }
    }

    /// Accentkleur per harnas (hex, past bij Signaal/Devin warm-neutral).
    pub fn color(&self) -> &'static str {
        match self {
            HarnessKind::Fleet => "#2563eb",    // blauw
            HarnessKind::Commerce => "#d97706", // amber
            HarnessKind::Sync => "#059669",     // emerald
            HarnessKind::Eval => "#7c3aed",     // violet
        }
    }

    /// Keyword-prefixen waarmee actions aan dit harnas worden gekoppeld.
    /// Panel filtert actions waarvan een keyword-token met één van deze
    /// prefixen begint (prefix-match op keywords).
    pub fn prefixes(&self) -> Vec<&'static str> {
        match self {
            HarnessKind::Fleet => vec!["fleet", "herdr", "focus", "ops", "agent"],
            HarnessKind::Commerce => vec!["commerce", "vault", "account", "commander", "clipboard"],
            HarnessKind::Sync => vec!["share", "sync", "desktop", "pull", "push"],
            HarnessKind::Eval => vec!["eval", "health", "dagscore", "dashboard", "score"],
        }
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

/// Bouw de vier harnassen uit bestaande snapshot-data.
/// - fleet: uit fleet-info (online/total/host/stale)
/// - commerce: uit actieve agents met "commerce" cwd of provider "vault"
/// - eval: uit day_score + health
/// - sync: uit share_sync-status (gedeelde bestanden pull/push)
///
/// Geen nieuwe netwerk-calls; alles komt uit `snapshot` en `ops`.
pub fn build_harnesses(snapshot: &Snapshot, ops: &OpsSnapshot) -> Vec<Harness> {
    let mut out: Vec<Harness> = Vec::with_capacity(4);

    // ---- Fleet harnas -----------------------------------------------------
    // Afgeleid uit FleetInfo: total/online/host/stale.
    let fleet = &snapshot.fleet;
    let fleet_status = if fleet.stale {
        HarnessStatus::Blocked
    } else if fleet.total == 0 {
        HarnessStatus::Idle
    } else if fleet.online == 0 {
        HarnessStatus::Blocked
    } else if fleet.online < fleet.total {
        // gedeeltelijk online → aandacht nodig
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
        // provider_label voor fleet: eerste provider als hint, anders host
        snapshot
            .providers
            .first()
            .map(|p| p.label.clone())
            .or_else(|| fleet.host.clone()),
    ));

    // ---- Commerce harnas --------------------------------------------------
    // Commerce = agents waarvan cwd/workspace/name "commerce" bevat,
    // óf provider met id/label "vault".
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
            p.provider.to_lowercase().contains("vault")
                || p.label.to_lowercase().contains("vault")
        })
        .map(|p| p.label.clone());
    let commerce_queue = commerce_agents.len();
    let commerce_status = if commerce_agents.iter().any(|a| a.status == "blocked") {
        HarnessStatus::Blocked
    } else if commerce_agents.iter().any(|a| a.status == "working") {
        HarnessStatus::Running
    } else if commerce_queue > 0 {
        HarnessStatus::Idle
    } else if vault_provider_label.is_some() {
        // vault-provider aanwezig maar geen commerce-agent → idle
        HarnessStatus::Idle
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
        vault_provider_label,
    ));

    // ---- Eval harnas ------------------------------------------------------
    // Evaluatie = health + day_score.
    let health = &snapshot.health;
    let day_score = &snapshot.day_score;
    let eval_status = if health.level == "down" {
        HarnessStatus::Blocked
    } else if health.level == "warn" {
        HarnessStatus::Blocked
    } else if health.total == 0 && day_score.score.is_none() {
        HarnessStatus::Idle
    } else if day_score.score.is_some() && health.level == "ok" {
        HarnessStatus::Running
    } else if health.level == "ok" {
        HarnessStatus::Idle
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
        eval_label,
        HarnessKind::Eval,
        eval_status,
        health.total,
        eval_active,
        day_score.source.clone(),
    ));

    // ---- Sync harnas ------------------------------------------------------
    // Afgeleid uit share-sync/status: gedeelde bestanden (pull/push-aware).
    let share_sync = &snapshot.share_sync;
    let sync_error = share_sync.contains_key("error")
        || share_sync.get("status").and_then(|v| v.as_str()) == Some("error")
        || share_sync.get("status").and_then(|v| v.as_str()) == Some("blocked");
    let sync_status = if sync_error {
        HarnessStatus::Blocked
    } else if !share_sync.is_empty() {
        HarnessStatus::Idle
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
        .or_else(|| share_sync.get("updated_at"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    out.push(Harness::new(
        HarnessKind::Sync.id(),
        "Sync",
        HarnessKind::Sync,
        sync_status,
        sync_pending,
        sync_active,
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
    fn bouwt_vier_harnassen() {
        let snap = empty_snapshot();
        let ops = OpsSnapshot::default();
        let h = build_harnesses(&snap, &ops);
        assert_eq!(h.len(), 4);
        assert_eq!(h[0].id, "fleet");
        assert_eq!(h[1].id, "commerce");
        assert_eq!(h[2].id, "eval");
        assert_eq!(h[3].id, "sync");
    }

    #[test]
    fn sync_harnas_blocked_bij_error() {
        let mut snap = empty_snapshot();
        snap.share_sync.insert("status".into(), serde_json::Value::String("error".into()));
        let h = build_harnesses(&snap, &OpsSnapshot::default());
        assert_eq!(h[3].status, HarnessStatus::Blocked);
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
        assert_eq!(h[0].status, HarnessStatus::Blocked);
        assert_eq!(h[0].queue_depth, 2);
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
        let commerce = &h[1];
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
        assert_eq!(h[2].status, HarnessStatus::Blocked);
        assert_eq!(h[2].queue_depth, 3);
    }

    #[test]
    fn harnas_kleuren_zijn_hex() {
        for kind in [HarnessKind::Fleet, HarnessKind::Commerce, HarnessKind::Sync, HarnessKind::Eval] {
            assert!(kind.color().starts_with('#'));
        }
    }
}
