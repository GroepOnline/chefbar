//! ChefBar-hoofdvenster: één echte app (Signaal v2), geen floating bar.
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

use crate::actions::{build_actions, Executor};
use crate::harness::{build_harnesses, Harness, HarnessKind};
use crate::motion::{fade_in, fade_out, PANEL_MS};
use crate::palette::{rank_actions_with, Action, RankContext};
use crate::state::{Shared, VAULT_POLL_MS};
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// E6: gedeelde render- en toetsenbord-state, meegegeven aan render_into.
/// Centraliseert de P3.1-revisie-check (handtekening + statusregel) en de
/// actie-rijen waar de pijltjestoetsen doorheen lopen. Overleeft re-renders:
/// de rijen worden elke render herbouwd, de selectie-index blijft staan.
#[derive(Clone)]
struct RenderSlots {
    /// P3.1: laatste render-handtekening — bij onveranderde snapshot wordt de
    /// dure full-rebuild overgeslagen in de refresh-loop.
    sig: Rc<RefCell<Option<u64>>>,
    /// Opgeslagen statusregel-label zodat poll-leeftijd + versheid ook zonder
    /// rebuild blijven tikken (P3.1-skip-pad).
    updated_label: Rc<RefCell<Option<gtk::Label>>>,
    /// E6: actie-rijen in zichtvolgorde — de toetsenbord-navigeerbare resultaten.
    action_rows: Rc<RefCell<Vec<gtk::Button>>>,
    /// E6: geselecteerde rij-index binnen action_rows (0 = eerste).
    selected: Rc<Cell<usize>>,
}

impl RenderSlots {
    fn new() -> Self {
        Self {
            sig: Rc::new(RefCell::new(None)),
            updated_label: Rc::new(RefCell::new(None)),
            action_rows: Rc::new(RefCell::new(Vec::new())),
            selected: Rc::new(Cell::new(0)),
        }
    }
}

/// E6: volgende selectie-index bij ↑/↓ (wrap-around, Raycast-geest) — pure,
/// apart gehouden zodat de pijl-logica unit-testbaar is zonder GTK.
fn next_selection(current: usize, len: usize, down: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if down {
        if current + 1 < len {
            current + 1
        } else {
            0
        }
    } else if current > 0 {
        current - 1
    } else {
        len - 1
    }
}

pub struct Panel {
    pub window: gtk::Window,
    content: gtk::Box,
    search: gtk::SearchEntry,
    shared: Shared,
    executor: Executor,
    // geselecteerde harnas binnen de room — default naar eerste harnas
    pub active_harness: String,
    harness_state: Rc<RefCell<String>>,
    nav_buttons: Rc<Vec<(String, gtk::Button)>>,
    /// UI-state (harnas + zoekterm) is gewijzigd maar nog niet naar disk.
    persist_dirty: Rc<Cell<bool>>,
    /// E6: render- + toetsenbord-state (P3.1-handtekening, statusregel, actie-rijen).
    slots: RenderSlots,
}

