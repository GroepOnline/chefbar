//! Signaal CSS voor ChefBar — tokens uit `~/design-system/tokens.css`
//! (skin "strak", dark default), gemapt op de GTK-widget-classes.
//!
//! Type: General Sans (interface) / IBM Plex Mono (data), headings medium 500,
//! caps-labels met 0.07em tracking, r4/r6 radii, pills voor badges.

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

pub fn styles_css(theme: &str) -> String {
    // strak-skin tokens (dark default, light als expliciet gevraagd).
    let (bg, surface, sunk, text, text_muted, text_faint, line, line_strong, accent, accent_ink,
        _accent_soft, hover, green, _green_bg, red, amber, _amber_bg, _amber_line) =
        if theme == THEME_LIGHT {
            (
                "#F4F6F9", "#FFFFFF", "#EBEEF3", "#131417", "#525860", "#8F96A1",
                "#E2E6EC", "#C6CDD8", "#2563EB", "#1D4FD7", "rgba(37,99,235,0.09)",
                "rgba(19,20,23,0.05)", "#1F883D", "rgba(31,136,61,0.10)", "#CF222E",
                "#BF5B00", "rgba(191,91,0,0.06)", "rgba(191,91,0,0.35)",
            )
        } else {
            (
                "#0F1013", "#16181C", "#1E2126", "#ECEDF0", "#9BA1AB", "#676D78",
                "#25282E", "#363A43", "#4F8DFF", "#7FA9FF", "rgba(79,141,255,0.12)",
                "rgba(236,237,240,0.06)", "#3FB950", "rgba(63,185,80,0.12)", "#F85149",
                "#D9A038", "rgba(217,160,56,0.08)", "rgba(217,160,56,0.40)",
            )
        };

    format!(
        r#"
/* ===== Basis ===== */
.chefbar-panel {{
  background-color: {bg};
  color: {text};
  font-family: "General Sans", Inter, Cantarell, sans-serif;
  font-size: 13.5px;
}}
.chefbar-panel * {{
  outline-style: none;
  background-clip: padding-box;
}}

/* ===== Header ===== */
.chefbar-header {{
  background-color: {surface};
  border-bottom: 1px solid {line};
  padding: 14px 18px 12px 18px;
}}
.chefbar-title {{
  font-family: "IBM Plex Mono", "JetBrains Mono", monospace;
  font-size: 13px;
  font-weight: 500;               /* devin-meetwaarde: medium, niet bold */
  letter-spacing: -0.02em;
  color: {text};
}}
.chefbar-subtitle {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10.5px;
  color: {text_muted};
}}

/* ===== Sectie-labels (caps, 0.07em tracking) ===== */
.chefbar-section-label {{
  font-size: 10.5px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: {text_faint};
  padding: 18px 18px 6px 18px;
}}

/* ===== Cards (strak r-lg 6px) ===== */
.chefbar-card {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 6px;
  padding: 10px 14px;
}}
.chefbar-card:hover {{
  border-color: {line_strong};
}}
.chefbar-card-title {{
  font-size: 13.5px;
  font-weight: 500;
  color: {text};
}}
.chefbar-card-meta {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10.5px;
  color: {text_muted};
}}

/* ===== Status dots (badge-pill) ===== */
.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: 200px;           /* pill */
  background-color: {text_faint};
}}
.chefbar-dot.ok {{ background-color: {green}; }}
.chefbar-dot.warn {{ background-color: {amber}; }}
.chefbar-dot.down {{ background-color: {red}; }}
.chefbar-dot.info {{ background-color: {accent}; }}
.chefbar-dot.pulse {{
  animation: chefbar-pulse 1.6s ease-in-out infinite alternate;
}}
@keyframes chefbar-pulse {{
  from {{ opacity: 0.35; }}
  to {{ opacity: 1; }}
}}

