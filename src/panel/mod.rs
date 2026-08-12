//! ChefApp-hoofdvenster: één echte app (Signaal v2), geen floating bar.
//!
//! Undecorated window met custom header (drag + minimize + sluiten), zoek-
//! input die alle secties live filtert, gegroepeerde cards per sectie
//! (zones met hairlines), en footer. Inhoud wordt elke poll-cyclus
//! opnieuw gevuld uit de gedeelde snapshot: geen eigen poll-loops, geen
//! netwerk op de UI-thread.
//!
//! Room-model: meerdere harnassen tegelijk zichtbaar (fleet / commerce / eval).
//! Navigatie loopt via de sidebar; acties worden gefilterd op het geselecteerde
//! harnas via prefix-match op keywords.
//!
//! Lane C: 1504 r monoliet gesplitst in 5 modules. Dit bestand blijft de
//! lifecycle (Panel struct, new(), show/toggle, refresh-loop).

pub mod domains;
pub mod drawer;
pub mod header;
pub mod overlay;
pub mod sidebar;
pub mod zones;

use crate::actions::{build_actions, Executor};
use crate::harness::{build_harnesses, Harness, HarnessKind};
use crate::motion::{fade_in, fade_out, PANEL_MS};
use crate::palette::{rank_actions_with, Action, RankContext};
use crate::state::{Shared, VAULT_POLL_MS};
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use drawer::Drawer;
use overlay::Overlay;
use zones::{
    empty_state, group_box, group_box_attention, row_wrap, section_title, stamp_label, truncate_q,
};

pub struct Panel {
    pub window: gtk::Window,
    content: gtk::Box,
    search: gtk::SearchEntry,
    shared: Shared,
    executor: Executor,
    // geselecteerde harnas binnen de room — default naar eerste harnas
    // alias: active_group == active_harness (backwards compat)
    pub active_harness: String,
    pub active_group: String,
    harness_state: Rc<RefCell<String>>,
    nav_buttons: Rc<Vec<(String, gtk::Button)>>,
    /// UI-state (harnas + zoekterm) is gewijzigd maar nog niet naar disk.
    persist_dirty: Rc<Cell<bool>>,
    drawer: Rc<Drawer>,
    overlay: Rc<Overlay>,
    density: Rc<RefCell<String>>,
    window_overlay: gtk::Overlay,
    footer_label: gtk::Label,
    theme: Rc<RefCell<String>>,
    header_title: gtk::Label,
}

