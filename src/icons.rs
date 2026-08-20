//! Lucide-stroke iconen voor ChefApp (GTK3 pixbuf, geen emoji, geen Adwaita).
//!
//! SVG's gebruiken `currentColor`; [`image`] vervangt door de actieve Signaal-ink.

use std::cell::RefCell;

use gtk::glib::object::ObjectExt;
use gtk::prelude::*;

const SVG_HEAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"#;
const SVG_TAIL: &str = "</svg>";

pub fn svg_body(name: &str) -> &'static str {
    match name {
        "inbox" => {
            r#"<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>"#
        }
        "list" => {
            r#"<path d="M8 6h13"/><path d="M8 12h13"/><path d="M8 18h13"/><path d="M3 6h.01"/><path d="M3 12h.01"/><path d="M3 18h.01"/>"#
        }
        "circle-dot" => r#"<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/>"#,
        "server" => {
            r#"<rect width="20" height="8" x="2" y="2" rx="2" ry="2"/><rect width="20" height="8" x="2" y="14" rx="2" ry="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/>"#
        }
        "layout" => {
            r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M9 21V9"/>"#
        }
        "message" => r#"<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>"#,
        "box" => {
            r#"<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>"#
        }
        "lock" => {
            r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>"#
        }
        "wallet" => {
            r#"<path d="M19 7V4a1 1 0 0 0-1-1H5a2 2 0 0 0 0 4h15a1 1 0 0 1 1 1v4h-3a2 2 0 0 0 0 4h3a1 1 0 0 0 1-1v-2.5"/><path d="M3 5v14a2 2 0 0 0 2 2h15a1 1 0 0 0 1-1v-4"/>"#
        }
        "users" => {
            r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#
        }
        "folder-sync" => {
            r#"<path d="M9 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v.5"/><path d="M15 17a3 3 0 1 0 6 0 3 3 0 1 0-6 0"/><path d="M21 14v3h-3"/>"#
        }
        "clipboard" => {
            r#"<rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>"#
        }
        "monitor" => {
            r#"<rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/>"#
        }
        "refresh" => {
            r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#
        }
        "key" => {
            r#"<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>"#
        }
        "radio" => {
            r#"<circle cx="12" cy="12" r="2"/><path d="M4.93 19.07a10 10 0 0 1 0-14.14"/><path d="M7.76 16.24a6 6 0 0 1 0-8.49"/><path d="M16.24 7.76a6 6 0 0 1 0 8.49"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>"#
        }
        "brain" => {
            r#"<path d="M12 5a3 3 0 1 0-5.997.125 4 4 0 0 0-2.526 5.77 4 4 0 0 0 .556 6.588A4 4 0 1 0 12 18Z"/><path d="M12 5a3 3 0 1 1 5.997.125 4 4 0 0 1 2.526 5.77 4 4 0 0 1-.556 6.588A4 4 0 1 1 12 18Z"/><path d="M12 5v13"/>"#
        }
        "activity" => r#"<path d="M22 12h-4l-3 7L9 5l-3 7H2"/>"#,
        "flask" => {
            r#"<path d="M10 2v7.31"/><path d="M14 9.3V2"/><path d="M8.5 2h7"/><path d="M14 9.3a6.5 6.5 0 1 1-4 0"/>"#
        }
        "bot" => {
            r#"<path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/>"#
        }
        "git-branch" => {
            r#"<line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>"#
        }
        "search" => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
        "send" => r#"<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>"#,
        "minus" => r#"<path d="M5 12h14"/>"#,
        "x" => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
        "zap" => {
            r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/>"#
        }
        _ => r#"<circle cx="12" cy="12" r="10"/>"#,
    }
}

pub fn for_nav(id: &str) -> &'static str {
    match id {
        "inbox" => "inbox",
        "tasks" => "list",
        "linear" => "circle-dot",
        "fleet" => "server",
        "herdr" => "layout",
        "control" => "message",
        "containers" => "box",
        "vault" => "lock",
        "commerce" => "wallet",
        "crm" => "users",
        "share" => "folder-sync",
        "clipboard" => "clipboard",
        "desktop" => "monitor",
        "sync" => "refresh",
        "secrets" => "key",
        "kater" => "radio",
        "brain" => "brain",
        "health" => "activity",
        "eval" => "flask",
        "agents" => "bot",
        "flows" => "git-branch",
        _ => "circle-dot",
    }
}

