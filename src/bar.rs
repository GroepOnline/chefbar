//! Command-bar: search-entry + gerangschikte suggesties + tekstdialog.
//!
//! Eén venster, opnieuw gevuld uit de laatste snapshot; uitvoer via de
//! declaratieve registry (actions.rs). CopyText draait op de UI-thread via de
//! GTK-clipboard; alle netwerk-uitvoer gaat naar de executor-thread.
//!
//! Signal-closures vangen uitsluitend widgets + executor: geen
//! self-referentiële borrows, geen Rc<RefCell> door de UI.

use crate::actions::{build_actions, Executor, RunSpec};
use crate::motion::{fade_in, fade_out, PANEL_MS};
use crate::palette::{rank_actions, Action};
use crate::state::Shared;
use gtk::prelude::*;

pub struct ChefBar {
    pub window: gtk::Window,
    entry: gtk::SearchEntry,
    list: gtk::ListBox,
    shared: Shared,
    executor: Executor,
}

impl ChefBar {
    pub fn new(shared: Shared, executor: Executor) -> Self {
        let window = gtk::Window::new(gtk::WindowType::Popup);
        window.set_decorated(false);
        window.set_keep_above(true);
        window.set_accept_focus(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.style_context().add_class("chefbar-bar-root");
        window.add(&root);

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Zoek een actie…"));
        entry.style_context().add_class("chefbar-bar-entry");
        entry.set_hexpand(true);
        root.pack_start(&entry, false, false, 0);

        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_min_content_width(430);
        scroller.set_max_content_height(360);
        root.pack_start(&scroller, true, true, 0);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(false);
        scroller.add(&list);

        let bar = Self {
            window,
            entry,
            list,
            shared,
            executor,
        };
        bar.connect_signals();
        bar
    }

    pub fn open(&self) {
        self.rebuild_actions();
        self.position_near_pointer();
        self.entry.grab_focus();
        fade_in(&self.window, PANEL_MS);
    }

    pub fn hide(&self) {
        fade_out(&self.window, PANEL_MS);
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn toggle(&self) {
        if self.is_visible() {
            self.hide();
        } else {
            self.open();
        }
    }

    /// Herbouw de catalogus en her-rankt met de huidige entry-tekst.
    pub fn rebuild_actions(&self) {
        let query = self.entry.text().to_string();
        let (snap, ops) = {
            let snap = self.shared.snapshot.read().unwrap().clone();
            let ops = self.shared.ops.read().unwrap().clone();
            (snap, ops)
        };
        let sessions = crate::sessions::load_ranked_sessions(&snap.events);
        let profile = crate::config::global_profile().clone();
        let all = build_actions(&ops, &snap, &profile, sessions);
        let ranked = rank_actions(&all, &query, 12);
        self.render_with(&ranked);
    }

    fn render_with(&self, ranked: &[Action]) {
        for child in self.list.children() {
            self.list.remove(&child);
        }
        let query_at_render = self.entry.text().to_string();
        let executor = self.executor.clone();
        let window = self.window.clone();
        for (index, action) in ranked.iter().enumerate() {
            let row = suggestion_row(action);
            let run = action.clone();
            let executor = executor.clone();
            let window = window.clone();
            let list = self.list.clone();
            let query_at = query_at_render.clone();
            row.connect_activate(move |_row| {
                activate(&executor, &window, &list, &run, &query_at, index);
            });
            self.list.add(&row);
        }
    }

    fn position_near_pointer(&self) {
        if let Some(display) = gdk::Display::default() {
            if let Some(seat) = display.default_seat() {
                if let Some(pointer) = seat.pointer() {
                    let (_, root_x, root_y) = pointer.position();
                    if let Some(monitor) = display.monitor_at_point(root_x, root_y) {
                        let geo = monitor.geometry();
                        let (width, height) = (self.window.width_request() as i32, 220);
                        let x = root_x - width / 2;
                        let mut y = root_y + 14;
                        if y + height > geo.y() + geo.height() {
                            y = (root_y - 40 - height).max(geo.y());
                        }
                        self.window.move_(x.max(geo.x()), y);
                    }
                }
            }
        }
    }

    fn connect_signals(&self) {
        let entry = self.entry.clone();
        let executor = self.executor.clone();
        let shared = self.shared.clone();

        let window_changed = self.window.clone();
        let list_changed = self.list.clone();
        let entry_changed = entry.clone();
        entry_changed.connect_changed(move |entry| {
            let query = entry.text().to_string();
            rebuild_from(&window_changed, &list_changed, &shared, &executor, &query);
        });

        let entry_keys = entry.clone();
        let list_keys = self.list.clone();
        let window_keys = self.window.clone();
        entry_keys.connect_key_press_event(move |_entry, event| {
            match event.keyval() {
                gdk::keys::constants::Down | gdk::keys::constants::Up => {
                    let rows = list_keys.children();
                    let current = list_keys
                        .selected_row()
                        .and_then(|r| list_keys.row_at_index(r.index()))
                        .map(|r| r.index());
                    let next = match event.keyval() {
                        gdk::keys::constants::Down => Some(current.unwrap_or(0) + 1),
                        _ => current.and_then(|c| c.checked_sub(1)),
                    };
                    if let Some(next) = next {
                        let max = rows.len() as i32 - 1;
                        if next <= max && next >= 0 {
                            if let Some(row) = list_keys.row_at_index(next) {
                                list_keys.unselect_all();
                                list_keys.select_row(Some(&row));
                                row.grab_focus();
                            }
                        }
                    }
                    return glib::Propagation::Stop;
                }
                gdk::keys::constants::Return | gdk::keys::constants::KP_Enter => {
                    if let Some(row) = list_keys.selected_row() {
                        row.activate();
                    }
                    return glib::Propagation::Stop;
                }
                gdk::keys::constants::Escape => {
                    fade_out(&window_keys, PANEL_MS);
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
            glib::Propagation::Proceed
        });

        let window_esc = self.window.clone();
        let window_esc_cb = window_esc.clone();
        window_esc.connect_key_press_event(move |_window, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                fade_out(&window_esc_cb, PANEL_MS);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });

        // Activaties kunnen het venster sluiten; entry-focus blijft binnen.
        let window_focus = self.window.clone();
        entry.connect_focus_out_event(move |_entry, _event| {
            let window_focus = window_focus.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(120), move || {
                let _ = window_focus;
            });
            glib::Propagation::Proceed
        });
    }
}

/// Rebuild de suggestielijst vanuit de widgets (gebruikt door changed-signal).
fn rebuild_from(
    window: &gtk::Window,
    list: &gtk::ListBox,
    shared: &Shared,
    executor: &Executor,
    query: &str,
) {
    let (snap, ops) = {
        let snap = shared.snapshot.read().unwrap().clone();
        let ops = shared.ops.read().unwrap().clone();
        (snap, ops)
    };
    let sessions = crate::sessions::load_ranked_sessions(&snap.events);
    let profile = crate::config::global_profile().clone();
    let all = build_actions(&ops, &snap, &profile, sessions);
    let ranked = rank_actions(&all, query, 12);

    for child in list.children() {
        list.remove(&child);
    }
    let executor = executor.clone();
    let window = window.clone();
    for (index, action) in ranked.iter().enumerate() {
        let row = suggestion_row(action);
        let run = action.clone();
        let executor = executor.clone();
        let window = window.clone();
        let list_cb = list.clone();
        let query = query.to_string();
        row.connect_activate(move |_row| {
            activate(&executor, &window, &list_cb, &run, &query, index);
        });
        list.add(&row);
    }
    if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
    }
}

/// Uitvoer bij activatie: tekstdialog voor acties die tekst vragen.
fn activate(
    executor: &Executor,
    window: &gtk::Window,
    list: &gtk::ListBox,
    action: &Action,
    query: &str,
    _index: usize,
) {
    if action.needs_text {
        prompt_for(executor, window, list, action, query);
    } else {
        match &action.run {
            RunSpec::CopyText(text) => {
                if let Some(display) = gdk::Display::default() {
                    if let Some(clipboard) = gtk::Clipboard::default(&display) {
                        clipboard.set_text(text);
                    }
                }
                crate::notify::notify(
                    "Gekopieerd",
                    &text.chars().take(60).collect::<String>(),
                    "ok",
                );
            }
            _ => executor.run(&action.run, query),
        }
        fade_out(window, PANEL_MS);
    }
}

fn prompt_for(
    executor: &Executor,
    bar_window: &gtk::Window,
    _list: &gtk::ListBox,
    action: &Action,
    query: &str,
) {
    let dialog = gtk::Window::new(gtk::WindowType::Popup);

    dialog.set_decorated(false);
    dialog.set_keep_above(true);

    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    box_.set_margin_top(14);
    box_.set_margin_bottom(14);
    box_.set_margin_start(14);
    box_.set_margin_end(14);
    box_.style_context().add_class("chefbar-dialog");
    dialog.add(&box_);

    let title = gtk::Label::new(Some(&action.title));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    box_.pack_start(&title, false, false, 0);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(&action.meta));
    entry.set_text(query);
    entry.set_activates_default(true);
    entry.style_context().add_class("chefbar-dialog-entry");
    box_.pack_start(&entry, false, false, 0);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Annuleren");
    let submit = gtk::Button::with_label("Uitvoeren");
    buttons.pack_start(&cancel, false, false, 0);
    buttons.pack_start(&submit, false, false, 0);
    box_.pack_start(&buttons, false, false, 0);

