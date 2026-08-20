//! Zones en card-grid voor het ChefApp-panel.
//!
//! Herbruikbare zone-header + card-helpers. In Fase 0 geen domein-specifieke
//! `Zone<T>` generic; bestaande render-sections (Inbox, Acties, Gezondheid,
//! Providers, Agents, Aandacht) blijven in `panel::mod::render_into` en roepen
//! hier alleen gedeelde helpers aan. Domein-zones volgen na Lane A+B.

use gtk::prelude::*;

use crate::palette::Action;

/// GTK3 has no `text-transform`. Caps eyebrows live here, once.
pub fn caps(text: &str) -> String {
    text.to_uppercase()
}

/// Bouwt een zone-container met titel, subtitle en een count-badge (optioneel).
/// De caller vult de zone met cards via `build_card` of eigen widgets.
pub fn build_zone(title: &str, subtitle: &str, count: Option<usize>) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.style_context().add_class("chefbar-zone");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.style_context().add_class("chefbar-zone-header");
    if let Some(n) = count {
        let t = gtk::Label::new(Some(&format!("{} · {}", caps(title), n)));
        t.set_halign(gtk::Align::Start);
        t.set_xalign(0.0);
        t.set_ellipsize(pango::EllipsizeMode::End);
        t.style_context().add_class("chefbar-section-title");
        header.pack_start(&t, false, false, 0);
    } else {
        let t = gtk::Label::new(Some(&caps(title)));
        t.set_halign(gtk::Align::Start);
        t.set_xalign(0.0);
        t.set_ellipsize(pango::EllipsizeMode::End);
        t.style_context().add_class("chefbar-section-title");
        header.pack_start(&t, false, false, 0);
    }
    outer.pack_start(&header, false, false, 0);
    if !subtitle.is_empty() {
        let sub = gtk::Label::new(Some(subtitle));
        sub.set_halign(gtk::Align::Start);
        sub.set_xalign(0.0);
        sub.set_ellipsize(pango::EllipsizeMode::End);
        sub.style_context().add_class("chefbar-section-sub");
        outer.pack_start(&sub, false, false, 0);
    }
    outer
}

/// Bouwt een card-widget uit een `palette::Action`. Klik-gedrag (executor,
/// prompt, clipboard) blijft in `panel::mod::render_into`; deze helper bouwt
/// alleen de visuele rij (titel + stamp + padding).
pub fn build_card(action: &Action) -> gtk::Button {
    let row = gtk::Button::new();
    row.set_relief(gtk::ReliefStyle::None);
    row.style_context().add_class("chefbar-row-btn");
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&action.title));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.set_line_wrap(false);
    title.set_max_width_chars(48);
    title.style_context().add_class("chefbar-card-title");
    row_box.pack_start(&title, true, true, 0);
    let stamp = stamp_label(&action.stamp);
    row_box.pack_end(&stamp, false, false, 0);
    row.add(&row_box);
    row.set_hexpand(true);
    row.set_halign(gtk::Align::Fill);
    if let Some(inner) = row.child() {
        inner.set_margin_start(10);
        inner.set_margin_end(10);
        inner.set_margin_top(6);
        inner.set_margin_bottom(6);
    }
    row
}

// ---------------------------------------------------------------------------
// Shared helpers — voorheen private in mod.rs, nu herbruikbaar + getest.
// Deze functies zijn exact de monoliet-versies, alleen verplaatst.

pub fn section_title(content: &gtk::Box, title: &str, sub: &str) {
    let label = gtk::Label::new(Some(&caps(title)));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::End);
    label.style_context().add_class("chefbar-section-title");
    content.pack_start(&label, false, false, 0);
    if !sub.is_empty() {
        let sub_label = gtk::Label::new(Some(sub));
        sub_label.set_halign(gtk::Align::Start);
        sub_label.set_xalign(0.0);
        sub_label.set_ellipsize(pango::EllipsizeMode::End);
        sub_label.style_context().add_class("chefbar-section-sub");
        content.pack_start(&sub_label, false, false, 0);
    }
}

pub fn group_box() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.style_context().add_class("chefbar-group");
    group
}

pub fn group_box_attention() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.style_context().add_class("chefbar-group-attention");
    group
}

pub fn row_wrap(inner: &gtk::Box) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrap.style_context().add_class("chefbar-row");
    wrap.set_margin_start(16);
    wrap.set_margin_end(16);
    wrap.pack_start(inner, false, false, 0);
    wrap
}

