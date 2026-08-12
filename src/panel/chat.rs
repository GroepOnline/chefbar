//! Control-chat canvas — persistent, buiten de zone-rebuild.
//!
//! Signaal v2: één accent, radius 6/10, General Sans + IBM Plex Mono data,
//! geen tweede signature, geen emoji, warm Nederlands.
//! Default-harnas is Pi; jcode is geheugen, geen kiezer-optie.

use crate::chat::{list_targets, resolve_target, ChatLog, ChatRole};
use crate::state::Shared;
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

pub struct ChatPane {
    pub root: gtk::Box,
    transcript: gtk::Box,
    entry: gtk::Entry,
    send: gtk::Button,
    meta: gtk::Label,
    combo: gtk::ComboBoxText,
    shared: Shared,
    last_rev: Rc<Cell<i64>>,
    suppress_combo: Rc<Cell<bool>>,
    combo_fp: Rc<RefCell<String>>,
}

impl ChatPane {
    pub fn new(shared: Shared) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.style_context().add_class("chefbar-chat");
        root.set_hexpand(true);
        root.set_vexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Vertical, 4);
        header.set_margin_top(12);
        header.set_margin_start(16);
        header.set_margin_end(16);
        header.set_margin_bottom(8);
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = gtk::Label::new(Some("Control"));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.style_context().add_class("chefbar-title");
        title_row.pack_start(&title, true, true, 0);
        let combo = gtk::ComboBoxText::new();
        combo.style_context().add_class("chefbar-chat-combo");
        combo.set_tooltip_text(Some("Live Herdr-harnas. Standaard Pi."));
        title_row.pack_end(&combo, false, false, 0);
        header.pack_start(&title_row, false, false, 0);
        let meta = gtk::Label::new(Some("Pi · devops en overzicht"));
        meta.set_halign(gtk::Align::Start);
        meta.set_xalign(0.0);
        meta.set_ellipsize(pango::EllipsizeMode::End);
        meta.style_context().add_class("chefbar-title-sub");
        header.pack_start(&meta, false, false, 0);
        root.pack_start(&header, false, false, 0);

        let scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_vexpand(true);
        scroller.set_hexpand(true);
        let transcript = gtk::Box::new(gtk::Orientation::Vertical, 8);
        transcript.style_context().add_class("chefbar-chat-log");
        transcript.set_margin_start(16);
        transcript.set_margin_end(16);
        transcript.set_margin_bottom(8);
        scroller.add(&transcript);
        root.pack_start(&scroller, true, true, 0);

        let composer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        composer.style_context().add_class("chefbar-chat-composer");
        composer.set_margin_start(16);
        composer.set_margin_end(16);
        composer.set_margin_bottom(14);
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("Vraag over fleet, deploy, status…"));
        entry.set_hexpand(true);
        entry.style_context().add_class("chefbar-chat-entry");
        let send = gtk::Button::with_label("Stuur");
        send.style_context().add_class("chefbar-btn");
        send.style_context().add_class("chefbar-primary");
        composer.pack_start(&entry, true, true, 0);
        composer.pack_start(&send, false, false, 0);
        root.pack_start(&composer, false, false, 0);

        let last_rev = Rc::new(Cell::new(-1));
        let suppress_combo = Rc::new(Cell::new(false));
        let combo_fp = Rc::new(RefCell::new(String::new()));
        {
            let shared_send = shared.clone();
            entry.connect_activate(move |widget| {
                let text = widget.text().to_string();
                widget.set_text("");
                crate::chat::submit(&shared_send, &text);
            });
        }
        {
            let shared_send = shared.clone();
            let entry_send = entry.clone();
            send.connect_clicked(move |_| {
                let text = entry_send.text().to_string();
                entry_send.set_text("");
                crate::chat::submit(&shared_send, &text);
            });
        }
        {
            let shared_pin = shared.clone();
            let suppress = suppress_combo.clone();
            combo.connect_changed(move |combo| {
                if suppress.get() {
                    return;
                }
                if let Some(id) = combo.active_id() {
                    crate::chat::pin_target(&shared_pin, &id);
                }
            });
        }
        {
            let shared_t = shared.clone();
            let transcript_t = transcript.clone();
            let meta_t = meta.clone();
            let entry_t = entry.clone();
            let send_t = send.clone();
            let combo_t = combo.clone();
            let last = last_rev.clone();
            let suppress = suppress_combo.clone();
            let fp = combo_fp.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(400), move || {
                paint(&PaintCtx {
                    shared: &shared_t,
                    transcript: &transcript_t,
                    meta: &meta_t,
                    entry: &entry_t,
                    send: &send_t,
                    combo: &combo_t,
                    last_rev: &last,
                    suppress: &suppress,
                    combo_fp: &fp,
                    force: false,
                });
                ControlFlow::Continue
            });
        }

        Self {
            root,
            transcript,
            entry,
            send,
            meta,
            combo,
            shared,
            last_rev,
            suppress_combo,
            combo_fp,
        }
    }

    pub fn refresh(&self) {
        paint(&PaintCtx {
            shared: &self.shared,
            transcript: &self.transcript,
            meta: &self.meta,
            entry: &self.entry,
            send: &self.send,
            combo: &self.combo,
            last_rev: &self.last_rev,
            suppress: &self.suppress_combo,
            combo_fp: &self.combo_fp,
            force: true,
        });
    }

    pub fn focus_composer(&self) {
        self.entry.grab_focus();
    }
}

