//! Signaal v2 (Devin-richting) CSS voor ChefBar.
//!
//! Tokens 1-op-1 uit `GroepOnline/design-system` (`DESIGN.md` v2 + `tokens.css`,
//! skin `devin`), gemapt op de GTK3-subset. Joep, 2026-08-18: ChefBar volgt
//! deze taal, niet Huly/`.ulpi`.
//!
//! Canvas light-first `#F7F6F5`, donker basalt-warm `#121111`. Eén accent
//! `#317CFF` / `#5C97FF`. General Sans UI + IBM Plex Mono data. Radius 6
//! (controls) / 10 (cards, overlay, dialog, composer). Hairlines, geen
//! elevation, geen pillen, geen thema-gradients. De 2px-streep is de
//! v2 worked-row (line-strong in rust, accent tijdens een run).
//! Groen = git/PR/toestemming; amber = wacht-op-jou; rood = fout/destructive.

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

use gtk::prelude::*;

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

thread_local! {
    static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}
static ACTIVE: OnceLock<Mutex<String>> = OnceLock::new();

fn set_active(theme: &str) {
    if let Ok(mut active) = ACTIVE
        .get_or_init(|| Mutex::new(THEME_LIGHT.to_string()))
        .lock()
    {
        *active = theme.to_string();
    }
}

/// Het nu actieve thema (voor footer-toggle en state-persist). Valt terug op
/// licht: v2 is light-first.
pub fn active_theme() -> String {
    ACTIVE
        .get_or_init(|| Mutex::new(THEME_LIGHT.to_string()))
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| THEME_LIGHT.to_string())
}

/// Laadt de stylesheet voor `theme` en geeft de provider voor
/// `add_provider_for_screen`. Herhaald aanroepen met een ander thema
/// herlaadt dezelfde provider (live skin-wissel).
pub fn theme_provider(theme: &str) -> gtk::CssProvider {
    let provider = PROVIDER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            let provider = gtk::CssProvider::new();
            if let Err(err) = provider.load_from_data(styles_css(theme).as_bytes()) {
                eprintln!("chefbar: css-load mislukt ({err})");
            }
            *slot = Some(provider);
        }
        slot.as_ref().expect("provider net aangemaakt").clone()
    });
    set_active(theme);
    provider
}

/// Live thema-wissel: herlaad de gedeelde provider en onthoud de keuze.
pub fn set_theme(theme: &str) {
    set_active(theme);
    PROVIDER.with(|slot| {
        let slot = slot.borrow();
        if let Some(provider) = slot.as_ref() {
            if let Err(err) = provider.load_from_data(styles_css(theme).as_bytes()) {
                eprintln!("chefbar: css herladen mislukt ({err})");
            }
        }
    });
}

