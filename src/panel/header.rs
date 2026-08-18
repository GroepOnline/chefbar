//! Header voor het ChefApp-panel: titel + zoekbalk + controls.
//!
//! Single source of truth voor de SearchEntry; lifecycle (wire_search,
//! focus-shortcuts) blijft in `panel::mod` omdat die de gedeelde state
//! nodig heeft. Deze module bouwt alleen de widget-boom.

use gtk::prelude::*;

use crate::harness::HarnessKind;

/// Nav-ids die wel een rij krijgen maar geen eigen domein zijn (compat).
const COMPAT_IDS: [&str; 2] = ["eval", "sync"];

/// Aantal live domeinen, afgeleid van `HarnessKind` zodat de subtitel niet
/// scheefloopt zodra er een domein bijkomt (stond hardcoded op 15, terwijl
/// het er 16 zijn).
pub(crate) fn live_domain_count() -> usize {
    HarnessKind::all()
        .iter()
        .filter(|kind| !COMPAT_IDS.contains(&kind.id()))
        .count()
}

/// Devin v2 heading-tracking (−0.02em) omgerekend naar Pango-units.
///
/// Pango rekent letter-spacing in Pango-units, waarbij 1 device-unit (px op
/// scherm) gelijk is aan `pango::SCALE` (1024) units. Voor een kop van 18px
/// is −0.02em dus −0.02 × 18 × 1024 ≈ −369 units. De eerdere −20 was
/// effectief nul tracking (≈ −0.001em) en leverde dus geen v2-kop op.
pub fn heading_tracking_units(font_px: f64) -> i32 {
    const TRACKING_EM: f64 = -0.02;
    let units = TRACKING_EM * font_px * pango::SCALE as f64;
    units.round() as i32
}

/// Bouwt de header en geeft `(header_box, title_label, search_entry,
/// refresh_btn, min_btn, close_btn)` terug. De caller wiret de knoppen en
/// zet de titel op de actieve domeinnaam (deviant van statisch "ChefBar"
/// in de sidebar).
pub fn build_header() -> (
    gtk::Box,
    gtk::Label,
    gtk::SearchEntry,
    gtk::Button,
    gtk::Button,
    gtk::Button,
) {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    header.style_context().add_class("chefbar-header");
    header.set_margin_bottom(0);

    let title_block = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let title = gtk::Label::new(Some("ChefBar"));
    title.set_halign(gtk::Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(pango::EllipsizeMode::End);
    title.style_context().add_class("chefbar-title");
    // v2-heading tracking: −0.02em bij 18px (zie heading_tracking_units).
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrInt::new_letter_spacing(heading_tracking_units(
        18.0,
    )));
    title.set_attributes(Some(&attrs));
    title_block.pack_start(&title, false, false, 0);
    let subtitle = format!("ChefGroep \u{00b7} {} domeinen", live_domain_count());
    let title_sub = gtk::Label::new(Some(subtitle.as_str()));
    title_sub.set_halign(gtk::Align::Start);
    title_sub.set_xalign(0.0);
    title_sub.set_ellipsize(pango::EllipsizeMode::End);
    title_sub.style_context().add_class("chefbar-title-sub");
    title_block.pack_start(&title_sub, false, false, 0);
    header.pack_start(&title_block, false, false, 0);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(super::overlay::OVERLAY_PLACEHOLDER));
    search.style_context().add_class("chefbar-search");
    search.set_hexpand(true);
    search.set_halign(gtk::Align::Fill);
    header.pack_start(&search, true, true, 0);

    let header_controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let refresh_btn = gtk::Button::new();
    let refresh_icon =
        gtk::Image::from_icon_name(Some("view-refresh-symbolic"), gtk::IconSize::Button);
    refresh_btn.set_image(Some(&refresh_icon));
    refresh_btn.set_relief(gtk::ReliefStyle::None);
    refresh_btn.style_context().add_class("chefbar-gbtn");
    refresh_btn.set_tooltip_text(Some("Verversen"));

    let min_btn = gtk::Button::new();
    let min_icon =
        gtk::Image::from_icon_name(Some("window-minimize-symbolic"), gtk::IconSize::Button);
    min_btn.set_image(Some(&min_icon));
    min_btn.set_relief(gtk::ReliefStyle::None);
    min_btn.style_context().add_class("chefbar-gbtn");
    min_btn.set_tooltip_text(Some("Minimaliseren"));

    let close_btn = gtk::Button::new();
    let close_icon =
        gtk::Image::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Button);
    close_btn.set_image(Some(&close_icon));
    close_btn.set_relief(gtk::ReliefStyle::None);
    close_btn.style_context().add_class("chefbar-gbtn");
    close_btn.set_tooltip_text(Some("Sluiten"));

    header_controls.pack_start(&refresh_btn, false, false, 0);
    header_controls.pack_start(&min_btn, false, false, 0);
    header_controls.pack_start(&close_btn, false, false, 0);
    header.pack_end(&header_controls, false, false, 0);

    (header, title, search, refresh_btn, min_btn, close_btn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_tracking_volgt_v2_spec() {
        // −0.02em × 18px × 1024 = −368,64 → −369.
        assert_eq!(heading_tracking_units(18.0), -369);
        // −0.02em × 14px × 1024 = −286,72 → −287.
        assert_eq!(heading_tracking_units(14.0), -287);
    }

    #[test]
    fn heading_tracking_is_altijd_tighter() {
        for px in [11.0, 13.0, 14.0, 18.0, 24.0] {
            assert!(
                heading_tracking_units(px) < 0,
                "tracking moet negatief zijn bij {px}px"
            );
        }
    }

    #[test]
    fn live_domain_count_telt_compat_ids_niet_mee() {
        let total = HarnessKind::all().len();
        assert_eq!(live_domain_count(), total - COMPAT_IDS.len());
    }

    #[test]
    fn compat_ids_bestaan_nog_als_harness_kind() {
        let ids: Vec<&str> = HarnessKind::all().iter().map(|kind| kind.id()).collect();
        for compat in COMPAT_IDS {
            assert!(ids.contains(&compat), "compat-id {compat} ontbreekt");
        }
    }
}
