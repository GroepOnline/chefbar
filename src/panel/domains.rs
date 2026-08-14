//! Per-domein operate-views — ChefApp 5.0.
//!
//! Elke sidebar-domein is een eigen operate-surface (KPI, buckets, typed
//! rijen), geen gekopieerde Acties-lijst. Data komt uit de gedeelde Snapshot.
//! Geen netwerk, geen eigen poll-loop, geen plaintext secrets.
//! Compacte Doen-chips staan in `mod.rs` onder de domein-view.

use gtk::prelude::*;

use super::zones::{
    bucket_title, domain_row, empty_state, group_box, info_row, kpi_strip, section_title, short_ts,
    state_label, status_dot_cls,
};
use crate::actions::Executor;
use crate::harness::HarnessKind;
use crate::models::Snapshot;

/// Max rijen per domein-zone; de rest wordt "+n meer" in de sectie-sub.
const MAX_ROWS: usize = 8;

fn fleet_dot(online: usize, total: usize) -> &'static str {
    if total == 0 {
        ""
    } else if online == total {
        "ok"
    } else if online == 0 {
        "down"
    } else {
        "warn"
    }
}

/// Render de domein-sectie(s) voor `kind` in `content`.
pub fn render_domain(
    content: &gtk::Box,
    kind: &HarnessKind,
    snap: &Snapshot,
    q: &str,
    executor: &Executor,
    window: &gtk::Window,
) {
    match kind {
        HarnessKind::Inbox => render_inbox(content, snap, q),
        HarnessKind::Fleet => render_fleet(content, snap, q),
        HarnessKind::Herdr => render_herdr(content, snap, q),
        HarnessKind::Containers => render_containers(content, snap, q),
        HarnessKind::Vault => render_vault(content, snap, q),
        HarnessKind::Commerce => render_providers(content, snap, q),
        HarnessKind::Crm => render_crm(content, snap, q),
        HarnessKind::Share => render_share(content, snap, q),
        HarnessKind::Sync => render_sync(content, snap, q),
        HarnessKind::Clipboard => render_clipboard(content, snap, q),
        HarnessKind::Desktop => render_desktop(content, snap, q),
        HarnessKind::Tasks => render_taken(content, snap, q),
        HarnessKind::Linear => render_linear(content, snap, q, executor, window),
        HarnessKind::Secrets => render_secrets(content, snap, q),
        HarnessKind::Kater => render_kater(content, snap, q),
        HarnessKind::Health => render_health(content, snap, q),
        HarnessKind::Eval => render_eval(content, snap, q),
        // Control is the persistent chat canvas, not an operate-view rebuild.
        HarnessKind::Control => {}
    }
}

/// Linear/taken-status → vaste bucket. Pure, GTK-vrij, getest.
pub(crate) fn status_bucket(status: &str) -> &'static str {
    let s = status.to_ascii_lowercase();
    if s.contains("fail")
        || s.contains("error")
        || s.contains("fout")
        || s.contains("block")
        || s.contains("hold")
        || s.contains("hulp")
    {
        "Vast"
    } else if s.contains("progress") || s == "bezig" || s == "started" || s == "doing" {
        "Bezig"
    } else if s.contains("todo")
        || s.contains("backlog")
        || s == "open"
        || s == "te doen"
        || s.contains("triage")
    {
        "Te doen"
    } else if s.contains("done")
        || s.contains("complete")
        || s.contains("merged")
        || s == "klaar"
        || s == "closed"
    {
        "Klaar"
    } else {
        "Overig"
    }
}

const BUCKET_ORDER: [&str; 5] = ["Bezig", "Vast", "Te doen", "Klaar", "Overig"];

fn inbox_bucket(status: &str) -> &'static str {
    match status_dot_cls(status) {
        "down" => "Fout",
        "warn" => "Wacht op jou",
        "live" => "Bezig",
        _ => "Overig",
    }
}

/// Sectie-sub met telling: "{shown} van {total}" (plus zoekhint bij filter).
fn count_sub(q: &str, shown: usize, total: usize) -> String {
    if q.trim().is_empty() {
        format!("{shown} van {total}")
    } else {
        format!("{shown} van {total} · zoekfilter")
    }
}