/// Bouwt de volledige stylesheet voor het gekozen thema.
pub fn styles_css(theme: &str) -> String {
    let t = Tokens::for_theme(theme);
    format!(
        r#"
/* ============ GTK3-thema reset (hele widget-set) ============ */
/* Standaard GTK-thema's zetten gradients, text-shadows en elevation op
   bijna elk control. v2 is mat + hairline: overal uit, daarna onze klassen. */
window, .background, .titlebar, headerbar, dialog, messagedialog,
button, entry, combobox, combobox > box, combobox button, combobox button.combo,
combobox arrow, spinbutton, spinbutton entry, textview, textview > text,
list, list row, treeview, treeview.view, iconview, flowbox, flowboxchild,
notebook, notebook > header, notebook > header tabs, notebook > header tab,
frame, frame > border, viewport, scrolledwindow, scrollbar, scrollbar trough,
scale, scale trough, progressbar, progressbar trough, switch, checkbutton,
radiobutton, calendar, menu, .menu, menuitem, popover, .popover, tooltip,
.search-bar, actionbar, infobar, separator, .separator {{
  background-image: none;
  box-shadow: none;
  text-shadow: none;
  -gtk-icon-shadow: none;
}}
overshoot, undershoot, junction {{
  background-image: none;
  background-color: transparent;
  border: none;
  box-shadow: none;
}}
window, .background {{
  background-color: {canvas};
  color: {text};
}}
*:disabled {{
  opacity: 0.38;
}}
selection, entry selection, textview selection {{
  background-color: {accent_soft};
  color: {text};
}}

/* ============ Primitives: button / entry / combo / menu ============ */
button {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  font-family: "General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif;
  font-size: 13px;
  font-weight: 500;
  min-height: 32px;
  padding: 6px 13px;
  outline: none;
  box-shadow: none;
  text-shadow: none;
  -gtk-icon-shadow: none;
  transition: background-color 140ms, border-color 140ms, color 140ms;
}}
button:hover {{
  background-color: {sunk};
}}
button:active {{
  background-color: {sunk};
}}
button:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
button.flat, button.image-button {{
  background-color: transparent;
  border-color: transparent;
  min-height: 28px;
  min-width: 28px;
  padding: 2px 6px;
}}
button.flat:hover, button.image-button:hover {{
  background-color: {hover};
}}
button.suggested-action {{
  background-color: {text};
  border-color: {text};
  color: {canvas};
}}
button.destructive-action {{
  background-color: {hold_bg};
  border-color: {red};
  color: {red};
}}

entry, spinbutton, spinbutton entry {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  caret-color: {text};
  font-family: "General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif;
  font-size: 13px;
  min-height: 28px;
  padding: 4px 10px;
  box-shadow: none;
  outline: none;
}}
entry:focus, spinbutton:focus, spinbutton entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
entry image {{
  color: {text_faint};
  margin-left: 6px;
  margin-right: 4px;
}}
entry progress {{
  background-color: {accent_soft};
  border: none;
  box-shadow: none;
}}

combobox, combobox > box {{
  background-color: transparent;
  border: none;
  box-shadow: none;
}}
combobox button, combobox button.combo {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  font-weight: 400;
  min-height: 28px;
  padding: 2px 10px;
  box-shadow: none;
}}
combobox button:hover, combobox button.combo:hover {{
  background-color: {sunk};
}}
combobox button:focus, combobox button.combo:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
combobox arrow {{
  color: {text_muted};
  min-width: 12px;
}}

textview, textview > text {{
  background-color: {surface};
  color: {text};
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
}}

list, list row {{
  background-color: transparent;
  border: none;
  box-shadow: none;
  color: {text};
}}
list row {{
  border-radius: 6px;
  min-height: 28px;
  padding: 4px 8px;
}}
list row:hover {{
  background-color: {hover};
}}
list row:selected, list row:selected:hover {{
  background-color: {accent_soft};
  color: {text};
}}

treeview, treeview.view, iconview {{
  background-color: {surface};
  color: {text};
  border: 1px solid {line};
  border-radius: 10px;
}}
treeview.view:selected {{
  background-color: {accent_soft};
  color: {text};
}}

notebook, notebook > header, notebook > header tabs {{
  background-color: {canvas};
  border: none;
  box-shadow: none;
}}
notebook > header tab {{
  background-color: transparent;
  border: none;
  border-radius: 6px;
  color: {text_muted};
  font-size: 13px;
  font-weight: 500;
  min-height: 28px;
  padding: 4px 10px;
}}
notebook > header tab:hover {{
  background-color: {hover};
  color: {text};
}}
notebook > header tab:checked {{
  background-color: {surface};
  color: {text};
}}

frame, frame > border {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  box-shadow: none;
}}

switch {{
  background-color: {sunk};
  border: 1px solid {line_strong};
  border-radius: 10px;
  min-width: 36px;
  min-height: 20px;
  box-shadow: none;
}}
switch:checked {{
  background-color: {accent};
  border-color: {accent};
}}
switch slider {{
  background-color: {text_muted};
  border: none;
  border-radius: 10px;
  box-shadow: none;
  min-width: 14px;
  min-height: 14px;
}}
switch:checked slider {{
  background-color: {surface};
}}

