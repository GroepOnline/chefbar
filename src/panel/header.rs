//! Header voor het ChefApp-panel: titel + zoekbalk + controls.
//!
//! Single source of truth voor de SearchEntry; lifecycle (wire_search,
//! focus-shortcuts) blijft in `panel::mod` omdat die de gedeelde state
//! nodig heeft. Deze module bouwt alleen de widget-boom.

use gtk::prelude::*;

/// Bouwt de header en geeft `(header_box, title_label, search_entry,
/// refresh_btn, min_btn, close_btn)` terug. De caller wiret de knoppen en
/// zet de titel op de actieve domeinnaam.
pub fn build_header() -> (
    gtk::Box,
    gtk::Label,
    gtk::SearchEntry,
    gtk::Button,
    gtk::Button,
    gtk::Button,
) {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.style_context().add_class("chefbar-header");
    header.set_margin_bottom(0);

    let title_block = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let title = gtk::Label::new(Some("ChefApp"));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.style_context().add_class("chefbar-title");
    title.set_attributes(Some(&crate::css::heading_attrs()));
    title_block.pack_start(&title, false, false, 0);
    let title_sub = gtk::Label::new(Some("ChefGroep · operate"));
    title_sub.set_halign(gtk::Align::Start);
    title_sub.set_xalign(0.0);
    title_sub.set_ellipsize(pango::EllipsizeMode::End);
    title_sub.style_context().add_class("chefbar-title-sub");
    title_block.pack_start(&title_sub, false, false, 0);
    header.pack_start(&title_block, false, false, 0);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Zoek of typ een opdracht · / of Ctrl+K"));
    search.set_tooltip_text(Some("Zoek in alle domeinen"));
    search.style_context().add_class("chefbar-search");
    search.set_hexpand(true);
    search.set_halign(gtk::Align::Fill);
    header.pack_start(&search, true, true, 0);

    let header_controls = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    let refresh_btn = gtk::Button::new();
    let refresh_icon = crate::icons::image("refresh", 15);
    refresh_btn.set_image(Some(&refresh_icon));
    refresh_btn.set_relief(gtk::ReliefStyle::None);
    refresh_btn.set_tooltip_text(Some("Vernieuw nu"));
    refresh_btn.style_context().add_class("chefbar-gbtn");

    let min_btn = gtk::Button::new();
    let min_icon = crate::icons::image("minus", 15);
    min_btn.set_image(Some(&min_icon));
    min_btn.set_relief(gtk::ReliefStyle::None);
    min_btn.set_tooltip_text(Some("Minimaliseren"));
    min_btn.style_context().add_class("chefbar-gbtn");

    let close_btn = gtk::Button::new();
    let close_icon = crate::icons::image("x", 15);
    close_btn.set_image(Some(&close_icon));
    close_btn.set_relief(gtk::ReliefStyle::None);
    close_btn.set_tooltip_text(Some("Verbergen"));
    close_btn.style_context().add_class("chefbar-gbtn");

    header_controls.pack_start(&refresh_btn, false, false, 0);
    header_controls.pack_start(&min_btn, false, false, 0);
    header_controls.pack_start(&close_btn, false, false, 0);
    header.pack_end(&header_controls, false, false, 0);

    (header, title, search, refresh_btn, min_btn, close_btn)
}