impl Panel {
    pub fn new(shared: Shared, executor: Executor) -> Self {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("ChefApp");
        window.set_decorated(false);
        // Vaste geometrie 860×880 (plan §4.3 / §5.1): min==max via size_request + resizable(false)
        window.set_default_size(860, 880);
        window.set_size_request(860, 880);
        window.set_resizable(false);
        window.set_keep_above(true);
        window.set_position(gtk::WindowPosition::Center);

        // Persisted state — tolerant, backwards compat (harness → active_group)
        let persisted = crate::panel_state::load();
        let persisted_density = crate::panel_state::normalize_density(&persisted.density);
        let persisted_query = persisted.query.clone().unwrap_or_default();
        let persisted_drawer_open = persisted.drawer_open;
        let initial = persisted
            .effective_group()
            .map(|s| s.to_string())
            .filter(|id| sidebar::NAV_IDS.contains(&id.as_str()))
            .unwrap_or_else(|| "fleet".to_string());

        // Density-token klas op window
        let density_class = if persisted_density == crate::panel_state::DENSITY_COMPACT {
            "density-compact"
        } else {
            "density-comfortable"
        };
        window.style_context().add_class(density_class);
        window.style_context().add_class("chefbar-app");

        // ---- Room layout: sidebar (240px fixed) + main canvas (+ drawer) ----
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.style_context().add_class("chefbar-app");

        // Sidebar via module
        let (sidebar, nav_buttons_vec) = sidebar::build_sidebar(&initial);
        let nav_buttons_rc = Rc::new(nav_buttons_vec);
        root.pack_start(&sidebar, false, false, 0);

        // ---- Main canvas ----
        let main = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main.style_context().add_class("chefbar-main");
        main.set_hexpand(true);

        // Header via module — titel toont de actieve domeinnaam (render_into).
        let (header, header_title, search, refresh_btn, min_btn, close_btn) =
            header::build_header();
        if !persisted_query.trim().is_empty() {
            search.set_text(&persisted_query);
        }
        // Wire header knoppen
        refresh_btn.connect_clicked(move |_| crate::state::refresh_global());
        let window_for_min = window.clone();
        min_btn.connect_clicked(move |_| window_for_min.iconify());
        let window_for_close = window.clone();
        close_btn.connect_clicked(move |_| fade_out(&window_for_close, PANEL_MS));

        // Drag via header
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
            gtk::glib::Propagation::Proceed
        });
        main.pack_start(&header, false, false, 0);

        // Drawer + Overlay instanties
        let drawer = Rc::new(Drawer::new());
        let overlay = Rc::new(Overlay::new());

        // Drawer initieel verborgen; open als persisted_drawer_open (alleen als we content hebben om te tonen)
        // We tonen hem later via Panel::render als er een geselecteerde action is; hier alleen state onthouden.
        // Bewaar persisted waarde voor wiring; niet direct reveal.

        // ---- Content ----
        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_vexpand(true);
        main.pack_start(&scroller, true, true, 0);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_margin_bottom(8);
        scroller.add(&content);

        root.pack_start(&main, true, true, 0);

        // ---- Footer: gepind onder de scroller (live counts + toggles) ----
        let persist_dirty = Rc::new(Cell::new(false));
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        footer.style_context().add_class("chefbar-footer");
        let footer_label = gtk::Label::new(Some(""));
        footer_label.set_halign(gtk::Align::Start);
        footer_label.set_xalign(0.0);
        footer_label.set_ellipsize(pango::EllipsizeMode::End);
        footer_label.set_line_wrap(false);
        footer_label.set_max_width_chars(64);
        footer_label
            .style_context()
            .add_class("chefbar-footer-label");
        footer.pack_start(&footer_label, true, true, 0);

        // Density-toggle (rustig ↔ compact) — CSS-klas op het window
        let density = Rc::new(RefCell::new(persisted_density.clone()));
        let theme = Rc::new(RefCell::new(crate::css::active_theme()));
        let density_btn = gtk::Button::with_label(
            if persisted_density == crate::panel_state::DENSITY_COMPACT {
                "Compact"
            } else {
                "Rustig"
            },
        );
        density_btn.set_relief(gtk::ReliefStyle::None);
        density_btn.style_context().add_class("chefbar-footer-btn");
        if persisted_density == crate::panel_state::DENSITY_COMPACT {
            density_btn.style_context().add_class("on");
        }
        let theme_btn =
            gtk::Button::with_label(if theme.borrow().as_str() == crate::css::THEME_LIGHT {
                "Licht"
            } else {
                "Donker"
            });
        theme_btn.set_relief(gtk::ReliefStyle::None);
        theme_btn.style_context().add_class("chefbar-footer-btn");
        if theme.borrow().as_str() == crate::css::THEME_LIGHT {
            theme_btn.style_context().add_class("on");
        }
        {
            let window_cls = window.clone();
            let density_toggle = density.clone();
            let dirty_cls = persist_dirty.clone();
            let btn = density_btn.clone();
            density_btn.connect_clicked(move |_| {
                let compact =
                    density_toggle.borrow().as_str() == crate::panel_state::DENSITY_COMPACT;
                let next = if compact {
                    crate::panel_state::DENSITY_COMFORTABLE
                } else {
                    crate::panel_state::DENSITY_COMPACT
                };
                *density_toggle.borrow_mut() = next.to_string();
                window_cls.style_context().remove_class(if compact {
                    "density-compact"
                } else {
                    "density-comfortable"
                });
                window_cls.style_context().add_class(
                    if next == crate::panel_state::DENSITY_COMPACT {
                        "density-compact"
                    } else {
                        "density-comfortable"
                    },
                );
                btn.set_label(if next == crate::panel_state::DENSITY_COMPACT {
                    "Compact"
                } else {
                    "Rustig"
                });
                if next == crate::panel_state::DENSITY_COMPACT {
                    btn.style_context().add_class("on");
                } else {
                    btn.style_context().remove_class("on");
                }
                dirty_cls.set(true);
            });
        }
        {
            let theme_toggle = theme.clone();
            let dirty_cls = persist_dirty.clone();
            let btn = theme_btn.clone();
            theme_btn.connect_clicked(move |_| {
                let current = theme_toggle.borrow().clone();
                let next = if current == crate::css::THEME_LIGHT {
                    crate::css::THEME_DARK
                } else {
                    crate::css::THEME_LIGHT
                };
                crate::css::set_theme(next);
                *theme_toggle.borrow_mut() = next.to_string();
                btn.set_label(if next == crate::css::THEME_LIGHT {
                    "Licht"
                } else {
                    "Donker"
                });
                if next == crate::css::THEME_LIGHT {
                    btn.style_context().add_class("on");
                } else {
                    btn.style_context().remove_class("on");
                }
                dirty_cls.set(true);
            });
        }
        let quit_btn = gtk::Button::with_label("Verbergen");
        quit_btn.set_relief(gtk::ReliefStyle::None);
        quit_btn.style_context().add_class("chefbar-footer-btn");
        let window_hide = window.clone();
        quit_btn.connect_clicked(move |_| fade_out(&window_hide, PANEL_MS));
        footer.pack_end(&quit_btn, false, false, 0);
        footer.pack_end(&theme_btn, false, false, 0);
        footer.pack_end(&density_btn, false, false, 0);
        main.pack_start(&footer, false, false, 0);

        // Drawer als derde kolom naast main (Revealer 300px)
        root.pack_start(drawer.widget(), false, false, 0);
        // Zorgt dat drawer initially hidden maar wel in layout
        drawer.widget().set_visible(true);
        drawer.widget().set_reveal_child(persisted_drawer_open);

        // GtkOverlay voor palette-overlay bovenop root
        let window_overlay = gtk::Overlay::new();
        window_overlay.add(&root);
        // Palette overlay: zwevend boven het canvas (pass-through wanneer
        // hidden, zodat het canvas gewoon klikbaar blijft).
        overlay.widget().set_halign(gtk::Align::Center);
        overlay.widget().set_valign(gtk::Align::Start);
        overlay.widget().set_margin_top(56);
        overlay.widget().set_margin_start(96);
        overlay.widget().set_margin_end(96);
        window_overlay.add_overlay(overlay.widget());
        window_overlay.set_overlay_pass_through(overlay.widget(), true);
        window.add(&window_overlay);

        // Esc / "/" / focus — nu met drawer > overlay > panel prioriteit
        {
            let search_focus = search.clone();
            let window_esc = window.clone();
            let drawer_esc = drawer.clone();
            let overlay_esc = overlay.clone();
            window.connect_key_press_event(move |_, event| {
                let kv = event.keyval();
                if kv == gdk::keys::constants::Escape {
                    if drawer_esc.is_open() {
                        drawer_esc.hide();
                        return gtk::glib::Propagation::Stop;
                    }
                    if overlay_esc.is_visible() {
                        overlay_esc.hide();
                        return gtk::glib::Propagation::Stop;
                    }
                    fade_out(&window_esc, PANEL_MS);
                    return gtk::glib::Propagation::Stop;
                }
                let ctrl_or_cmd = event.state().contains(
                    gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::META_MASK
                        | gdk::ModifierType::SUPER_MASK,
                );
                if (kv == gdk::keys::constants::slash
                    || (kv == gdk::keys::constants::k && ctrl_or_cmd))
                    && !search_focus.has_focus()
                {
                    search_focus.grab_focus();
                    search_focus.select_region(0, -1);
                    return gtk::glib::Propagation::Stop;
                }
                gtk::glib::Propagation::Proceed
            });
        }

        let persist_dirty = Rc::new(Cell::new(false));
        let harness_state = Rc::new(RefCell::new(initial.clone()));

        // density + theme Rcs bestaan al (footer-wiring hierboven).

        // Wire sidebar nav → harness_state + content re-render + sync_nav_buttons + recent_domains
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
                let dirty_clone = persist_dirty.clone();
                let density_clone = density.clone();
                let drawer_clone = drawer.clone();
                let footer_label_clone = footer_label.clone();
                let header_title_clone = header_title.clone();
                let _density_class = density_class.to_string();
                btn_clone.connect_clicked(move |_| {
                    // Ctx binnen de closure: de clones leven in de closure zelf.
                    let render_ctx = RenderCtx {
                        executor: &executor_clone,
                        window: &window_clone,
                        drawer: &drawer_clone,
                        footer_label: &footer_label_clone,
                        header_title: &header_title_clone,
                    };
                    *harness_state_clone.borrow_mut() = id.clone();
                    dirty_clone.set(true);
                    // recent_domains wordt bij persist meegeschreven (push hier is impliciet via dirty)
                    let q = search_clone.text().to_string();
                    render_into(
                        &content_clone,
                        &shared_clone,
                        &q,
                        &harness_state_clone,
                        &render_ctx,
                    );
                    sync_nav_buttons(&nav_rc, &shared_clone, &id_for_class);
                    let _ = &density_clone;
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
            active_group: initial.clone(),
            harness_state: harness_state.clone(),
            nav_buttons: nav_buttons_rc.clone(),
            persist_dirty: persist_dirty.clone(),
            drawer,
            overlay,
            density,
            window_overlay,
            footer_label: footer_label.clone(),
            theme: theme.clone(),
            header_title: header_title.clone(),
        };
        panel.wire_search();
        panel.wire_overlay();
        let initial_query = panel.search.text().to_string();
        panel.render(&initial_query);
        panel
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            // Esc-prioriteit: drawer > overlay > window
            if self.drawer.is_open() {
                self.drawer.hide();
                self.persist_dirty.set(true);
                return;
            }
            if self.overlay.is_visible() {
                self.overlay.hide();
                return;
            }
            fade_out(&self.window, PANEL_MS);
        } else {
            self.show();
        }
    }

    pub fn focus_domain(&self, domain: &str) {
        let id_raw = domain.trim().to_lowercase();
        if id_raw.is_empty() {
            return;
        }
        // Aliasen voor oude/visual-shot namen → canonieke HarnessKind-ids.
        let id = match id_raw.as_str() {
            "accounts" | "providers" => "commerce",
            "taken" => "tasks",
            "instellingen" | "settings" => "health",
            other => other,
        };
        *self.harness_state.borrow_mut() = id.to_string();
        self.persist_dirty.set(true);
        self.show();
        let query = self.search.text().to_string();
        self.render(&query);
    }

    pub fn toggle_palette(&self) {
        self.show();
        if self.overlay.is_visible() {
            self.overlay.hide();
        } else {
            self.overlay.show();
        }
    }

    pub fn open_inbox(&self) {
        self.focus_domain("inbox");
    }

    /// Preview de detail-drawer met de eerste actie (CI/visual-shot path,
    /// `chefbar --ipc drawer`). Voert niets uit.
    pub fn preview_drawer(&self) {
        self.show();
        let (snap, ops) = {
            let snap = self.shared.snapshot.read().unwrap().clone();
            let ops = self.shared.ops.read().unwrap().clone();
            (snap, ops)
        };
        let profile = crate::config::global_profile().clone();
        let sessions = crate::sessions::load_ranked_sessions(&snap.events);
        let actions = build_actions(&ops, &snap, &profile, sessions);
        if let Some(action) = actions.first() {
            self.drawer.show_for(action);
        }
    }

    /// Idempotent tonen — voor tray-/hotkey-/IPC-commando's die "openen"
    /// bedoelen (open/show/bar), nooit verbergen.
    pub fn show(&self) {
        self.window.deiconify();
        if !self.window.is_visible() {
            let query = self.search.text().to_string();
            self.render(&query);
            self.window.show_all();
            // Drawer/overlay hadden no_show_all — herstel correct
            if !self.drawer.is_open() {
                self.drawer.widget().set_reveal_child(false);
            }
            if !self.overlay.is_visible() {
                self.overlay.widget().set_visible(false);
                self.overlay.widget().set_no_show_all(true);
            }
            self.window_overlay.show_all();
            if !self.overlay.is_visible() {
                self.overlay.widget().set_visible(false);
                self.overlay.widget().set_no_show_all(true);
            }
            fade_in(&self.window, PANEL_MS);
            self.window.present();
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn drawer(&self) -> &Drawer {
        &self.drawer
    }

    pub fn overlay(&self) -> &Overlay {
        &self.overlay
    }

    fn wire_overlay(&self) {
        let overlay = self.overlay.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        self.overlay.entry.connect_changed(move |entry| {
            let query = entry.text().to_string();
            let snap = shared.snapshot.read().unwrap().clone();
            let ops = shared.ops.read().unwrap().clone();
            let profile = crate::config::global_profile().clone();
            let sessions = crate::sessions::load_ranked_sessions(&snap.events);
            let actions = build_actions(&ops, &snap, &profile, sessions);
            let rank_ctx = RankContext::local();
            let ranked = rank_actions_with(&actions, &query, 8, Some(&rank_ctx));
            let overlay_for_action = overlay.clone();
            let executor_for_action = executor.clone();
            overlay.render_actions(&ranked, move |action| {
                let frecency_id = action.frecency_id();
                let spec = action.run.clone();
                if let crate::actions::RunSpec::CopyText(text) = &spec {
                    let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                    clipboard.set_text(text);
                    notify_copied();
                } else {
                    executor_for_action.run_for_ui(&spec);
                }
                crate::frecency::record(&frecency_id);
                overlay_for_action.hide();
            });
        });
    }

    fn wire_search(&self) {
        let content = self.content.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        let window = self.window.clone();
        let harness_state = self.harness_state.clone();
        let dirty = self.persist_dirty.clone();
        let drawer = self.drawer.clone();
        let footer_label = self.footer_label.clone();
        let header_title = self.header_title.clone();
        self.search.connect_changed(move |search| {
            dirty.set(true);
            let query = search.text().to_string();
            if window.is_visible() {
                let render_ctx = RenderCtx {
                    executor: &executor,
                    window: &window,
                    drawer: &drawer,
                    footer_label: &footer_label,
                    header_title: &header_title,
                };
                render_into(&content, &shared, &query, &harness_state, &render_ctx);
            }
        });
    }

    /// Herbouw de hele inhoud uit de gedeelde snapshot, gefilterd op `query`.
    pub fn render(&self, query: &str) {
        let current = self.harness_state.borrow().clone();
        {
            let snap = self.shared.snapshot.read().unwrap().clone();
            let ops = self.shared.ops.read().unwrap().clone();
            let harnesses = build_harnesses(&snap, &ops);
            let current_valid = harnesses.iter().any(|h| h.id == current)
                || HarnessKind::all().iter().any(|k| k.id() == current);
            if !harnesses.is_empty() && !current_valid {
                if let Some(first) = harnesses.first() {
                    *self.harness_state.borrow_mut() = first.id.clone();
                    self.persist_dirty.set(true);
                }
            }
        }
        self.sync_sidebar_nav();
        let render_ctx = RenderCtx {
            executor: &self.executor,
            window: &self.window,
            drawer: &self.drawer,
            footer_label: &self.footer_label,
            header_title: &self.header_title,
        };
        render_into(
            &self.content,
            &self.shared,
            query,
            &self.harness_state,
            &render_ctx,
        );
    }

    fn sync_sidebar_nav(&self) {
        let active = self.harness_state.borrow().clone();
        sync_nav_buttons(&self.nav_buttons, &self.shared, &active);
    }

    /// Schrijf gewijzigde panel-state direct weg, bijvoorbeeld bij afsluiten.
    pub fn flush_panel_state(&self) {
        if self.persist_dirty.get() {
            let current = self.harness_state.borrow().clone();
            let mut state = crate::panel_state::PanelState {
                active_group: Some(current.clone()),
                harness: None,
                query: Some(self.search.text().to_string())
                    .filter(|q: &String| !q.trim().is_empty()),
                drawer_open: self.drawer.is_open(),
                density: self.density.borrow().clone(),
                theme: self.theme.borrow().clone(),
                recent_domains: crate::panel_state::load().recent_domains.clone(),
            };
            // push huidige group naar recent_domains MRU
            state.push_recent_domain(&current);
            if crate::panel_state::save(&state) {
                self.persist_dirty.set(false);
                // active_* fields in Panel zelf syncen
                // (we kunnen niet &mut self, dus via try)
            }
        }
    }

    /// Start de periodieke render-loop (één glib-timer, geen eigen polls) plus
    /// een rustige persist-timer: gewijzigde UI-state gaat 1× per 2s naar disk.
    pub fn start_refresh_loop(&self) {
        let content = self.content.clone();
        let shared = self.shared.clone();
        let executor = self.executor.clone();
        let window = self.window.clone();
        let search = self.search.clone();
        let harness_state = self.harness_state.clone();
        let nav_buttons = self.nav_buttons.clone();
        let shared_nav = self.shared.clone();
        let drawer = self.drawer.clone();
        let footer_label = self.footer_label.clone();
        let header_title = self.header_title.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(VAULT_POLL_MS), move || {
            if window.is_visible() {
                let render_ctx = RenderCtx {
                    executor: &executor,
                    window: &window,
                    drawer: &drawer,
                    footer_label: &footer_label,
                    header_title: &header_title,
                };
                let query = search.text().to_string();
                render_into(&content, &shared, &query, &harness_state, &render_ctx);
                let active = harness_state.borrow().clone();
                sync_nav_buttons(&nav_buttons, &shared_nav, &active);
            }
            ControlFlow::Continue
        });
        let dirty_persist = self.persist_dirty.clone();
        let harness_persist = self.harness_state.clone();
        let search_persist = self.search.clone();
        let drawer_persist = self.drawer.clone();
        let density_persist = self.density.clone();
        let theme_persist = self.theme.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            if dirty_persist.get() {
                let current = harness_persist.borrow().clone();
                let mut state = crate::panel_state::load();
                state.active_group = Some(current.clone());
                state.harness = None;
                state.query = Some(search_persist.text().to_string())
                    .filter(|q: &String| !q.trim().is_empty());
                state.drawer_open = drawer_persist.is_open();
                state.density = density_persist.borrow().clone();
                state.theme = theme_persist.borrow().clone();
                state.push_recent_domain(&current);
                if crate::panel_state::save(&state) {
                    dirty_persist.set(false);
                }
            }
            ControlFlow::Continue
        });
    }
}