/* ===== Usage bars ===== */
.chefbar-bar-track {{
  min-height: 4px;
  border-radius: 200px;
  background-color: {sunk};
}}
.chefbar-bar-fill {{
  border-radius: 200px;
  background-color: {accent};
}}
.chefbar-bar-fill.ok {{ background-color: {green}; }}
.chefbar-bar-fill.warn {{ background-color: {amber}; }}
.chefbar-bar-fill.down {{ background-color: {red}; }}

/* ===== Knoppen (strak r-md 4px, shadcn-geest) ===== */
.chefbar-actions button {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 4px;
  color: {text};
  padding: 7px 12px;
  font-size: 12.5px;
  font-weight: 500;
}}
.chefbar-actions button:hover {{
  background-color: {sunk};
}}
.chefbar-actions button:focus {{
  border-color: {accent};
}}
.chefbar-actions button:active {{
  background-color: {surface};
}}
.chefbar-actions button.chefbar-primary {{
  background-color: {text};
  border-color: {text};
  color: {bg};
}}

/* ===== Footer / ghost-knoppen ===== */
.chefbar-footer {{
  background-color: {surface};
  border-top: 1px solid {line};
  padding: 8px 14px;
  font-size: 11px;
  color: {text_muted};
}}
.chefbar-switch-btn {{
  background-color: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: {text_muted};
  padding: 4px 9px;
  font-size: 11.5px;
}}
.chefbar-switch-btn:hover {{
  background-color: {hover};
  border-color: {line};
  color: {text};
}}

/* ===== Popover / dialog ===== */
.chefbar-popover contents {{
  background-color: {bg};
  border: 1px solid {line_strong};
  border-radius: 6px;
}}
.chefbar-popover button {{
  background-color: transparent;
  border: none;
  border-radius: 4px;
  color: {text};
  padding: 6px 10px;
  font-size: 12.5px;
}}
.chefbar-popover button:hover {{
  background-color: {sunk};
}}
.chefbar-dialog {{
  background-color: {bg};
  color: {text};
}}
.chefbar-dialog entry {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 4px;
  color: {text};
  padding: 8px 10px;
}}
.chefbar-dialog entry:focus {{
  border-color: {accent};
}}
.chefbar-dialog button {{
  background-color: {surface};
  border: 1px solid {line_strong};
  border-radius: 4px;
  color: {text};
  padding: 6px 12px;
  font-size: 12.5px;
}}
.chefbar-dialog button:hover {{
  background-color: {sunk};
}}

/* ===== Zoek-head / suggesties ===== */
.chefbar-bar-entry {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 6px;
  color: {text};
  font-size: 13.5px;
  padding: 9px 12px;
}}
.chefbar-bar-entry:focus {{
  border-color: {accent};
}}
.chefbar-bar-suggestion {{
  background-color: {surface};
  border-bottom: 1px solid {line};
  border-radius: 0;
}}
.chefbar-bar-suggestion:last-child {{
  border-bottom: none;
}}
.chefbar-bar-suggestion:hover,
.chefbar-bar-suggestion:selected {{
  background-color: {sunk};
}}
.chefbar-bar-suggestion-act {{
  background-color: {accent};
  border: none;
  border-radius: 4px;
  color: {bg};
  font-size: 11px;
  font-weight: 600;
  padding: 4px 8px;
}}
.chefbar-bar-suggestion-act:hover {{
  background-color: {accent_ink};
}}
.chefbar-bar-row-stamp {{
  background-color: {sunk};
  border-radius: 200px;
  padding: 2px 8px;
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  font-weight: 600;
  color: {text_muted};
}}
"#
    )
}

/// Welk thema actief is: donker tenzij de desktop expliciet om light vraagt.
pub fn detect_theme(settings: &gtk::Settings) -> String {
    use gtk::prelude::*;
    if settings
        .property::<bool>("gtk-application-prefer-dark-theme")
    {
        THEME_DARK.into()
    } else {
        THEME_LIGHT.into()
    }
}