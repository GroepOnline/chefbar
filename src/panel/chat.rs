//! Control-chat canvas — persistent, buiten de zone-rebuild.
//!
//! Signaal v2: één accent, radius 6/10, General Sans + IBM Plex Mono data,
//! geen tweede signature, geen emoji, warm Nederlands.

use crate::chat::{ChatLog, ChatRole};
use crate::state::Shared;
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub struct ChatPane {
    pub root: gtk::Box,
    transcript: gtk::Box,
    entry: gtk::Entry,
    send: gtk::Button,
    meta: gtk::Label,
    shared: Shared,
    last_rev: Rc<Cell<i64>>,
}

impl ChatPane {
    pub fn new(shared: Shared) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.style_context().add_class("chefbar-chat");
        root.set_hexpand(true);
        root.set_vexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Vertical, 2);
        header.set_margin_top(12);
        header.set_margin_start(16);
        header.set_margin_end(16);
        header.set_margin_bottom(8);
        let title = gtk::Label::new(Some("Control"));
        title.set_halign(gtk::Align::Start);
        title.set_xalign(0.0);
        title.style_context().add_class("chefbar-title");
        header.pack_start(&title, false, false, 0);
        let meta = gtk::Label::new(Some("devops en overzicht"));
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
        {
            let shared_send = shared.clone();
            let entry_send = entry.clone();
            entry.connect_activate(move |widget| {
                let text = widget.text().to_string();
                widget.set_text("");
                crate::chat::submit(&shared_send, &text);
                let _ = &entry_send;
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
            let shared_t = shared.clone();
            let transcript_t = transcript.clone();
            let meta_t = meta.clone();
            let entry_t = entry.clone();
            let send_t = send.clone();
            let last = last_rev.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(400), move || {
                paint(
                    &shared_t,
                    &transcript_t,
                    &meta_t,
                    &entry_t,
                    &send_t,
                    &last,
                    false,
                );
                ControlFlow::Continue
            });
        }

        Self {
            root,
            transcript,
            entry,
            send,
            meta,
            shared,
            last_rev,
        }
    }

    pub fn refresh(&self) {
        paint(
            &self.shared,
            &self.transcript,
            &self.meta,
            &self.entry,
            &self.send,
            &self.last_rev,
            true,
        );
    }

    pub fn focus_composer(&self) {
        self.entry.grab_focus();
    }
}

fn paint(
    shared: &Shared,
    transcript: &gtk::Box,
    meta: &gtk::Label,
    entry: &gtk::Entry,
    send: &gtk::Button,
    last_rev: &Rc<Cell<i64>>,
    force: bool,
) {
    let rev = shared
        .chat_revision
        .load(std::sync::atomic::Ordering::Relaxed);
    let log = shared.chat.read().unwrap().clone();
    entry.set_sensitive(!log.busy);
    send.set_sensitive(!log.busy);
    let status = if log.busy {
        "bezig"
    } else if log.target.is_some() {
        "klaar"
    } else {
        "wacht"
    };
    meta.set_text(&format!("{} · {}", log.target_label(), status));
    if !force && rev == last_rev.get() {
        return;
    }
    last_rev.set(rev);
    render_messages(transcript, &log);
}

fn render_messages(transcript: &gtk::Box, log: &ChatLog) {
    for child in transcript.children() {
        transcript.remove(&child);
    }
    if log.messages.is_empty() {
        let empty = gtk::Label::new(Some(
            "Praat hier met een Herdr-agent over fleet, deploys en status. Geen tweede app, geen ACP.",
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
                "agent"
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
