//! Devin-skin CSS voor ChefBar — tokens uit `~/design-system/tokens.css`
//! (v2 "Devin-richting"), gemapt op GTK3-CSS (beperkte subset).
//!
//! Devin-meetwaarden: warme bg, zachte lijnen, serif display, medium koppen,
//! pill-badges, accent #317CFF/#5C97FF. Alleen GTK3-compatibele properties.

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

/// Bouwt de volledige stylesheet voor het gekozen thema.
pub fn styles_css(theme: &str) -> String {
    let (
        bg,
        surface,
        sunk,
        hover,
        line,
        line_strong,
        text,
        text_muted,
        text_faint,
        accent,
        _accent_ink,
        accent_soft,
        green,
        green_bg,
        red,
        amber,
        hold_bg,
        hold_line,
    ) = if theme == THEME_LIGHT {
        (
            "#F7F6F5",
            "#FFFFFF",
            "#EFEFEF",
            "rgba(0,0,0,0.045)",
            "rgba(0,0,0,0.08)",
            "rgba(0,0,0,0.14)",
            "#191919",
            "rgba(0,0,0,0.55)",
            "rgba(0,0,0,0.38)",
            "#317CFF",
            "#1D5FD6",
            "rgba(49,124,255,0.09)",
            "#1F883D",
            "rgba(31,136,61,0.10)",
            "#CF222E",
            "#BF5B00",
            "rgba(191,91,0,0.06)",
            "rgba(191,91,0,0.35)",
        )
    } else {
        (
            "#121111",
            "#1B1A19",
            "#242322",
            "rgba(255,255,255,0.05)",
            "rgba(255,255,255,0.09)",
            "rgba(255,255,255,0.16)",
            "#F0EEEB",
            "rgba(240,238,235,0.55)",
            "rgba(240,238,235,0.35)",
            "#5C97FF",
            "#8AB4FF",
            "rgba(92,151,255,0.12)",
            "#3FB950",
            "rgba(63,185,80,0.12)",
            "#F85149",
            "#D9A038",
            "rgba(217,160,56,0.08)",
            "rgba(217,160,56,0.40)",
        )
    };

    format!(
        r#"
/* ============ App-window (Devin-dark) ============ */
.chefbar-app {{
  background-color: {bg};
  color: {text};
  font-family: "General Sans", "Inter", "Cantarell", sans-serif;
  font-size: 13.5px;
}}

/* Header — custom titlebar (undecorated window) — strak en compact */
.chefbar-header {{
  background-color: {bg};
  padding: 12px 16px 10px 16px;
  border-bottom: 1px solid {line};
}}
.chefbar-title {{
  font-family: "Instrument Serif", "General Sans", Georgia, serif;
  font-size: 22px;
  font-weight: 500;
  color: {text};
}}
.chefbar-title-sub {{
  font-family: "IBM Plex Mono", "JetBrains Mono", monospace;
  font-size: 10px;
  color: {text_faint};
  letter-spacing: 0.02em;
}}

/* Status-badge (pill) — compact premium */
.chefbar-badge {{
  border-radius: 200px;
  padding: 2px 10px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
  background-color: {sunk};
  color: {text_muted};
}}
.chefbar-badge.ok    {{ background-color: {green_bg}; color: {green}; }}
.chefbar-badge.warn  {{ background-color: {hold_bg};  color: {amber}; }}
.chefbar-badge.error {{ background-color: rgba(248,81,73,0.12); color: {red}; }}
.chefbar-badge.info  {{ background-color: {accent_soft}; color: {accent}; }}

/* Ghost icon-knoppen (header controls) */
.chefbar-gbtn {{
  background-color: transparent;
  border: none;
  border-radius: 6px;
  color: {text_muted};
  min-width: 28px;
  min-height: 28px;
  padding: 2px 6px;
  font-size: 13px;
}}
.chefbar-gbtn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-gbtn:active {{
  color: {accent};
}}

/* Zoek-input met focus-ring — strak */
.chefbar-search, .chefbar-search entry {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  font-size: 13px;
  padding: 7px 10px;
}}
.chefbar-search:focus,
.chefbar-search entry:focus {{
  border-color: {accent};
  box-shadow: 0 0 0 3px {accent_soft};
}}

/* Section headers — strak, consistente 16px zijkanten */
.chefbar-section-title {{
  font-size: 13.5px;
  font-weight: 600;
  color: {text};
  padding: 16px 16px 2px 16px;
}}
.chefbar-section-sub {{
  font-size: 11.5px;
  color: {text_muted};
  padding: 0 16px 6px 16px;
}}

/* Grouped cards: één card per sectie, hairlines tussen rows — strak 16px */
.chefbar-group {{
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
  border-left: 2px solid {amber};
  border-radius: 10px;
  margin: 2px 16px 6px 16px;
}}

/* Card-titels en meta */
/* Card-titels en meta — strak, geen overflow-bleed */
.chefbar-card-title {{
  font-size: 13.5px;
  font-weight: 500;
  color: {text};
}}
.chefbar-card-meta {{
  font-family: "IBM Plex Mono", monospace;
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
.chefbar-empty-icon {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  font-weight: 600;
  color: {text_faint};
  letter-spacing: 0.06em;
  text-transform: uppercase;
}}

/* Status-dots (pill) */
.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: 200px;
  background-color: {text_faint};
}}
.chefbar-dot.ok    {{ background-color: {green}; }}
.chefbar-dot.warn  {{ background-color: {amber}; }}
.chefbar-dot.down  {{ background-color: {red}; }}
.chefbar-dot.info  {{ background-color: {accent}; }}