    let run = action.run.clone();
    let executor = executor.clone();

    let dialog_cancel = dialog.clone();
    let bar_window_cancel = bar_window.clone();
    cancel.connect_clicked(move |_| {
        dialog_cancel.close();
        bar_window_cancel.show();
    });

    let dialog_submit = dialog.clone();
    let bar_window_submit = bar_window.clone();
    let entry_submit = entry.clone();
    let executor_submit = executor.clone();
    let run_submit = run.clone();
    submit.connect_clicked(move |_| {
        let text = entry_submit.text().to_string();
        dialog_submit.close();
        bar_window_submit.hide();
        executor_submit.run(&run_submit, &text);
    });

    let dialog_keys = dialog.clone();
    let bar_window_keys = bar_window.clone();
    let executor_keys = executor.clone();
    let run_keys = run.clone();
    entry.connect_key_press_event(move |_entry, event| {
        if event.keyval() == gdk::keys::constants::Return {
            let text = _entry.text().to_string();
            dialog_keys.close();
            bar_window_keys.hide();
            executor_keys.run(&run_keys, &text);
            return glib::Propagation::Stop;
        }
        if event.keyval() == gdk::keys::constants::Escape {
            dialog_keys.close();
            bar_window_keys.show();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    // Positioneer naast de bar.
    let (x, y) = bar_window.position();
    dialog.move_(x + 8, y + 8);
    dialog.show_all();
    entry.grab_focus();
}

fn suggestion_row(action: &Action) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.style_context().add_class("chefbar-bar-suggestion");

    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    box_.set_margin_top(4);
    box_.set_margin_bottom(4);
    box_.set_margin_start(8);
    box_.set_margin_end(8);
    row.add(&box_);

    let stamp = gtk::Label::new(Some(&action.stamp));
    stamp.style_context().add_class("chefbar-bar-row-stamp");
    stamp.set_valign(gtk::Align::Center);
    box_.pack_start(&stamp, false, false, 0);

    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let title = gtk::Label::new(Some(&action.title));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(pango::EllipsizeMode::End);
    text_box.pack_start(&title, false, false, 0);
    if !action.meta.is_empty() {
        let meta = gtk::Label::new(Some(&action.meta));
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        meta.set_ellipsize(pango::EllipsizeMode::End);
        meta.style_context().add_class("chefbar-card-meta");
        text_box.pack_start(&meta, false, false, 0);
    }
    box_.pack_start(&text_box, true, true, 0);

    let shortcut = gtk::Label::new(Some(&action.shortcut));
    shortcut.set_halign(gtk::Align::End);
    shortcut.set_valign(gtk::Align::Center);
    shortcut.style_context().add_class("chefbar-card-meta");
    box_.pack_start(&shortcut, false, false, 0);

    row
}
