//! ChefBar-hoofdvenster: één echte app (Devin-stijl), geen floating bar.
//!
//! Undecorated window met custom header (drag + minimize + sluiten), zoek-
//! input die alle secties live filtert, gegroepeerde cards per sectie
//! (Devin-sgroup met hairlines), en footer. Inhoud wordt elke poll-cyclus
//! opnieuw gevuld uit de gedeelde snapshot: geen eigen poll-loops, geen
//! netwerk op de UI-thread.

use crate::actions::{build_actions, Executor};
use crate::motion::{fade_in, fade_out, PANEL_MS};
use crate::palette::{rank_actions, Action};
use crate::state::{Shared, VAULT_POLL_MS};
use glib::ControlFlow;
use gtk::prelude::*;

pub struct Panel {
    pub window: gtk::Window,
    content: gtk::Box,
    search: gtk::SearchEntry,
    shared: Shared,
    executor: Executor,
}

impl Panel {
    pub fn new(shared: Shared, executor: Executor) -> Self {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("ChefBar");
        window.set_decorated(false);
        window.set_default_size(760, 840);
        window.set_keep_above(true);
        window.set_position(gtk::WindowPosition::Center);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.style_context().add_class("chefbar-app");
        window.add(&root);

        // ---- Custom header (drag + controls) ----
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        header.style_context().add_class("chefbar-header");
        header.set_margin_bottom(0);

        let title_block = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let title = gtk::Label::new(Some("ChefBar"));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.style_context().add_class("chefbar-title");
        title_block.pack_start(&title, false, false, 0);
        let title_sub = gtk::Label::new(Some("agentische assistent · devin-skin"));
        title_sub.set_halign(gtk::Align::Start);
        title_sub.set_xalign(0.0);
        title_sub.style_context().add_class("chefbar-title-sub");
        title_block.pack_start(&title_sub, false, false, 0);
        header.pack_start(&title_block, true, true, 0);

        let header_controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let refresh_btn = gtk::Button::new();
        let refresh_icon = gtk::Image::from_icon_name(
            Some("view-refresh-symbolic"),
            gtk::IconSize::Button,
        );
        refresh_btn.set_image(Some(&refresh_icon));
        refresh_btn.set_relief(gtk::ReliefStyle::None);
        refresh_btn.style_context().add_class("chefbar-gbtn");
        refresh_btn.connect_clicked(move |_| crate::state::refresh_global());
        let min_btn = gtk::Button::new();
        let min_icon = gtk::Image::from_icon_name(
            Some("window-minimize-symbolic"),
            gtk::IconSize::Button,
        );
        min_btn.set_image(Some(&min_icon));
        min_btn.set_relief(gtk::ReliefStyle::None);
        min_btn.style_context().add_class("chefbar-gbtn");
        let window_for_min = window.clone();
        min_btn.connect_clicked(move |_| window_for_min.iconify());
        let close_btn = gtk::Button::new();
        let close_icon = gtk::Image::from_icon_name(
            Some("window-close-symbolic"),
            gtk::IconSize::Button,
        );
        close_btn.set_image(Some(&close_icon));
        close_btn.set_relief(gtk::ReliefStyle::None);
        close_btn.style_context().add_class("chefbar-gbtn");
        let window_for_close = window.clone();
        close_btn.connect_clicked(move |_| fade_out(&window_for_close, PANEL_MS));
        header_controls.pack_start(&refresh_btn, false, false, 0);
        header_controls.pack_start(&min_btn, false, false, 0);
        header_controls.pack_start(&close_btn, false, false, 0);
        header.pack_end(&header_controls, false, false, 0);
        root.pack_start(&header, false, false, 0);

        // Drag het venster via de header.
        let window_drag = window.clone();
        header.connect_button_press_event(move |_widget, event| {
            if event.button() == 1 {
                let (root_x, root_y) = event.root();
                window_drag.begin_move_drag(
                    event.button() as i32,
                    root_x as i32,
                    root_y as i32,
                    event.time(),
                );
            }
            glib::Propagation::Proceed
        });

        // ---- Zoek-input (filtert de hele surface) ----
        let search_wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
        search_wrap.style_context().add_class("chefbar-search-wrap");
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Zoek acties, agents, providers, sessies…"));
        search.style_context().add_class("chefbar-search");
        search_wrap.pack_start(&search, false, false, 0);
        root.pack_start(&search_wrap, false, false, 0);

        // ---- Content ----
        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_min_content_height(480);
        root.pack_start(&scroller, true, true, 0);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_margin_bottom(8);
        scroller.add(&content);

        let panel = Self {
            window,
            content,
            search,
            shared,
            executor,
        };
        panel.wire_search();
        panel.render("");
        panel
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            fade_out(&self.window, PANEL_MS);
        } else {
            self.render("");
            self.window.show();
            fade_in(&self.window, PANEL_MS);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    fn wire_search(&self) {
        let content = self.content.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        let window = self.window.clone();
        self.search.connect_changed(move |search| {
            let query = search.text().to_string();
            if window.is_visible() {
                render_into(&content, &shared, &executor, &window, &query);
            }
        });
    }

    /// Herbouw de hele inhoud uit de gedeelde snapshot, gefilterd op `query`.
    pub fn render(&self, query: &str) {
        render_into(&self.content, &self.shared, &self.executor, &self.window, query);
    }

    /// Start de periodieke render-loop (één glib-timer, geen eigen polls).
    pub fn start_refresh_loop(&self) {
        let content = self.content.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        let window = self.window.clone();
        let search = self.search.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(VAULT_POLL_MS), move || {
            if window.is_visible() {
                let query = search.text().to_string();
                render_into(&content, &shared, &executor, &window, &query);
            }
            ControlFlow::Continue
        });
    }
}

