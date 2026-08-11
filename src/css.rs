//! Signaal · Huly CSS voor ChefBar — tokens uit `.ulpi/design/DESIGN.md`
//! (lock 2026-07-23), gemapt op de GTK3-CSS subset.
//!
//! Huly-meetwaarden: OLED-canvas #090A0C, charcoal surfaces, Electric Iris
//! #5683DA + Ember Pulse #FF8964, dual-font (IBM Plex Mono display/data,
//! Inter interface), radius 8/12/14, pure-black schaduwen, één signature:
//! de verticale CG-statuslijn. Donker is standaard; licht = inset-island
//! met volledige pariteit. Geen serif-display, geen gradients, geen glow.

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

/// Bouwt de volledige stylesheet voor het gekozen thema.
#[allow(clippy::too_many_arguments)]
pub fn styles_css(theme: &str) -> String {
    let t = if theme == THEME_LIGHT { Tokens::light() } else { Tokens::dark() };
    format!(
        r#"
/* ============ App-window (Huly OLED) ============ */
.chefbar-app {{
  background-color: {canvas};
  color: {text};
  font-family: "Inter", "Cantarell", "Noto Sans", sans-serif;
  font-size: 13px;
}}

/* Header — custom titlebar (undecorated window) */
.chefbar-header {{
  background-color: {canvas};
  padding: 14px 16px 12px 16px;
  border-bottom: 1px solid {line};
}}
.chefbar-title {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 19px;
  font-weight: 600;
  color: {text};
}}
.chefbar-title-sub {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_faint};
}}

/* ============ Signature: de CG-statuslijn ============ */
/* 2px verticale lijn in statuskleur + één compacte statusregel. */
.chefbar-signature {{
  background-color: {text_faint};
  min-width: 2px;
  border-radius: 1px;
}}
.chefbar-signature.ok    {{ background-color: {success}; }}
.chefbar-signature.warn  {{ background-color: {warm}; }}
.chefbar-signature.error {{ background-color: {error}; }}
.chefbar-signature.info  {{ background-color: {brand}; }}
.chefbar-statuslijn {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 8px;
  padding: 8px 12px;
}}
.chefbar-statuslijn-text {{
  font-family: "Inter", "Cantarell", sans-serif;
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}


/* Ghost icon-knoppen (header controls) */
.chefbar-gbtn {{
  background-color: transparent;
  border: none;
  border-radius: 8px;
  color: {text_muted};
  min-width: 28px;
  min-height: 28px;
  padding: 2px 6px;
  font-size: 13px;
  transition: background-color 180ms, color 180ms;
}}
.chefbar-gbtn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-gbtn:active {{
  color: {brand};
}}

/* Zoek-input — pill affordance, focus-ring in Iris */
.chefbar-search, .chefbar-search entry {{
  background-color: {surface};
  border: 1px solid {control_border};
  border-radius: 999px;
  color: {text};
  font-size: 13px;
  padding: 7px 14px;
}}
.chefbar-search:focus,
.chefbar-search entry:focus {{
  border-color: {focus};
  box-shadow: 0 0 0 3px {focus_soft};
}}

/* Section eyebrows — Inter caps, kort en rustig */
.chefbar-section-title {{
  font-family: "Inter", "Cantarell", sans-serif;
  font-size: 11px;
  font-weight: 600;
  color: {text_muted};
  padding: 18px 16px 4px 16px;
}}
.chefbar-section-sub {{
  font-size: 11.5px;
  color: {text_faint};
  padding: 0 16px 6px 16px;
}}

/* Zones: één surface per sectie, hairlines tussen rows, radius 12 */
.chefbar-group {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 12px;
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
  border: 1px solid {line};
  border-left: 3px solid {warm};
  border-radius: 12px;
  margin: 2px 16px 6px 16px;
}}

/* Rij-titels en meta (data in Plex Mono) */
.chefbar-card-title {{
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}
.chefbar-card-meta {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 11px;
  color: {text_muted};
}}
.chefbar-empty {{
  padding: 16px 16px;
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
.chefbar-empty-icon {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  font-weight: 600;
  color: {text_faint};
}}

/* Status-dots — status = kleur + vorm + tekst (dot naast label) */
.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: 999px;
  background-color: {text_faint};
}}
.chefbar-dot.ok    {{ background-color: {success}; }}
.chefbar-dot.warn  {{ background-color: {warm}; }}
.chefbar-dot.down  {{ background-color: {error}; }}
.chefbar-dot.info  {{ background-color: {brand}; }}