/* Usage bars */
.chefbar-bar-track {{
  min-height: 4px;
  border-radius: 200px;
  background-color: {sunk};
}}
.chefbar-bar-fill {{
  border-radius: 200px;
  background-color: {accent};
}}
.chefbar-bar-fill.ok    {{ background-color: {green}; }}
.chefbar-bar-fill.warn  {{ background-color: {amber}; }}
.chefbar-bar-fill.down  {{ background-color: {red}; }}

/* Knoppen — shadcn/devin-geest */
.chefbar-btn {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  padding: 6px 13px;
  font-size: 13px;
  font-weight: 500;
  min-height: 30px;
}}
.chefbar-btn:hover {{
  background-color: {sunk};
}}
.chefbar-btn:focus {{
  border-color: {accent};
}}
.chefbar-btn:active {{
  background-color: {surface};
}}
.chefbar-btn.chefbar-primary {{
  background-color: {text};
  border-color: {text};
  color: {bg};
}}
.chefbar-btn.chefbar-primary:hover {{
  background-color: {text};
  opacity: 0.87;
}}
.chefbar-btn.chefbar-danger {{
  border-color: {hold_line};
  color: {amber};
  background-color: {hold_bg};
}}

/* Stamp-badges (KLAAR/HULP/FOUT/BEZIG) */
.chefbar-stamp {{
  border-radius: 200px;
  padding: 2px 8px;
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  background-color: {sunk};
  color: {text_muted};
}}
.chefbar-stamp.ok    {{ background-color: {green_bg}; color: {green}; }}
.chefbar-stamp.warn  {{ background-color: {hold_bg};  color: {amber}; }}
.chefbar-stamp.error {{ background-color: rgba(248,81,73,0.12); color: {red}; }}
.chefbar-stamp.info  {{ background-color: {accent_soft}; color: {accent}; }}

/* Actierows (klikbare rijen in een group) */
.chefbar-row-btn {{
  background-color: transparent;
  border: none;
  border-bottom: 1px solid {line};
  border-radius: 0;
  min-height: 0;
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

/* Room — sidebar + main canvas (tokens dark: bg #121111, surface #1B1A19, sunk #242322, line 09%, line-strong 16%, accent #5C97FF) */
.chefbar-sidebar {{
  background-color: {sunk};
  border-right: 1px solid {line};
}}
.chefbar-sidebar-title {{
  font-family: "Instrument Serif", "General Sans", Georgia, serif;
  font-size: 15px;
  font-weight: 500;
  color: {text};
}}
.chefbar-sidebar-sub {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10.5px;
  color: {text_faint};
}}
.chefbar-nav-item {{
  background-color: transparent;
  border: 1px solid transparent;
  border-left: 2px solid transparent;
  border-radius: 6px;
  color: {text_muted};
  font-size: 13px;
  font-weight: 500;
  padding: 7px 10px;
  min-height: 30px;
}}
.chefbar-nav-item:hover {{
  background-color: {sunk};
  color: {text};
}}
.chefbar-nav-item.active {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-left: 2px solid {accent};
  color: {text};
}}
.chefbar-nav-item.active:hover {{
  background-color: {surface};
}}
.chefbar-nav-item:active {{
  background-color: {accent_soft};
}}
.chefbar-sidebar-footer {{
  border-top: 1px solid {line};
  padding-top: 10px;
}}
.chefbar-sidebar-footer-title {{
  font-size: 11px;
  font-weight: 600;
  color: {text_muted};
  letter-spacing: 0.04em;
  text-transform: uppercase;
}}
.chefbar-sidebar-footer-meta {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 11px;
  color: {text_faint};
}}
.chefbar-main {{
  background-color: {bg};
}}

/* Footer — strak, mono, niet overheersend */
.chefbar-footer {{
  background-color: {bg};
  border-top: 1px solid {line};
  padding: 7px 16px;
  font-family: "IBM Plex Mono", monospace;
  font-size: 10.5px;
  color: {text_faint};
}}
.chefbar-footer-label {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10.5px;
  color: {text_faint};
}}

/* Textdialog (acties met needs_text) */
.chefbar-dialog {{
  background-color: {bg};
  border: 1px solid {line};
  border-radius: 10px;
}}
.chefbar-dialog entry {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 6px;
  color: {text};
  padding: 8px 10px;
}}
.chefbar-dialog entry:focus {{
  border-color: {accent};
}}

/* Harnas-tabs (room) — compact pills */
.chefbar-harness-row {{
  padding: 0;
}}
.chefbar-harness {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 200px;
  padding: 3px 11px;
  font-size: 11.5px;
  font-weight: 500;
  color: {text_muted};
}}
.chefbar-harness:hover {{
  background-color: {sunk};
  color: {text};
}}
.chefbar-harness-active {{
  background-color: {text};
  border-color: {text};
  border-radius: 200px;
  padding: 3px 11px;
  font-size: 11.5px;
  font-weight: 600;
  color: {bg};
}}
"#
    )
}

/// Welk thema actief is: donker tenzij de desktop expliciet om light vraagt.
pub fn detect_theme(settings: &gtk::Settings) -> String {
    use gtk::prelude::*;
    if settings.property::<bool>("gtk-application-prefer-dark-theme") {
        THEME_DARK.into()
    } else {
        THEME_LIGHT.into()
    }
}