// ---------------------------------------------------------------------------
// Render: Devin-grouped sections
// ---------------------------------------------------------------------------

fn render_into(
    content: &gtk::Box,
    shared: &Shared,
    executor: &Executor,
    window: &gtk::Window,
    query: &str,
) {
    for child in content.children() {
        content.remove(&child);
    }

    let (snap, ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let profile = crate::config::global_profile().clone();
    let sessions = crate::sessions::load_ranked_sessions(&snap.events);
    let (state, line) = snap.tray_state();
    let q = query.to_lowercase();
    let all_actions = build_actions(&ops, &snap, &profile, sessions.clone());
    let ranked = rank_actions(&all_actions, query, 40);

    // Status-badge in de header-positie (bovenaan de content-stroom).
    let badge_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    badge_row.set_margin_top(8);
    badge_row.set_margin_start(20);
    badge_row.set_margin_end(20);
    let badge = gtk::Label::new(Some(&line));
    let badge_class = match state.as_str() {
        "offline" | "fout" => "error",
        "hulp" => "warn",
        "bezig" => "info",
        _ => "ok",
    };
    badge.style_context().add_class("chefbar-badge");
    badge.style_context().add_class(badge_class);
    badge_row.pack_start(&badge, false, false, 0);
    let updated = gtk::Label::new(Some(&format!(
        "{} · {}",
        profile.label("vaultApi"),
        snap.fetched_label()
    )));
    updated.set_halign(gtk::Align::End);
    updated.style_context().add_class("chefbar-card-meta");
    badge_row.pack_end(&updated, false, false, 0);
    content.pack_start(&badge_row, false, false, 0);

    // ---- Sectie: Acties (eerste, want interactie eerst) ----
    let actions_visible: Vec<&Action> = ranked.iter().filter(|a| !a.needs_text).take(6).collect();
    section_title(content, "Acties", "direct uitvoerbaar");
    let group = group_box();
    if actions_visible.is_empty() && !q.is_empty() {
        let empty = gtk::Label::new(Some("Geen acties voor deze zoekterm"));
        empty.set_halign(gtk::Align::Start);
        empty.set_xalign(0.0);
        empty.style_context().add_class("chefbar-card-meta");
        group.pack_start(&empty, false, false, 0);
    }
    for action in &actions_visible {
        let spec = action.run.clone();
        let executor = executor.clone();
        let window = window.clone();
        let needs_text = action.needs_text;
        let action = (*action).clone();
        let row = gtk::Button::new();
        row.set_relief(gtk::ReliefStyle::None);
        row.style_context().add_class("chefbar-row-btn");
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = gtk::Label::new(Some(&action.title));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.style_context().add_class("chefbar-card-title");
        row_box.pack_start(&title, true, true, 0);
        let stamp = stamp_label(&action.stamp);
        row_box.pack_end(&stamp, false, false, 0);
        row.add(&row_box);
        row.set_hexpand(true);
        let row_inner = row.child().unwrap();
        row_inner.set_margin_start(12);
        row_inner.set_margin_end(12);
        row_inner.set_margin_top(7);
        row_inner.set_margin_bottom(7);
        row.connect_clicked(move |_| {
            if needs_text {
                prompt_for(&executor, &window, &action);
            } else {
                executor.run_for_ui(&spec);
            }
        });
        group.pack_start(&row, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // Tekstacties onder de directe acties (kleine knoppenrij).
    let text_actions: Vec<&Action> = ranked.iter().filter(|a| a.needs_text).take(3).collect();
    if !text_actions.is_empty() {
        let wrap = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        wrap.set_margin_top(6);
        wrap.set_margin_start(20);
        wrap.set_margin_end(20);
        for action in text_actions {
            let btn = gtk::Button::with_label(&action.title);
            btn.style_context().add_class("chefbar-btn");
            let executor = executor.clone();
            let window = window.clone();
            let action = action.clone();
            btn.connect_clicked(move |_| {
                prompt_for(&executor, &window, &action);
            });
            wrap.pack_start(&btn, false, false, 0);
        }
        content.pack_start(&wrap, false, false, 0);
    }

    // ---- Sectie: Gezondheid ----
    section_title(content, "Gezondheid", "watchdog + dagscore + fleet");
    let group = group_box();
    let health_row = info_row(&snap.health.line(), Some(&state_label(&snap.health)));
    group.pack_start(&health_row, false, false, 0);
    let day_line = match (&snap.day_score.letter, snap.day_score.score) {
        (Some(letter), Some(score)) => format!("Dagscore {letter} ({score}/100)"),
        (None, Some(score)) => format!("Dagscore {score}/100"),
        _ => "Dagscore · n.v.t.".to_string(),
    };
    let day_row = info_row(&day_line, snap.day_score.source.as_deref());
    group.pack_start(&day_row, false, false, 0);
    if snap.fleet.total > 0 {
        let fleet_row = info_row(
            &format!("Fleet · {}/{} online", snap.fleet.online, snap.fleet.total),
            snap.fleet.host.as_deref(),
        );
        group.pack_start(&fleet_row, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Providers ----
    section_title(content, "Providers", "budgets en actief account");
    let group = group_box();
    let mut any_provider = false;
    for row in snap.providers.iter().filter(|r| {
        q.is_empty()
            || r.label.to_lowercase().contains(&q)
            || r.usage_text.to_lowercase().contains(&q)
    }) {
        any_provider = true;
        let card = gtk::Box::new(gtk::Orientation::Vertical, 3);
        card.set_margin_top(8);
        card.set_margin_bottom(8);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let name = gtk::Label::new(Some(&row.label));
        name.set_halign(gtk::Align::Start);
        name.set_xalign(0.0);
        name.style_context().add_class("chefbar-card-title");
        top.pack_start(&name, true, true, 0);
        let active = gtk::Label::new(Some(&row.usage_text));
        active.set_halign(gtk::Align::End);
        active.style_context().add_class("chefbar-card-meta");
        top.pack_end(&active, false, false, 0);
        card.pack_start(&top, false, false, 0);
        if row.requests.is_some() {
            let frac = row.usage_frac;
            let level = row.usage_level.clone();
            let track = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            track.set_size_request(200, 4);
            track.set_hexpand(true);
            track.style_context().add_class("chefbar-bar-track");
            let fill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            fill.set_size_request(((200.0 * frac).round() as i32).clamp(4, 200), 4);
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
            nums.style_context().add_class("chefbar-card-meta");
            bottom.pack_end(&nums, false, false, 0);
            card.pack_start(&bottom, false, false, 0);
        }
        let wrap = row_wrap(&card);
        group.pack_start(&wrap, false, false, 0);
    }
    if !any_provider {
        let empty = gtk::Label::new(Some("Geen provider-data"));
        empty.set_halign(gtk::Align::Start);
        empty.set_xalign(0.0);
        empty.style_context().add_class("chefbar-card-meta");
        empty.set_margin_start(16);
        empty.set_margin_top(8);
        empty.set_margin_bottom(8);
        group.pack_start(&empty, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Agents ----
    section_title(content, "Agents", "lopende werkstromen");
    let group = group_box();
    let mut any_agent = false;
    for agent in snap.agents.iter().filter(|a| {
        q.is_empty()
            || a.agent.to_lowercase().contains(&q)
            || a.workspace.to_lowercase().contains(&q)
    }) {
        any_agent = true;
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.set_size_request(8, 8);
        dot.set_halign(gtk::Align::Start);
        dot.set_valign(gtk::Align::Center);
        let (cls, stamp) = match agent.status.as_str() {
            "running" => ("info", "BEZIG"),
            "blocked" | "waiting" | "needs_input" | "input" | "attention" => ("warn", "HULP"),
            "failed" | "error" | "crashed" => ("down", "FOUT"),
            _ => ("ok", "STIL"),
        };
        dot.style_context().add_class("chefbar-dot");
        dot.style_context().add_class(cls);
        row.pack_start(&dot, false, false, 0);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
        let title = gtk::Label::new(Some(&format!("{} · {}", agent.agent, agent.workspace)));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.style_context().add_class("chefbar-card-title");
        text.pack_start(&title, false, false, 0);
        if !agent.summary.is_empty() {
            let summary = gtk::Label::new(Some(&agent.summary));
            summary.set_halign(gtk::Align::Start);
            summary.set_xalign(0.0);
            summary.set_ellipsize(pango::EllipsizeMode::End);
            summary.style_context().add_class("chefbar-card-meta");
            text.pack_start(&summary, false, false, 0);
        }
        row.pack_start(&text, true, true, 0);
        row.pack_end(&stamp_label(stamp), false, false, 0);
        let wrap = row_wrap(&row);
        group.pack_start(&wrap, false, false, 0);
    }
    if !any_agent {
        let empty = gtk::Label::new(Some("Geen agents actief"));
        empty.set_halign(gtk::Align::Start);
        empty.set_xalign(0.0);
        empty.style_context().add_class("chefbar-card-meta");
        empty.set_margin_start(16);
        empty.set_margin_top(8);
        empty.set_margin_bottom(8);
        group.pack_start(&empty, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Aandacht (sessions die jou nodig hebben) ----
    let attention: Vec<_> = sessions.iter().filter(|s| s.needs_attention()).collect();
    if !attention.is_empty() {
        section_title(content, "Heeft jou nodig", "aanhechtbare sessies");
        let group = group_box();
        for session in attention.iter().take(4) {
            let row = gtk::Box::new(gtk::Orientation::Vertical, 1);
            let title = gtk::Label::new(Some(&format!("{} · {}", session.source, session.title)));
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            title.set_ellipsize(pango::EllipsizeMode::End);
            title.style_context().add_class("chefbar-card-title");
            row.pack_start(&title, false, false, 0);
            if !session.summary.is_empty() {
                let meta = gtk::Label::new(Some(&session.summary));
                meta.set_halign(gtk::Align::Start);
                meta.set_xalign(0.0);
                meta.set_ellipsize(pango::EllipsizeMode::End);
                meta.style_context().add_class("chefbar-card-meta");
                row.pack_start(&meta, false, false, 0);
            }
            let wrap = row_wrap(&row);
            group.pack_start(&wrap, false, false, 0);
        }
        content.pack_start(&group, false, false, 0);
    }

    // ---- Footer ----
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.style_context().add_class("chefbar-footer");
    let footer_label = gtk::Label::new(Some(&format!(
        "ChefBar v{} · profiel {} · {}",
        crate::VERSION,
        profile.name,
        snap.fetched_label()
    )));
    footer_label.set_halign(gtk::Align::Start);
    footer.pack_start(&footer_label, true, true, 0);
    let quit_btn = gtk::Button::with_label("Verbergen");
    quit_btn.style_context().add_class("chefbar-gbtn");
    let window_hide = window.clone();
    quit_btn.connect_clicked(move |_| fade_out(&window_hide, PANEL_MS));
    footer.pack_end(&quit_btn, false, false, 0);
    content.pack_start(&footer, false, false, 0);

    content.show_all();
}

fn state_label(health: &crate::models::HealthInfo) -> String {
    if health.total == 0 {
        "onbekend".into()
    } else {
        format!("{} van {} ok", health.ok, health.total)
    }
}

fn section_title(content: &gtk::Box, title: &str, sub: &str) {
    let label = gtk::Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.style_context().add_class("chefbar-section-title");
    content.pack_start(&label, false, false, 0);
    if !sub.is_empty() {
        let sub_label = gtk::Label::new(Some(sub));
        sub_label.set_halign(gtk::Align::Start);
        sub_label.set_xalign(0.0);
        sub_label.style_context().add_class("chefbar-section-sub");
        content.pack_start(&sub_label, false, false, 0);
    }
}

fn group_box() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.style_context().add_class("chefbar-group");
    group
}

fn row_wrap(inner: &gtk::Box) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrap.style_context().add_class("chefbar-row");
    wrap.set_margin_start(16);
    wrap.set_margin_end(16);
    wrap.pack_start(inner, false, false, 0);
    wrap
}

fn stamp_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::End);
    label.set_valign(gtk::Align::Center);
    let cls = match text {
        "KLAAR" | "STIL" => "ok",
        "HULP" => "warn",
        "FOUT" | "LIMIET" => "error",
        "BEZIG" | "TAAK" => "info",
        _ => "ok",
    };
    label.style_context().add_class("chefbar-stamp");
    label.style_context().add_class(cls);
    label
}

fn info_row(text: &str, meta: Option<&str>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.style_context().add_class("chefbar-card-title");
    row.pack_start(&label, true, true, 0);
    if let Some(meta) = meta {
        let meta_label = gtk::Label::new(Some(meta));
        meta_label.set_halign(gtk::Align::End);
        meta_label.style_context().add_class("chefbar-card-meta");
        row.pack_end(&meta_label, false, false, 0);
    }
    let wrap = row_wrap(&row);
    wrap
}

/// Tekstdialog voor acties die input vragen (taak aanmaken, clipboard, prompt).
fn prompt_for(executor: &Executor, window: &gtk::Window, action: &Action) {
    let dialog = gtk::Window::new(gtk::WindowType::Toplevel);
    dialog.set_decorated(false);
    dialog.set_keep_above(true);
    dialog.set_position(gtk::WindowPosition::CenterOnParent);

    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    box_.set_margin_top(16);
    box_.set_margin_bottom(16);
    box_.set_margin_start(16);
    box_.set_margin_end(16);
    box_.style_context().add_class("chefbar-dialog");
    dialog.add(&box_);

    let title = gtk::Label::new(Some(&action.title));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.style_context().add_class("chefbar-card-title");
    box_.pack_start(&title, false, false, 0);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(&action.meta));
    entry.set_activates_default(true);
    box_.pack_start(&entry, false, false, 0);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Annuleren");
    cancel.style_context().add_class("chefbar-btn");
    let submit = gtk::Button::with_label("Uitvoeren");
    submit.style_context().add_class("chefbar-btn");
    submit.style_context().add_class("chefbar-primary");
    buttons.pack_start(&cancel, false, false, 0);
    buttons.pack_start(&submit, false, false, 0);
    box_.pack_start(&buttons, false, false, 0);

    let run = action.run.clone();
    let executor = executor.clone();

    let dialog_cancel = dialog.clone();
    cancel.connect_clicked(move |_| dialog_cancel.close());

    let dialog_submit = dialog.clone();
    let entry_submit = entry.clone();
    let executor_submit = executor.clone();
    let run_submit = run.clone();
    submit.connect_clicked(move |_| {
        let text = entry_submit.text().to_string();
        dialog_submit.close();
        executor_submit.run(&run_submit, &text);
    });

    let dialog_keys = dialog.clone();
    let executor_keys = executor.clone();
    let run_keys = run.clone();
    entry.connect_key_press_event(move |entry, event| {
        if event.keyval() == gdk::keys::constants::Return {
            let text = entry.text().to_string();
            dialog_keys.close();
            executor_keys.run(&run_keys, &text);
            return glib::Propagation::Stop;
        }
        if event.keyval() == gdk::keys::constants::Escape {
            dialog_keys.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    let (x, y) = window.position();
    dialog.move_(x + 24, y + 24);
    dialog.show_all();
    entry.grab_focus();
}