pub fn stamp_label(text: &str) -> gtk::Label {
    let text = caps(text);
    let label = gtk::Label::new(Some(&text));
    label.set_halign(gtk::Align::End);
    label.set_valign(gtk::Align::Center);
    let cls = match text.as_str() {
        "KLAAR" => "ok",
        "HULP" => "warn",
        "FOUT" | "LIMIET" => "error",
        "BEZIG" | "TAAK" => "info",
        _ => "",
    };
    label.style_context().add_class("chefbar-stamp");
    if !cls.is_empty() {
        label.style_context().add_class(cls);
    }
    label
}

/// Status-string → dot-klasse (één mapping voor alle domein-rijen).
/// live/bezig = accent, ok/klaar/online = groen, wacht/blok = amber,
/// fout/offline = rood, rest = neutraal.
pub fn status_dot_cls(status: &str) -> &'static str {
    match status.to_ascii_lowercase().as_str() {
        "running" | "bezig" | "active" | "working" | "live" => "live",
        "ok" | "klaar" | "done" | "healthy" | "online" | "up" | "completed" | "success"
        | "merged" | "stil" => "ok",
        "blocked" | "waiting" | "needs_input" | "input" | "attention" | "hulp" | "hold"
        | "pending" | "warn" | "warning" | "stale" | "limiet" => "warn",
        "failed" | "error" | "crashed" | "down" | "offline" | "fout" | "unhealthy" | "kapot" => {
            "down"
        }
        _ => "",
    }
}

/// Generieke domein-rij: dot + titel (+ optionele meta-regel) + stamp rechts.
/// Teruggeven als gewrapte rij voor `.chefbar-group` (hairlines + hover).
pub fn domain_row(
    dot_cls: &str,
    title: &str,
    meta: Option<&str>,
    stamp: Option<(&str, &str)>,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.set_size_request(8, 8);
    dot.set_halign(gtk::Align::Start);
    dot.set_valign(gtk::Align::Center);
    dot.style_context().add_class("chefbar-dot");
    if !dot_cls.is_empty() {
        dot.style_context().add_class(dot_cls);
    }
    row.pack_start(&dot, false, false, 0);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let title_l = gtk::Label::new(Some(title));
    title_l.set_halign(gtk::Align::Start);
    title_l.set_xalign(0.0);
    title_l.set_ellipsize(pango::EllipsizeMode::End);
    title_l.set_line_wrap(false);
    title_l.set_max_width_chars(40);
    title_l.set_tooltip_text(Some(title));
    title_l.style_context().add_class("chefbar-card-title");
    text.pack_start(&title_l, false, false, 0);
    if let Some(meta) = meta {
        if !meta.is_empty() {
            let meta_l = gtk::Label::new(Some(meta));
            meta_l.set_halign(gtk::Align::Start);
            meta_l.set_xalign(0.0);
            meta_l.set_line_wrap(true);
            meta_l.set_lines(1);
            meta_l.set_ellipsize(pango::EllipsizeMode::End);
            meta_l.set_max_width_chars(56);
            meta_l.set_tooltip_text(Some(meta));
            meta_l.style_context().add_class("chefbar-card-meta");
            text.pack_start(&meta_l, false, false, 0);
        }
    }
    row.pack_start(&text, true, true, 0);
    if let Some((stamp_text, stamp_cls)) = stamp {
        let stamp_label = stamp_label(stamp_text);
        if !stamp_cls.is_empty() {
            stamp_label.style_context().add_class(stamp_cls);
        }
        row.pack_end(&stamp_label, false, false, 0);
    }
    row_wrap(&row)
}

pub fn clickable_row(
    inner: gtk::Box,
    spec: crate::actions::RunSpec,
    executor: &crate::actions::Executor,
) -> gtk::Button {
    let row_btn = gtk::Button::new();
    row_btn.set_relief(gtk::ReliefStyle::None);
    row_btn.set_hexpand(true);
    row_btn.set_halign(gtk::Align::Fill);
    row_btn.style_context().add_class("chefbar-row-btn");
    row_btn.add(&inner);
    let executor = executor.clone();
    row_btn.connect_clicked(move |_| {
        executor.run_for_ui(&spec);
    });
    row_btn
}

pub fn info_row(text: &str, meta: Option<&str>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::End);
    label.style_context().add_class("chefbar-card-title");
    row.pack_start(&label, true, true, 0);
    if let Some(meta) = meta {
        let meta_label = gtk::Label::new(Some(meta));
        meta_label.set_halign(gtk::Align::End);
        meta_label.set_xalign(1.0);
        meta_label.set_ellipsize(pango::EllipsizeMode::End);
        meta_label.style_context().add_class("chefbar-card-meta");
        row.pack_end(&meta_label, false, false, 0);
    }
    row_wrap(&row)
}

