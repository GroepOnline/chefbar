//! Palette-overlay voor het ChefApp-panel (Super+Space fast-path).
//!
//! Zelfde venster, zelfde snapshot, zelfde ranking als de hoofdzoekbalk.
//! Geen tweede socket, poll-loop of dataset.

use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type OverlayActivate = Rc<dyn Fn(crate::palette::Action)>;

pub struct Overlay {
    pub scrim: gtk::EventBox,
    pub container: gtk::Box,
    pub entry: gtk::SearchEntry,
    results: gtk::Box,
    revealed: Rc<Cell<bool>>,
    first_action: Rc<RefCell<Option<crate::palette::Action>>>,
    on_enter: Rc<RefCell<Option<OverlayActivate>>>,
    host: Rc<RefCell<Option<gtk::Overlay>>>,
}

impl Overlay {
    pub fn new() -> Self {
        let scrim = gtk::EventBox::new();
        scrim.style_context().add_class("chefbar-palette-scrim");
        scrim.set_visible(false);
        scrim.set_no_show_all(true);
        scrim.set_opacity(0.6);
        scrim.set_hexpand(true);
        scrim.set_vexpand(true);
        scrim.set_halign(gtk::Align::Fill);
        scrim.set_valign(gtk::Align::Fill);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        container.style_context().add_class("chefbar-overlay");
        container
            .style_context()
            .add_class("chefbar-palette-overlay");
        container.set_visible(false);
        container.set_no_show_all(true);
        container.set_size_request(560, -1);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Zoek of typ een opdracht"));
        entry.style_context().add_class("chefbar-palette-entry");
        container.pack_start(&entry, false, false, 0);

        let results = gtk::Box::new(gtk::Orientation::Vertical, 2);
        results.style_context().add_class("chefbar-palette-results");
        let hint = gtk::Label::new(Some("Typ om te filteren · esc sluit"));
        hint.set_halign(gtk::Align::Start);
        hint.set_xalign(0.0);
        hint.set_ellipsize(pango::EllipsizeMode::End);
        hint.style_context().add_class("chefbar-card-meta");
        results.pack_start(&hint, false, false, 0);
        container.pack_start(&results, false, false, 0);

        let revealed = Rc::new(Cell::new(false));
        let first_action = Rc::new(RefCell::new(None));
        let on_enter: Rc<RefCell<Option<OverlayActivate>>> = Rc::new(RefCell::new(None));
        let host: Rc<RefCell<Option<gtk::Overlay>>> = Rc::new(RefCell::new(None));
        let container_esc = container.clone();
        let scrim_esc = scrim.clone();
        let entry_esc = entry.clone();
        let revealed_esc = revealed.clone();
        let host_esc = host.clone();
        let first_keys = first_action.clone();
        let enter_keys = on_enter.clone();
        entry.connect_key_press_event(move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                hide_widgets(
                    &container_esc,
                    &scrim_esc,
                    &entry_esc,
                    &revealed_esc,
                    &host_esc,
                );
                return gtk::glib::Propagation::Stop;
            }
            if (event.keyval() == gdk::keys::constants::Return
                || event.keyval() == gdk::keys::constants::KP_Enter)
                && activate_first(&first_keys, &enter_keys)
            {
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        let first_act = first_action.clone();
        let enter_act = on_enter.clone();
        entry.connect_activate(move |_| {
            activate_first(&first_act, &enter_act);
        });

        let container_scrim = container.clone();
        let scrim_hide = scrim.clone();
        let entry_scrim = entry.clone();
        let revealed_scrim = revealed.clone();
        let host_scrim = host.clone();
        scrim.connect_button_press_event(move |_, _| {
            hide_widgets(
                &container_scrim,
                &scrim_hide,
                &entry_scrim,
                &revealed_scrim,
                &host_scrim,
            );
            gtk::glib::Propagation::Stop
        });

        Self {
            scrim,
            container,
            entry,
            results,
            revealed,
            first_action,
            on_enter,
            host,
        }
    }

    /// Vervang de resultaatlijst en koppel elk resultaat aan de caller.
    pub fn render_actions<F>(&self, actions: &[crate::palette::Action], on_activate: F)
    where
        F: Fn(crate::palette::Action) + Clone + 'static,
    {
        for child in self.results.children() {
            self.results.remove(&child);
        }
        let callback: OverlayActivate = Rc::new(on_activate);
        *self.on_enter.borrow_mut() = Some(callback.clone());
        *self.first_action.borrow_mut() = actions.first().cloned();
        if actions.is_empty() {
            let empty = gtk::Label::new(Some("Niets gevonden. Probeer een domein of /"));
            empty.set_halign(gtk::Align::Start);
            empty.set_xalign(0.0);
            empty.style_context().add_class("chefbar-card-meta");
            self.results.pack_start(&empty, false, false, 0);
            self.results.show_all();
            return;
        }
        let cap = gtk::Label::new(Some(&format!(
            "{} · {}",
            super::zones::caps("Acties"),
            actions.len()
        )));
        cap.set_halign(gtk::Align::Start);
        cap.set_xalign(0.0);
        cap.style_context().add_class("chefbar-palette-section");
        self.results.pack_start(&cap, false, false, 0);
        for (idx, action) in actions.iter().take(9).enumerate() {
            let button = gtk::Button::new();
            button.set_relief(gtk::ReliefStyle::None);
            button.set_halign(gtk::Align::Fill);
            button.set_hexpand(true);
            button.set_tooltip_text(Some(&action.meta));
            button.style_context().add_class("chefbar-palette-row");
            if idx == 0 {
                button.style_context().add_class("selected");
            }
            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let glyph = crate::icons::image_muted(
                crate::icons::for_action(&action.keywords, &action.section),
                15,
            );
            glyph.style_context().add_class("chefbar-palette-glyph");
            glyph.set_valign(gtk::Align::Center);
            inner.pack_start(&glyph, false, false, 0);
            let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
            let title = gtk::Label::new(Some(&action.title));
            title.set_halign(gtk::Align::Start);
            title.set_xalign(0.0);
            title.set_ellipsize(pango::EllipsizeMode::End);
            title.style_context().add_class("chefbar-card-title");
            text.pack_start(&title, false, false, 0);
            if !action.meta.is_empty() {
                let meta = gtk::Label::new(Some(&action.meta));
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
            let hint = if action.shortcut.is_empty() {
                "↵".to_string()
            } else {
                action.shortcut.clone()
            };
            let kbd = gtk::Label::new(Some(&hint));
            kbd.style_context().add_class("chefbar-kbd");
            kbd.set_valign(gtk::Align::Center);
            inner.pack_end(&kbd, false, false, 0);
            let stamp = super::zones::stamp_label(&action.stamp);
            inner.pack_end(&stamp, false, false, 0);
            button.add(&inner);
            let action = action.clone();
            let callback = callback.clone();
            button.connect_clicked(move |_| callback(action.clone()));
            self.results.pack_start(&button, false, false, 0);
        }
        self.results.show_all();
    }

    pub fn bind_host(&self, host: gtk::Overlay) {
        *self.host.borrow_mut() = Some(host);
        apply_pass_through(&self.host, &self.scrim, &self.container, true);
    }

    pub fn show(&self) {
        self.scrim.set_no_show_all(false);
        self.scrim.set_visible(true);
        self.scrim.show();
        self.container.set_no_show_all(false);
        self.container.set_visible(true);
        self.container.show_all();
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
        self.revealed.set(true);
        apply_pass_through(&self.host, &self.scrim, &self.container, false);
    }

    pub fn hide(&self) {
        if !self.revealed.get() {
            return;
        }
        hide_widgets(
            &self.container,
            &self.scrim,
            &self.entry,
            &self.revealed,
            &self.host,
        );
    }

    pub fn is_visible(&self) -> bool {
        self.revealed.get()
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn scrim(&self) -> &gtk::EventBox {
        &self.scrim
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

fn hide_widgets(
    container: &gtk::Box,
    scrim: &gtk::EventBox,
    entry: &gtk::SearchEntry,
    revealed: &Rc<Cell<bool>>,
    host: &RefCell<Option<gtk::Overlay>>,
) {
    container.set_visible(false);
    container.set_no_show_all(true);
    scrim.set_visible(false);
    scrim.set_no_show_all(true);
    entry.set_text("");
    revealed.set(false);
    apply_pass_through(host, scrim, container, true);
}

fn apply_pass_through(
    host: &RefCell<Option<gtk::Overlay>>,
    scrim: &gtk::EventBox,
    container: &gtk::Box,
    pass: bool,
) {
    if let Some(host) = host.borrow().as_ref() {
        host.set_overlay_pass_through(scrim, pass);
        host.set_overlay_pass_through(container, pass);
    }
}

fn activate_first(
    first: &RefCell<Option<crate::palette::Action>>,
    on_enter: &RefCell<Option<OverlayActivate>>,
) -> bool {
    let Some(action) = first.borrow().clone() else {
        return false;
    };
    let Some(cb) = on_enter.borrow().clone() else {
        return false;
    };
    cb(action);
    true
}

pub fn build_overlay() -> Overlay {
    Overlay::new()
}
