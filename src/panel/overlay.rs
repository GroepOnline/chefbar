//! Palette-overlay voor het ChefApp-panel (Super+Space fast-path).
//!
//! Dunne stub voor Fase 0: compileerbaar, geen ranking-logica (Lane D levert
//! die). Panel wint, overlay is een overlay-kind bovenop hetzelfde window.
//! Denk: spotlight-mode boven het bestaande panel — één window, één socket.

use gtk::prelude::*;

/// Minimale overlay-state. Lane D wiret ranking + query + results hier in.
pub struct Overlay {
    pub container: gtk::Box,
    pub entry: gtk::SearchEntry,
    revealed: std::cell::Cell<bool>,
}

impl Overlay {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
        container
            .style_context()
            .add_class("chefbar-palette-overlay");
        container.set_visible(false);
        container.set_no_show_all(true);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Palette · typ om te zoeken (Lane D vult ranking)"));
        entry.style_context().add_class("chefbar-palette-entry");
        container.pack_start(&entry, false, false, 0);

        let hint = gtk::Label::new(Some("TODO Lane D will wire ranking · Esc sluit overlay"));
        hint.set_halign(gtk::Align::Start);
        hint.set_xalign(0.0);
        hint.set_ellipsize(pango::EllipsizeMode::End);
        hint.style_context().add_class("chefbar-card-meta");
        container.pack_start(&hint, false, false, 0);

        // Esc in de overlay sluit alleen overlay, niet het venster.
        let container_esc = container.clone();
        let entry_esc = entry.clone();
        entry.connect_key_press_event(move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                container_esc.set_visible(false);
                container_esc.set_no_show_all(true);
                entry_esc.set_text("");
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });

        Self {
            container,
            entry,
            revealed: std::cell::Cell::new(false),
        }
    }

    /// Bouw-overlay helper (function API uit het contract) — thin wrapper.
    pub fn show(&self) {
        self.container.set_no_show_all(false);
        self.container.set_visible(true);
        self.container.show_all();
        self.entry.grab_focus();
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

/// Contract-functie: `build_overlay() -> Overlay`.
pub fn build_overlay() -> Overlay {
    Overlay::new()
}
