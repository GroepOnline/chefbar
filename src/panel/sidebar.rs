//! Sidebar voor het ChefApp-panel (240 px, groepen, dots).
//!
//! Canonieke 5.0-navigatie: 15 live-domeinen + compat-ids `eval`/`sync`.
//! Groepen komen uit `HarnessKind::group()`; de scroller houdt de kolom
//! binnen 880 px zonder een tweede venster.

use gtk::prelude::*;

use crate::harness::{HarnessGroup, HarnessKind};

pub const SIDEBAR_WIDTH: i32 = 240;

/// Canonieke nav-ids (15 domeinen + eval/sync compat).
pub const NAV_IDS: &[&str] = &[
    "inbox",
    "fleet",
    "herdr",
    "containers",
    "vault",
    "commerce",
    "crm",
    "share",
    "clipboard",
    "desktop",
    "sync",
    "tasks",
    "linear",
    "secrets",
    "kater",
    "health",
    "eval",
];
pub const NAV_LABELS: &[&str] = &[
    "Inbox",
    "Fleet",
    "Herdr",
    "Containers",
    "Vault",
    "Accounts",
    "CRM",
    "Share",
    "Clipboard",
    "Desktop",
    "Sync",
    "Taken",
    "Linear",
    "Secrets",
    "Kater",
    "Health",
    "Evaluatie",
];

/// Bouwt de volledige sidebar en geeft `(sidebar_container, nav_buttons)` terug.
///
/// `active_group` bepaalt welke knop de `active` klas krijgt. De footer
/// (status) is onderdeel van de sidebar zodat het bestaande `pack_end`-patroon
/// intact blijft.
pub fn build_sidebar(active_group: &str) -> (gtk::Box, Vec<(String, gtk::Button)>) {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.style_context().add_class("chefbar-sidebar");
    sidebar.set_size_request(SIDEBAR_WIDTH, -1);

    // App-titel
    let title_wrap = gtk::Box::new(gtk::Orientation::Vertical, 2);
    title_wrap.set_margin_top(14);
    title_wrap.set_margin_start(14);
    title_wrap.set_margin_end(14);
    title_wrap.set_margin_bottom(10);
    let title = gtk::Label::new(Some("ChefApp"));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.style_context().add_class("chefbar-sidebar-title");
    title_wrap.pack_start(&title, false, false, 0);
    let sub = gtk::Label::new(Some("chefgroep-online"));
    sub.set_halign(gtk::Align::Start);
    sub.set_xalign(0.0);
    sub.style_context().add_class("chefbar-sidebar-sub");
    title_wrap.pack_start(&sub, false, false, 0);
    sidebar.pack_start(&title_wrap, false, false, 0);

    let nav_scroller = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    nav_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    nav_scroller.set_vexpand(true);

    let nav_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    nav_box.style_context().add_class("chefbar-nav");
    nav_box.set_margin_start(8);
    nav_box.set_margin_end(8);

    let mut nav_buttons: Vec<(String, gtk::Button)> = Vec::new();
    let mut last_group: Option<HarnessGroup> = None;
    for (idx, (id, label)) in NAV_IDS.iter().zip(NAV_LABELS.iter()).enumerate() {
        let kind = HarnessKind::all().into_iter().find(|kind| kind.id() == *id);
        if let Some(kind) = kind {
            let group = kind.group();
            if last_group.as_ref() != Some(&group) {
                if last_group.is_some() {
                    let hairline = gtk::Separator::new(gtk::Orientation::Horizontal);
                    hairline.style_context().add_class("chefbar-nav-sep");
                    hairline.set_margin_top(6);
                    hairline.set_margin_bottom(4);
                    nav_box.pack_start(&hairline, false, false, 0);
                }
                let group_label = gtk::Label::new(Some(group.label()));
                group_label.set_halign(gtk::Align::Start);
                group_label.set_xalign(0.0);
                group_label.set_margin_start(6);
                group_label.set_margin_top(4);
                group_label
                    .style_context()
                    .add_class("chefbar-sidebar-footer-title");
                nav_box.pack_start(&group_label, false, false, 0);
                last_group = Some(group);
            }
        }
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
        if *id == active_group || (idx == 0 && !NAV_IDS.contains(&active_group)) {
            btn.style_context().add_class("active");
        }
        nav_buttons.push((id.to_string(), btn.clone()));
        nav_box.pack_start(&btn, false, false, 0);
    }
    nav_scroller.add(&nav_box);
    sidebar.pack_start(&nav_scroller, true, true, 0);

    // Status-footer
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 4);
    footer.style_context().add_class("chefbar-sidebar-footer");
    footer.set_margin_start(12);
    footer.set_margin_end(12);
    footer.set_margin_top(10);
    footer.set_margin_bottom(12);
    let footer_title = gtk::Label::new(Some("Status"));
    footer_title.set_halign(gtk::Align::Start);
    footer_title.set_xalign(0.0);
    footer_title
        .style_context()
        .add_class("chefbar-sidebar-footer-title");
    footer.pack_start(&footer_title, false, false, 0);
    let footer_meta = gtk::Label::new(Some("online \u{00b7} signaal v2"));
    footer_meta.set_halign(gtk::Align::Start);
    footer_meta.set_xalign(0.0);
    footer_meta
        .style_context()
        .add_class("chefbar-sidebar-footer-meta");
    footer.pack_start(&footer_meta, false, false, 0);
    sidebar.pack_end(&footer, false, false, 0);

    (sidebar, nav_buttons)
}
