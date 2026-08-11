//! ChefBar-hoofdvenster: één echte app (Devin-stijl), geen floating bar.
//!
//! Undecorated window met custom header (drag + minimize + sluiten), zoek-
//! input die alle secties live filtert, gegroepeerde cards per sectie
//! (Devin-sgroup met hairlines), en footer. Inhoud wordt elke poll-cyclus
//! opnieuw gevuld uit de gedeelde snapshot: geen eigen poll-loops, geen
//! netwerk op de UI-thread.
//!
//! Room-model: meerdere harnassen tegelijk zichtbaar (fleet / commerce / eval).
//! Het panel toont harnas-tabs; acties worden gefilterd op het geselecteerde
//! harnas via prefix-match op keywords.

use crate::actions::{build_actions, Executor};
use crate::harness::{build_harnesses, Harness, HarnessKind};
use crate::motion::{fade_in, fade_out, PANEL_MS};
use crate::palette::{rank_actions, Action};
use crate::state::{Shared, VAULT_POLL_MS};
use glib::ControlFlow;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Panel {
    pub window: gtk::Window,
    content: gtk::Box,
    search: gtk::SearchEntry,
    shared: Shared,
    executor: Executor,
    // Backward compat: mirror of harness_state (sidebar/harness tabs drive it)
    pub active_harness: String,
    harness_state: Rc<RefCell<String>>,
    nav_buttons: Rc<Vec<(String, gtk::Button)>>,
}