checkbutton, radiobutton {{
  background-color: transparent;
  border: none;
  color: {text};
  font-size: 13px;
  min-height: 28px;
  padding: 2px 0;
}}
checkbutton check, radiobutton radio {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  box-shadow: none;
  min-width: 14px;
  min-height: 14px;
}}
radiobutton radio {{
  border-radius: 10px;
}}
checkbutton check:checked, radiobutton radio:checked {{
  background-color: {accent};
  border-color: {accent};
}}

scale, scale trough {{
  background-color: {sunk};
  border: none;
  border-radius: 6px;
  min-height: 4px;
  box-shadow: none;
}}
scale highlight {{
  background-color: {accent};
  border-radius: 6px;
}}
scale slider {{
  background-color: {text};
  border: none;
  border-radius: 10px;
  box-shadow: none;
  min-width: 12px;
  min-height: 12px;
}}

progressbar, progressbar trough {{
  background-color: {sunk};
  border: none;
  border-radius: 6px;
  min-height: 4px;
  box-shadow: none;
}}
progressbar progress {{
  background-color: {accent};
  border: none;
  border-radius: 6px;
}}

menu, .menu, popover, .popover {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 10px;
  box-shadow: none;
  padding: 4px;
  color: {text};
}}
menuitem {{
  background-color: transparent;
  border: none;
  border-radius: 6px;
  color: {text};
  font-size: 13px;
  min-height: 28px;
  padding: 4px 10px;
}}
menuitem:hover, menuitem:hover > label {{
  background-color: {hover};
  color: {text};
}}

separator, .separator {{
  background-color: {line};
  border: none;
  min-height: 1px;
  min-width: 1px;
}}

tooltip, tooltip.background, tooltip * {{
  background-color: {surface};
  background-image: none;
  color: {text};
  border: 1px solid {line_strong};
  border-radius: 6px;
  box-shadow: none;
}}

scrollbar {{
  background-color: transparent;
  border: none;
  box-shadow: none;
}}
scrollbar slider {{
  background-color: {line_strong};
  border: none;
  border-radius: 6px;
  min-width: 6px;
  min-height: 6px;
}}
scrollbar slider:hover {{
  background-color: {text_faint};
}}

/* ============ App-window ============ */
.chefbar-app {{
  background-color: {canvas};
  color: {text};
  font-family: "General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif;
  font-size: 13px;
}}

/* ============ Header ============ */
.chefbar-header {{
  background-color: {canvas};
  padding: 14px 16px 12px 16px;
  border-bottom: 1px solid {line};
}}
.chefbar-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 18px;
  font-weight: 500;
  color: {text};
}}
.chefbar-title-sub {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}

.chefbar-gbtn {{
  background-color: transparent;
  background-image: none;
  border: none;
  border-radius: 6px;
  color: {text_muted};
  min-width: 28px;
  min-height: 28px;
  padding: 2px 6px;
  font-size: 13px;
  box-shadow: none;
  transition: background-color 140ms, color 140ms;
}}
.chefbar-gbtn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-gbtn:active {{
  color: {accent};
}}
.chefbar-gbtn:focus {{
  border: 1px solid {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}

/* Zoek: r-6 control, geen pill. Focus = accent + soft ring. */
.chefbar-search, .chefbar-search entry,
.chefbar-palette-entry, .chefbar-palette-entry entry,
.chefbar-dialog entry, .chefbar-dialog-entry, .chefbar-dialog-entry entry {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  caret-color: {text};
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  min-height: 28px;
  padding: 6px 12px;
  box-shadow: none;
}}
.chefbar-search:focus,
.chefbar-search entry:focus,
.chefbar-palette-entry:focus,
.chefbar-palette-entry entry:focus,
.chefbar-dialog entry:focus,
.chefbar-dialog-entry:focus,
.chefbar-dialog-entry entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
.chefbar-search image,
.chefbar-palette-entry image {{
  color: {text_faint};
}}

/* ============ Worked-row streep + statusregel ============ */
.chefbar-signature {{
  background-color: {line_strong};
  min-width: 2px;
  border-radius: 1px;
}}
.chefbar-signature.ok       {{ background-color: {green}; }}
.chefbar-signature.warn     {{ background-color: {amber}; }}
.chefbar-signature.error    {{ background-color: {red}; }}
.chefbar-signature.info     {{ background-color: {accent_ink}; }}
.chefbar-signature.running  {{ background-color: {accent}; }}
.chefbar-statuslijn {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  padding: 8px 12px 8px 6px;
  margin: 10px 16px 2px 16px;
}}
.chefbar-statuslijn-text {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}