/* Usage bars */
.chefbar-bar-track {{
  min-height: 4px;
  border-radius: 3px;
  background-color: {surface_muted};
}}
.chefbar-bar-fill {{
  border-radius: 3px;
  background-color: {brand};
}}
.chefbar-bar-fill.ok    {{ background-color: {success}; }}
.chefbar-bar-fill.warn  {{ background-color: {warm}; }}
.chefbar-bar-fill.down  {{ background-color: {error}; }}

/* Knoppen — Huly: primary = Iris fill, witte tekst */
.chefbar-btn {{
  background-color: {surface};
  border: 1px solid {control_border};
  border-radius: 8px;
  color: {text};
  padding: 6px 13px;
  font-size: 13px;
  font-weight: 500;
  min-height: 30px;
  transition: background-color 180ms, border-color 180ms;
}}
.chefbar-btn:hover {{
  background-color: {surface_muted};
}}
.chefbar-btn:focus {{
  border-color: {focus};
}}
.chefbar-btn:active {{
  background-color: {surface};
}}
.chefbar-btn.chefbar-primary {{
  background-color: {brand};
  border-color: {brand};
  color: #FFFFFF;
}}
.chefbar-btn.chefbar-primary:hover {{
  background-color: {brand_hover};
  border-color: {brand_hover};
}}
.chefbar-btn.chefbar-danger {{
  border-color: {warm_line};
  color: {warm};
  background-color: {warm_soft};
}}

/* Stamps (KLAAR/HULP/FOUT/BEZIG) — Inter caps, geen mono-stempel */
.chefbar-stamp {{
  border-radius: 8px;
  padding: 2px 8px;
  font-family: "Inter", "Cantarell", sans-serif;
  font-size: 10px;
  font-weight: 600;
  background-color: {surface_muted};
  color: {text_muted};
}}
.chefbar-stamp.ok    {{ background-color: {success_soft}; color: {success}; }}
.chefbar-stamp.warn  {{ background-color: {warm_soft};    color: {warm}; }}
.chefbar-stamp.error {{ background-color: {error_soft};   color: {error}; }}
.chefbar-stamp.info  {{ background-color: {brand_soft};   color: {brand}; }}

/* Actierows (klikbare rijen in een zone) */
.chefbar-row-btn {{
  background-color: transparent;
  border: none;
  border-bottom: 1px solid {line};
  border-radius: 0;
  min-height: 0;
  transition: background-color 180ms;
}}
.chefbar-group .chefbar-row-btn:last-child {{
  border-bottom: none;
}}
.chefbar-group-attention .chefbar-row-btn:last-child {{
  border-bottom: none;
}}
.chefbar-row-btn:hover {{
  background-color: {hover};
}}
.chefbar-row-btn:focus {{
  box-shadow: inset 2px 0 0 {focus};
}}

/* Room — sidebar + main canvas */
.chefbar-sidebar {{
  background-color: {surface_muted};
  border-right: 1px solid {line};
}}
.chefbar-sidebar-title {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 14px;
  font-weight: 600;
  color: {text};
}}
.chefbar-sidebar-sub {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_faint};
}}
.chefbar-nav-item {{
  background-color: transparent;
  border: 1px solid transparent;
  border-left: 2px solid transparent;
  border-radius: 8px;
  color: {text_muted};
  font-size: 13px;
  font-weight: 500;
  padding: 7px 10px;
  min-height: 30px;
  transition: background-color 180ms, color 180ms;
}}
.chefbar-nav-item:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-nav-item.active {{
  background-color: {surface};
  border: 1px solid {line};
  border-left: 2px solid {brand};
  color: {text};
}}
.chefbar-nav-item.active:hover {{
  background-color: {surface};
}}
.chefbar-nav-item:active {{
  background-color: {brand_soft};
}}
.chefbar-sidebar-footer {{
  border-top: 1px solid {line};
  padding-top: 10px;
}}
.chefbar-sidebar-footer-title {{
  font-size: 11px;
  font-weight: 600;
  color: {text_muted};
}}
.chefbar-sidebar-footer-meta {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 11px;
  color: {text_faint};
}}
.chefbar-main {{
  background-color: {canvas};
}}

/* Footer — mono microcopy, niet overheersend */
.chefbar-footer {{
  background-color: {canvas};
  border-top: 1px solid {line};
  padding: 7px 16px;
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_faint};
}}
.chefbar-footer-label {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_faint};
}}

/* Textdialog (acties met needs_text) — radius 14, diepe schaduw */
.chefbar-dialog {{
  background-color: {canvas};
  border: 1px solid {line};
  border-radius: 14px;
  box-shadow: 0 14px 20px rgba(0, 0, 0, 0.50);
}}
.chefbar-dialog entry {{
  background-color: {surface};
  border: 1px solid {control_border};
  border-radius: 8px;
  color: {text};
  padding: 8px 10px;
}}
.chefbar-dialog entry:focus {{
  border-color: {focus};
}}


