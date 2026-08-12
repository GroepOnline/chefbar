//! Detail-drawer voor het ChefApp-panel: slide + focus-trap + Esc.

use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub const DRAWER_WIDTH: i32 = 300;
pub const DRAWER_SLIDE_MS: u32 = 160;

/// Eén drawer-instantie, gekoppeld als derde kolom naast de main-canvas.
pub struct Drawer {
    pub container: gtk::Revealer,
    inner: gtk::Box,
    title: gtk::Label,
    meta: gtk::Label,
    actions: gtk::Box,
    open: Rc<Cell<bool>>,
}

impl Drawer {
    pub fn new() -> Self {
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
        revealer.set_transition_duration(DRAWER_SLIDE_MS);
        revealer.set_reveal_child(false);
        revealer.set_halign(gtk::Align::End);
        revealer.set_valign(gtk::Align::Fill);

        let inner = gtk::Box::new(gtk::Orientation::Vertical, 10);
        inner.style_context().add_class("chefbar-drawer");
        inner.set_size_request(DRAWER_WIDTH, -1);
        inner.set_hexpand(false);
        inner.set_halign(gtk::Align::Fill);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.set_margin_top(12);
        header.set_margin_start(12);
        header.set_margin_end(12);
        let title = gtk::Label::new(Some("\u{2014}"));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_ellipsize(pango::EllipsizeMode::End);
        title.style_context().add_class("chefbar-drawer-title");
        title.set_hexpand(true);
        header.pack_start(&title, true, true, 0);
        let close = gtk::Button::new();
        let icon = gtk::Image::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Button);
        close.set_image(Some(&icon));
        close.set_relief(gtk::ReliefStyle::None);
        close.style_context().add_class("chefbar-gbtn");
        header.pack_end(&close, false, false, 0);
        inner.pack_start(&header, false, false, 0);

        let meta = gtk::Label::new(Some(""));
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        meta.set_line_wrap(true);
        meta.set_ellipsize(pango::EllipsizeMode::End);
        meta.style_context().add_class("chefbar-card-meta");
        meta.set_margin_start(12);
        meta.set_margin_end(12);
        inner.pack_start(&meta, false, false, 0);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.style_context().add_class("chefbar-drawer-actions");
        actions.set_margin_start(12);
        actions.set_margin_end(12);
        actions.set_margin_bottom(6);
        inner.pack_start(&actions, false, false, 0);

        let hint = gtk::Label::new(Some("\u{21b5} uitvoeren \u{00b7} esc sluit"));
        hint.set_halign(gtk::Align::Start);
        hint.set_xalign(0.0);
        hint.style_context().add_class("chefbar-drawer-hint");
        inner.pack_start(&hint, false, false, 0);

        let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        inner.pack_start(&spacer, true, true, 0);

        revealer.add(&inner);

        let open = Rc::new(Cell::new(false));
        let revealer_for_close = revealer.clone();
        let open_for_close = open.clone();
        close.connect_clicked(move |_| {
            slide_drawer(&revealer_for_close, false);
            open_for_close.set(false);
        });

        Self {
            container: revealer,
            inner,
            title,
            meta,
            actions,
            open,
        }
    }

    pub fn show_for(&self, action: &crate::palette::Action) {
        self.show_for_with(action, || {});
    }

    pub fn show_for_with<F>(&self, action: &crate::palette::Action, on_activate: F)
    where
        F: Fn() + 'static,
    {
        self.title.set_text(&action.title);
        self.meta.set_text(&action.meta);
        for child in self.actions.children() {
            self.actions.remove(&child);
        }
        let execute = gtk::Button::with_label("Uitvoeren");
        execute.style_context().add_class("chefbar-btn");
        execute.style_context().add_class("chefbar-primary");
        execute.connect_clicked(move |_| on_activate());
        self.actions.pack_start(&execute, false, false, 0);
        let cancel = gtk::Button::with_label("Annuleren");
        cancel.style_context().add_class("chefbar-btn");
        let revealer_cancel = self.container.clone();
        let open_cancel = self.open.clone();
        cancel.connect_clicked(move |_| {
            slide_drawer(&revealer_cancel, false);
            open_cancel.set(false);
        });
        self.actions.pack_end(&cancel, false, false, 0);
        self.actions.show_all();
        // Enter voert uit (focused button), Esc zit al op het window.
        execute.grab_focus();
        slide_drawer(&self.container, true);
        self.open.set(true);
    }

    pub fn hide(&self) {
        if !self.open.get() {
            return;
        }
        slide_drawer(&self.container, false);
        self.open.set(false);
    }

    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    pub fn widget(&self) -> &gtk::Revealer {
        &self.container
    }

    pub fn inner(&self) -> &gtk::Box {
        &self.inner
    }

    pub fn actions_box(&self) -> &gtk::Box {
        &self.actions
    }
}

impl Default for Drawer {
    fn default() -> Self {
        Self::new()
    }
}

fn slide_drawer(revealer: &gtk::Revealer, show: bool) {
    if show {
        revealer.set_visible(true);
        revealer.set_reveal_child(true);
    } else {
        revealer.set_reveal_child(false);
    }
}