impl Panel {
    pub fn new(shared: Shared, executor: Executor) -> Self {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("ChefBar");
        window.set_decorated(false);
        window.set_default_size(760, 840);
        window.set_keep_above(true);
        window.set_position(gtk::WindowPosition::Center);

        // ---- Room layout: sidebar (220px fixed) + main canvas ----
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.style_context().add_class("chefbar-app");
        window.add(&root);

        // ---- Sidebar (fixed 220px) ----
        let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar.style_context().add_class("chefbar-sidebar");
        sidebar.set_size_request(220, -1);

        // App-title
        let sidebar_title_wrap = gtk::Box::new(gtk::Orientation::Vertical, 2);
        sidebar_title_wrap.set_margin_top(14);
        sidebar_title_wrap.set_margin_start(14);
        sidebar_title_wrap.set_margin_end(14);
        sidebar_title_wrap.set_margin_bottom(10);
        let sidebar_title = gtk::Label::new(Some("ChefBar"));
        sidebar_title.set_halign(gtk::Align::Start);
        sidebar_title.set_xalign(0.0);
        sidebar_title.style_context().add_class("chefbar-sidebar-title");
        sidebar_title_wrap.pack_start(&sidebar_title, false, false, 0);
        let sidebar_sub = gtk::Label::new(Some("agentische assistent"));
        sidebar_sub.set_halign(gtk::Align::Start);
        sidebar_sub.set_xalign(0.0);
        sidebar_sub.style_context().add_class("chefbar-sidebar-sub");
        sidebar_title_wrap.pack_start(&sidebar_sub, false, false, 0);
        sidebar.pack_start(&sidebar_title_wrap, false, false, 0);

        // Nav-lijst — live gekoppeld aan harness-state (fleet/commerce/eval)
        let nav_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        nav_box.style_context().add_class("chefbar-nav");
        nav_box.set_margin_start(8);
        nav_box.set_margin_end(8);
        // We bouwen de nav-knoppen hier maar wire pas na harness_state.
        // Placeholder: wordt direct hieronder gevuld.
        let nav_ids = ["fleet", "commerce", "eval"];
        let nav_labels = ["Fleet", "Commerce", "Eval"];
        let mut nav_buttons: Vec<(String, gtk::Button)> = Vec::new();
        for (idx, (id, label)) in nav_ids.iter().zip(nav_labels.iter()).enumerate() {
            let btn = gtk::Button::with_label(*label);
            btn.set_relief(gtk::ReliefStyle::None);
            btn.style_context().add_class("chefbar-nav-item");
            btn.set_hexpand(true);
            btn.set_halign(gtk::Align::Fill);
            if let Some(child) = btn.child() {
                if let Some(lbl) = child.downcast_ref::<gtk::Label>() {
                    lbl.set_halign(gtk::Align::Start);
                    lbl.set_xalign(0.0);
                }
            }
            if idx == 0 {
                btn.style_context().add_class("active");
            }
            nav_buttons.push((id.to_string(), btn.clone()));
            nav_box.pack_start(&btn, false, false, 0);
        }
        let nav_buttons_rc = Rc::new(nav_buttons);
        sidebar.pack_start(&nav_box, false, false, 0);

        // Spacer zodat footer onderaan blijft
        let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar.pack_start(&spacer, true, true, 0);

        // Status-footer
        let status_footer = gtk::Box::new(gtk::Orientation::Vertical, 4);
        status_footer.style_context().add_class("chefbar-sidebar-footer");
        status_footer.set_margin_start(12);
        status_footer.set_margin_end(12);
        status_footer.set_margin_top(10);
        status_footer.set_margin_bottom(12);
        let footer_title = gtk::Label::new(Some("Status"));
        footer_title.set_halign(gtk::Align::Start);
        footer_title.set_xalign(0.0);
        footer_title.style_context().add_class("chefbar-sidebar-footer-title");
        status_footer.pack_start(&footer_title, false, false, 0);
        let footer_meta = gtk::Label::new(Some("online \u{00b7} devin-skin"));
        footer_meta.set_halign(gtk::Align::Start);
        footer_meta.set_xalign(0.0);
        footer_meta.style_context().add_class("chefbar-sidebar-footer-meta");
        status_footer.pack_start(&footer_meta, false, false, 0);
        sidebar.pack_end(&status_footer, false, false, 0);

        root.pack_start(&sidebar, false, false, 0);

        // ---- Main canvas ----
        let main = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main.style_context().add_class("chefbar-main");
        main.set_hexpand(true);

        // Header (title + search + controls)
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        header.style_context().add_class("chefbar-header");
        header.set_margin_bottom(0);

        let title_block = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let title = gtk::Label::new(Some("ChefBar"));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.style_context().add_class("chefbar-title");
        title_block.pack_start(&title, false, false, 0);
        let title_sub = gtk::Label::new(Some("agentische assistent \u{00b7} devin-skin"));
        title_sub.set_halign(gtk::Align::Start);
        title_sub.set_xalign(0.0);
        title_sub.set_ellipsize(pango::EllipsizeMode::End);
        title_sub.style_context().add_class("chefbar-title-sub");
        title_block.pack_start(&title_sub, false, false, 0);
        header.pack_start(&title_block, false, false, 0);

        // Search in header (hexpand) — enige SearchEntry, single source of truth
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Zoek acties, agents, providers, sessies\u{2026}"));
        search.style_context().add_class("chefbar-search");
        search.set_hexpand(true);
        search.set_halign(gtk::Align::Fill);
        header.pack_start(&search, true, true, 0);

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
        main.pack_start(&header, false, false, 0);

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

        // "/" → focus search, Esc → verbergen (Raycast-geest).
        {
            let search_focus = search.clone();
            let window_esc = window.clone();
            window.connect_key_press_event(move |_, event| {
                let kv = event.keyval();
                if kv == gdk::keys::constants::Escape {
                    fade_out(&window_esc, PANEL_MS);
                    return glib::Propagation::Stop;
                }
                if kv == gdk::keys::constants::slash && !search_focus.has_focus() {
                    search_focus.grab_focus();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }

        // ---- Content ----
        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_min_content_height(480);
        main.pack_start(&scroller, true, true, 0);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_margin_bottom(8);
        scroller.add(&content);

        root.pack_start(&main, true, true, 0);
        // harnas-state: default naar eerste harnas (fleet)
        let initial = "fleet".to_string();
        let harness_state = Rc::new(RefCell::new(initial.clone()));
        // Wire sidebar nav → harness_state + content re-render + active-class sync
        {
            for (id, btn) in nav_buttons_rc.iter() {
                let id = id.clone();
                let harness_state_clone = harness_state.clone();
                let content_clone = content.clone();
                let shared_clone = shared.clone();
                let executor_clone = executor.clone();
                let window_clone = window.clone();
                let search_clone = search.clone();
                let nav_rc = nav_buttons_rc.clone();
                let id_for_class = id.clone();
                let btn_clone = btn.clone();
                btn_clone.connect_clicked(move |_| {
                    *harness_state_clone.borrow_mut() = id.clone();
                    for (other_id, other_btn) in nav_rc.iter() {
                        if *other_id == id_for_class {
                            other_btn.style_context().add_class("active");
                        } else {
                            other_btn.style_context().remove_class("active");
                        }
                    }
                    let q = search_clone.text().to_string();
                    render_into(&content_clone, &shared_clone, &executor_clone, &window_clone, &q, &harness_state_clone);
                });
            }
        }
        let panel = Self {
            window,
            content,
            search,
            shared,
            executor,
            active_harness: initial.clone(),
            harness_state: harness_state.clone(),
            nav_buttons: nav_buttons_rc.clone(),
        };
        panel.wire_search();
        // Initieel ook nav active sync via harness_state
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
        let harness_state = self.harness_state.clone();
        self.search.connect_changed(move |search| {
            let query = search.text().to_string();
            if window.is_visible() {
                render_into(&content, &shared, &executor, &window, &query, &harness_state);
            }
        });
    }

    /// Herbouw de hele inhoud uit de gedeelde snapshot, gefilterd op `query`.
    pub fn render(&self, query: &str) {
        // houd active_harness in sync met gedeelde state
        let current = self.harness_state.borrow().clone();
        // als snapshot al beschikbaar is, valideer tegen echte harnassen
        {
            let snap = self.shared.snapshot.read().unwrap().clone();
            let ops = self.shared.ops.read().unwrap().clone();
            let harnesses = build_harnesses(&snap, &ops);
            if !harnesses.is_empty() && !harnesses.iter().any(|h| h.id == current) {
                if let Some(first) = harnesses.first() {
                    *self.harness_state.borrow_mut() = first.id.clone();
                }
            }
        }
        self.sync_sidebar_nav();
        render_into(
            &self.content,
            &self.shared,
            &self.executor,
            &self.window,
            query,
            &self.harness_state,
        );
    }

    fn sync_sidebar_nav(&self) {
        let active = self.harness_state.borrow().clone();
        for (id, btn) in self.nav_buttons.iter() {
            if *id == active {
                btn.style_context().add_class("active");
            } else {
                btn.style_context().remove_class("active");
            }
        }
    }

    /// Start de periodieke render-loop (één glib-timer, geen eigen polls).
    pub fn start_refresh_loop(&self) {
        let content = self.content.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        let window = self.window.clone();
        let search = self.search.clone();
        let harness_state = self.harness_state.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(VAULT_POLL_MS), move || {
            if window.is_visible() {
                let query = search.text().to_string();
                render_into(&content, &shared, &executor, &window, &query, &harness_state);
            }
            ControlFlow::Continue
        });
    }
}

// ---------------------------------------------------------------------------
// Room-helpers: harnas-filtering
// ---------------------------------------------------------------------------

/// Bepaalt of een action bij het geselecteerde harnas hoort via prefix-match
/// op keywords (elke keyword-token wordt tegen de prefixes van het harnas
/// getest).
fn action_matches_harness(action: &Action, kind: &HarnessKind) -> bool {
    let prefixes = kind.prefixes();
    let kw = action.keywords.to_lowercase();
    let tokens: Vec<&str> = kw.split_whitespace().collect();
    for prefix in prefixes {
        let p = prefix.to_lowercase();
        for token in &tokens {
            if token.starts_with(&p) {
                return true;
            }
        }
    }
    false
}

/// Filter acties op harnas; als kind None is, geen filtering.
fn filter_actions_by_harness(actions: Vec<Action>, kind: Option<&HarnessKind>) -> Vec<Action> {
    if let Some(k) = kind {
        actions
            .into_iter()
            .filter(|a| action_matches_harness(a, k))
            .collect()
    } else {
        actions
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
    harness_state: &Rc<RefCell<String>>,
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
    // Cache eenmalig (vermijd dubbele alloc + Url::parse per render).
    let vault_label = profile.label("vaultApi");
    let fetched = snap.fetched_label();
    let sessions = crate::sessions::load_ranked_sessions(&snap.events);
    let (state, line) = snap.tray_state();
    let q = query.to_lowercase();

    // ---- Harnassen (room) -------------------------------------------------
    let harnesses: Vec<Harness> = build_harnesses(&snap, &ops);
    // valideer geselecteerde harnas, fallback naar eerste
    let active_id = {
        let current = harness_state.borrow().clone();
        if harnesses.iter().any(|h| h.id == current) {
            current
        } else if let Some(first) = harnesses.first() {
            let id = first.id.clone();
            *harness_state.borrow_mut() = id.clone();
            id
        } else {
            "fleet".to_string()
        }
    };
    let active_kind = harnesses
        .iter()
        .find(|h| h.id == active_id)
        .map(|h| h.kind.clone());

    // harnas-tabs: room-navigatie
    if !harnesses.is_empty() {
        let harness_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        harness_row.style_context().add_class("chefbar-harness-row");
        harness_row.set_margin_top(10);
        harness_row.set_margin_start(20);
        harness_row.set_margin_end(20);
        for h in &harnesses {
            let label_text = if h.queue_depth > 0 {
                format!("{} · {}", h.label, h.queue_depth)
            } else {
                h.label.clone()
            };
            let btn = gtk::Button::with_label(&label_text);
            btn.set_relief(gtk::ReliefStyle::None);
            if h.id == active_id {
                btn.style_context().add_class("chefbar-harness-active");
            } else {
                btn.style_context().add_class("chefbar-harness");
            }
            // kleur-accent als tooltip/status
            btn.set_tooltip_text(Some(&format!("{} — {}", h.id, h.status.label())));
            let harness_state_clone = harness_state.clone();
            let content_clone = content.clone();
            let shared_clone = shared.clone();
            let executor_clone = executor.clone();
            let window_clone = window.clone();
            let query_clone = query.to_string();
            let id_clone = h.id.clone();
            btn.connect_clicked(move |_| {
                *harness_state_clone.borrow_mut() = id_clone.clone();
                render_into(
                    &content_clone,
                    &shared_clone,
                    &executor_clone,
                    &window_clone,
                    &query_clone,
                    &harness_state_clone,
                );
            });
            harness_row.pack_start(&btn, false, false, 0);
        }
        content.pack_start(&harness_row, false, false, 0);
    }

    let all_actions = build_actions(&ops, &snap, &profile, sessions.clone());
    let ranked = rank_actions(&all_actions, query, 40);
    // filter op geselecteerde harnas (prefix-match op keywords)
    let ranked = filter_actions_by_harness(ranked, active_kind.as_ref());

    // Status-badge in de header-positie (bovenaan de content-stroom).
    let badge_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    badge_row.set_margin_top(8);
    badge_row.set_margin_start(20);
    badge_row.set_margin_end(20);
    let badge = gtk::Label::new(Some(&line));
    badge.set_xalign(0.0);
    badge.set_ellipsize(pango::EllipsizeMode::End);
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
        vault_label,
        fetched
    )));
    updated.set_halign(gtk::Align::End);
    updated.set_xalign(1.0);
    updated.set_ellipsize(pango::EllipsizeMode::End);
    updated.style_context().add_class("chefbar-card-meta");
    badge_row.pack_end(&updated, false, false, 0);
    content.pack_start(&badge_row, false, false, 0);

