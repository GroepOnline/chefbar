//! Signaal v2 (Devin-richting) CSS voor ChefApp — tokens uit
//! `GroepOnline/design-system` (`DESIGN.md` v2 + `tokens.css`, skin `devin`),
//! gemapt op de GTK3-CSS subset. Joep, 2026-08-18: ChefBar/tray/ChefApp
//! volgen deze design-system, geen Huly en geen tweede dialect.
//!
//! Light-first warm off-white canvas, donker basalt-warm met volledige
//! pariteit, één accent, General Sans interface + IBM Plex Mono data,
//! radius 6 / 10 / 200. Scheiding via hairlines. De 2px verticale streep
//! is de v2 worked-row-streep (line-strong in rust, accent tijdens een run).
//! Groen = git/PR/toestemming; amber = wacht-op-jou; rood = fout/destructive.
//!
//! GTK3-subset: geen custom properties, geen `gap`, geen `inset`, geen
//! gradients, geen glow. Tokens worden in Rust geïnterpoleerd.

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

/// Het nu actieve thema (voor footer-toggle). Valt terug op licht: v2 is light-first.
pub fn active_theme() -> String {
    ACTIVE
        .get_or_init(|| Mutex::new(THEME_LIGHT.to_string()))
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| THEME_LIGHT.to_string())
}

/// Solid ink for Lucide pixbufs (GTK3 SVG has no currentColor inheritance).
pub fn ink_hex() -> &'static str {
    if active_theme() == THEME_DARK {
        "#F0EEEB"
    } else {
        "#191919"
    }
}

/// Contrasting canvas color for rasterized icons on solid ink controls.
pub fn canvas_hex() -> &'static str {
    if active_theme() == THEME_DARK {
        "#121111"
    } else {
        "#F7F6F5"
    }
}

pub fn muted_hex() -> &'static str {
    if active_theme() == THEME_DARK {
        "#8A8886"
    } else {
        "#707070"
    }
}

pub fn accent_hex() -> &'static str {
    if active_theme() == THEME_DARK {
        "#5C97FF"
    } else {
        "#317CFF"
    }
}

/// Koppen in product-UI: tracking −0.02em. 18px ≈ 13.5pt → 276 pango-units.
pub fn heading_attrs() -> pango::AttrList {
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrInt::new_letter_spacing(-276));
    attrs
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
    let t = if theme == THEME_LIGHT {
        Tokens::light()
    } else {
        Tokens::dark()
    };
    t.stylesheet()
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
    green_soft: &'static str,
    red: &'static str,
    red_soft: &'static str,
    amber: &'static str,
    amber_soft: &'static str,
    font_ui: &'static str,
    font_mono: &'static str,
    t_2xs: &'static str,
    t_xs: &'static str,
    t_sm: &'static str,
    t_md: &'static str,
    t_lg: &'static str,
    t_xl: &'static str,
    r_md: &'static str,
    r_lg: &'static str,
    r_pill: &'static str,
    r_micro: &'static str,
    dur: &'static str,
}