/* Scrollbars — dun en stil */
scrollbar {{
  background-color: transparent;
}}
scrollbar slider {{
  background-color: {surface_raised};
  border-radius: 999px;
  min-width: 6px;
  min-height: 6px;
}}
scrollbar slider:hover {{
  background-color: {control_border};
}}
"#
        ,
        canvas = t.canvas,
        surface = t.surface,
        surface_muted = t.surface_muted,
        surface_raised = t.surface_raised,
        line = t.line,
        control_border = t.control_border,
        text = t.text,
        text_muted = t.text_muted,
        text_faint = t.text_faint,
        brand = t.brand,
        brand_hover = t.brand_hover,
        brand_soft = t.brand_soft,
        warm = t.warm,
        warm_soft = t.warm_soft,
        warm_line = t.warm_line,
        focus = t.focus,
        focus_soft = t.focus_soft,
        success = t.success,
        success_soft = t.success_soft,
        error = t.error,
        error_soft = t.error_soft,
        hover = t.hover,
    )
}

/// Huly-tokenwaarden per thema (DESIGN.md kleurcontract).
struct Tokens {
    canvas: &'static str,
    surface: &'static str,
    surface_muted: &'static str,
    surface_raised: &'static str,
    line: &'static str,
    control_border: &'static str,
    text: &'static str,
    text_muted: &'static str,
    text_faint: &'static str,
    brand: &'static str,
    brand_hover: &'static str,
    brand_soft: &'static str,
    warm: &'static str,
    warm_soft: &'static str,
    warm_line: &'static str,
    focus: &'static str,
    focus_soft: &'static str,
    success: &'static str,
    success_soft: &'static str,
    error: &'static str,
    error_soft: &'static str,
    hover: &'static str,
}

impl Tokens {
    /// Donker (Huly OLED — standaard).
    fn dark() -> Self {
        Self {
            canvas: "#090A0C",
            surface: "#111111",
            surface_muted: "#1A1B1E",
            surface_raised: "#303236",
            line: "#4A4B50",
            control_border: "#6B6C6D",
            text: "#FFFFFF",
            text_muted: "#95979E",
            text_faint: "#6B6C6D",
            brand: "#5683DA",
            brand_hover: "#6B93E3",
            brand_soft: "rgba(86,131,218,0.14)",
            warm: "#FF8964",
            warm_soft: "rgba(255,137,100,0.12)",
            warm_line: "rgba(255,137,100,0.40)",
            focus: "#7BA3F0",
            focus_soft: "rgba(123,163,240,0.16)",
            success: "#47D18C",
            success_soft: "rgba(71,209,140,0.12)",
            error: "#FF4D4D",
            error_soft: "rgba(255,77,77,0.12)",
            hover: "rgba(255,255,255,0.045)",
        }
    }

    /// Licht (inset island — volledige pariteit).
    fn light() -> Self {
        Self {
            canvas: "#EFEFF0",
            surface: "#FAFAFA",
            surface_muted: "#F6F6F6",
            surface_raised: "#FFFFFF",
            line: "#D1D1D1",
            control_border: "#95979E",
            text: "#090A0C",
            text_muted: "#6B6C6D",
            text_faint: "#95979E",
            brand: "#3D7EFF",
            brand_hover: "#5C97FF",
            brand_soft: "rgba(61,126,255,0.10)",
            warm: "#E56A3F",
            warm_soft: "rgba(229,106,63,0.10)",
            warm_line: "rgba(229,106,63,0.35)",
            focus: "#0B5FFF",
            focus_soft: "rgba(11,95,255,0.14)",
            success: "#1F8A65",
            success_soft: "rgba(31,138,101,0.10)",
            error: "#CF2D56",
            error_soft: "rgba(207,45,86,0.10)",
            hover: "rgba(9,10,12,0.045)",
        }
    }
}

/// Welk thema actief is. Spec (Signaal · Huly, lock 2026-07-23): donker is
/// de standaard. `gtk-application-prefer-dark-theme` is op GNOME vrijwel
/// altijd false (GTK3 krijgt dark-pref niet door), dus daarop defaulten zou
/// de app onbedoeld licht maken. Licht heeft pariteit en is bereikbaar via
/// CHEFBAR_THEME=light (dev/parity-checks); donker kan expliciet met
/// CHEFBAR_THEME=dark.
pub fn detect_theme(_settings: &gtk::Settings) -> String {
    if let Ok(force) = std::env::var("CHEFBAR_THEME") {
        match force.trim() {
            THEME_LIGHT => return THEME_LIGHT.into(),
            THEME_DARK => return THEME_DARK.into(),
            _ => {}
        }
    }
    THEME_DARK.into()
}