/* ============ Section eyebrows (caps) ============ */
.chefbar-section-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_faint};
  padding: 14px 16px 4px 16px;
}}
.chefbar-section-sub {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 11px;
  font-weight: 400;
  color: {text_muted};
  padding: 0 16px 8px 16px;
}}

/* ============ Lists / grouped cards ============ */
.chefbar-group, .chefbar-list {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  margin: 2px 16px 6px 16px;
}}
.chefbar-row {{
  padding: 8px 2px;
  border-bottom: 1px solid {line};
  margin: 0 12px;
}}
.chefbar-row:last-child {{ border-bottom: none; }}
.chefbar-row:hover {{ background-color: {hover}; }}
.chefbar-group-attention {{
  background-color: {surface};
  border: 1px solid {hold_line};
  border-left: 3px solid {amber};
  border-radius: 10px;
  margin: 2px 16px 6px 16px;
}}
.chefbar-kpi {{
  padding: 0;
}}

.chefbar-card-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}
.chefbar-card-meta {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  color: {text_muted};
}}
.chefbar-empty {{
  padding: 14px 16px;
  margin: 0 12px;
}}
.chefbar-empty-title {{
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}
.chefbar-empty-sub {{
  font-size: 12px;
  color: {text_muted};
  padding-top: 3px;
}}

.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: 999px;
  background-color: {line_strong};
}}
.chefbar-dot.ok    {{ background-color: {green}; }}
.chefbar-dot.warn  {{ background-color: {amber}; }}
.chefbar-dot.down  {{ background-color: {red}; }}
.chefbar-dot.info  {{ background-color: {accent_ink}; }}
.chefbar-dot.live  {{ background-color: {accent}; }}

.chefbar-bar-track {{
  min-height: 4px;
  border-radius: 3px;
  background-color: {sunk};
}}
.chefbar-bar-fill {{
  border-radius: 3px;
  background-color: {accent};
}}
.chefbar-bar-fill.ok    {{ background-color: {green}; }}
.chefbar-bar-fill.warn  {{ background-color: {amber}; }}
.chefbar-bar-fill.down  {{ background-color: {red}; }}

/* ============ Knoppen (v2 .btn: hairline, r-6, primary = inverse) ============ */
.chefbar-btn {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  padding: 6px 13px;
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  font-weight: 500;
  min-height: 32px;
  box-shadow: none;
  text-shadow: none;
  transition: background-color 140ms, border-color 140ms;
}}
.chefbar-btn:hover {{
  background-color: {sunk};
}}
.chefbar-btn:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
.chefbar-btn.chefbar-primary {{
  background-color: {text};
  border-color: {text};
  color: {canvas};
}}
.chefbar-btn.chefbar-primary:hover {{
  background-color: {text};
  opacity: 0.87;
}}
.chefbar-btn.chefbar-danger {{
  border-color: {red};
  color: {red};
  background-color: {hold_bg};
}}

.chefbar-kbd {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
  border: 1px solid {line_strong};
  border-radius: 3px;
  padding: 1px 5px;
  background-color: {sunk};
}}

/* Stamps: r-6 outline, geen pill. */
.chefbar-stamp {{
  border-radius: 6px;
  padding: 2px 7px;
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.4px;
  background-color: transparent;
  border: 1px solid {line_strong};
  color: {text_muted};
}}
.chefbar-stamp.ok    {{ border-color: {green}; color: {green}; background-color: {green_bg}; }}
.chefbar-stamp.warn  {{ border-color: {amber}; color: {amber}; background-color: {hold_bg}; }}
.chefbar-stamp.error {{ border-color: {red}; color: {red}; background-color: {hold_bg}; }}
.chefbar-stamp.info {{
  border-color: {accent};
  color: {accent_ink};
  background-color: {accent_soft};
}}