impl Tokens {
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
            green_soft: "rgba(63,185,80,0.12)",
            red: "#F85149",
            red_soft: "rgba(248,81,73,0.12)",
            amber: "#D9A038",
            amber_soft: "rgba(217,160,56,0.08)",
            font_ui: r#""General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif"#,
            font_mono: r#""IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace"#,
            t_2xs: "10.5px",
            t_xs: "11.5px",
            t_sm: "12.5px",
            t_md: "13.5px",
            t_lg: "15px",
            t_xl: "18px",
            r_md: "6px",
            r_lg: "10px",
            r_pill: "200px",
            r_micro: "3px",
            dur: "140ms",
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
            green_soft: "rgba(31,136,61,0.10)",
            red: "#CF222E",
            red_soft: "rgba(207,34,46,0.10)",
            amber: "#BF5B00",
            amber_soft: "rgba(191,91,0,0.06)",
            font_ui: r#""General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif"#,
            font_mono: r#""IBM Plex Mono", "JetBrains Mono", ui-monospace, monospace"#,
            t_2xs: "10.5px",
            t_xs: "11.5px",
            t_sm: "12.5px",
            t_md: "13.5px",
            t_lg: "15px",
            t_xl: "18px",
            r_md: "6px",
            r_lg: "10px",
            r_pill: "200px",
            r_micro: "3px",
            dur: "140ms",
        }
    }

    fn stylesheet(&self) -> String {
        format!(
            r#"
/* ============ Kill Adwaita chrome ============ */
* {{
  outline-color: transparent;
  -gtk-icon-shadow: none;
}}
window, .chefbar-app {{
  background-color: {canvas};
  color: {text};
  font-family: {font_ui};
  font-size: {t_md};
}}
button, entry, searchentry, combobox, combobox button, combobox button.combo,
combobox arrow, combobox cellview, combobox header, headerbar, notebook,
scrollbar, scrollbar contents, scrollbar trough, menu, menuitem, tooltip,
.combo, spinbutton, checkbutton, radiobutton {{
  background-image: none;
  box-shadow: none;
  text-shadow: none;
  -gtk-icon-shadow: none;
}}
undershoot.top, undershoot.right, undershoot.bottom, undershoot.left,
overshoot.top, overshoot.right, overshoot.bottom, overshoot.left {{
  background-image: none;
  background-color: transparent;
}}
scrolledwindow, viewport {{
  background-color: transparent;
  border: none;
}}
separator {{
  background-color: {line};
  min-height: 1px;
  min-width: 1px;
}}
label {{
  color: {text};
}}
button:disabled, entry:disabled, combobox:disabled, .chefbar-btn:disabled,
.chefbar-gbtn:disabled, .chefbar-nav-item:disabled {{
  opacity: 0.38;
}}
entry selection, label selection {{
  background-color: {accent_soft};
  color: {text};
}}

/* ============ Header — custom titlebar ============ */
.chefbar-header {{
  background-color: {canvas};
  padding: 12px 16px 10px 16px;
  border-bottom: 1px solid {line};
}}
.chefbar-title {{
  font-family: {font_ui};
  font-size: {t_xl};
  font-weight: 500;
  letter-spacing: -0.02em;
  color: {text};
}}
.chefbar-title-sub {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_muted};
}}
.chefbar-gbtn {{
  background-color: transparent;
  background-image: none;
  border: none;
  border-radius: {r_md};
  color: {text_muted};
  min-width: 28px;
  min-height: 28px;
  padding: 2px 6px;
  font-size: {t_md};
  transition: background-color {dur}, color {dur};
}}
.chefbar-gbtn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-gbtn:active {{
  color: {accent};
}}
.chefbar-gbtn:focus {{
  box-shadow: 0 0 0 2px {accent};
}}

/* Zoek-input: control r-6, focus = accent + soft ring. Pillen alleen badges. */
.chefbar-search, .chefbar-search entry,
.chefbar-palette-entry, .chefbar-palette-entry entry,
.chefbar-dialog entry {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_md};
  color: {text};
  font-family: {font_ui};
  font-size: {t_md};
  min-height: 28px;
  padding: 6px 10px;
}}
.chefbar-search:focus,
.chefbar-search entry:focus,
.chefbar-palette-entry:focus,
.chefbar-palette-entry entry:focus,
.chefbar-dialog entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
.chefbar-search placeholder,
.chefbar-search entry placeholder,
.chefbar-palette-entry placeholder,
.chefbar-palette-entry entry placeholder,
.chefbar-dialog entry placeholder,
.chefbar-chat-entry placeholder,
.chefbar-chat-entry entry placeholder {{
  color: {text_faint};
}}

/* ============ Worked-row streep + statusregel ============ */
.chefbar-signature {{
  background-color: {line_strong};
  min-width: 2px;
  min-height: 18px;
  border-radius: 1px;
}}
.chefbar-signature.ok       {{ background-color: {green}; }}
.chefbar-signature.warn     {{ background-color: {amber}; }}
.chefbar-signature.error    {{ background-color: {red}; }}
.chefbar-signature.info     {{ background-color: {accent_ink}; }}
.chefbar-signature.running  {{ background-color: {accent}; }}
.chefbar-statuslijn {{
  background-color: {canvas};
  border-bottom: 1px solid {line};
  padding: 10px 16px 10px 16px;
  margin: 0;
}}
.chefbar-statuslijn-text {{
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  color: {text};
}}

