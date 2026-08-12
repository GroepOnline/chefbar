//! Palette-overlay voor het ChefApp-panel (Super+Space fast-path).
//!
//! Zelfde venster, zelfde snapshot, zelfde ranking als de hoofdzoekbalk.
//! Geen tweede socket, poll-loop of dataset.

use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub struct Overlay {
    pub container: gtk::Box,
    pub entry: gtk::SearchEntry,
    results: gtk::Box,
    revealed: Rc<Cell<bool>>,
}

impl Overlay {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        container.style_context().add_class("chefbar-overlay");
        container
            .style_context()
            .add_class("chefbar-palette-overlay");
        container.set_visible(false);
        container.set_no_show_all(true);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Zoek in alle 15 domeinen"));
        entry.style_context().add_class("chefbar-palette-entry");
        container.pack_start(&entry, false, false, 0);

        let results = gtk::Box::new(gtk::Orientation::Vertical, 2);
        results.style_context().add_class("chefbar-palette-results");
        let hint = gtk::Label::new(Some("Typ om te filteren \u{00b7} esc sluit"));
        hint.set_halign(gtk::Align::Start);
        hint.set_xalign(0.0);
        hint.set_ellipsize(pango::EllipsizeMode::End);
        hint.style_context().add_class("chefbar-card-meta");
        results.pack_start(&hint, false, false, 0);
        container.pack_start(&results, false, false, 0);

        let revealed = Rc::new(Cell::new(false));
        let container_esc = container.clone();
        let entry_esc = entry.clone();
        let revealed_esc = revealed.clone();
        entry.connect_key_press_event(move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                container_esc.set_visible(false);
                container_esc.set_no_show_all(true);
                entry_esc.set_text("");
                revealed_esc.set(false);
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });

        Self {
            container,
            entry,
            results,
            revealed,
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
        if actions.is_empty() {
            let empty = gtk::Label::new(Some("Geen actie gevonden · probeer een domein of /"));
            empty.set_halign(gtk::Align::Start);
            empty.set_xalign(0.0);
            empty.style_context().add_class("chefbar-card-meta");
            self.results.pack_start(&empty, false, false, 0);
            self.results.show_all();
            return;
        }
        let cap = gtk::Label::new(Some(&format!("ACTIES · {}", actions.len())));
        cap.set_halign(gtk::Align::Start);
        cap.set_xalign(0.0);
        cap.style_context().add_class("chefbar-palette-section");
        self.results.pack_start(&cap, false, false, 0);
        for action in actions.iter().take(8) {
            let button = gtk::Button::new();
            button.set_relief(gtk::ReliefStyle::None);
            button.set_halign(gtk::Align::Fill);
            button.set_hexpand(true);
            button.set_tooltip_text(Some(&action.meta));
            button.style_context().add_class("chefbar-palette-row");
            let inner = gtk::Box::new(gtk::Orientation::Horizontal, 10);
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
                meta.set_max_width_chars(58);
                meta.style_context().add_class("chefbar-card-meta");
                text.pack_start(&meta, false, false, 0);
            }
            inner.pack_start(&text, true, true, 0);
            let stamp = super::zones::stamp_label(&action.stamp);
            inner.pack_end(&stamp, false, false, 0);
            button.add(&inner);
            let action = action.clone();
            let callback = on_activate.clone();
            button.connect_clicked(move |_| callback(action.clone()));
            self.results.pack_start(&button, false, false, 0);
        }
        self.results.show_all();
    }

    pub fn show(&self) {
        self.container.set_no_show_all(false);
        self.container.set_visible(true);
        self.container.show_all();
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
        self.revealed.set(true);
    }

    pub fn hide(&self) {
        if !self.revealed.get() {
            return;
        }
        self.container.set_visible(false);
        self.container.set_no_show_all(true);
        self.entry.set_text("");
        self.revealed.set(false);
    }

    pub fn is_visible(&self) -> bool {
        self.revealed.get()
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_overlay() -> Overlay {
    Overlay::new()
}