/// Sectie-sub voor een enkel rijtje (geen telling nodig).
fn single_sub(extra: &str) -> String {
    extra.to_string()
}

// ---------------------------------------------------------------------------
// Inbox
// ---------------------------------------------------------------------------

fn render_inbox(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let mut all: Vec<_> = snap
        .inbox
        .iter()
        .filter(|i| {
            ql.is_empty()
                || i.title.to_lowercase().contains(&ql)
                || i.meta.to_lowercase().contains(&ql)
        })
        .collect();
    if all.is_empty() {
        section_title(content, "Inbox", "triage");
        content.pack_start(
            &empty_state(
                "Inbox is leeg",
                "Nieuwe meldingen van watchers verschijnen hier zodra er iets signaleert.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let total = all.len();
    let fout = all
        .iter()
        .filter(|i| inbox_bucket(&i.status) == "Fout")
        .count();
    let wacht = all
        .iter()
        .filter(|i| inbox_bucket(&i.status) == "Wacht op jou")
        .count();
    section_title(content, "Inbox", &count_sub(q, total.min(MAX_ROWS), total));
    let fout_s = fout.to_string();
    let wacht_s = wacht.to_string();
    let total_s = total.to_string();
    content.pack_start(
        &kpi_strip(&[("fout", &fout_s), ("wacht", &wacht_s), ("totaal", &total_s)]),
        false,
        false,
        0,
    );
    all.sort_by_key(|i| match inbox_bucket(&i.status) {
        "Fout" => 0u8,
        "Wacht op jou" => 1,
        "Bezig" => 2,
        _ => 3,
    });
    // One remaining-row budget for the whole domain (not per bucket).
    let mut remaining = MAX_ROWS;
    for bucket in ["Fout", "Wacht op jou", "Bezig", "Overig"] {
        if remaining == 0 {
            break;
        }
        let rows: Vec<_> = all
            .iter()
            .filter(|i| inbox_bucket(&i.status) == bucket)
            .copied()
            .collect();
        if rows.is_empty() {
            continue;
        }
        bucket_title(content, bucket);
        let group = group_box();
        let take_n = remaining.min(rows.len());
        for item in rows.iter().take(take_n) {
            let stamp = if item.status.is_empty() {
                None
            } else {
                Some((item.status.as_str(), status_dot_cls(&item.status)))
            };
            group.pack_start(
                &domain_row(
                    status_dot_cls(&item.status),
                    &item.title,
                    (!item.meta.is_empty()).then_some(item.meta.as_str()),
                    stamp,
                ),
                false,
                false,
                0,
            );
        }
        remaining -= take_n;
        content.pack_start(&group, false, false, 0);
    }
}

// ---------------------------------------------------------------------------
// Fleet
// ---------------------------------------------------------------------------

fn render_fleet(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .fleet_nodes
        .iter()
        .filter(|n| {
            ql.is_empty()
                || n.title.to_lowercase().contains(&ql)
                || n.host
                    .as_deref()
                    .map(|h| h.to_lowercase().contains(&ql))
                    .unwrap_or(false)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    let sub = if snap.fleet.total > 0 {
        count_sub(q, shown, total)
    } else {
        single_sub("wacht op de eerste fleet-scan")
    };
    section_title(content, "Fleet", &sub);
    if snap.fleet.total > 0 {
        let online_s = snap.fleet.online.to_string();
        let total_s = snap.fleet.total.to_string();
        content.pack_start(
            &kpi_strip(&[("online", &online_s), ("nodes", &total_s)]),
            false,
            false,
            0,
        );
    }
    if total == 0 && snap.fleet.total == 0 {
        content.pack_start(
            &empty_state(
                "Geen nodes bekend",
                "Zodra de ops-API fleet-data levert, staan de nodes hier met status en host.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let mut remaining = MAX_ROWS;
    for (bucket, pred) in [("Online", true), ("Offline", false)] {
        if remaining == 0 {
            break;
        }
        let rows: Vec<_> = all.iter().filter(|n| n.online == pred).collect();
        if rows.is_empty() {
            continue;
        }
        bucket_title(content, bucket);
        let group = group_box();
        let take_n = remaining.min(rows.len());
        for node in rows.iter().take(take_n) {
            group.pack_start(
                &domain_row(
                    if node.online { "ok" } else { "down" },
                    &node.title,
                    node.host.as_deref(),
                    Some((&node.status, status_dot_cls(&node.status))),
                ),
                false,
                false,
                0,
            );
        }
        remaining -= take_n;
        content.pack_start(&group, false, false, 0);
    }
}

// ---------------------------------------------------------------------------
// Herdr
// ---------------------------------------------------------------------------

fn render_herdr(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .herdr_workspaces
        .iter()
        .filter(|w| {
            ql.is_empty()
                || w.title.to_lowercase().contains(&ql)
                || w.cwd
                    .as_deref()
                    .map(|c| c.to_lowercase().contains(&ql))
                    .unwrap_or(false)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    let running = snap.agents.iter().filter(|a| a.running).count();
    let sub = if running > 0 {
        format!("{} · {} aan het werk", count_sub(q, shown, total), running)
    } else {
        count_sub(q, shown, total)
    };
    section_title(content, "Herdr", &sub);
    if running > 0 || total > 0 {
        let run_s = running.to_string();
        let ws_s = total.to_string();
        content.pack_start(
            &kpi_strip(&[("aan het werk", &run_s), ("workspaces", &ws_s)]),
            false,
            false,
            0,
        );
    }
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen workspaces actief",
                "Start een herdr-workspace en hij verschijnt hier met cwd en status.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for ws in all.iter().take(MAX_ROWS) {
        group.pack_start(
            &domain_row(
                status_dot_cls(&ws.status),
                &ws.title,
                ws.cwd.as_deref(),
                Some((&ws.status, status_dot_cls(&ws.status))),
            ),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

fn render_containers(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let unfiltered_empty = snap.containers.drift.is_empty();
    let drift: Vec<_> = snap
        .containers
        .drift
        .iter()
        .filter(|d| ql.is_empty() || d.to_lowercase().contains(&ql))
        .collect();
    let observed = snap.containers.observed.len();
    let desired = snap.containers.desired.len();
    let sub = if observed + desired > 0 {
        format!("{observed} draaien · {desired} gewenst")
    } else {
        single_sub("wacht op de eerste containers-scan")
    };
    section_title(content, "Containers", &sub);
    if observed + desired > 0 {
        let obs_s = observed.to_string();
        let des_s = desired.to_string();
        let drift_s = snap.containers.drift.len().to_string();
        content.pack_start(
            &kpi_strip(&[("draait", &obs_s), ("gewenst", &des_s), ("drift", &drift_s)]),
            false,
            false,
            0,
        );
    }
    let group = group_box();
    if observed + desired == 0 {
        content.pack_start(
            &empty_state(
                "Nog geen containers bekend",
                "De ops-API rapporteert hier observed vs desired en drift.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    if unfiltered_empty {
        group.pack_start(
            &domain_row(
                "ok",
                "Geen drift",
                Some("observed en desired liggen in lijn"),
                None,
            ),
            false,
            false,
            0,
        );
    } else if drift.is_empty() {
        group.pack_start(
            &domain_row(
                "warn",
                "Drift gefilterd",
                Some("er is drift, maar niets matcht de zoekterm"),
                None,
            ),
            false,
            false,
            0,
        );
    } else {
        for d in drift.iter().take(MAX_ROWS) {
            group.pack_start(
                &domain_row("warn", d, None, Some(("DRIFT", "warn"))),
                false,
                false,
                0,
            );
        }
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Vault
// ---------------------------------------------------------------------------

fn render_vault(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .vault_accounts
        .iter()
        .filter(|a| {
            ql.is_empty()
                || a.title.to_lowercase().contains(&ql)
                || a.provider.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Vault", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen accounts gekoppeld",
                "Accounts en providers uit de vault verschijnen hier met status.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for acc in all.iter().take(MAX_ROWS) {
        group.pack_start(
            &domain_row(
                status_dot_cls(&acc.status),
                &acc.title,
                (!acc.provider.is_empty()).then_some(acc.provider.as_str()),
                Some((&acc.status, status_dot_cls(&acc.status))),
            ),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Commerce (providers + usage) — port van de generieke provider-zone
// ---------------------------------------------------------------------------

fn render_providers(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .providers
        .iter()
        .filter(|r| {
            ql.is_empty()
                || r.label.to_lowercase().contains(&ql)
                || r.usage_text.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Providers", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Nog geen providers",
                "Koppel een account in de vault of vernieuw de status.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for row in all.iter().take(MAX_ROWS) {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 3);
        card.set_margin_top(6);
        card.set_margin_bottom(6);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let name = gtk::Label::new(Some(&row.label));
        name.set_halign(gtk::Align::Start);
        name.set_xalign(0.0);
        name.set_ellipsize(pango::EllipsizeMode::End);
        name.set_line_wrap(false);
        name.set_max_width_chars(24);
        name.set_tooltip_text(Some(&row.label));
        name.style_context().add_class("chefbar-card-title");
        top.pack_start(&name, true, true, 0);
        if row.stale || !row.available {
            let stale_badge = gtk::Label::new(Some("STALE"));
            stale_badge.set_halign(gtk::Align::Start);
            stale_badge.set_xalign(0.0);
            stale_badge.style_context().add_class("chefbar-stamp");
            stale_badge.style_context().add_class("warn");
            super::zones::row_top_stale(
                &top,
                &stale_badge,
                row.refresh_at.as_deref(),
                row.stale_reason.as_deref(),
            );
        } else if let Some(ref at) = row.refresh_at {
            let meta = gtk::Label::new(Some(&format!("update {}", short_ts(at))));
            meta.set_halign(gtk::Align::Start);
            meta.set_xalign(0.0);
            meta.set_ellipsize(pango::EllipsizeMode::End);
            meta.set_line_wrap(false);
            meta.style_context().add_class("chefbar-card-meta");
            top.pack_start(&meta, false, false, 0);
        }
        let active = gtk::Label::new(Some(&row.usage_text));
        active.set_halign(gtk::Align::End);
        active.set_xalign(1.0);
        active.set_ellipsize(pango::EllipsizeMode::End);
        active.set_line_wrap(false);
        active.set_max_width_chars(22);
        active.set_tooltip_text(Some(&row.usage_text));
        active.style_context().add_class("chefbar-card-meta");
        top.pack_end(&active, false, false, 0);
        card.pack_start(&top, false, false, 0);
        if row.requests.is_some() {
            let frac = row.usage_frac;
            let level = row.usage_level.clone();
            let track = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            track.set_size_request(120, 4);
            track.set_hexpand(true);
            track.style_context().add_class("chefbar-bar-track");
            let fill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            fill.set_size_request(((120.0 * frac).round() as i32).clamp(4, 120), 4);
            fill.style_context().add_class("chefbar-bar-fill");
            fill.style_context().add_class(&level);
            track.pack_start(&fill, false, false, 0);
            let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            bottom.pack_start(&track, true, true, 0);
            let nums = gtk::Label::new(Some(&format!(
                "{} req · {}M tok",
                row.requests.unwrap_or(0),
                row.tokens.unwrap_or(0) / 1_000_000
            )));
            nums.set_halign(gtk::Align::End);
            nums.set_xalign(1.0);
            nums.set_ellipsize(pango::EllipsizeMode::End);
            nums.set_line_wrap(false);
            nums.style_context().add_class("chefbar-card-meta");
            bottom.pack_end(&nums, false, false, 0);
            card.pack_start(&bottom, false, false, 0);
        }
        let wrap = super::zones::row_wrap(&card);
        group.pack_start(&wrap, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// CRM
// ---------------------------------------------------------------------------

fn render_crm(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .crm_deals
        .iter()
        .filter(|d| {
            ql.is_empty()
                || d.title.to_lowercase().contains(&ql)
                || d.meta.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "CRM", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen deals bekend",
                "Organisaties en deals uit de CRM verschijnen hier met bedrag en status.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for deal in all.iter().take(MAX_ROWS) {
        let amount = deal.amount.clone().unwrap_or_default();
        // Keep human-readable status when amount is present (amount beside status).
        let stamp_owned: Option<(String, &'static str)> =
            match (!amount.is_empty(), !deal.status.is_empty()) {
                (true, true) => Some((
                    format!("{} · {}", deal.status, amount),
                    status_dot_cls(&deal.status),
                )),
                (true, false) => Some((amount, "")),
                (false, true) => Some((deal.status.clone(), status_dot_cls(&deal.status))),
                (false, false) => None,
            };
        let stamp = stamp_owned
            .as_ref()
            .map(|(text, cls)| (text.as_str(), *cls));
        group.pack_start(
            &domain_row(
                status_dot_cls(&deal.status),
                &deal.title,
                (!deal.meta.is_empty()).then_some(deal.meta.as_str()),
                stamp,
            ),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Share
// ---------------------------------------------------------------------------

fn render_share(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let mut entries: Vec<_> = snap
        .share_sync
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let all: Vec<_> = entries
        .iter()
        .filter(|(k, v)| {
            ql.is_empty() || k.to_lowercase().contains(&ql) || v.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Share", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Share-sync is stil",
                "Zodra de vault share-sync rapporteert, staan de statussen hier.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for (k, v) in all.iter().take(MAX_ROWS) {
        group.pack_start(&info_row(k, Some(v)), false, false, 0);
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Sync — laatste poll per bron, niet dezelfde share-lijst
// ---------------------------------------------------------------------------

/// Sync-stamp: verse tijd alleen bij ok. Fail toont fout + laatste goede tijd.
pub(crate) fn sync_stamp(ok: bool, last_good_iso: &str) -> (String, &'static str) {
    let ts = short_ts(last_good_iso);
    if ok {
        (ts, "ok")
    } else {
        (format!("fout · {ts}"), "down")
    }
}

fn render_sync(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let mut entries: Vec<_> = snap
        .last_poll_at
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let all: Vec<_> = entries
        .iter()
        .filter(|(k, v)| {
            ql.is_empty() || k.to_lowercase().contains(&ql) || v.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    section_title(content, "Sync", "ok of fout per bron");
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Nog geen poll-tijden",
                "Zodra de actor bronnen polt, staat hier per bron of de laatste poll slaagde.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for (k, v) in all.iter().take(MAX_ROWS) {
        let ok = snap.last_poll_ok.get(k.as_str()).copied().unwrap_or(false);
        let (meta, cls) = sync_stamp(ok, v);
        group.pack_start(
            &domain_row(cls, k, Some(meta.as_str()), None),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Clipboard
// ---------------------------------------------------------------------------

fn render_clipboard(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .clipboard
        .iter()
        .filter(|e| {
            ql.is_empty()
                || e.title.to_lowercase().contains(&ql)
                || e.text.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(
        content,
        "Clipboard",
        &format!("{} · klik om te kopiëren", count_sub(q, shown, total)),
    );
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Klembord is leeg",
                "Gekopieerde teksten uit de vault verschijnen hier.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for entry in all.iter().take(MAX_ROWS) {
        let title: String = entry.text.chars().take(60).collect();
        let row_btn = gtk::Button::new();
        row_btn.set_relief(gtk::ReliefStyle::None);
        row_btn.set_hexpand(true);
        row_btn.set_halign(gtk::Align::Fill);
        row_btn.style_context().add_class("chefbar-row-btn");
        let inner = domain_row("", &title, entry.created_at.as_deref(), None);
        row_btn.add(&inner);
        if let Some(child) = row_btn.child() {
            child.set_margin_start(10);
            child.set_margin_end(10);
            child.set_margin_top(6);
            child.set_margin_bottom(6);
        }
        let text = entry.text.clone();
        let id = entry.id.clone();
        row_btn.connect_clicked(move |_| {
            let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
            clipboard.set_text(&text);
            crate::frecency::record(&format!("clipboard:{id}"));
            super::notify_copied();
        });
        group.pack_start(&row_btn, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Desktop
// ---------------------------------------------------------------------------

fn render_desktop(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let mut entries: Vec<_> = snap
        .desktop
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let all: Vec<_> = entries
        .iter()
        .filter(|(k, v)| {
            ql.is_empty() || k.to_lowercase().contains(&ql) || v.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Desktop", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Webtop niet bereikbaar",
                "Zodra de desktop-status rapporteert, staan status en adres hier.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for (k, v) in all.iter().take(MAX_ROWS) {
        let ok =
            v.to_lowercase().contains("ok") || v.to_lowercase().contains("running") || v == "true";
        group.pack_start(
            &domain_row(if ok { "ok" } else { "" }, k, Some(v), None),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Taken (Commander)
// ---------------------------------------------------------------------------

fn render_taken(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .commander_tasks
        .iter()
        .filter(|t| {
            ql.is_empty()
                || t.title.to_lowercase().contains(&ql)
                || t.meta.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Taken", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen taken",
                "Commander-taken verschijnen hier met status zodra ze bestaan.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    // One remaining-row budget for the whole domain (not per bucket).
    let mut remaining = MAX_ROWS;
    for bucket in BUCKET_ORDER {
        if remaining == 0 {
            break;
        }
        let rows: Vec<_> = all
            .iter()
            .filter(|t| status_bucket(&t.status) == bucket)
            .collect();
        if rows.is_empty() {
            continue;
        }
        bucket_title(content, bucket);
        let group = group_box();
        let take_n = remaining.min(rows.len());
        for task in rows.iter().take(take_n) {
            group.pack_start(
                &domain_row(
                    status_dot_cls(&task.status),
                    &task.title,
                    (!task.meta.is_empty()).then_some(task.meta.as_str()),
                    Some((&task.status, status_dot_cls(&task.status))),
                ),
                false,
                false,
                0,
            );
        }
        remaining -= take_n;
        content.pack_start(&group, false, false, 0);
    }
}

// ---------------------------------------------------------------------------
// Linear
// ---------------------------------------------------------------------------

fn render_linear(
    content: &gtk::Box,
    snap: &Snapshot,
    q: &str,
    executor: &Executor,
    _window: &gtk::Window,
) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .linear_issues
        .iter()
        .filter(|i| {
            ql.is_empty()
                || i.title.to_lowercase().contains(&ql)
                || i.meta.to_lowercase().contains(&ql)
                || i.project
                    .as_deref()
                    .map(|p| p.to_lowercase().contains(&ql))
                    .unwrap_or(false)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(content, "Linear", &count_sub(q, shown, total));
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen issues",
                "Assigned-to-me en sprint-issues verschijnen hier met project en status.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let bezig = all
        .iter()
        .filter(|i| status_bucket(&i.status) == "Bezig")
        .count();
    let todo = all
        .iter()
        .filter(|i| status_bucket(&i.status) == "Te doen")
        .count();
    let bezig_s = bezig.to_string();
    let todo_s = todo.to_string();
    let total_s = total.to_string();
    content.pack_start(
        &kpi_strip(&[
            ("bezig", &bezig_s),
            ("te doen", &todo_s),
            ("totaal", &total_s),
        ]),
        false,
        false,
        0,
    );
    // One remaining-row budget for the whole domain (not per bucket).
    let mut remaining = MAX_ROWS;
    for bucket in BUCKET_ORDER {
        if remaining == 0 {
            break;
        }
        let rows: Vec<_> = all
            .iter()
            .filter(|i| status_bucket(&i.status) == bucket)
            .collect();
        if rows.is_empty() {
            continue;
        }
        bucket_title(content, bucket);
        let group = group_box();
        let take_n = remaining.min(rows.len());
        for issue in rows.iter().take(take_n) {
            let meta = match (&issue.project, issue.meta.is_empty()) {
                (Some(p), false) => format!("{p} · {}", issue.meta),
                (Some(p), true) => p.clone(),
                (None, _) => issue.meta.clone(),
            };
            let row_btn = gtk::Button::new();
            row_btn.set_relief(gtk::ReliefStyle::None);
            row_btn.set_hexpand(true);
            row_btn.set_halign(gtk::Align::Fill);
            row_btn.style_context().add_class("chefbar-row-btn");
            let inner = domain_row(
                status_dot_cls(&issue.status),
                &issue.title,
                (!meta.is_empty()).then_some(meta.as_str()),
                Some((&issue.status, status_dot_cls(&issue.status))),
            );
            row_btn.add(&inner);
            if let Some(child) = row_btn.child() {
                child.set_margin_start(10);
                child.set_margin_end(10);
                child.set_margin_top(6);
                child.set_margin_bottom(6);
            }
            if let Some(url) = issue.url.clone() {
                let executor = executor.clone();
                row_btn.connect_clicked(move |_| {
                    executor.run_for_ui(&crate::actions::RunSpec::OpenUrl(url.clone()));
                });
            } else {
                row_btn.set_sensitive(false);
            }
            group.pack_start(&row_btn, false, false, 0);
        }
        remaining -= take_n;
        content.pack_start(&group, false, false, 0);
    }
}

// ---------------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------------

fn render_secrets(content: &gtk::Box, snap: &Snapshot, q: &str) {
    let ql = q.to_lowercase();
    let all: Vec<_> = snap
        .secrets_meta
        .iter()
        .filter(|s| {
            ql.is_empty()
                || s.title.to_lowercase().contains(&ql)
                || s.meta.to_lowercase().contains(&ql)
        })
        .collect();
    let total = all.len();
    let shown = total.min(MAX_ROWS);
    section_title(
        content,
        "Secrets",
        &format!(
            "{} · alleen meta, nooit plaintext",
            count_sub(q, shown, total)
        ),
    );
    if total == 0 {
        content.pack_start(
            &empty_state(
                "Geen secrets gekoppeld",
                "Vaultwarden-collecties verschijnen hier als meta — geen plaintext in de UI.",
            ),
            false,
            false,
            0,
        );
        return;
    }
    let group = group_box();
    for secret in all.iter().take(MAX_ROWS) {
        group.pack_start(
            &domain_row(
                status_dot_cls(&secret.status),
                &secret.title,
                (!secret.meta.is_empty()).then_some(secret.meta.as_str()),
                Some((&secret.status, status_dot_cls(&secret.status))),
            ),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
}

// ---------------------------------------------------------------------------
// Kater
// ---------------------------------------------------------------------------

fn render_kater(content: &gtk::Box, snap: &Snapshot, q: &str) {
    section_title(content, "Kater", "gateway · geen tweede poll");
    let k = &snap.kater_status;
    let online = k.online;
    content.pack_start(
        &kpi_strip(&[("gateway", if online { "online" } else { "offline" })]),
        false,
        false,
        0,
    );
    let group = group_box();
    let status = if k.status.is_empty() {
        "onbekend"
    } else {
        k.status.as_str()
    };
    group.pack_start(&info_row("Status", Some(status)), false, false, 0);
    if let Some(profile) = k.profile.as_deref() {
        group.pack_start(&info_row("Profiel", Some(profile)), false, false, 0);
    }
    let obs = &snap.observability;
    let obs_st = if obs.status.is_empty() {
        "onbekend"
    } else {
        obs.status.as_str()
    };
    group.pack_start(&info_row("Observability", Some(obs_st)), false, false, 0);
    for err in obs.errors.iter().take(3) {
        group.pack_start(
            &domain_row("down", err, None, Some(("FOUT", "error"))),
            false,
            false,
            0,
        );
    }
    let j = &snap.jcode_memory;
    if j.online || !j.host.is_empty() {
        group.pack_start(
            &info_row(
                "jcode-geheugen",
                Some(if j.host.is_empty() {
                    "lokaal"
                } else {
                    j.host.as_str()
                }),
            ),
            false,
            false,
            0,
        );
    }
    content.pack_start(&group, false, false, 0);
    let _ = q;
}

// ---------------------------------------------------------------------------
// Health — watchdog + services. Geen dagscore, geen fleet-kloon.
// ---------------------------------------------------------------------------

fn render_health(content: &gtk::Box, snap: &Snapshot, q: &str) {
    section_title(content, "Gezondheid", "watchdog + services");
    let ok_s = snap.health.ok.to_string();
    let warn_s = snap.health.warn.to_string();
    let down_s = snap.health.down.to_string();
    content.pack_start(
        &kpi_strip(&[("ok", &ok_s), ("wacht", &warn_s), ("down", &down_s)]),
        false,
        false,
        0,
    );
    let group = group_box();
    let health_meta = match snap.health.updated_at.as_deref() {
        Some(at) => format!("{} · update {}", state_label(&snap.health), short_ts(at)),
        None => state_label(&snap.health),
    };
    group.pack_start(
        &info_row(&snap.health.line(), Some(&health_meta)),
        false,
        false,
        0,
    );
    content.pack_start(&group, false, false, 0);
    if let Some(services) = snap
        .raw
        .get("status")
        .and_then(|s| s.get("services"))
        .and_then(|s| s.as_array())
    {
        bucket_title(content, "Services");
        let group = group_box();
        for svc in services.iter().take(MAX_ROWS) {
            let name = svc.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let st = svc.get("state").and_then(|v| v.as_str()).unwrap_or("");
            group.pack_start(
                &domain_row(
                    status_dot_cls(st),
                    name,
                    None,
                    Some((st, status_dot_cls(st))),
                ),
                false,
                false,
                0,
            );
        }
        content.pack_start(&group, false, false, 0);
    }
    let _ = q;
}

// ---------------------------------------------------------------------------
// Eval — dagscore. Geen service-lijst, geen Acties-kloon.
// ---------------------------------------------------------------------------

fn render_eval(content: &gtk::Box, snap: &Snapshot, q: &str) {
    section_title(content, "Evaluatie", "dagscore");
    let score = snap.day_score.line();
    content.pack_start(&kpi_strip(&[("vandaag", &score)]), false, false, 0);
    let group = group_box();
    group.pack_start(
        &info_row("Bron", snap.day_score.source.as_deref()),
        false,
        false,
        0,
    );
    group.pack_start(
        &info_row("OS health", Some(&snap.health.line())),
        false,
        false,
        0,
    );
    if snap.fleet.total > 0 {
        let fleet_line = format!("{}/{} online", snap.fleet.online, snap.fleet.total);
        let _ = fleet_dot(snap.fleet.online, snap.fleet.total);
        group.pack_start(&info_row("Fleet-peek", Some(&fleet_line)), false, false, 0);
    }
    content.pack_start(&group, false, false, 0);
    let _ = q;
}

#[cfg(test)]
mod tests {
    use super::{inbox_bucket, status_bucket, sync_stamp};

    #[test]
    fn status_bucket_groups_linear_states() {
        assert_eq!(status_bucket("In Progress"), "Bezig");
        assert_eq!(status_bucket("Todo"), "Te doen");
        assert_eq!(status_bucket("Done"), "Klaar");
        assert_eq!(status_bucket("Blocked"), "Vast");
        assert_eq!(status_bucket("mystery"), "Overig");
    }

    #[test]
    fn inbox_bucket_uses_status_dots() {
        assert_eq!(inbox_bucket("failed"), "Fout");
        assert_eq!(inbox_bucket("hulp"), "Wacht op jou");
        assert_eq!(inbox_bucket("running"), "Bezig");
        assert_eq!(inbox_bucket("stil"), "Overig");
    }

    #[test]
    fn sync_stamp_does_not_look_fresh_on_failure() {
        let (ok_meta, ok_cls) = sync_stamp(true, "2026-08-14T01:50:00Z");
        assert_eq!(ok_cls, "ok");
        assert_eq!(ok_meta, "2026-08-14");
        let (fail_meta, fail_cls) = sync_stamp(false, "2026-08-14T01:50:00Z");
        assert_eq!(fail_cls, "down");
        assert!(fail_meta.starts_with("fout · "), "{fail_meta}");
        assert!(fail_meta.contains("2026-08-14"));
    }
}
