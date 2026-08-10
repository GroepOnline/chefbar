//! ChefBar-hoofdvenster: één volwaardig app-surface (geen mini-bar).
//!
//! Zoek-head bovenaan filtert alle secties live; eronder status, acties,
//! providers, agents, fleet en sessies. Inhoud wordt elke poll-cyclus opnieuw
//! gevuld uit de gedeelde snapshot: geen eigen poll-loops, geen netwerk op de
//! UI-thread.

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
        window.set_default_size(520, 680);
        window.set_keep_above(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.style_context().add_class("chefbar-panel");
        window.add(&root);

        // Zoek-head: filtert de hele surface.
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Zoek acties, agents, providers, sessies…"));
        search.set_margin_top(10);
        search.set_margin_bottom(10);
        search.set_margin_start(14);
        search.set_margin_end(14);
        search.style_context().add_class("chefbar-bar-entry");
        root.pack_start(&search, false, false, 0);

        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        root.pack_start(&scroller, true, true, 0);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
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
        render_into(
            &self.content,
            &self.shared,
            &self.executor,
            &self.window,
            query,
        );
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
    let all_actions = build_actions(&ops, &snap, &profile, sessions.clone());
    let ranked = rank_actions(&all_actions, query, 24);

    // Status-head met status-dot en metadata.
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.style_context().add_class("chefbar-header");
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.set_size_request(10, 10);
    dot.set_halign(gtk::Align::Start);
    dot.set_valign(gtk::Align::Center);
    let dot_class = match state.as_str() {
        "offline" => "down",
        "fout" => "down",
        "hulp" => "warn",
        "bezig" => "info",
        _ => "ok",
    };
    dot.style_context().add_class("chefbar-dot");
    dot.style_context().add_class(dot_class);
    header.pack_start(&dot, false, false, 0);

    let title_block = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let title = gtk::Label::new(Some(&line));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.style_context().add_class("chefbar-title");
    title_block.pack_start(&title, false, false, 0);
    let subtitle = gtk::Label::new(Some(&format!(
        "{} · geupdated {} · fleet {}/{} online",
        profile.label("vaultApi"),
        snap.fetched_label(),
        snap.fleet.online,
        snap.fleet.total
    )));
    subtitle.set_halign(gtk::Align::Start);
    subtitle.set_xalign(0.0);
    subtitle.style_context().add_class("chefbar-subtitle");
    title_block.pack_start(&subtitle, false, false, 0);
    header.pack_start(&title_block, true, true, 0);

    let refresh_btn = gtk::Button::with_label("Ververs");
    refresh_btn.style_context().add_class("chefbar-switch-btn");
    refresh_btn.connect_clicked(move |_| crate::state::refresh_global());
    header.pack_end(&refresh_btn, false, false, 0);
    content.pack_start(&header, false, false, 0);

    // Gezondheid + dagscore.
    section_label(content, "GEZONDHEID");
    let health_card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    health_card.style_context().add_class("chefbar-card");
    let health_line = gtk::Label::new(Some(&snap.health.line()));
    health_line.set_halign(gtk::Align::Start);
    health_line.set_xalign(0.0);
    health_card.pack_start(&health_line, false, false, 0);
    let day_line = match (&snap.day_score.letter, snap.day_score.score) {
        (Some(letter), Some(score)) => format!("Dagscore {letter} ({score}/100)"),
        (None, Some(score)) => format!("Dagscore {score}/100"),
        _ => "Dagscore · n.v.t.".to_string(),
    };
    let day_label = gtk::Label::new(Some(&day_line));
    day_label.set_halign(gtk::Align::Start);
    day_label.set_xalign(0.0);
    day_label.style_context().add_class("chefbar-card-meta");
    health_card.pack_start(&day_label, false, false, 0);
    content.pack_start(&health_card, false, false, 0);

    // Acties (gefilterd, ranked).
    let clickable: Vec<&Action> = ranked.iter().filter(|a| !a.needs_text).collect();
    section_label(content, "ACTIES");
    let grid = gtk::FlowBox::new();
    grid.set_min_children_per_line(2);
    grid.set_max_children_per_line(2);
    for action in clickable.iter().take(10) {
        let button = gtk::Button::with_label(&action.title);
        button.style_context().add_class("chefbar-actions");
        if action.destructive {
            button.style_context().add_class("chefbar-primary");
        }
        let spec = action.run.clone();
        let executor = executor.clone();
        button.connect_clicked(move |_| {
            executor.run_for_ui(&spec);
        });
        grid.add(&button);
    }
    content.pack_start(&grid, false, false, 0);

    // Providers met usage-bars.
    section_label(content, "PROVIDERS");
    let providers_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    for row in &snap.providers {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 3);
        card.style_context().add_class("chefbar-card");
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let name = gtk::Label::new(Some(&format!("{} · {}", row.label, row.usage_text)));
        name.set_halign(gtk::Align::Start);
        name.set_xalign(0.0);
        name.set_ellipsize(pango::EllipsizeMode::End);
        top.pack_start(&name, true, true, 0);
        card.pack_start(&top, false, false, 0);
        if row.requests.is_some() && row.available {
            let frac = row.usage_frac;
            let level = row.usage_level.clone();
            let track = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            track.set_size_request(200, 4);
            track.style_context().add_class("chefbar-bar-track");
            let fill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            fill.set_size_request(((200.0 * frac).round() as i32).clamp(4, 200), 4);
            fill.style_context().add_class("chefbar-bar-fill");
            fill.style_context().add_class(&level);
            track.pack_start(&fill, false, false, 0);
            let meta = gtk::Label::new(Some(&format!(
                "{} req · {}M tok",
                row.requests.unwrap_or(0),
                row.tokens.unwrap_or(0) / 1_000_000
            )));
            meta.set_halign(gtk::Align::End);
            meta.style_context().add_class("chefbar-card-meta");
            let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            bottom.pack_start(&track, true, true, 0);
            bottom.pack_start(&meta, false, false, 0);
            card.pack_start(&bottom, false, false, 0);
        }
        providers_box.pack_start(&card, false, false, 0);
    }
    content.pack_start(&providers_box, false, false, 0);

    // Agents.
    section_label(content, "AGENTS");
    let agents_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let q = query.to_lowercase();
    for agent in snap.agents.iter().filter(|a| {
        q.is_empty()
            || a.agent.to_lowercase().contains(&q)
            || a.workspace.to_lowercase().contains(&q)
    }) {
        let card = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        card.style_context().add_class("chefbar-card");
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
        card.pack_start(&dot, false, false, 0);
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
        card.pack_start(&text, true, true, 0);
        let stamp_label = gtk::Label::new(Some(stamp));
        stamp_label
            .style_context()
            .add_class("chefbar-bar-row-stamp");
        card.pack_end(&stamp_label, false, false, 0);
        agents_box.pack_start(&card, false, false, 0);
    }
    content.pack_start(&agents_box, false, false, 0);

    // Sessies (CHEF-shaped, aandacht eerst).
    let attention: Vec<_> = sessions.iter().filter(|s| s.needs_attention()).collect();
    if !attention.is_empty() {
        section_label(content, "HEEFT JOU NODIG");
        let sessions_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        for session in attention.iter().take(4) {
            let card = gtk::Box::new(gtk::Orientation::Vertical, 2);
            card.style_context().add_class("chefbar-card");
            let title = gtk::Label::new(Some(&format!("{} · {}", session.source, session.title)));
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            title.set_ellipsize(pango::EllipsizeMode::End);
            title.style_context().add_class("chefbar-card-title");
            card.pack_start(&title, false, false, 0);
            if !session.summary.is_empty() {
                let meta = gtk::Label::new(Some(&session.summary));
                meta.set_halign(gtk::Align::Start);
                meta.set_xalign(0.0);
                meta.set_ellipsize(pango::EllipsizeMode::End);
                meta.style_context().add_class("chefbar-card-meta");
                card.pack_start(&meta, false, false, 0);
            }
            sessions_box.pack_start(&card, false, false, 0);
        }
        content.pack_start(&sessions_box, false, false, 0);
    }

    // Footer.
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
    let close_btn = gtk::Button::with_label("Sluiten");
    close_btn.style_context().add_class("chefbar-switch-btn");
    let window_for_close = window.clone();
    close_btn.connect_clicked(move |_| {
        fade_out(&window_for_close, PANEL_MS);
    });
    footer.pack_end(&close_btn, false, false, 0);
    content.pack_start(&footer, false, false, 0);
}

fn section_label(content: &gtk::Box, text: &str) {
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.style_context().add_class("chefbar-section-label");
    content.pack_start(&label, false, false, 0);
}