impl Panel {
    pub fn new(shared: Shared, executor: Executor) -> Self {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("ChefBar");
        window.set_decorated(false);
        // Vaste geometrie (W1/D2): min==max via size_request + resizable(false)
        // zodat inhoud-hoogte het venster nooit kan laten resizen/jumpen,
        // op welke backend dan ook (X11/XWayland/Wayland).
        window.set_default_size(760, 840);
        window.set_size_request(760, 840);
        window.set_resizable(false);
        // E2: op echte Wayland-sessies het paneel als laag (top-right, marge);
        // anders de bestaande X11-positionering (fallback, XWayland-proof).
        let layered = crate::layer_shell::apply(&window);
        if !layered {
            window.set_keep_above(true);
            window.set_position(gtk::WindowPosition::Center);
        }

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
        sidebar_title
            .style_context()
            .add_class("chefbar-sidebar-title");
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
        let nav_ids = ["fleet", "commerce", "eval", "sync"];
        let nav_labels = ["Fleet", "Commerce", "Evaluatie", "Sync"];
        let mut nav_buttons: Vec<(String, gtk::Button)> = Vec::new();
        for (idx, (id, label)) in nav_ids.iter().zip(nav_labels.iter()).enumerate() {
            let btn = gtk::Button::with_label(label);
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
        status_footer
            .style_context()
            .add_class("chefbar-sidebar-footer");
        status_footer.set_margin_start(12);
        status_footer.set_margin_end(12);
        status_footer.set_margin_top(10);
        status_footer.set_margin_bottom(12);
        let footer_title = gtk::Label::new(Some("Status"));
        footer_title.set_halign(gtk::Align::Start);
        footer_title.set_xalign(0.0);
        footer_title
            .style_context()
            .add_class("chefbar-sidebar-footer-title");
        status_footer.pack_start(&footer_title, false, false, 0);
        let footer_meta = gtk::Label::new(Some("online \u{00b7} signaal v2"));
        footer_meta.set_halign(gtk::Align::Start);
        footer_meta.set_xalign(0.0);
        footer_meta
            .style_context()
            .add_class("chefbar-sidebar-footer-meta");
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
        // v2 heading-tracking: -0.02em op koppen (Pango-eenheden).
        let attrs = pango::AttrList::new();
        attrs.insert(pango::AttrInt::new_letter_spacing(-380));
        title.set_attributes(Some(&attrs));
        title_block.pack_start(&title, false, false, 0);
        let title_sub = gtk::Label::new(Some("agentische assistent \u{00b7} signaal v2"));
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
        let refresh_icon =
            gtk::Image::from_icon_name(Some("view-refresh-symbolic"), gtk::IconSize::Button);
        refresh_btn.set_image(Some(&refresh_icon));
        refresh_btn.set_relief(gtk::ReliefStyle::None);
        refresh_btn.style_context().add_class("chefbar-gbtn");
        refresh_btn.connect_clicked(move |_| crate::state::refresh_global());
        let min_btn = gtk::Button::new();
        let min_icon =
            gtk::Image::from_icon_name(Some("window-minimize-symbolic"), gtk::IconSize::Button);
        min_btn.set_image(Some(&min_icon));
        min_btn.set_relief(gtk::ReliefStyle::None);
        min_btn.style_context().add_class("chefbar-gbtn");
        let window_for_min = window.clone();
        min_btn.connect_clicked(move |_| window_for_min.iconify());
        let close_btn = gtk::Button::new();
        let close_icon =
            gtk::Image::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Button);
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
            gtk::glib::Propagation::Proceed
        });

        // E6: toetsenbord-first. Esc verbergt; "/" en Ctrl+K focussen zoeken;
        // ↑/↓ lopen door de actie-rijen van het actieve harnas (wrap-around) en
        // markeren de geselecteerde rij (.selected + focus) — Enter activeert
        // hem via GTK's eigen button-activation.
        let slots = RenderSlots::new();
        {
            let search_focus = search.clone();
            let window_esc = window.clone();
            let rows_rc = slots.action_rows.clone();
            let sel_rc = slots.selected.clone();
            window.connect_key_press_event(move |_, event| {
                let kv = event.keyval();
                if kv == gdk::keys::constants::Escape {
                    fade_out(&window_esc, PANEL_MS);
                    return gtk::glib::Propagation::Stop;
                }
                if kv == gdk::keys::constants::slash && !search_focus.has_focus() {
                    search_focus.grab_focus();
                    return gtk::glib::Propagation::Stop;
                }
                // Ctrl+K op Linux, Super(Cmd)+K op andere toetsenborden (MOD4).
                let ctrl = event.state().contains(gdk::ModifierType::CONTROL_MASK)
                    || event.state().contains(gdk::ModifierType::MOD4_MASK);
                if ctrl && kv == gdk::keys::constants::k {
                    search_focus.grab_focus();
                    return gtk::glib::Propagation::Stop;
                }
                if kv == gdk::keys::constants::Down || kv == gdk::keys::constants::Up {
                    let rows = rows_rc.borrow();
                    if rows.is_empty() {
                        // Geen navigeerbare rijen: doorgeven zodat de scroller
                        // zelf kan scrollen (geen keyboard-blokkade).
                        return gtk::glib::Propagation::Proceed;
                    }
                    let idx =
                        next_selection(sel_rc.get(), rows.len(), kv == gdk::keys::constants::Down);
                    sel_rc.set(idx);
                    for (i, row) in rows.iter().enumerate() {
                        if i == idx {
                            row.style_context().add_class("selected");
                            row.grab_focus();
                        } else {
                            row.style_context().remove_class("selected");
                        }
                    }
                    return gtk::glib::Propagation::Stop;
                }
                gtk::glib::Propagation::Proceed
            });
        }

        // ---- Content ----
        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        // Vaste scroller-hoogte (W1/D2): content scrollt intern i.p.v. het
        // venster op te rekken tijdens poll-renders.
        scroller.set_min_content_height(480);
        scroller.set_max_content_height(480);
        main.pack_start(&scroller, true, true, 0);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_margin_bottom(8);
        scroller.add(&content);

        root.pack_start(&main, true, true, 0);
        // harnas-state: herstel het laatst gekozen harnas (panel dat onthoudt),
        // anders default naar eerste harnas (fleet). render() valideert tegen
        // echte harnassen zodra de snapshot er is.
        let persisted = crate::panel_state::load();
        let initial = persisted
            .harness
            .clone()
            .filter(|id| nav_ids.contains(&id.as_str()))
            .unwrap_or_else(|| "fleet".to_string());
        if let Some(query) = persisted.query.as_deref() {
            if !query.trim().is_empty() {
                search.set_text(query);
            }
        }
        let persist_dirty = Rc::new(Cell::new(false));
        let harness_state = Rc::new(RefCell::new(initial.clone()));
        // Wire sidebar nav → harness_state + content re-render + sync_nav_buttons
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
                let slots_clone = slots.clone();
                btn_clone.connect_clicked(move |_| {
                    *harness_state_clone.borrow_mut() = id.clone();
                    dirty_clone.set(true);
                    let q = search_clone.text().to_string();
                    render_into(
                        &content_clone,
                        &shared_clone,
                        &executor_clone,
                        &window_clone,
                        &q,
                        &harness_state_clone,
                        &slots_clone,
                    );
                    // Eén pad met de poll-timer: labels + active-class + tooltips.
                    sync_nav_buttons(&nav_rc, &shared_clone, &id_for_class);
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
            persist_dirty: persist_dirty.clone(),
            slots,
        };
        panel.wire_search();
        // Initieel renderen met de (mogelijk herstelde) zoekterm — nooit een
        // gefilterd veld met ongefilterde inhoud.
        let initial_query = panel.search.text().to_string();
        panel.render(&initial_query);
        panel
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            fade_out(&self.window, PANEL_MS);
        } else {
            self.show();
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
            fade_in(&self.window, PANEL_MS);
            // Alleen present() bij overgang verborgen→zichtbaar (W1/D2): elke
            // show -> her-positionering/re-focus, dus geen present bij herhaalde
            // Super+Space terwijl het venster al open staat.
            self.window.present();
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
        let dirty = self.persist_dirty.clone();
        let slots = self.slots.clone();
        self.search.connect_changed(move |search| {
            dirty.set(true);
            let query = search.text().to_string();
            if window.is_visible() {
                render_into(
                    &content,
                    &shared,
                    &executor,
                    &window,
                    &query,
                    &harness_state,
                    &slots,
                );
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
                    self.persist_dirty.set(true);
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
            &self.slots,
        );
    }

    fn sync_sidebar_nav(&self) {
        let active = self.harness_state.borrow().clone();
        sync_nav_buttons(&self.nav_buttons, &self.shared, &active);
    }

    /// Schrijf gewijzigde panel-state direct weg, bijvoorbeeld bij afsluiten.
    pub fn flush_panel_state(&self) {
        if self.persist_dirty.get() {
            let state = crate::panel_state::PanelState {
                harness: Some(self.harness_state.borrow().clone()),
                query: Some(self.search.text().to_string())
                    .filter(|q: &String| !q.trim().is_empty()),
            };
            if crate::panel_state::save(&state) {
                self.persist_dirty.set(false);
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
        let slots = self.slots.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(VAULT_POLL_MS), move || {
            if !window.is_visible() {
                return ControlFlow::Continue;
            }
            // P3.1: bij onveranderde snapshot geen dure full-rebuild — alleen
            // de statusregel (poll-leeftijd + versheid) blijft live tikken.
            if *slots.sig.borrow() == Some(render_signature(&shared)) {
                if let Some(label) = slots.updated_label.borrow().as_ref() {
                    label.set_text(&status_right_text(&shared));
                }
                return ControlFlow::Continue;
            }
            let query = search.text().to_string();
            render_into(
                &content,
                &shared,
                &executor,
                &window,
                &query,
                &harness_state,
                &slots,
            );
            let active = harness_state.borrow().clone();
            sync_nav_buttons(&nav_buttons, &shared_nav, &active);
            ControlFlow::Continue
        });
        let dirty_persist = self.persist_dirty.clone();
        let harness_persist = self.harness_state.clone();
        let search_persist = self.search.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            if dirty_persist.get() {
                let state = crate::panel_state::PanelState {
                    harness: Some(harness_persist.borrow().clone()),
                    query: Some(search_persist.text().to_string())
                        .filter(|q: &String| !q.trim().is_empty()),
                };
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
// Render: Signaal v2 grouped sections
// ---------------------------------------------------------------------------

/// P3.1: compacte vingerafdruk van alles wat de render zichtbaar beïnvloedt.
/// Velden die elke poll veranderen zonder zichtbaar effect (poll-leeftijd,
/// fetched_at_unix) zitten er bewust niet in — de statusregel ververst los van
/// de full-rebuild via `status_right_text`.
fn render_signature(shared: &Shared) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[derive(Hash)]
    struct Sig<'a> {
        revision: i64,
        error: Option<&'a str>,
        agents: Vec<(&'a str, &'a str)>,           // key, status
        suggestions: Vec<(&'a str, &'a str, i64)>, // key, stamp, created_unix
        fleet: (usize, usize, bool),               // online, total, stale
        health: (usize, usize, &'a str),           // ok, total, level
        day_score: Option<i64>,
        providers: Vec<(&'a str, bool)>, // label, available
        events_len: usize,
        tasks: Vec<(&'a str, &'a str)>,             // id, status
        desktop_state: Option<&'a str>,             // desktop-actielabel (start/stop)
        share_sync: (Option<&'a str>, i64),         // status, pendingFiles (sync-harnas)
        ops: (bool, Vec<(&'a str, &'a str, bool)>), // ok, (terminal_id, status, focused)
    }

    let mut hasher = DefaultHasher::new();
    {
        let snap = shared.snapshot.read().unwrap();
        let ops = shared.ops.read().unwrap();
        let sig = Sig {
            revision: snap.revision,
            error: snap.error.as_deref(),
            agents: snap
                .agents
                .iter()
                .map(|a| (a.key.as_str(), a.status.as_str()))
                .collect(),
            suggestions: snap
                .suggestions
                .iter()
                .map(|s| (s.key.as_str(), s.stamp.as_str(), s.created_unix))
                .collect(),
            fleet: (snap.fleet.online, snap.fleet.total, snap.fleet.stale),
            health: (
                snap.health.ok,
                snap.health.total,
                snap.health.level.as_str(),
            ),
            day_score: snap.day_score.score,
            providers: snap
                .providers
                .iter()
                .map(|p| (p.label.as_str(), p.available))
                .collect(),
            events_len: snap.events.len(),
            tasks: snap
                .tasks
                .iter()
                .map(|t| {
                    (
                        t.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        t.get("status").and_then(|v| v.as_str()).unwrap_or(""),
                    )
                })
                .collect(),
            desktop_state: snap.desktop.get("state").and_then(|v| v.as_str()),
            share_sync: (
                snap.share_sync.get("status").and_then(|v| v.as_str()),
                snap.share_sync
                    .get("pendingFiles")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
            ),
            ops: (
                ops.ok,
                ops.agents
                    .iter()
                    .map(|a| (a.terminal_id.as_str(), a.status.as_str(), a.focused))
                    .collect(),
            ),
        };
        sig.hash(&mut hasher);
    }
    hasher.finish()
}

/// Tekst van de statusregel-rechts (vault-label · versheid · poll-gezondheid).
/// Gedeeld door de full-render en het P3.1-skip-pad (statusregel blijft live).
fn status_right_text(shared: &Shared) -> String {
    let (snap, _ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let profile = crate::config::global_profile();
    format!(
        "{} · {} · {}",
        profile.label("vaultApi"),
        snap.fetched_label(),
        snap.poll.label()
    )
}

fn render_into(
    content: &gtk::Box,
    shared: &Shared,
    executor: &Executor,
    window: &gtk::Window,
    query: &str,
    harness_state: &Rc<RefCell<String>>,
    slots: &RenderSlots,
) {
    // P3.1: na elke render is de handtekening de actuele waarheid.
    *slots.sig.borrow_mut() = Some(render_signature(shared));
    // E6: actie-rijen worden elke render herbouwd — start leeg, selectie 0
    // (onderaan wordt de selectie geklemd en visueel gezet).
    slots.action_rows.borrow_mut().clear();
    slots.selected.set(0);
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

    // Harnas-navigatie loopt via de sidebar (één weg, geen dubbele pill-rij).

    let all_actions = build_actions(&ops, &snap, &profile, sessions.clone());
    // Filter eerst op het geselecteerde harnas, zodat de globale limiet geen
    // relevante acties van dit harnas wegdrukt.
    let filtered = filter_actions_by_harness(all_actions, active_kind.as_ref());
    // Zoeken dat kiest: recency-boost uit sessies die om jou vragen en agents
    // die nu draaien. Alleen woorden van 4+ tekens (geen ruis op korte tokens).
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
    let rank_ctx = RankContext { boost_terms };
    let ranked = rank_actions_with(&filtered, query, 40, Some(&rank_ctx));

    // ---- Signature: CG-statuslijn — verbinding + dringendste lijn --------
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    status_row.set_margin_top(10);
    status_row.set_margin_start(16);
    status_row.set_margin_end(16);
    status_row.style_context().add_class("chefbar-statuslijn");
    let status_class = match state.as_str() {
        "offline" | "fout" => "error",
        "hulp" => "warn",
        "bezig" => "info",
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
    // Poll-gezondheid (E1): laatste poll · vault · ops — naast de data-versheid.
    let poll_label = snap.poll.label();
    let updated = gtk::Label::new(Some(&format!(
        "{} · {} · {}",
        vault_label, fetched, poll_label
    )));
    updated.set_halign(gtk::Align::End);
    updated.set_xalign(1.0);
    updated.set_ellipsize(pango::EllipsizeMode::End);
    updated.set_line_wrap(false);
    updated.set_max_width_chars(60);
    updated.set_tooltip_text(Some(&poll_label));
    updated.style_context().add_class("chefbar-card-meta");
    // P3.1: sla het label op zodat de refresh-loop het zonder rebuild kan
    // bijwerken (poll-leeftijd blijft live).
    *slots.updated_label.borrow_mut() = Some(updated.clone());
    status_row.pack_end(&updated, false, false, 0);
    content.pack_start(&status_row, false, false, 0);

    // ---- Sectie: Acties (eerste, want interactie eerst) ----
    let actions_visible: Vec<&Action> = ranked.iter().filter(|a| !a.needs_text).take(6).collect();
    let harness_label = harnesses
        .iter()
        .find(|h| h.id == active_id)
        .map(|h| h.label.clone())
        .unwrap_or_else(|| active_id.clone());
    // ---- Inbox: watcher-suggesties die jou opvallen (vers binnen TTL) ----
    {
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
            section_title(content, "Inbox", &sub);
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
                // Actie aan de suggestie: FocusAgent/OpenDashboard via executor.
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
    section_title(
        content,
        "Acties",
        &format!("zoek of kies — {}", harness_label.to_lowercase()),
    );
    let group = group_box();
    if actions_visible.is_empty() && !q.is_empty() {
        let sub = format!(
            "Pas je zoekterm “{}” aan of wissel van harnas.",
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
        row.connect_clicked(move |_| {
            if needs_text {
                prompt_for(&executor, &window, &action);
            } else {
                if let crate::actions::RunSpec::CopyText(text) = &spec {
                    let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                    clipboard.set_text(text);
                    notify_copied();
                } else {
                    executor.run_for_ui(&spec);
                }
            }
        });
        // E6: deze rij is toetsenbord-navigeerbaar (pijltjes + Enter).
        slots.action_rows.borrow_mut().push(row.clone());
        group.pack_start(&row, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);

    // Tekstacties onder de directe acties (kleine knoppenrij) — strak.
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

    // ---- Sectie: Gezondheid ----
    section_title(content, "Gezondheid", "watchdog + dagscore + fleet");
    let group = group_box();
    let health_meta = match snap.health.updated_at.as_deref() {
        Some(at) => format!("{} · update {}", state_label(&snap.health), short_ts(at)),
        None => state_label(&snap.health),
    };
    let health_row = info_row(&snap.health.line(), Some(&health_meta));
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

    // ---- Sectie: Providers — strak, overflow-veilig ----
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
        // Versheids-indicator: "STALE" als de provider-data achterloopt op de
        // connector-refresh, anders de laatste update-tijd.
        if row.stale || !row.available {
            let stale_badge = gtk::Label::new(Some("STALE"));
            stale_badge.set_halign(gtk::Align::Start);
            stale_badge.set_xalign(0.0);
            stale_badge.style_context().add_class("chefbar-stamp");
            stale_badge.style_context().add_class("error");
            row_top_stale(
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
        let wrap = row_wrap(&card);
        group.pack_start(&wrap, false, false, 0);
    }
    if !any_provider {
        if q.is_empty() {
            group.pack_start(
                &empty_state(
                    "Nog geen providers",
                    "Koppel een account in de vault of vernieuw de status.",
                ),
                false,
                false,
                0,
            );
        } else {
            let sub = format!("Geen providers voor “{}”.", truncate_q(&q, 28));
            group.pack_start(&empty_state("Niks gevonden", &sub), false, false, 0);
        }
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Herdr — ops-agents met pane/focus + inline prompt (E5) ----
    render_herdr_section(content, &ops, executor, window, &q);

    // ---- Sectie: Agents — strak, ellipsis, premium empty ----
    section_title(content, "Agents", "vault watcher-feed · werkstromen");
    let group = group_box();
    let mut any_agent = false;
    for agent in snap.agents.iter().filter(|a| {
        q.is_empty()
            || a.agent.to_lowercase().contains(&q)
            || a.workspace.to_lowercase().contains(&q)
    }) {
        any_agent = true;
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 9);
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
        let title_raw = format!("{} · {}", agent.agent, agent.workspace);
        let title = gtk::Label::new(Some(&title_raw));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.set_line_wrap(false);
        title.set_max_width_chars(36);
        title.set_tooltip_text(Some(&title_raw));
        title.style_context().add_class("chefbar-card-title");
        text.pack_start(&title, false, false, 0);
        if !agent.summary.is_empty() {
            let summary = gtk::Label::new(Some(&agent.summary));
            summary.set_halign(gtk::Align::Start);
            summary.set_xalign(0.0);
            summary.set_line_wrap(true);
            summary.set_lines(1);
            summary.set_ellipsize(pango::EllipsizeMode::End);
            summary.set_max_width_chars(56);
            summary.set_tooltip_text(Some(&agent.summary));
            summary.style_context().add_class("chefbar-card-meta");
            text.pack_start(&summary, false, false, 0);
        }
        row.pack_start(&text, true, true, 0);
        row.pack_end(&stamp_label(stamp), false, false, 0);
        let wrap = row_wrap(&row);
        group.pack_start(&wrap, false, false, 0);
    }
    if !any_agent {
        if q.is_empty() {
            group.pack_start(
                &empty_state(
                    "Geen agents actief",
                    "Start een herdr-agent of open een workspace — ze verschijnen hier.",
                ),
                false,
                false,
                0,
            );
        } else {
            let sub = format!("Geen agents voor “{}”.", truncate_q(&q, 28));
            group.pack_start(&empty_state("Niks gevonden", &sub), false, false, 0);
        }
    }
    content.pack_start(&group, false, false, 0);

    // ---- Sectie: Commander — taak-queue met per-taak acties (E5-staart) ----
    if !snap.tasks.is_empty() {
        section_title(content, "Commander", "taak-queue · stop per taak");
        let group = group_box();
        // Echte queue-positie (niet de gefilterde index) bij een zoekterm.
        let mut shown = 0usize;
        for (position, task) in snap.tasks.iter().enumerate() {
            if shown >= 8 {
                break;
            }
            let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let status = task
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("queued");
            let prompt: String = task
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("Taak zonder omschrijving")
                .chars()
                .take(52)
                .collect();
            if !q.is_empty() && !prompt.to_lowercase().contains(&q) {
                continue;
            }
            shown += 1;
            let (cls, stamp) = match status {
                "running" => ("info", "BEZIG"),
                "queued" => ("ok", "WACHT"),
                "failed" | "error" => ("down", "FOUT"),
                _ => ("ok", "KLAAR"),
            };
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 9);
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.set_size_request(8, 8);
            dot.set_halign(gtk::Align::Start);
            dot.set_valign(gtk::Align::Center);
            dot.style_context().add_class("chefbar-dot");
            dot.style_context().add_class(cls);
            row.pack_start(&dot, false, false, 0);
            let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
            let title = gtk::Label::new(Some(&prompt));
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            title.set_ellipsize(pango::EllipsizeMode::End);
            title.set_line_wrap(false);
            title.set_max_width_chars(40);
            title.set_tooltip_text(Some(&prompt));
            title.style_context().add_class("chefbar-card-title");
            text.pack_start(&title, false, false, 0);
            if !task_id.is_empty() {
                let short_id: String = task_id.chars().take(16).collect();
                let meta = gtk::Label::new(Some(&format!("#{position} · {short_id}")));
                meta.set_halign(gtk::Align::Start);
                meta.set_xalign(0.0);
                meta.set_ellipsize(pango::EllipsizeMode::End);
                meta.style_context().add_class("chefbar-card-meta");
                text.pack_start(&meta, false, false, 0);
            }
            row.pack_start(&text, true, true, 0);
            // Per-taak actie: Stop (cancel) voor queued/running taken.
            if matches!(status, "queued" | "running") && !task_id.is_empty() {
                let spec = crate::actions::RunSpec::CancelTask(task_id.to_string());
                let executor_clone = executor.clone();
                let stop = gtk::Button::with_label("Stop");
                stop.set_tooltip_text(Some(&format!("Stop taak {task_id}")));
                stop.style_context().add_class("chefbar-btn");
                stop.style_context().add_class("chefbar-danger");
                stop.connect_clicked(move |_| executor_clone.run_for_ui(&spec));
                row.pack_end(&stop, false, false, 0);
            }
            row.pack_end(&stamp_label(stamp), false, false, 0);
            group.pack_start(&row_wrap(&row), false, false, 0);
        }
        content.pack_start(&group, false, false, 0);
    }

    // ---- Sectie: Aandacht (sessions die jou nodig hebben) — premium, actionable ----
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
        // Toon max 4, maar footer hint als er meer zijn
        for session in attention.iter().take(4) {
            let spec_and_label = session_cta(session, &profile);
            let row_btn = gtk::Button::new();
            row_btn.set_relief(gtk::ReliefStyle::None);
            row_btn.set_hexpand(true);
            row_btn.set_halign(gtk::Align::Fill);
            row_btn.style_context().add_class("chefbar-row-btn");
            // Tooltip met volledige context
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
                "blocked" => "warn",
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
            // vervang stamp-text door CTA hint als beschikbaar
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
                    // CopyText vs OpenUrl/Focus — via zelfde executor-pad
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
    } else if !q.is_empty() && sessions.is_empty() {
        // geen losse lege sectie wanneer er überhaupt geen sessies zijn — stil
    }

    // ---- Footer — strak, mono, met live counts ----
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.style_context().add_class("chefbar-footer");
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
    let footer_label = gtk::Label::new(Some(&footer_text));
    footer_label.set_halign(gtk::Align::Start);
    footer_label.set_xalign(0.0);
    footer_label.set_ellipsize(pango::EllipsizeMode::End);
    footer_label.set_line_wrap(false);
    footer_label.set_max_width_chars(64);
    footer_label.set_tooltip_text(Some(&footer_text));
    footer_label
        .style_context()
        .add_class("chefbar-footer-label");
    footer.pack_start(&footer_label, true, true, 0);
    let quit_btn = gtk::Button::with_label("Verbergen");
    quit_btn.style_context().add_class("chefbar-gbtn");
    let window_hide = window.clone();
    quit_btn.connect_clicked(move |_| fade_out(&window_hide, PANEL_MS));
    footer.pack_end(&quit_btn, false, false, 0);
    content.pack_start(&footer, false, false, 0);

    content.show_all();

    // E6: selectie vastzetten op de nieuwe rij-lijst. Na een rebuild is de
    // focus gereset (children verwijderd) — herstel hem op de geselecteerde
    // rij, tenzij de gebruiker aan het typen is (search houdt focus).
    let rows = slots.action_rows.borrow();
    let n = rows.len();
    if n > 0 {
        let mut idx = slots.selected.get();
        if idx >= n {
            idx = n - 1;
            slots.selected.set(idx);
        }
        for (i, row) in rows.iter().enumerate() {
            if i == idx {
                row.style_context().add_class("selected");
                if window.focused_widget().is_none() {
                    row.grab_focus();
                }
            } else {
                row.style_context().remove_class("selected");
            }
        }
    }
}

/// Sidebar-nav syncen op live state: queue-depth in het label ("Fleet · 3"),
/// status als tooltip, actieve room gemarkeerd. Gedeeld door Panel::render,
/// de periodieke refresh-timer en nav-click handlers.
fn sync_nav_buttons(buttons: &[(String, gtk::Button)], shared: &Shared, active: &str) {
    let (snap, ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let harnesses = build_harnesses(&snap, &ops);
    for (id, btn) in buttons.iter() {
        if let Some(h) = harnesses.iter().find(|h| &h.id == id) {
            let text = if h.queue_depth > 0 {
                format!("{} · {}", h.label, h.queue_depth)
            } else {
                h.label.clone()
            };
            // Poll-vriendelijk: alleen schrijven als het label wijzigt,
            // anders forceert GTK elke tick een herlayout van de knop.
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

/// Privacy-safe kopie-melding: nooit klembord-inhoud in notificaties.
fn notify_copied() {
    crate::notify::notify("Gekopieerd", "Tekst staat op het klembord.", "ok");
}

fn state_label(health: &crate::models::HealthInfo) -> String {
    if health.total == 0 {
        "onbekend".into()
    } else {
        format!("{} van {} ok", health.ok, health.total)
    }
}

fn section_title(content: &gtk::Box, title: &str, sub: &str) {
    // v2 eyebrow (.caps): korte caps in de interface-face. GTK3 kent geen
    // text-transform, dus de caps gebeuren hier.
    let label = gtk::Label::new(Some(&title.to_uppercase()));
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

fn group_box_attention() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.style_context().add_class("chefbar-group-attention");
    group
}

/// E5: herdr-sectie — lopende agent-terminals uit OpsSnapshot met
/// pane/focus-status; klik op een rij = inline prompt-sturen (tekst-dialog →
/// SendPrompt via de executor). Stil overslaan als er geen ops-agents zijn.
fn render_herdr_section(
    content: &gtk::Box,
    ops: &crate::models::OpsSnapshot,
    executor: &Executor,
    window: &gtk::Window,
    q: &str,
) {
    let herdr: Vec<_> = ops
        .agents
        .iter()
        .filter(|a| {
            q.is_empty()
                || a.name.to_lowercase().contains(q)
                || a.workspace.to_lowercase().contains(q)
                || a.cwd.to_lowercase().contains(q)
        })
        .collect();
    if herdr.is_empty() {
        return;
    }
    let home_str = crate::home_dir().to_string_lossy().to_string();
    section_title(
        content,
        "Herdr",
        "lopende agent-terminals · klik = prompt sturen",
    );
    let group = group_box();
    for agent in herdr.iter().take(8) {
        let (cls, stamp) = match agent.status.as_str() {
            "working" => ("info", "BEZIG"),
            "blocked" => ("warn", "HULP"),
            "idle" => ("ok", "KLAAR"),
            _ => ("ok", "STIL"),
        };
        let row = gtk::Button::new();
        row.set_relief(gtk::ReliefStyle::None);
        row.set_hexpand(true);
        row.set_halign(gtk::Align::Fill);
        row.style_context().add_class("chefbar-row-btn");
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 9);
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.set_size_request(8, 8);
        dot.set_halign(gtk::Align::Start);
        dot.set_valign(gtk::Align::Center);
        dot.style_context().add_class("chefbar-dot");
        dot.style_context().add_class(cls);
        inner.pack_start(&dot, false, false, 0);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
        let title_raw = format!("{} · {}", agent.name, agent.workspace);
        let title = gtk::Label::new(Some(&title_raw));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.set_line_wrap(false);
        title.set_max_width_chars(36);
        title.set_tooltip_text(Some(&title_raw));
        title.style_context().add_class("chefbar-card-title");
        text.pack_start(&title, false, false, 0);
        let cwd_label = agent.cwd.replace(&home_str, "~");
        if !cwd_label.is_empty() {
            let meta = gtk::Label::new(Some(&cwd_label));
            meta.set_halign(gtk::Align::Start);
            meta.set_xalign(0.0);
            meta.set_ellipsize(pango::EllipsizeMode::End);
            meta.set_line_wrap(false);
            meta.set_max_width_chars(40);
            meta.set_tooltip_text(Some(&cwd_label));
            meta.style_context().add_class("chefbar-card-meta");
            text.pack_start(&meta, false, false, 0);
        }
        inner.pack_start(&text, true, true, 0);
        // Pane/focus-status (E5) naast de status-stamp.
        let status = stamp_label(stamp);
        if !agent.pane_id.is_empty() {
            status.set_text(&format!("{} · pane {}", stamp, agent.pane_id));
            let tip = if agent.focused {
                format!("pane {} · gefocust", agent.pane_id)
            } else {
                format!("pane {} · niet gefocust", agent.pane_id)
            };
            status.set_tooltip_text(Some(&tip));
        } else if agent.focused {
            status.set_tooltip_text(Some("gefocust"));
        }
        inner.pack_end(&status, false, false, 0);
        row.add(&inner);
        if let Some(child) = row.child() {
            child.set_margin_start(10);
            child.set_margin_end(10);
            child.set_margin_top(6);
            child.set_margin_bottom(6);
        }
        // Inline prompt-sturen: klik → tekst-dialog → SendPrompt via executor.
        let action = Action {
            title: format!("Stuur naar {} · {}", agent.name, agent.workspace),
            meta: "typ je opdracht en druk op Enter".into(),
            stamp: "TAAK".into(),
            keywords: String::new(),
            section: "Acties".into(),
            shortcut: "↵".into(),
            needs_text: true,
            destructive: false,
            pinned: false,
            run: crate::actions::RunSpec::SendPrompt {
                terminal_id: agent.terminal_id.clone(),
                pane_id: if agent.pane_id.is_empty() {
                    None
                } else {
                    Some(agent.pane_id.clone())
                },
            },
        };
        let executor_clone = executor.clone();
        let window_clone = window.clone();
        row.connect_clicked(move |_| prompt_for(&executor_clone, &window_clone, &action));
        group.pack_start(&row, false, false, 0);
    }
    content.pack_start(&group, false, false, 0);
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
    row_wrap(&row)
}

fn truncate_q(q: &str, max: usize) -> String {
    let chars: Vec<char> = q.chars().collect();
    if chars.len() <= max {
        q.to_string()
    } else {
        chars[..max].iter().collect::<String>() + "…"
    }
}

/// Zet een watcher-suggestie om naar een uitvoerbare actie voor de executor.
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

/// Korte, locale tijdstempel (HH:MM of <1d → "12:03", anders "04-08").
fn short_ts(ts: &str) -> String {
    // Aanname: ISO-8601 zonder tijdzone, gereedschapsdatum—toon alleen de delen
    // die we betrouwbaar uit de string kunnen knippen.
    let body = ts
        .chars()
        .take_while(|c| *c != 'T' && *c != ' ' && *c != '.')
        .collect::<String>();
    if body.is_empty() {
        return ts.to_string();
    }
    body
}

/// Plaatst een STALE-badge + eventuele oude refresh-tijd achteraan de top-row.
fn row_top_stale(
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

fn empty_state(title: &str, sub: &str) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.style_context().add_class("chefbar-empty");
    let icon = gtk::Label::new(Some("—"));
    icon.set_halign(gtk::Align::Start);
    icon.set_xalign(0.0);
    icon.style_context().add_class("chefbar-empty-icon");
    outer.pack_start(&icon, false, false, 0);
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
    outer
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

#[cfg(test)]
mod tests {
    use super::next_selection;

    #[test]
    fn selectie_stapt_en_wrapt_rond() {
        assert_eq!(next_selection(0, 4, true), 1);
        assert_eq!(next_selection(3, 4, true), 0);
        assert_eq!(next_selection(0, 4, false), 3);
        assert_eq!(next_selection(2, 4, false), 1);
    }

    #[test]
    fn selectie_bij_lege_lijst_is_nul() {
        assert_eq!(next_selection(0, 0, true), 0);
        assert_eq!(next_selection(0, 0, false), 0);
    }
}