/* ============ Section eyebrows (.caps) ============ */
.chefbar-section-title {{
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  color: {text_faint};
  padding: 14px 16px 4px 16px;
}}
.chefbar-section-sub {{
  font-family: {font_ui};
  font-size: {t_xs};
  font-weight: 400;
  color: {text_muted};
  padding: 0 16px 8px 16px;
}}

/* ============ Zones / grouped cards ============ */
.chefbar-zone {{
  background-color: transparent;
}}
.chefbar-zone-header {{
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  color: {text_faint};
  padding: 10px 12px 4px 12px;
}}
.chefbar-card-grid {{
  padding: 8px 12px;
}}
.chefbar-group {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: {r_lg};
  margin: 2px 16px 8px 16px;
}}
.chefbar-row {{
  padding: 8px 2px;
  border-bottom: 1px solid {line};
  margin: 0 14px;
}}
.chefbar-row:last-child {{ border-bottom: none; }}
.chefbar-row:hover {{ background-color: {hover}; }}
.chefbar-group-attention {{
  background-color: {surface};
  border: 1px solid {line};
  border-left: 3px solid {amber};
  border-radius: {r_lg};
  margin: 2px 16px 8px 16px;
}}
.chefbar-card-title {{
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  color: {text};
}}
.chefbar-card-meta {{
  font-family: {font_mono};
  font-size: {t_xs};
  color: {text_muted};
}}
.chefbar-empty {{
  padding: 16px 16px;
  margin: 0 12px;
}}
.chefbar-empty-title {{
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  color: {text};
}}
.chefbar-empty-sub {{
  font-family: {font_ui};
  font-size: {t_sm};
  color: {text_muted};
  padding-top: 3px;
}}
.chefbar-kpi {{
  background-color: transparent;
}}
.chefbar-kpi .chefbar-card-title {{
  font-family: {font_mono};
  font-size: {t_lg};
  font-weight: 500;
}}

.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: {r_pill};
  background-color: {line_strong};
}}
.chefbar-dot.ok    {{ background-color: {green}; }}
.chefbar-dot.warn  {{ background-color: {amber}; }}
.chefbar-dot.down  {{ background-color: {red}; }}
.chefbar-dot.info  {{ background-color: {accent_ink}; }}
.chefbar-dot.live  {{ background-color: {accent}; }}

.chefbar-bar-track {{
  min-height: 4px;
  border-radius: {r_micro};
  background-color: {sunk};
}}
.chefbar-bar-fill {{
  border-radius: {r_micro};
  background-color: {accent};
}}
.chefbar-bar-fill.ok    {{ background-color: {green}; }}
.chefbar-bar-fill.warn  {{ background-color: {amber}; }}
.chefbar-bar-fill.down  {{ background-color: {red}; }}

/* ============ Knoppen (.btn) ============ */
.chefbar-btn {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_md};
  color: {text};
  padding: 0 13px;
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  min-height: 32px;
  transition: background-color {dur}, border-color {dur};
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
  background-color: {red_soft};
}}
.chefbar-kbd {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_muted};
  border: 1px solid {line_strong};
  border-radius: {r_micro};
  padding: 1px 5px;
}}

/* ============ Stamps (badge-pill) ============ */
.chefbar-stamp {{
  border-radius: {r_pill};
  padding: 2.5px 10px;
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  background-color: {sunk};
  color: {text_muted};
}}
.chefbar-stamp.ok    {{ background-color: {green_soft}; color: {green}; }}
.chefbar-stamp.warn  {{ background-color: {amber_soft}; color: {amber}; }}
.chefbar-stamp.error {{ background-color: {red_soft};    color: {red}; }}
.chefbar-stamp.info  {{ background-color: {accent_soft}; color: {accent_ink}; }}