.chefbar-row-btn {{
  background-color: transparent;
  background-image: none;
  border: none;
  border-bottom: 1px solid {line};
  border-radius: 0;
  min-height: 0;
  padding: 0;
  box-shadow: none;
  text-shadow: none;
  transition: background-color 140ms;
}}
.chefbar-group .chefbar-row-btn:last-child,
.chefbar-list .chefbar-row-btn:last-child,
.chefbar-group-attention .chefbar-row-btn:last-child {{
  border-bottom: none;
}}
.chefbar-row-btn:hover {{
  background-color: {hover};
}}
.chefbar-row-btn:focus {{
  border-left: 2px solid {accent};
  box-shadow: inset 0 0 0 1px {accent_soft};
}}

/* ============ Sidebar ============ */
.chefbar-sidebar {{
  background-color: {canvas};
  border-right: 1px solid {line};
}}
.chefbar-sidebar-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 14px;
  font-weight: 500;
  color: {text};
}}
.chefbar-sidebar-sub {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-nav-item {{
  background-color: transparent;
  background-image: none;
  border: 1px solid transparent;
  border-left: 2px solid transparent;
  border-radius: 6px;
  color: {text_muted};
  font-size: 13px;
  font-weight: 500;
  padding: 6px 10px;
  min-height: 28px;
  box-shadow: none;
  text-shadow: none;
  transition: background-color 140ms, color 140ms;
}}
.chefbar-nav-item:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-nav-item:focus {{
  border-color: {accent};
}}
.chefbar-nav-item.active {{
  background-color: {accent_soft};
  border: 1px solid transparent;
  border-left: 2px solid {accent};
  color: {text};
}}
.chefbar-nav-item.active:hover {{
  background-color: {accent_soft};
}}
.chefbar-nav-sep {{
  background-color: {line};
  min-height: 1px;
  margin: 4px 10px;
}}
.chefbar-sidebar-group-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_faint};
  padding: 6px 12px 2px 12px;
}}
.chefbar-sidebar-footer {{
  border-top: 1px solid {line};
  padding-top: 10px;
}}
.chefbar-sidebar-footer-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_faint};
}}
.chefbar-sidebar-footer-meta {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  color: {text_muted};
}}
.chefbar-main {{
  background-color: {canvas};
}}

/* ============ Footer ============ */
.chefbar-footer {{
  background-color: {canvas};
  border-top: 1px solid {line};
  padding: 8px 16px;
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-footer-label {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-footer-btn {{
  background-color: transparent;
  background-image: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: {text_muted};
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 11px;
  font-weight: 500;
  min-height: 28px;
  padding: 2px 9px;
  box-shadow: none;
  text-shadow: none;
  transition: background-color 140ms, color 140ms, border-color 140ms;
}}
.chefbar-footer-btn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-footer-btn:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
.chefbar-footer-btn.on {{
  color: {accent_ink};
  border-color: {line};
  background-color: {accent_soft};
}}

/* ============ Drawer ============ */
.chefbar-drawer {{
  min-width: 300px;
  background-color: {canvas};
  border-left: 1px solid {line};
}}
.chefbar-drawer-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 15px;
  font-weight: 500;
  color: {text};
}}
.chefbar-drawer-actions {{
  padding-top: 8px;
}}
.chefbar-drawer-hint {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
  padding: 4px 12px 12px 12px;
}}