    // ---- Sectie: Acties (eerste, want interactie eerst) ----
    let actions_visible: Vec<&Action> = ranked.iter().filter(|a| !a.needs_text).take(6).collect();
    section_title(content, "Acties", "zoek of kies — gefilterd op dit harnas");
    let group = group_box();
    if actions_visible.is_empty() && !q.is_empty() {
        let empty = gtk::Label::new(Some("Geen acties voor deze zoekterm"));
        empty.set_halign(gtk::Align::Start);
        empty.set_xalign(0.0);
        empty.set_ellipsize(pango::EllipsizeMode::End);
        empty.style_context().add_class("chefbar-card-meta");
        group.pack_start(&empty, false, false, 0);
    } else if actions_visible.is_empty() {
        let empty = gtk::Label::new(Some("Geen acties voor dit harnas"));
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
    section_title(content, "Providers", "accounts, budgets en fleet");
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
        name.set_ellipsize(pango::EllipsizeMode::End);
        name.style_context().add_class("chefbar-card-title");
        top.pack_start(&name, true, true, 0);
        let active = gtk::Label::new(Some(&row.usage_text));
        active.set_halign(gtk::Align::End);
        active.set_xalign(1.0);
        active.set_ellipsize(pango::EllipsizeMode::End);
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
            nums.set_xalign(1.0);
            nums.set_ellipsize(pango::EllipsizeMode::End);
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
        empty.set_ellipsize(pango::EllipsizeMode::End);
        empty.style_context().add_class("chefbar-card-meta");
        empty.set_margin_start(16);
        empty.set_margin_top(8);
        empty.set_margin_bottom(8);
        group.pack_start(&empty, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Agents ----
    section_title(content, "Agents", "herdr en lopende werkstromen");
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
            summary.set_line_wrap(true);
            summary.set_lines(2);
            summary.set_ellipsize(pango::EllipsizeMode::End);
            summary.set_max_width_chars(64);
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
        empty.set_ellipsize(pango::EllipsizeMode::End);
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
                meta.set_line_wrap(true);
                meta.set_lines(2);
                meta.set_ellipsize(pango::EllipsizeMode::End);
                meta.set_max_width_chars(64);
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
        fetched
    )));
    footer_label.set_halign(gtk::Align::Start);
    footer_label.set_xalign(0.0);
    footer_label.set_ellipsize(pango::EllipsizeMode::End);
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
    title.set_ellipsize(pango::EllipsizeMode::End);
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