.chefbar-row-btn {{
  background-color: transparent;
  background-image: none;
  border: 1px solid transparent;
  border-bottom: 1px solid {line};
  border-radius: 0;
  min-height: 0;
  box-shadow: none;
  transition: background-color {dur}, border-color {dur};
}}
.chefbar-group .chefbar-row-btn:last-child,
.chefbar-group-attention .chefbar-row-btn:last-child {{
  border-bottom: none;
}}
.chefbar-row-btn:hover {{
  background-color: {hover};
  border-color: {line_strong};
}}
.chefbar-row-btn:focus {{
  border-left: 2px solid {accent};
  box-shadow: inset 0 0 0 1px {accent_soft};
}}

/* ============ Sidebar + nav ============ */
.chefbar-sidebar {{
  background-color: {sunk};
  border-right: 1px solid {line};
}}
.chefbar-sidebar-title {{
  font-family: {font_ui};
  font-size: {t_lg};
  font-weight: 500;
  letter-spacing: -0.02em;
  color: {text};
}}
.chefbar-sidebar-sub {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_muted};
}}
.chefbar-nav {{
  background-color: transparent;
}}
.chefbar-nav-item {{
  background-color: transparent;
  background-image: none;
  border: 1px solid transparent;
  border-left: 2px solid transparent;
  border-radius: {r_md};
  color: {text_muted};
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  padding: 5px 10px 5px 8px;
  min-height: 28px;
  box-shadow: none;
  transition: background-color {dur}, color {dur};
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
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  color: {text_faint};
  padding: 6px 12px 2px 12px;
}}
.chefbar-sidebar-footer {{
  border-top: 1px solid {line};
  padding-top: 10px;
}}
.chefbar-sidebar-footer-title {{
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  color: {text_faint};
}}
.chefbar-sidebar-footer-meta {{
  font-family: {font_ui};
  font-size: {t_xs};
  color: {text_muted};
}}
.chefbar-main {{
  background-color: {canvas};
}}

/* ============ Footer / modeline ============ */
.chefbar-footer {{
  background-color: {canvas};
  border-top: 1px solid {line};
  padding: 6px 16px;
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_muted};
}}
.chefbar-footer-label {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_muted};
}}
.chefbar-footer-btn {{
  background-color: transparent;
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_md};
  color: {text_muted};
  font-family: {font_ui};
  font-size: {t_xs};
  font-weight: 500;
  min-height: 28px;
  padding: 2px 9px;
  box-shadow: none;
  transition: background-color {dur}, color {dur};
}}
.chefbar-footer-btn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-footer-btn.on {{
  color: {accent};
  border-color: {accent_soft};
  background-color: {accent_soft};
}}
.chefbar-footer-btn:focus {{
  box-shadow: 0 0 0 2px {accent};
}}

/* ============ Drawer ============ */
.chefbar-drawer {{
  min-width: 300px;
  background-color: {canvas};
  border-left: 1px solid {line};
}}
.chefbar-drawer-title {{
  font-family: {font_ui};
  font-size: {t_lg};
  font-weight: 500;
  letter-spacing: -0.02em;
  color: {text};
}}
.chefbar-drawer-actions {{
  padding-top: 8px;
}}
.chefbar-drawer-hint {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_faint};
  padding: 4px 12px 12px 12px;
}}