/* ============ Overlay / palette ============ */
.chefbar-overlay,
.chefbar-palette-overlay {{
  min-width: 560px;
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 10px;
  padding: 10px;
}}
.chefbar-palette-entry {{
  min-height: 36px;
}}
.chefbar-palette-results {{
  padding-top: 6px;
}}
.chefbar-palette-section {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_faint};
  padding: 6px 8px 4px 8px;
}}
.chefbar-palette-row {{
  background-color: transparent;
  background-image: none;
  border: none;
  border-left: 2px solid transparent;
  border-radius: 6px;
  padding: 6px 8px;
  min-height: 0;
  box-shadow: none;
  text-shadow: none;
  transition: background-color 140ms;
}}
.chefbar-palette-row:hover {{
  background-color: {hover};
}}
.chefbar-palette-row:focus {{
  box-shadow: inset 0 0 0 1px {accent};
}}
.chefbar-palette-row.selected {{
  border-left: 2px solid {accent};
  background-color: {accent_soft};
}}
.chefbar-overlay-foot {{
  border-top: 1px solid {line};
  padding: 8px 4px 2px 4px;
}}
.chefbar-overlay-foot-label {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 11px;
  color: {text_muted};
}}

/* ============ Zone header + card grid ============ */
.chefbar-zone-header {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  color: {text_faint};
  padding: 10px 12px 6px 12px;
}}
.chefbar-card-grid {{
  padding: 8px 12px;
}}

/* ============ Dialog (needs_text) ============ */
.chefbar-dialog, .chefbar-dialog-window {{
  background-color: {canvas};
  border: 1px solid {line_strong};
  border-radius: 10px;
}}
.chefbar-dialog-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 15px;
  font-weight: 500;
  color: {text};
}}
.chefbar-dialog-hint {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}

/* ============ Density ============ */
.chefbar-app.density-compact {{
  font-size: 12px;
}}
.chefbar-app.density-compact .chefbar-header {{
  padding: 8px 12px 8px 12px;
}}
.chefbar-app.density-compact .chefbar-title {{
  font-size: 16px;
}}
.chefbar-app.density-compact .chefbar-statuslijn {{
  padding: 6px 10px;
  margin: 6px 16px 1px 16px;
}}
.chefbar-app.density-compact .chefbar-section-title {{
  padding: 10px 16px 3px 16px;
}}
.chefbar-app.density-compact .chefbar-section-sub {{
  padding-bottom: 4px;
}}
.chefbar-app.density-compact .chefbar-row {{
  padding: 5px 2px;
  margin: 0 12px;
}}
.chefbar-app.density-compact .chefbar-group,
.chefbar-app.density-compact .chefbar-list,
.chefbar-app.density-compact .chefbar-group-attention {{
  margin: 1px 16px 4px 16px;
}}
.chefbar-app.density-compact .chefbar-card-title {{
  font-size: 12px;
}}
.chefbar-app.density-compact .chefbar-card-meta {{
  font-size: 10px;
}}
.chefbar-app.density-compact .chefbar-empty {{
  padding: 8px 16px;
}}
.chefbar-app.density-compact .chefbar-nav-item {{
  padding: 4px 10px;
  min-height: 24px;
}}
.chefbar-app.density-compact .chefbar-stamp {{
  font-size: 10px;
  padding: 1px 6px;
}}
.chefbar-app.density-compact .chefbar-search,
.chefbar-app.density-compact .chefbar-search entry {{
  padding: 4px 12px;
  font-size: 12px;
}}
.chefbar-app.density-compact .chefbar-footer {{
  padding: 4px 16px;
}}
.chefbar-app.density-compact .chefbar-footer-btn {{
  min-height: 24px;
}}

/* ============ Control-chat ============ */
.chefbar-chat {{
  background-color: {canvas};
}}
.chefbar-chat-log {{
  padding-top: 4px;
}}
.chefbar-chat-msg {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  padding: 8px 12px;
}}
.chefbar-chat-msg.operator {{
  border-color: {accent};
}}
.chefbar-chat-msg.system {{
  background-color: {sunk};
}}
.chefbar-chat-who {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-chat-body {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  color: {text};
}}
.chefbar-chat-composer {{
  padding-top: 4px;
}}
.chefbar-chat-combo {{
  font-family: "IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  min-height: 28px;
  border-radius: 6px;
}}
.chefbar-chat-entry, .chefbar-chat-entry entry {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 10px;
  color: {text};
  min-height: 36px;
  padding: 6px 12px;
}}
.chefbar-chat-entry:focus,
.chefbar-chat-entry entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
"#,
        canvas = t.canvas,
        surface = t.surface,
        sunk = t.sunk,
        hover = t.hover,
        line = t.line,
        line_strong = t.line_strong,
        text = t.text,
        text_muted = t.text_muted,
        text_faint = t.text_faint,
        accent = t.accent,
        accent_ink = t.accent_ink,
        accent_soft = t.accent_soft,
        green = t.green,
        green_bg = t.green_bg,
        red = t.red,
        amber = t.amber,
        hold_bg = t.hold_bg,
        hold_line = t.hold_line,
    )
}