pub fn truncate_q(q: &str, max: usize) -> String {
    let chars: Vec<char> = q.chars().collect();
    if chars.len() <= max {
        q.to_string()
    } else {
        chars[..max].iter().collect::<String>() + "…"
    }
}

pub fn empty_state(title: &str, sub: &str) -> gtk::Box {
    empty_state_cta(title, sub, "")
}

pub fn empty_state_cta(title: &str, sub: &str, cta: &str) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.style_context().add_class("chefbar-empty");
    let t = gtk::Label::new(Some(title));
    t.set_halign(gtk::Align::Start);
    t.set_xalign(0.0);
    t.set_ellipsize(pango::EllipsizeMode::End);
    t.style_context().add_class("chefbar-empty-title");
    outer.pack_start(&t, false, false, 0);
    if !sub.is_empty() {
        let s = gtk::Label::new(Some(sub));
        s.set_halign(gtk::Align::Start);
        s.set_xalign(0.0);
        s.set_line_wrap(true);
        s.set_lines(2);
        s.set_ellipsize(pango::EllipsizeMode::End);
        s.set_max_width_chars(62);
        s.style_context().add_class("chefbar-empty-sub");
        outer.pack_start(&s, false, false, 0);
    }
    if !cta.is_empty() {
        let c = gtk::Label::new(Some(cta));
        c.set_halign(gtk::Align::Start);
        c.set_xalign(0.0);
        c.set_ellipsize(pango::EllipsizeMode::End);
        c.style_context().add_class("chefbar-empty-cta");
        outer.pack_start(&c, false, false, 0);
    }
    outer
}

pub fn short_ts(ts: &str) -> String {
    let body = ts
        .chars()
        .take_while(|c| *c != 'T' && *c != ' ' && *c != '.')
        .collect::<String>();
    if body.is_empty() {
        return ts.to_string();
    }
    body
}

pub fn row_top_stale(
    top: &gtk::Box,
    badge: &gtk::Label,
    refresh_at: Option<&str>,
    reason: Option<&str>,
) {
    if let Some(reason) = reason {
        badge.set_tooltip_text(Some(reason));
    }
    let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    inner.pack_start(badge, false, false, 0);
    if let Some(at) = refresh_at {
        let meta = gtk::Label::new(Some(&format!("sinds {}", short_ts(at))));
        meta.set_halign(gtk::Align::End);
        meta.set_xalign(1.0);
        meta.set_ellipsize(pango::EllipsizeMode::End);
        meta.set_line_wrap(false);
        meta.set_tooltip_text(reason);
        meta.style_context().add_class("chefbar-card-meta");
        inner.pack_start(&meta, false, false, 0);
    }
    top.pack_start(&inner, false, false, 0);
}

pub fn state_label(health: &crate::models::HealthInfo) -> String {
    if health.total == 0 {
        "onbekend".into()
    } else {
        format!("{} van {} ok", health.ok, health.total)
    }
}

/// Kleine subkop binnen een domein (niet de zone-titel). Bestaande tokens.
pub fn bucket_title(content: &gtk::Box, title: &str) {
    let label = gtk::Label::new(Some(&caps(title)));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::End);
    label.set_margin_start(16);
    label.set_margin_end(16);
    label.set_margin_top(8);
    label.style_context().add_class("chefbar-section-title");
    content.pack_start(&label, false, false, 0);
}

/// Horizontale KPI-strip: waarde (titel) + label (meta). Geen nieuwe CSS-klassen.
pub fn kpi_strip(items: &[(&str, &str)]) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    wrap.set_margin_start(16);
    wrap.set_margin_end(16);
    wrap.set_margin_top(4);
    wrap.set_margin_bottom(8);
    wrap.style_context().add_class("chefbar-kpi");
    for (label, value) in items {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let v = gtk::Label::new(Some(value));
        v.set_halign(gtk::Align::Start);
        v.set_xalign(0.0);
        v.style_context().add_class("chefbar-card-title");
        let l = gtk::Label::new(Some(label));
        l.set_halign(gtk::Align::Start);
        l.set_xalign(0.0);
        l.style_context().add_class("chefbar-card-meta");
        cell.pack_start(&v, false, false, 0);
        cell.pack_start(&l, false, false, 0);
        wrap.pack_start(&cell, false, false, 0);
    }
    wrap
}