struct PaintCtx<'a> {
    shared: &'a Shared,
    transcript: &'a gtk::Box,
    meta: &'a gtk::Label,
    entry: &'a gtk::Entry,
    send: &'a gtk::Button,
    combo: &'a gtk::ComboBoxText,
    last_rev: &'a Rc<Cell<i64>>,
    suppress: &'a Rc<Cell<bool>>,
    combo_fp: &'a Rc<RefCell<String>>,
    force: bool,
}

fn paint(ctx: &PaintCtx<'_>) {
    let rev = ctx
        .shared
        .chat_revision
        .load(std::sync::atomic::Ordering::Relaxed);
    let log = ctx.shared.chat.read().unwrap().clone();
    ctx.entry.set_sensitive(!log.busy);
    ctx.send.set_sensitive(!log.busy);
    ctx.combo.set_sensitive(!log.busy);
    let status = if log.busy {
        "bezig"
    } else if log.target.is_some() {
        "klaar"
    } else {
        "wacht op Pi"
    };
    ctx.meta
        .set_text(&format!("{} · {}", log.target_label(), status));
    paint_combo(
        ctx.shared,
        ctx.combo,
        &log,
        ctx.suppress,
        ctx.combo_fp,
        ctx.force,
    );
    if !ctx.force && rev == ctx.last_rev.get() {
        return;
    }
    ctx.last_rev.set(rev);
    render_messages(ctx.transcript, &log);
}

fn paint_combo(
    shared: &Shared,
    combo: &gtk::ComboBoxText,
    log: &ChatLog,
    suppress: &Rc<Cell<bool>>,
    combo_fp: &Rc<RefCell<String>>,
    force: bool,
) {
    let ops = shared.ops.read().unwrap().clone();
    let targets = list_targets(&ops);
    let pinned = if log.pinned {
        log.target.clone()
    } else {
        None
    };
    let current = log
        .target
        .clone()
        .or_else(|| resolve_target(&ops, pinned.as_deref()));
    let fp = format!(
        "{}#{}",
        targets
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>()
            .join("|"),
        current.as_deref().unwrap_or("")
    );
    if !force && fp == *combo_fp.borrow() {
        return;
    }
    suppress.set(true);
    combo.remove_all();
    for target in &targets {
        combo.append(Some(&target.id), &target.label);
    }
    if let Some(id) = &current {
        if !targets.iter().any(|t| t.id == *id) {
            combo.append(Some(id), &format!("vast · {id}"));
        }
        combo.set_active_id(Some(id));
    }
    suppress.set(false);
    *combo_fp.borrow_mut() = fp;
}

fn render_messages(transcript: &gtk::Box, log: &ChatLog) {
    for child in transcript.children() {
        transcript.remove(&child);
    }
    if log.messages.is_empty() {
        let empty = gtk::Label::new(Some(
            "Standaard Pi, over fleet en deploys. jcode is geheugen, geen chat. Andere live harnassen kies je hierboven.",
        ));
        empty.set_line_wrap(true);
        empty.set_xalign(0.0);
        empty.set_halign(gtk::Align::Start);
        empty.style_context().add_class("chefbar-card-meta");
        transcript.pack_start(&empty, false, false, 0);
        transcript.show_all();
        return;
    }
    for msg in &log.messages {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.style_context().add_class("chefbar-chat-msg");
        let who = match msg.role {
            ChatRole::Operator => {
                row.style_context().add_class("operator");
                "jij"
            }
            ChatRole::Agent => {
                row.style_context().add_class("agent");
                log.kind.as_deref().unwrap_or("agent")
            }
            ChatRole::System => {
                row.style_context().add_class("system");
                "app"
            }
        };
        let stamp = gtk::Label::new(Some(who));
        stamp.set_xalign(0.0);
        stamp.style_context().add_class("chefbar-chat-who");
        let body = gtk::Label::new(Some(&msg.text));
        body.set_line_wrap(true);
        body.set_xalign(0.0);
        body.set_selectable(true);
        body.style_context().add_class("chefbar-chat-body");
        row.pack_start(&stamp, false, false, 0);
        row.pack_start(&body, false, false, 0);
        transcript.pack_start(&row, false, false, 0);
    }
    transcript.show_all();
}
