//! Palette-overlay voor het ChefApp-panel (Super+Space fast-path).
//!
//! Zelfde venster, zelfde snapshot, zelfde ranking als de hoofdzoekbalk.
//! Geen tweede socket, poll-loop of dataset. Chrome: Devin v2 palette
//! (560px, r-10, hairline, kbd-hints).

use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type OverlayActivate = Rc<dyn Fn(crate::palette::Action)>;

pub(crate) const OVERLAY_PLACEHOLDER: &str = "Zoek of typ een opdracht\u{2026}";
pub(crate) const OVERLAY_IDLE_HINT: &str =
    "Typ om te filteren. Enter voert de eerste treffer uit.";
pub(crate) const OVERLAY_EMPTY: &str = "Niets gevonden. Probeer een domein of /.";
pub(crate) const OVERLAY_SECTION: &str = "Acties";

pub struct Overlay {
    pub container: gtk::Box,
    pub entry: gtk::SearchEntry,
    results: gtk::Box,
    revealed: Rc<Cell<bool>>,
    first_action: Rc<RefCell<Option<crate::palette::Action>>>,
    on_enter: Rc<RefCell<Option<OverlayActivate>>>,
}

impl Overlay {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.style_context().add_class("chefbar-overlay");
        container
            .style_context()
            .add_class("chefbar-palette-overlay");
        container.set_visible(false);
        container.set_no_show_all(true);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some(OVERLAY_PLACEHOLDER));
        entry.style_context().add_class("chefbar-palette-entry");
        container.pack_start(&entry, false, false, 0);

        let results = gtk::Box::new(gtk::Orientation::Vertical, 2);
        results.style_context().add_class("chefbar-palette-results");
        let hint = gtk::Label::new(Some(OVERLAY_IDLE_HINT));
        hint.set_halign(gtk::Align::Start);
        hint.set_xalign(0.0);
        hint.set_line_wrap(true);
        hint.set_ellipsize(pango::EllipsizeMode::End);
        hint.style_context().add_class("chefbar-card-meta");
        results.pack_start(&hint, false, false, 0);
        container.pack_start(&results, false, false, 0);

        container.pack_start(&overlay_foot(), false, false, 0);

        let revealed = Rc::new(Cell::new(false));
        let first_action = Rc::new(RefCell::new(None));
        let on_enter: Rc<RefCell<Option<OverlayActivate>>> = Rc::new(RefCell::new(None));
        let container_esc = container.clone();
        let entry_esc = entry.clone();
        let revealed_esc = revealed.clone();
        let first_keys = first_action.clone();
        let enter_keys = on_enter.clone();
        entry.connect_key_press_event(move |_, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                container_esc.set_visible(false);
                container_esc.set_no_show_all(true);
                entry_esc.set_text("");
                revealed_esc.set(false);
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

        Self {
            container,
            entry,
            results,
            revealed,
            first_action,
            on_enter,
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
            let empty = gtk::Label::new(Some(OVERLAY_EMPTY));
            empty.set_halign(gtk::Align::Start);
            empty.set_xalign(0.0);
            empty.set_line_wrap(true);
            empty.style_context().add_class("chefbar-card-meta");
            self.results.pack_start(&empty, false, false, 0);
            self.results.show_all();
            return;
        }
        let cap = gtk::Label::new(Some(&format!(
            "{} · {}",
            OVERLAY_SECTION,
            actions.len()
        )));
        cap.set_halign(gtk::Align::Start);
        cap.set_xalign(0.0);
        cap.style_context().add_class("chefbar-palette-section");
        self.results.pack_start(&cap, false, false, 0);
        for (idx, action) in actions.iter().take(8).enumerate() {
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
            let callback = callback.clone();
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

fn overlay_foot() -> gtk::Box {
    let foot = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    foot.style_context().add_class("chefbar-overlay-foot");
    foot.pack_start(&kbd_chip("enter"), false, false, 0);
    let enter_l = gtk::Label::new(Some("voert uit"));
    enter_l
        .style_context()
        .add_class("chefbar-overlay-foot-label");
    foot.pack_start(&enter_l, false, false, 0);
    foot.pack_start(&kbd_chip("esc"), false, false, 0);
    let esc_l = gtk::Label::new(Some("sluit"));
    esc_l
        .style_context()
        .add_class("chefbar-overlay-foot-label");
    foot.pack_start(&esc_l, false, false, 0);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    foot.pack_start(&spacer, true, true, 0);
    let hint = gtk::Label::new(Some("/ of ctrl+k zoekt overal"));
    hint.set_halign(gtk::Align::End);
    hint.style_context()
        .add_class("chefbar-overlay-foot-label");
    foot.pack_end(&hint, false, false, 0);
    foot
}

fn kbd_chip(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.style_context().add_class("chefbar-kbd");
    label
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_copy_is_warm_nederlands() {
        assert!(OVERLAY_PLACEHOLDER.contains("Zoek"));
        assert!(OVERLAY_IDLE_HINT.contains("Enter"));
        assert!(OVERLAY_EMPTY.starts_with("Niets gevonden"));
        assert_eq!(OVERLAY_SECTION, "Acties");
        for text in [OVERLAY_PLACEHOLDER, OVERLAY_IDLE_HINT, OVERLAY_EMPTY] {
            assert!(!text.contains('\u{2014}'), "geen em-dash in {text}");
            assert!(!text.contains("System"), "{text}");
            assert!(!text.contains("Idle"), "{text}");
        }
    }
}
