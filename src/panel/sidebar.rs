//! Sidebar voor het ChefApp-panel: domeinnavigatie, groepen en status.
//!
//! De navigatie wordt uit `HarnessKind::all()` opgebouwd zodat nieuwe domeinen
//! automatisch dezelfde selectie- en filterroute krijgen als de harnassen.

use crate::harness::HarnessKind;
use gtk::prelude::*;

pub const SIDEBAR_WIDTH: i32 = 240;

/// Compatibele canonieke ids voor callers die vóór de dynamische sidebar zijn
/// geschreven. Nieuwe code gebruikt `nav_ids()`.
pub const NAV_IDS: &[&str] = &["fleet", "commerce", "eval", "sync"];
pub const NAV_LABELS: &[&str] = &["Fleet", "Commerce", "Evaluatie", "Sync"];

pub fn nav_ids() -> Vec<&'static str> {
    HarnessKind::all()
        .into_iter()
        .map(|kind| kind.id())
        .collect()
}

pub fn nav_labels() -> Vec<&'static str> {
    HarnessKind::all()
        .into_iter()
        .map(|kind| kind.label())
        .collect()
}

pub fn contains_nav_id(id: &str) -> bool {
    nav_ids().contains(&id)
}

/// Bouwt de volledige sidebar en geeft `(sidebar_container, nav_buttons)` terug.
///
/// De buttons volgen de canonieke 5.0-domeinvolgorde. De actieve id bepaalt
/// de accentklasse; de live queue/statuslabels worden in `Panel::render` door
/// `sync_nav_buttons` bijgewerkt.
pub fn build_sidebar(active_group: &str) -> (gtk::Box, Vec<(String, gtk::Button)>) {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.style_context().add_class("chefbar-sidebar");
    sidebar.set_size_request(SIDEBAR_WIDTH, -1);

    let title_wrap = gtk::Box::new(gtk::Orientation::Vertical, 2);
    title_wrap.set_margin_top(14);
    title_wrap.set_margin_start(14);
    title_wrap.set_margin_end(14);
    title_wrap.set_margin_bottom(10);
    let title = gtk::Label::new(Some("ChefBar"));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.style_context().add_class("chefbar-sidebar-title");
    title_wrap.pack_start(&title, false, false, 0);
    let sub = gtk::Label::new(Some("17 domeinen · één werkruimte"));
    sub.set_halign(gtk::Align::Start);
    sub.set_xalign(0.0);
    sub.set_ellipsize(pango::EllipsizeMode::End);
    sub.style_context().add_class("chefbar-sidebar-sub");
    title_wrap.pack_start(&sub, false, false, 0);
    sidebar.pack_start(&title_wrap, false, false, 0);

    let nav_box = gtk::Box::new(gtk::Orientation::Vertical, 3);
    nav_box.style_context().add_class("chefbar-nav");
    nav_box.set_margin_start(8);
    nav_box.set_margin_end(8);

    let mut nav_buttons = Vec::new();
    for kind in HarnessKind::all() {
        let id = kind.id();
        let btn = gtk::Button::with_label(kind.label());
        btn.set_relief(gtk::ReliefStyle::None);
        btn.style_context().add_class("chefbar-nav-item");
        btn.set_hexpand(true);
        btn.set_halign(gtk::Align::Fill);
        btn.set_tooltip_text(Some(&format!(
            "{} · {}",
            kind.label(),
            kind.group().label()
        )));
        if let Some(child) = btn.child() {
            if let Some(label) = child.downcast_ref::<gtk::Label>() {
                label.set_halign(gtk::Align::Start);
                label.set_xalign(0.0);
                label.set_ellipsize(pango::EllipsizeMode::End);
            }
        }
        if id == active_group || (id == "inbox" && !contains_nav_id(active_group)) {
            btn.style_context().add_class("active");
        }
        nav_buttons.push((id.to_string(), btn.clone()));
        nav_box.pack_start(&btn, false, false, 0);
    }
    // De 17 domeinen blijven bereikbaar binnen de vaste paneelhoogte.
    let nav_scroll = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    nav_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    nav_scroll.set_vexpand(true);
    nav_scroll.set_min_content_height(220);
    nav_scroll.add(&nav_box);
    sidebar.pack_start(&nav_scroll, true, true, 0);

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
    let footer_meta = gtk::Label::new(Some("online · signaal v2 · één snapshot"));
    footer_meta.set_halign(gtk::Align::Start);
    footer_meta.set_xalign(0.0);
    footer_meta.set_ellipsize(pango::EllipsizeMode::End);
    footer_meta
        .style_context()
        .add_class("chefbar-sidebar-footer-meta");
    footer.pack_start(&footer_meta, false, false, 0);
    sidebar.pack_end(&footer, false, false, 0);

    (sidebar, nav_buttons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamische_navigatie_bevat_alle_5_0_domeinen() {
        let ids = nav_ids();
        assert!(ids.len() >= 15);
        for id in [
            "inbox",
            "fleet",
            "vault",
            "linear",
            "containers",
            "secrets",
            "kater",
        ] {
            assert!(ids.contains(&id), "domein {id} ontbreekt");
        }
    }

    #[test]
    fn labels_en_ids_hebben_dezelfde_lengte() {
        assert_eq!(nav_ids().len(), nav_labels().len());
    }
}