/// v2-tokenwaarden per thema (`tokens.css`, skin `devin`).
struct Tokens {
    canvas: &'static str,
    surface: &'static str,
    sunk: &'static str,
    hover: &'static str,
    line: &'static str,
    line_strong: &'static str,
    text: &'static str,
    text_muted: &'static str,
    text_faint: &'static str,
    accent: &'static str,
    accent_ink: &'static str,
    accent_soft: &'static str,
    green: &'static str,
    green_bg: &'static str,
    red: &'static str,
    amber: &'static str,
    hold_bg: &'static str,
    hold_line: &'static str,
}

impl Tokens {
    fn for_theme(theme: &str) -> Self {
        if theme == THEME_LIGHT {
            Self::light()
        } else {
            Self::dark()
        }
    }

    fn dark() -> Self {
        Self {
            canvas: "#121111",
            surface: "#1B1A19",
            sunk: "#242322",
            hover: "rgba(255,255,255,0.05)",
            line: "rgba(255,255,255,0.09)",
            line_strong: "rgba(255,255,255,0.16)",
            text: "#F0EEEB",
            text_muted: "rgba(240,238,235,0.55)",
            text_faint: "rgba(240,238,235,0.35)",
            accent: "#5C97FF",
            accent_ink: "#8AB4FF",
            accent_soft: "rgba(92,151,255,0.12)",
            green: "#3FB950",
            green_bg: "rgba(63,185,80,0.12)",
            red: "#F85149",
            amber: "#D9A038",
            hold_bg: "rgba(217,160,56,0.08)",
            hold_line: "rgba(217,160,56,0.40)",
        }
    }

    fn light() -> Self {
        Self {
            canvas: "#F7F6F5",
            surface: "#FFFFFF",
            sunk: "#EFEFEF",
            hover: "rgba(0,0,0,0.045)",
            line: "rgba(0,0,0,0.08)",
            line_strong: "rgba(0,0,0,0.14)",
            text: "#191919",
            text_muted: "rgba(0,0,0,0.55)",
            text_faint: "rgba(0,0,0,0.38)",
            accent: "#317CFF",
            accent_ink: "#1D5FD6",
            accent_soft: "rgba(49,124,255,0.09)",
            green: "#1F883D",
            green_bg: "rgba(31,136,61,0.10)",
            red: "#CF222E",
            amber: "#BF5B00",
            hold_bg: "rgba(191,91,0,0.06)",
            hold_line: "rgba(191,91,0,0.35)",
        }
    }
}