pub fn for_action(keywords: &str, section: &str) -> &'static str {
    let hay = format!("{keywords} {section}").to_lowercase();
    if hay.contains("inbox") {
        "inbox"
    } else if hay.contains("fleet") || hay.contains("node") {
        "server"
    } else if hay.contains("herdr") || hay.contains("agent") {
        "bot"
    } else if hay.contains("linear") {
        "circle-dot"
    } else if hay.contains("secret") {
        "key"
    } else if hay.contains("clipboard") {
        "clipboard"
    } else if hay.contains("share") || hay.contains("sync") {
        "folder-sync"
    } else if hay.contains("control") || hay.contains("chat") {
        "message"
    } else {
        "zap"
    }
}

#[cfg(test)]
fn all_named() -> &'static [&'static str] {
    &[
        "inbox",
        "list",
        "circle-dot",
        "server",
        "layout",
        "message",
        "box",
        "lock",
        "wallet",
        "users",
        "folder-sync",
        "clipboard",
        "monitor",
        "refresh",
        "key",
        "radio",
        "brain",
        "activity",
        "flask",
        "bot",
        "git-branch",
        "search",
        "send",
        "minus",
        "x",
        "zap",
    ]
}

#[derive(Clone, Copy)]
enum InkKind {
    Fg,
    Muted,
    Canvas,
}

struct LiveIcon {
    image: gtk::glib::object::WeakRef<gtk::Image>,
    name: String,
    px: i32,
    kind: InkKind,
}

thread_local! {
    static LIVE: RefCell<Vec<LiveIcon>> = const { RefCell::new(Vec::new()) };
}

fn markup(name: &str, color: &str) -> String {
    format!("{SVG_HEAD}{}{SVG_TAIL}", svg_body(name)).replace("currentColor", color)
}

fn color_for(kind: InkKind) -> &'static str {
    match kind {
        InkKind::Fg => crate::css::ink_hex(),
        InkKind::Muted => crate::css::muted_hex(),
        InkKind::Canvas => crate::css::canvas_hex(),
    }
}

fn from_kind(name: &str, px: i32, kind: InkKind) -> gtk::Image {
    let image = if let Some(pixbuf) = pixbuf_from_svg(&markup(name, color_for(kind)), px) {
        gtk::Image::from_pixbuf(Some(&pixbuf))
    } else {
        gtk::Image::from_icon_name(Some("image-missing"), gtk::IconSize::Button)
    };
    LIVE.with(|live| {
        let mut icons = live.borrow_mut();
        icons.retain(|icon| icon.image.upgrade().is_some());
        icons.push(LiveIcon {
            image: image.downgrade(),
            name: name.to_string(),
            px,
            kind,
        });
    });
    image
}

pub fn image(name: &str, px: i32) -> gtk::Image {
    from_kind(name, px, InkKind::Fg)
}

pub fn image_muted(name: &str, px: i32) -> gtk::Image {
    from_kind(name, px, InkKind::Muted)
}

/// Rasterized glyph for `.chefbar-solid` (canvas ink on text fill).
pub fn image_on_solid(name: &str, px: i32) -> gtk::Image {
    from_kind(name, px, InkKind::Canvas)
}

/// Rebuild tracked pixbufs after a live theme switch (CSS cannot recolor rasters).
pub fn recolor_all() {
    LIVE.with(|live| {
        let mut icons = live.borrow_mut();
        icons.retain(|icon| icon.image.upgrade().is_some());
        for icon in icons.iter() {
            let Some(image) = icon.image.upgrade() else {
                continue;
            };
            if let Some(pixbuf) =
                pixbuf_from_svg(&markup(&icon.name, color_for(icon.kind)), icon.px)
            {
                image.set_from_pixbuf(Some(&pixbuf));
            }
        }
    });
}

fn pixbuf_from_svg(svg: &str, px: i32) -> Option<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::with_mime_type("image/svg+xml").ok()?;
    loader.set_size(px.max(12), px.max(12));
    loader.write(svg.as_bytes()).ok()?;
    loader.close().ok()?;
    loader.pixbuf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_icon_has_paths() {
        for name in all_named() {
            assert!(svg_body(name).contains('<'), "icon {name} missing svg body");
        }
    }

    #[test]
    fn nav_ids_resolve() {
        for id in crate::panel::sidebar::NAV_IDS {
            let name = for_nav(id);
            assert!(
                svg_body(name).contains('<'),
                "nav {id} -> {name} has no body"
            );
        }
    }
}