/* ============ Overlay / palette ============ */
.chefbar-overlay,
.chefbar-palette-overlay {{
  min-width: 560px;
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: {r_lg};
  padding: 12px;
}}
.chefbar-palette-entry {{
  min-height: 36px;
}}
.chefbar-palette-results {{
  padding-top: 6px;
}}
.chefbar-palette-section {{
  font-family: {font_ui};
  font-size: {t_2xs};
  font-weight: 600;
  letter-spacing: 0.07em;
  color: {text_faint};
  padding: 6px 8px 4px 8px;
}}
.chefbar-palette-row {{
  background-color: transparent;
  background-image: none;
  border: none;
  border-left: 2px solid transparent;
  border-radius: {r_md};
  padding: 6px 8px;
  box-shadow: none;
  transition: background-color {dur};
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

/* ============ Dialog ============ */
.chefbar-dialog {{
  background-color: {canvas};
  border: 1px solid {line_strong};
  border-radius: {r_lg};
}}

/* ============ Control-chat ============ */
.chefbar-chat {{
  background-color: {canvas};
}}
.chefbar-chat-log {{
  padding-top: 4px;
}}
.chefbar-chat-msg {{
  background-color: transparent;
  border: none;
  padding: 6px 0;
}}
.chefbar-chat-msg.operator {{
  background-color: {sunk};
  border-radius: {r_lg};
  padding: 8px 12px;
}}
.chefbar-chat-msg.system {{
  background-color: transparent;
}}
.chefbar-chat-msg.agent {{
  background-color: transparent;
}}
.chefbar-chat-who {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_faint};
}}
.chefbar-chat-body {{
  font-family: {font_ui};
  font-size: {t_md};
  color: {text};
}}
.chefbar-chat-composer {{
  padding-top: 4px;
}}
.chefbar-chat-combo,
.chefbar-chat-combo button,
.chefbar-chat-combo button.combo,
combobox, combobox button, combobox button.combo {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_md};
  color: {text};
  font-family: {font_mono};
  font-size: {t_xs};
  min-height: 28px;
  box-shadow: none;
  text-shadow: none;
}}
.chefbar-chat-combo:focus, combobox:focus, combobox button:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}
combobox arrow {{
  color: {text_muted};
}}
.chefbar-chat-entry, .chefbar-chat-entry entry {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_lg};
  color: {text};
  font-family: {font_ui};
  font-size: {t_md};
  min-height: 36px;
  padding: 6px 12px;
}}
.chefbar-chat-entry:focus,
.chefbar-chat-entry entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}

/* ============ Menu / popup (combobox list) ============ */
menu, .menu, window.popup, combobox window.popup {{
  background-color: {surface};
  background-image: none;
  border: 1px solid {line_strong};
  border-radius: {r_lg};
  padding: 4px;
  box-shadow: none;
  color: {text};
}}
menuitem {{
  background-image: none;
  border-radius: {r_md};
  min-height: 28px;
  padding: 4px 10px;
  color: {text};
}}
menuitem:hover, menuitem:hover cellview, menuitem:selected {{
  background-color: {hover};
  color: {text};
}}

/* ============ Scrollbars + tooltips ============ */
scrollbar {{
  background-color: transparent;
  background-image: none;
  border: none;
}}
scrollbar slider {{
  background-color: {line_strong};
  background-image: none;
  border: none;
  border-radius: {r_pill};
  min-width: 6px;
  min-height: 6px;
}}
scrollbar slider:hover {{
  background-color: {text_faint};
}}
tooltip, tooltip.background, tooltip * {{
  background-color: {sunk};
  background-image: none;
  color: {text};
  border: 1px solid {line_strong};
  border-radius: {r_md};
  box-shadow: none;
}}

/* ============ Rail tiles + counts ============ */
.chefbar-nav-row {{
  background-color: transparent;
}}
.chefbar-nav-tile {{
  min-width: 28px;
  min-height: 28px;
  border: 1px solid {line};
  border-radius: 7px;
  background-color: {surface};
}}
.chefbar-nav-item.active .chefbar-nav-tile {{
  border-color: {accent};
}}
.chefbar-nav-name {{
  font-family: {font_ui};
  font-size: {t_md};
  font-weight: 500;
  color: inherit;
}}
.chefbar-nav-count {{
  font-family: {font_mono};
  font-size: {t_2xs};
  color: {text_faint};
}}

/* ============ Palette scrim + glyph ============ */
.chefbar-palette-scrim {{
  background-color: {canvas};
}}
.chefbar-overlay,
.chefbar-palette-overlay {{
  min-width: 560px;
}}
.chefbar-palette-glyph {{
  min-width: 20px;
  min-height: 20px;
}}
.chefbar-gbtn.chefbar-solid {{
  background-color: {text};
  color: {canvas};
}}
.chefbar-gbtn.chefbar-solid:hover {{
  opacity: 0.87;
}}
.chefbar-row-btn:hover {{
  border-color: {line_strong};
}}
.chefbar-empty-cta {{
  margin-top: 8px;
}}