// ---------------------------------------------------------------------------
// Room-helpers: harnas-filtering
// ---------------------------------------------------------------------------

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
// Render: Signaal v2 grouped sections
// ---------------------------------------------------------------------------

/// Gebundelde UI-referenties voor render_into (clippy: max 7 args).
struct RenderCtx<'a> {
    executor: &'a Executor,
    window: &'a gtk::Window,
    drawer: &'a Rc<Drawer>,
    footer_label: &'a gtk::Label,
    header_title: &'a gtk::Label,
}

fn render_into(
    content: &gtk::Box,
    shared: &Shared,
    query: &str,
    harness_state: &Rc<RefCell<String>>,
    ctx: &RenderCtx,
) {
    let executor = ctx.executor;
    let window = ctx.window;
    let drawer = ctx.drawer;
    let footer_label = ctx.footer_label;
    let header_title = ctx.header_title;
    for child in content.children() {
        content.remove(&child);
    }

    let (snap, ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let profile = crate::config::global_profile().clone();
    let vault_label = profile.label("vaultApi");
    let fetched = snap.fetched_label();
    let sessions = crate::sessions::load_ranked_sessions(&snap.events);
    let (state, line) = snap.tray_state();
    let q = query.to_lowercase();

    let harnesses: Vec<Harness> = build_harnesses(&snap, &ops);
    let active_id = {
        let current = harness_state.borrow().clone();
        let known = harnesses.iter().any(|h| h.id == current)
            || HarnessKind::all().iter().any(|k| k.id() == current);
        if known {
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
        .map(|h| h.kind.clone())
        .or_else(|| HarnessKind::all().into_iter().find(|k| k.id() == active_id));

    let all_actions = build_actions(&ops, &snap, &profile, sessions.clone());
    let filtered = filter_actions_by_harness(all_actions, active_kind.as_ref());
    let mut boost_terms: Vec<String> = Vec::new();
    for session in sessions.iter().filter(|s| s.needs_attention()).take(4) {
        boost_terms.push(session.source.to_lowercase());
        for word in session.title.split_whitespace() {
            let word = word.to_lowercase();
            if word.chars().count() >= 4 {
                boost_terms.push(word);
            }
        }
    }
    for agent in snap.agents.iter().filter(|a| a.running).take(4) {
        boost_terms.push(agent.agent.to_lowercase());
    }
    let mut seen = std::collections::HashSet::new();
    boost_terms.retain(|term| seen.insert(term.clone()));
    boost_terms.truncate(16);
    let rank_ctx = RankContext::local_with_terms(boost_terms);
    let ranked = rank_actions_with(&filtered, query, 40, Some(&rank_ctx));

    // ---- Signature: CG-statuslijn — verbinding + dringendste lijn --------
    // v2 worked-row: rust = line-strong, live = accent, hulp = amber,
    // fout = rood, ok = groen.
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    status_row.style_context().add_class("chefbar-statuslijn");
    let status_class = match state.as_str() {
        "offline" | "fout" => "error",
        "hulp" => "warn",
        "bezig" => "running",
        _ => "ok",
    };
    let signature = gtk::Box::new(gtk::Orientation::Vertical, 0);
    signature.set_valign(gtk::Align::Fill);
    signature.style_context().add_class("chefbar-signature");
    signature.style_context().add_class(status_class);
    status_row.pack_start(&signature, false, false, 0);
    let lijn_text = gtk::Label::new(Some(&line));
    lijn_text.set_xalign(0.0);
    lijn_text.set_halign(gtk::Align::Start);
    lijn_text.set_ellipsize(pango::EllipsizeMode::End);
    lijn_text.set_line_wrap(false);
    lijn_text.set_max_width_chars(40);
    lijn_text
        .style_context()
        .add_class("chefbar-statuslijn-text");
    status_row.pack_start(&lijn_text, true, true, 0);
    let updated = gtk::Label::new(Some(&format!("{} · {}", vault_label, fetched)));
    updated.set_halign(gtk::Align::End);
    updated.set_xalign(1.0);
    updated.set_ellipsize(pango::EllipsizeMode::End);
    updated.set_line_wrap(false);
    updated.set_max_width_chars(28);
    updated.style_context().add_class("chefbar-card-meta");
    status_row.pack_end(&updated, false, false, 0);
    content.pack_start(&status_row, false, false, 0);

    // ---- Sectie: Acties (eerste, want interactie eerst) ----
    let actions_visible: Vec<&Action> = ranked.iter().filter(|a| !a.needs_text).take(6).collect();
    let harness_label = harnesses
        .iter()
        .find(|h| h.id == active_id)
        .map(|h| h.label.clone())
        .unwrap_or_else(|| active_id.clone());
    section_title(
        content,
        "Acties",
        &format!("zoek of kies — {}", harness_label.to_lowercase()),
    );
    header_title.set_text(&harness_label);
    let group = group_box();
    if actions_visible.is_empty() && !q.is_empty() {
        let sub = format!(
            "Pas je zoekterm \u{201c}{}\u{201d} aan of wissel van harnas.",
            truncate_q(&q, 24)
        );
        group.pack_start(&empty_state("Niks gevonden", &sub), false, false, 0);
    } else if actions_visible.is_empty() {
        let sub = format!(
            "Geen acties voor {} — wissel via de zijbalk of zoek breder.",
            harness_label.to_lowercase()
        );
        group.pack_start(&empty_state("Niks hier", &sub), false, false, 0);
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
        title.set_line_wrap(false);
        title.set_max_width_chars(48);
        title.style_context().add_class("chefbar-card-title");
        row_box.pack_start(&title, true, true, 0);
        let stamp = stamp_label(&action.stamp);
        row_box.pack_end(&stamp, false, false, 0);
        row.add(&row_box);
        row.set_hexpand(true);
        row.set_halign(gtk::Align::Fill);
        let row_inner = row.child().unwrap();
        row_inner.set_margin_start(10);
        row_inner.set_margin_end(10);
        row_inner.set_margin_top(6);
        row_inner.set_margin_bottom(6);
        let drawer = drawer.clone();
        row.connect_clicked(move |_| {
            if needs_text {
                prompt_for(&executor, &window, &action);
                return;
            }
            let drawer_for_action = drawer.clone();
            let executor = executor.clone();
            let spec = spec.clone();
            let frecency_id = action.frecency_id();
            drawer.show_for_with(&action, move || {
                if let crate::actions::RunSpec::CopyText(text) = &spec {
                    let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                    clipboard.set_text(text);
                    notify_copied();
                } else {
                    executor.run_for_ui(&spec);
                }
                crate::frecency::record(&frecency_id);
                drawer_for_action.hide();
            });
        });
        group.pack_start(&row, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    let text_actions: Vec<&Action> = ranked.iter().filter(|a| a.needs_text).take(3).collect();
    if !text_actions.is_empty() {
        let wrap = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        wrap.set_margin_top(4);
        wrap.set_margin_start(16);
        wrap.set_margin_end(16);
        for action in text_actions {
            let btn = gtk::Button::with_label(&action.title);
            btn.set_tooltip_text(Some(&action.meta));
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

    // ---- Domein-view: typed data per harnas (domains.rs) ----
    let view_kind = active_kind.clone().unwrap_or(HarnessKind::Health);
    domains::render_domain(content, &view_kind, &snap, query, executor, window);

    // ---- Inbox: watcher-suggesties die jou opvallen (alleen op inbox) ----
    if view_kind == HarnessKind::Inbox {
        let fresh: Vec<&crate::models::Suggestion> = snap
            .suggestions
            .iter()
            .filter(|s| s.fresh(crate::models::SUGGESTION_TTL_SECONDS))
            .collect();
        if !fresh.is_empty() {
            let count = fresh.len();
            let sub = if count == 1 {
                "1 melding vraagt om jou".to_string()
            } else {
                format!("{count} meldingen vragen om jou")
            };
            section_title(content, "Signalen", &sub);
            let group = group_box_attention();
            for suggestion in fresh.iter().take(4) {
                let row_btn = gtk::Button::new();
                row_btn.set_relief(gtk::ReliefStyle::None);
                row_btn.set_hexpand(true);
                row_btn.set_halign(gtk::Align::Fill);
                row_btn.style_context().add_class("chefbar-row-btn");
                let tip = if suggestion.meta.is_empty() {
                    suggestion.title.clone()
                } else {
                    format!("{} — {}", suggestion.title, suggestion.meta)
                };
                row_btn.set_tooltip_text(Some(tip.as_str()));
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                dot.set_size_request(8, 8);
                dot.set_halign(gtk::Align::Start);
                dot.set_valign(gtk::Align::Center);
                let dot_cls = match suggestion.stamp.as_str() {
                    "FOUT" => "down",
                    "HULP" | "LIMIET" => "warn",
                    _ => "info",
                };
                dot.style_context().add_class("chefbar-dot");
                dot.style_context().add_class(dot_cls);
                row.pack_start(&dot, false, false, 0);
                let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
                let title_l = gtk::Label::new(Some(&suggestion.title));
                title_l.set_halign(gtk::Align::Start);
                title_l.set_xalign(0.0);
                title_l.set_ellipsize(pango::EllipsizeMode::End);
                title_l.set_line_wrap(false);
                title_l.set_max_width_chars(36);
                title_l.style_context().add_class("chefbar-card-title");
                text.pack_start(&title_l, false, false, 0);
                if !suggestion.meta.is_empty() {
                    let meta_l = gtk::Label::new(Some(&suggestion.meta));
                    meta_l.set_halign(gtk::Align::Start);
                    meta_l.set_xalign(0.0);
                    meta_l.set_line_wrap(true);
                    meta_l.set_lines(1);
                    meta_l.set_ellipsize(pango::EllipsizeMode::End);
                    meta_l.set_max_width_chars(52);
                    meta_l.style_context().add_class("chefbar-card-meta");
                    text.pack_start(&meta_l, false, false, 0);
                }
                row.pack_start(&text, true, true, 0);
                let cta_text = if suggestion.action_label.is_empty() {
                    "OPEN".to_string()
                } else {
                    suggestion.action_label.to_uppercase()
                };
                let cta = stamp_label(&cta_text);
                cta.style_context()
                    .add_class(match suggestion.stamp.as_str() {
                        "FOUT" => "error",
                        "HULP" => "warn",
                        _ => "info",
                    });
                row.pack_end(&cta, false, false, 0);
                row_btn.add(&row);
                if let Some(child) = row_btn.child() {
                    child.set_margin_start(10);
                    child.set_margin_end(10);
                    child.set_margin_top(6);
                    child.set_margin_bottom(6);
                }
                let spec = suggestion_spec(suggestion, &profile);
                if let Some(spec) = spec {
                    let executor_clone = executor.clone();
                    let window_clone = window.clone();
                    row_btn.connect_clicked(move |_| {
                        if let crate::actions::RunSpec::CopyText(text) = &spec {
                            let cb = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                            cb.set_text(text);
                            notify_copied();
                        } else {
                            executor_clone.run_for_ui(&spec);
                        }
                        fade_out(&window_clone, PANEL_MS);
                    });
                } else {
                    row_btn.set_sensitive(false);
                }
                group.pack_start(&row_btn, false, false, 0);
            }
            content.pack_start(&group, false, false, 0);
        }
    }

    // ---- Aandacht: sessies die jou nodig hebben (alleen werk-domeinen) ----
    let show_attention = matches!(
        view_kind,
        HarnessKind::Inbox
            | HarnessKind::Fleet
            | HarnessKind::Herdr
            | HarnessKind::Health
            | HarnessKind::Eval
            | HarnessKind::Tasks
            | HarnessKind::Linear
    );
    if show_attention {
        let attention: Vec<_> = sessions.iter().filter(|s| s.needs_attention()).collect();
        if !attention.is_empty() {
            let count = attention.len();
            let sub = if count == 1 {
                "1 sessie vraagt om jou".to_string()
            } else {
                format!("{count} sessies vragen om jou")
            };
            section_title(content, "Heeft jou nodig", &sub);
            let group = group_box_attention();
            for session in attention.iter().take(4) {
                let spec_and_label = session_cta(session, &profile);
                let row_btn = gtk::Button::new();
                row_btn.set_relief(gtk::ReliefStyle::None);
                row_btn.set_hexpand(true);
                row_btn.set_halign(gtk::Align::Fill);
                row_btn.style_context().add_class("chefbar-row-btn");
                let tooltip = if session.summary.is_empty() {
                    format!("{} · {} · {}", session.source, session.title, session.state)
                } else {
                    format!(
                        "{} · {} — {}",
                        session.source, session.title, session.summary
                    )
                };
                row_btn.set_tooltip_text(Some(&tooltip));
                let inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                dot.set_size_request(8, 8);
                dot.set_halign(gtk::Align::Start);
                dot.set_valign(gtk::Align::Center);
                dot.style_context().add_class("chefbar-dot");
                dot.style_context().add_class(match session.state.as_str() {
                    "failed" => "down",
                    _ => "warn",
                });
                inner.pack_start(&dot, false, false, 0);
                let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
                let title_raw = format!("{} · {}", session.source, session.title);
                let title = gtk::Label::new(Some(&title_raw));
                title.set_halign(gtk::Align::Start);
                title.set_xalign(0.0);
                title.set_ellipsize(pango::EllipsizeMode::End);
                title.set_line_wrap(false);
                title.set_max_width_chars(36);
                title.style_context().add_class("chefbar-card-title");
                text.pack_start(&title, false, false, 0);
                if !session.summary.is_empty() {
                    let meta = gtk::Label::new(Some(&session.summary));
                    meta.set_halign(gtk::Align::Start);
                    meta.set_xalign(0.0);
                    meta.set_line_wrap(true);
                    meta.set_lines(1);
                    meta.set_ellipsize(pango::EllipsizeMode::End);
                    meta.set_max_width_chars(52);
                    meta.style_context().add_class("chefbar-card-meta");
                    text.pack_start(&meta, false, false, 0);
                }
                inner.pack_start(&text, true, true, 0);
                let cta_label = spec_and_label
                    .as_ref()
                    .map(|(l, _)| l.as_str())
                    .unwrap_or("Open");
                let pill = stamp_label(match session.state.as_str() {
                    "failed" => "FOUT",
                    _ => "HULP",
                });
                if cta_label != "Open" {
                    pill.set_text(&format!("{} · {}", pill.text(), cta_label));
                }
                inner.pack_end(&pill, false, false, 0);
                row_btn.add(&inner);
                let inner_child = row_btn.child().unwrap();
                inner_child.set_margin_start(10);
                inner_child.set_margin_end(10);
                inner_child.set_margin_top(6);
                inner_child.set_margin_bottom(6);
                if let Some((_, spec)) = spec_and_label {
                    let executor_clone = executor.clone();
                    row_btn.connect_clicked(move |_| {
                        if let crate::actions::RunSpec::CopyText(ref text) = spec {
                            let cb = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                            cb.set_text(text);
                            notify_copied();
                        } else {
                            executor_clone.run_for_ui(&spec);
                        }
                    });
                } else {
                    row_btn.set_sensitive(false);
                }
                group.pack_start(&row_btn, false, false, 0);
            }
            if attention.len() > 4 {
                let more = gtk::Label::new(Some(&format!(
                    "+{} meer — zoek om te filteren",
                    attention.len() - 4
                )));
                more.set_halign(gtk::Align::Start);
                more.set_xalign(0.0);
                more.set_ellipsize(pango::EllipsizeMode::End);
                more.style_context().add_class("chefbar-card-meta");
                let wrap = row_wrap(&{
                    let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                    b.pack_start(&more, false, false, 0);
                    b
                });
                group.pack_start(&wrap, false, false, 0);
            }
            content.pack_start(&group, false, false, 0);
        }
    }

    // ---- Footer (gepind, in Panel::new gebouwd): live counts bijwerken ----
    let attention_n = sessions.iter().filter(|s| s.needs_attention()).count();
    let running_n = snap.agents.iter().filter(|a| a.running).count();
    let footer_text = if attention_n > 0 {
        format!(
            "v{} · {} · {} wacht · {} bezig · {}",
            crate::VERSION,
            profile.name,
            attention_n,
            running_n,
            fetched
        )
    } else {
        format!(
            "v{} · {} · {} bezig · {}",
            crate::VERSION,
            profile.name,
            running_n,
            fetched
        )
    };
    footer_label.set_text(&footer_text);
    footer_label.set_tooltip_text(Some(&footer_text));

    content.show_all();
}

fn sync_nav_buttons(buttons: &[(String, gtk::Button)], shared: &Shared, active: &str) {
    let (snap, ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let harnesses = build_harnesses(&snap, &ops);
    for (id, btn) in buttons.iter() {
        if let Some(h) = harnesses.iter().find(|h| &h.id == id) {
            // Statische label behouden; alleen de live queue-count erachter.
            let base = sidebar::label_for(id);
            let text = if h.queue_depth > 0 {
                format!("{base} · {}", h.queue_depth)
            } else if base.is_empty() {
                h.label.clone()
            } else {
                base.to_string()
            };
            if btn.label().as_deref() != Some(text.as_str()) {
                btn.set_label(&text);
            }
            btn.set_tooltip_text(Some(&format!("{} — {}", h.id, h.status.label())));
        }
        if id == active {
            btn.style_context().add_class("active");
        } else {
            btn.style_context().remove_class("active");
        }
    }
}

pub fn notify_copied() {
    crate::notify::notify("Gekopieerd", "Tekst staat op het klembord.", "ok");
}

fn session_cta(
    session: &crate::sessions::Session,
    profile: &crate::config::EndpointProfile,
) -> Option<(String, crate::actions::RunSpec)> {
    use crate::sessions::SessionActionKind;
    match session.primary_action() {
        SessionActionKind::None_ => None,
        SessionActionKind::Kater => {
            let base = profile.kater_workspace.as_deref()?;
            let kid = session.attach.kater_session_id.as_deref()?;
            Some((
                "Open sessie".into(),
                crate::actions::RunSpec::OpenUrl(format!("{}/{}", base.trim_end_matches('/'), kid)),
            ))
        }
        SessionActionKind::Focus => session.attach.focus.clone().map(|focus| {
            (
                "Neem over".into(),
                crate::actions::RunSpec::FocusAgent(focus),
            )
        }),
        SessionActionKind::Workspace => session.attach.workspace_url.clone().map(|url| {
            (
                "Open workspace".into(),
                crate::actions::RunSpec::OpenUrl(url),
            )
        }),
        SessionActionKind::Browser => session
            .attach
            .browser
            .clone()
            .map(|url| ("Open".into(), crate::actions::RunSpec::OpenUrl(url))),
        SessionActionKind::Evidence => session
            .attach
            .evidence_url
            .clone()
            .map(|url| ("Bewijs".into(), crate::actions::RunSpec::OpenUrl(url))),
    }
}

fn suggestion_spec(
    suggestion: &crate::models::Suggestion,
    profile: &crate::config::EndpointProfile,
) -> Option<crate::actions::RunSpec> {
    use crate::models::SuggestionKind;
    match &suggestion.kind {
        SuggestionKind::FocusAgent(id) => Some(crate::actions::RunSpec::FocusAgent(id.clone())),
        SuggestionKind::OpenDashboard => {
            Some(crate::actions::RunSpec::OpenUrl(profile.dashboard.clone()))
        }
        SuggestionKind::None_ => None,
    }
}

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
            return gtk::glib::Propagation::Stop;
        }
        if event.keyval() == gdk::keys::constants::Escape {
            dialog_keys.close();
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });

    let (x, y) = window.position();
    dialog.move_(x + 24, y + 24);
    dialog.show_all();
    entry.grab_focus();
}