/// Welk thema actief is. Signaal v2 is light-first: het warme off-white canvas
/// (#F7F6F5) is de standaard, donker basalt-warm (#121111) volgt het systeem en
/// houdt volledige pariteit.
///
/// Volgorde: `CHEFBAR_THEME=light|dark` wint altijd; daarna de GTK-dark-pref;
/// daarna de themanaam. Die tweede aanwijzing is er omdat GNOME
/// `gtk-application-prefer-dark-theme` op GTK3 niet altijd doorzet, terwijl het
/// GTK-thema dan wel op een `-dark`-variant staat.
///
/// Het opstartthema van de app zelf komt uit de persisted panel-state; die
/// default hoort bij `panel_state` en valt buiten deze CSS-laag.
pub fn detect_theme(settings: &gtk::Settings) -> String {
    if let Ok(force) = std::env::var("CHEFBAR_THEME") {
        match force.trim().to_ascii_lowercase().as_str() {
            THEME_LIGHT => return THEME_LIGHT.into(),
            THEME_DARK => return THEME_DARK.into(),
            _ => {}
        }
    }
    if settings.is_gtk_application_prefer_dark_theme() {
        return THEME_DARK.into();
    }
    let theme_name_is_dark = settings
        .gtk_theme_name()
        .is_some_and(|name| name.to_lowercase().ends_with("-dark"));
    if theme_name_is_dark {
        return THEME_DARK.into();
    }
    THEME_LIGHT.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheets() -> [String; 2] {
        [styles_css(THEME_LIGHT), styles_css(THEME_DARK)]
    }

    #[test]
    fn light_heeft_devin_canvas_en_accent() {
        let css = styles_css(THEME_LIGHT);
        assert!(css.contains("#F7F6F5"));
        assert!(css.contains("#FFFFFF"));
        assert!(css.contains("#191919"));
        assert!(css.contains("#317CFF"));
        assert!(css.contains("#1D5FD6"));
        assert!(css.contains("#1F883D"));
        assert!(css.contains("#BF5B00"));
        assert!(css.contains("#CF222E"));
    }

    #[test]
    fn dark_heeft_basalt_en_accent() {
        let css = styles_css(THEME_DARK);
        assert!(css.contains("#121111"));
        assert!(css.contains("#1B1A19"));
        assert!(css.contains("#F0EEEB"));
        assert!(css.contains("#5C97FF"));
        assert!(css.contains("#8AB4FF"));
        assert!(css.contains("#3FB950"));
        assert!(css.contains("#D9A038"));
        assert!(css.contains("#F85149"));
    }

    #[test]
    fn radius_is_6_en_10() {
        for css in sheets() {
            assert!(css.contains("border-radius: 6px"));
            assert!(css.contains("border-radius: 10px"));
            assert!(
                !css.contains("border-radius: 12px"),
                "cards blijven r-10, geen r-12"
            );
            assert!(
                !css.contains("border-radius: 14px"),
                "geen Huly-dialog radius"
            );
        }
    }

    #[test]
    fn stamps_zijn_geen_pillen() {
        let css = styles_css(THEME_LIGHT);
        let stamp = css.split(".chefbar-stamp {").nth(1).expect("stamp-blok");
        let block = stamp.split('}').next().expect("stamp body");
        assert!(
            block.contains("border-radius: 6px"),
            "stamp moet r-6 zijn, kreeg {block}"
        );
        assert!(
            !block.contains("border-radius: 200px") && !block.contains("999px"),
            "stamp mag geen pill zijn"
        );
    }

    fn has_prop(css: &str, name: &str) -> bool {
        css.contains(&format!("{name}:"))
    }

    #[test]
    fn gtk3_subset_geen_verboden_properties() {
        for css in sheets() {
            assert!(!has_prop(&css, "gap"), "GTK3 weigert flex-gap");
            assert!(!has_prop(&css, "inset"), "GTK3 weigert inset-shorthand");
            assert!(!css.contains("grid-gap"));
            assert!(!css.contains("place-items"));
            for line in css.lines() {
                let trimmed = line.trim();
                assert!(
                    !trimmed.starts_with("--"),
                    "custom property in output: {trimmed}"
                );
            }
        }
    }

    #[test]
    fn geen_huly_tokens_of_adwaita_in_sheet() {
        for css in sheets() {
            for banned in [
                "#090A0C", "#5683DA", "#FF8964", "Archivo", "Inter,", "Adwaita",
            ] {
                assert!(
                    !css.contains(banned),
                    "verboden restant {banned} in stylesheet"
                );
            }
        }
    }

    #[test]
    fn widget_set_dekt_overlay_dialog_list_search_footer() {
        let css = styles_css(THEME_LIGHT);
        for needle in [
            ".chefbar-overlay",
            ".chefbar-dialog",
            ".chefbar-group",
            ".chefbar-search",
            ".chefbar-footer",
            ".chefbar-palette-row",
            ".chefbar-list",
            "combobox button",
            "menuitem",
            "list row",
        ] {
            assert!(css.contains(needle), "widget-set mist {needle}");
        }
    }
}