/* ============ Density (padding-token swap) ============ */
.chefbar-app.density-compact {{
  font-size: {t_sm};
}}
.chefbar-app.density-compact .chefbar-header {{
  padding: 8px 12px 8px 12px;
}}
.chefbar-app.density-compact .chefbar-title {{
  font-size: {t_lg};
}}
.chefbar-app.density-compact .chefbar-statuslijn {{
  padding: 6px 12px;
}}
.chefbar-app.density-compact .chefbar-section-title {{
  padding: 8px 16px 2px 16px;
}}
.chefbar-app.density-compact .chefbar-section-sub {{
  padding-bottom: 4px;
}}
.chefbar-app.density-compact .chefbar-row {{
  padding: 5px 2px;
  margin: 0 12px;
}}
.chefbar-app.density-compact .chefbar-group,
.chefbar-app.density-compact .chefbar-group-attention {{
  margin: 1px 16px 4px 16px;
}}
.chefbar-app.density-compact .chefbar-card-title {{
  font-size: {t_sm};
}}
.chefbar-app.density-compact .chefbar-card-meta {{
  font-size: {t_2xs};
}}
.chefbar-app.density-compact .chefbar-empty {{
  padding: 8px 16px;
}}
.chefbar-app.density-compact .chefbar-nav-item {{
  padding: 3px 8px 3px 6px;
  min-height: 24px;
}}
.chefbar-app.density-compact .chefbar-stamp {{
  font-size: {t_2xs};
  padding: 1px 7px;
}}
.chefbar-app.density-compact .chefbar-search,
.chefbar-app.density-compact .chefbar-search entry {{
  padding: 4px 10px;
  min-height: 24px;
  font-size: {t_sm};
}}
.chefbar-app.density-compact .chefbar-footer {{
  padding: 4px 16px;
}}
.chefbar-app.density-compact .chefbar-footer-btn {{
  min-height: 24px;
}}
"#,
            canvas = self.canvas,
            surface = self.surface,
            sunk = self.sunk,
            hover = self.hover,
            line = self.line,
            line_strong = self.line_strong,
            text = self.text,
            text_muted = self.text_muted,
            text_faint = self.text_faint,
            accent = self.accent,
            accent_ink = self.accent_ink,
            accent_soft = self.accent_soft,
            green = self.green,
            green_soft = self.green_soft,
            red = self.red,
            red_soft = self.red_soft,
            amber = self.amber,
            amber_soft = self.amber_soft,
            font_ui = self.font_ui,
            font_mono = self.font_mono,
            t_2xs = self.t_2xs,
            t_xs = self.t_xs,
            t_sm = self.t_sm,
            t_md = self.t_md,
            t_lg = self.t_lg,
            t_xl = self.t_xl,
            r_md = self.r_md,
            r_lg = self.r_lg,
            r_pill = self.r_pill,
            r_micro = self.r_micro,
            dur = self.dur,
        )
    }
}

/// Welk thema actief is. Signaal v2 is light-first: het warme off-white canvas
/// is de standaard, donker basalt-warm volgt het systeem en houdt pariteit.
///
/// Volgorde: `CHEFBAR_THEME=light|dark` wint altijd; daarna de GTK-dark-pref;
/// daarna de themanaam. Het opstartthema van de app zelf komt uit de
/// persisted panel-state.
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

    fn has_prop(css: &str, name: &str) -> bool {
        let needle = format!("{name}:");
        css.split(['{', '}', ';']).any(|chunk| {
            let t = chunk.trim_start();
            t.starts_with(&needle) || t.starts_with(&format!("{name} :"))
        })
    }

    fn illegal_hits(css: &str) -> Vec<&'static str> {
        let mut hits = Vec::new();
        if has_prop(css, "gap") || has_prop(css, "grid-gap") {
            hits.push("gap");
        }
        if has_prop(css, "inset") {
            hits.push("inset");
        }
        let has_custom = css.lines().any(|line| {
            let t = line.trim();
            t.starts_with("--") && t.contains(':')
        });
        if has_custom {
            hits.push("custom-properties");
        }
        if css.contains("linear-gradient") || css.contains("radial-gradient") {
            hits.push("gradient");
        }
        if css.contains("text-transform") {
            hits.push("text-transform");
        }
        if css.contains("font-variant-numeric") {
            hits.push("font-variant-numeric");
        }
        hits
    }

    #[test]
    fn light_skin_matches_devin_tokens() {
        let css = styles_css(THEME_LIGHT);
        assert!(css.contains("#F7F6F5"));
        assert!(css.contains("#191919"));
        assert!(css.contains("#317CFF"));
        assert!(css.contains("#1F883D"));
        assert!(css.contains("#BF5B00"));
        assert!(css.contains("General Sans"));
        assert!(css.contains("IBM Plex Mono"));
        assert!(illegal_hits(&css).is_empty(), "{:?}", illegal_hits(&css));
    }

    #[test]
    fn dark_skin_keeps_accent_for_visual_shot() {
        let css = styles_css(THEME_DARK);
        assert!(css.contains("#121111"));
        assert!(css.contains("#F0EEEB"));
        assert!(css.contains("#5C97FF"));
        assert!(css.contains("#3FB950"));
        assert!(illegal_hits(&css).is_empty(), "{:?}", illegal_hits(&css));
    }

    #[test]
    fn widget_set_is_covered() {
        let css = styles_css(THEME_LIGHT);
        for class in [
            ".chefbar-app",
            ".chefbar-header",
            ".chefbar-title",
            ".chefbar-gbtn",
            ".chefbar-search",
            ".chefbar-signature",
            ".chefbar-statuslijn",
            ".chefbar-section-title",
            ".chefbar-group",
            ".chefbar-row-btn",
            ".chefbar-stamp",
            ".chefbar-sidebar",
            ".chefbar-nav-item",
            ".chefbar-nav-item.active",
            ".chefbar-footer",
            ".chefbar-drawer",
            ".chefbar-overlay",
            ".chefbar-palette-row",
            ".chefbar-dialog",
            ".chefbar-chat-msg",
            ".chefbar-chat-combo",
            ".chefbar-zone",
            ".chefbar-kpi",
            ".chefbar-nav-tile",
            ".chefbar-nav-count",
            ".chefbar-palette-scrim",
            ".chefbar-gbtn.chefbar-solid",
            ".chefbar-empty-cta",
            ".density-compact",
            "combobox",
            "menuitem",
            "scrollbar slider",
            "tooltip",
            "placeholder",
            "button:disabled",
        ] {
            assert!(css.contains(class), "missing selector {class}");
        }
    }

    #[test]
    fn skins_are_not_identical() {
        assert_ne!(styles_css(THEME_LIGHT), styles_css(THEME_DARK));
    }

    fn snapshot_block(css: &str, header: &str) -> String {
        let start = css
            .find(header)
            .unwrap_or_else(|| panic!("snapshot missing {header}"));
        let rest = &css[start + header.len()..];
        let end = rest.find('}').unwrap_or(rest.len());
        rest[..end].to_string()
    }

    fn token_value(block: &str, name: &str) -> String {
        for line in block.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix(&format!("{name}:")) {
                return rest.trim().trim_end_matches(';').to_string();
            }
        }
        panic!("token {name} missing in snapshot block");
    }

    #[test]
    fn gtk_tokens_match_pinned_design_system_snapshot() {
        const SNAP: &str = include_str!("../assets/design-tokens.snapshot.css");
        let light_block = snapshot_block(SNAP, ":root {");
        let dark_block = snapshot_block(SNAP, "[data-theme=\"dark\"] {");
        let light = styles_css(THEME_LIGHT);
        let dark = styles_css(THEME_DARK);
        let check = |css: &str, block: &str, keys: &[&str]| {
            for key in keys {
                let value = token_value(block, key);
                assert!(
                    css.contains(&value),
                    "theme css missing snapshot {key}={value}"
                );
            }
        };
        check(
            &light,
            &light_block,
            &[
                "--bg",
                "--surface",
                "--text",
                "--accent",
                "--accent-ink",
                "--open-green",
                "--red",
                "--amber",
                "--r-md",
                "--r-lg",
                "--text-md",
                "--dur-fast",
            ],
        );
        check(
            &dark,
            &dark_block,
            &[
                "--bg",
                "--surface",
                "--text",
                "--accent",
                "--accent-ink",
                "--open-green",
                "--red",
                "--amber",
            ],
        );
        assert!(light.contains("10.5px"));
        assert!(light.contains("13.5px"));
        assert!(light.contains("General Sans"));
        assert!(light.contains("IBM Plex Mono"));
    }
}
