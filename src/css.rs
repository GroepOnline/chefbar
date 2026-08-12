//! Signaal v2 (Devin-richting) CSS voor ChefBar — tokens uit
//! `GroepOnline/design-system` (`DESIGN.md` v2 + `tokens.css`, skin `devin`),
//! gemapt op de GTK3-CSS subset. Migratiebeslissing: Joep, 2026-08-12
//! (ChefBar volgt OpenCodex, dat per PR #26 op v2 zit).
//!
//! v2-meetwaarden: warm off-white canvas #F7F6F5 / basalt-warm donker
//! #121111, één accent #317CFF (licht) / #5C97FF (donker), General Sans
//! interface + IBM Plex Mono data (geen serif in product-UI), radius
//! 6/10/200, hairlines, geen glow/gradients. Signature blijft de verticale
//! CG-statuslijn (v2 worked-row-streep: line-strong in rust, accent live).
//! Groen blijft gereserveerd voor git/PR/toestemming; amber = wacht-op-jou.

use std::sync::OnceLock;

pub const THEME_DARK: &str = "dark";
pub const THEME_LIGHT: &str = "light";

/// Eén provider voor de hele app; herladen op thema-wissel past de skin
/// live toe zonder herstart (CssProvider::load_from_data is idempotent).
static PROVIDER: OnceLock<gtk::CssProvider> = OnceLock::new();
static ACTIVE: OnceLock<std::sync::Mutex<String>> = OnceLock::new();

fn active_mutex() -> &'static std::sync::Mutex<String> {
    ACTIVE.get_or_init(|| std::sync::Mutex::new(THEME_DARK.to_string()))
}

/// Het nu actieve thema (voor footer-toggle en state-persist).
pub fn active_theme() -> String {
    active_mutex().lock().map(|s| s.clone()).unwrap_or_else(|_| THEME_DARK.to_string())
}

/// Laadt de stylesheet voor `theme` en geeft de provider voor
/// `add_provider_for_screen`. Herhaald aanroepen met een ander thema
/// herlaadt dezelfde provider (live skin-wissel).
pub fn theme_provider(theme: &str) -> &'static gtk::CssProvider {
    if let Ok(mut active) = active_mutex().lock() {
        *active = theme.to_string();
    }
    PROVIDER.get_or_init(|| {
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(styles_css(theme).as_bytes())
            .expect("chefbar-css compileert");
        provider
    })
}

/// Live thema-wissel: herlaad de gedeelde provider en onthoud de keuze.
pub fn set_theme(theme: &str) {
    if let Ok(mut active) = active_mutex().lock() {
        *active = theme.to_string();
    }
    if let Some(provider) = PROVIDER.get() {
        if let Err(err) = provider.load_from_data(styles_css(theme).as_bytes()) {
            eprintln!("chefbar: css herladen mislukt ({err})");
        }
    }
}

/// Bouwt de volledige stylesheet voor het gekozen thema.
pub fn styles_css(theme: &str) -> String {
    let t = if theme == THEME_LIGHT {
        Tokens::light()
    } else {
        Tokens::dark()
    };
    format!(
        r#"
/* ============ App-window (v2 canvas) ============ */
.chefbar-app {{
  background-color: {canvas};
  color: {text};
  font-family: "General Sans", system-ui, "Cantarell", "Noto Sans", sans-serif;
  font-size: 13px;
}}

/* ============ Header — custom titlebar (undecorated window) ============ */
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
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
}}

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
  transition: background-color 140ms, color 140ms;
}}
.chefbar-gbtn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-gbtn:active {{
  color: {brand};
}}

/* Zoek-input — pill affordance, focus-ring in accent */
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

/* ============ Signature: de CG-statuslijn ============ */
/* 3px verticale lijn (v2 worked-row-streep) + één compacte statusregel.
   Rust = line-strong; live/bezig = accent; ok/warn/error/info volgen het
   statusspectrum. De streep blijft het enige uitgesproken element. */
.chefbar-signature {{
  background-color: {control_border};
  min-width: 3px;
  border-radius: 2px;
}}
.chefbar-signature.ok       {{ background-color: {success}; }}
.chefbar-signature.warn     {{ background-color: {warn}; }}
.chefbar-signature.error    {{ background-color: {error}; }}
.chefbar-signature.info     {{ background-color: {info}; }}
.chefbar-signature.running  {{ background-color: {brand}; }}
.chefbar-statuslijn {{
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  padding: 8px 12px;
  margin: 10px 16px 2px 16px;
}}
.chefbar-statuslijn-text {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 13px;
  font-weight: 500;
  color: {text};
}}

/* ============ Section eyebrows — v2 .caps (10.5/600, +spacing) ============ */
.chefbar-section-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_muted};
  padding: 16px 16px 4px 16px;
}}
.chefbar-section-sub {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 11px;
  font-weight: 400;
  color: {text_muted};
  padding: 0 16px 8px 16px;
}}

/* ============ Zones: één surface per sectie, hairlines tussen rows ============ */
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
  border: 1px solid {line};
  border-left: 3px solid {warm};
  border-radius: 10px;
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

/* Status-dots — status = kleur + vorm + tekst (dot naast label) */
.chefbar-dot {{
  min-width: 8px;
  min-height: 8px;
  border-radius: 999px;
  background-color: {control_border};
}}
.chefbar-dot.ok    {{ background-color: {success}; }}
.chefbar-dot.warn  {{ background-color: {warn}; }}
.chefbar-dot.down  {{ background-color: {error}; }}
.chefbar-dot.info  {{ background-color: {info}; }}
.chefbar-dot.live  {{ background-color: {brand}; }}

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
.chefbar-bar-fill.warn  {{ background-color: {warn}; }}
.chefbar-bar-fill.down  {{ background-color: {error}; }}

/* ============ Knoppen — v2 .btn: hairline-strong, r-6 ============ */
.chefbar-btn {{
  background-color: {surface};
  border: 1px solid {control_border};
  border-radius: 6px;
  color: {text};
  padding: 6px 13px;
  font-size: 13px;
  font-weight: 500;
  min-height: 32px;
  transition: background-color 140ms, border-color 140ms;
}}
.chefbar-btn:hover {{
  background-color: {surface_muted};
}}
.chefbar-btn:focus {{
  border-color: {focus};
  box-shadow: 0 0 0 3px {focus_soft};
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
  border-color: {error};
  color: {error};
  background-color: {error_soft};
}}

/* kbd-chips — toetsen in mono, hairline */
.chefbar-kbd {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
  border: 1px solid {control_border};
  border-radius: 4px;
  padding: 1px 5px;
}}

/* ============ Stamps — v2 badge-pill, caps ============ */
/* Neutraal (geen status-klasse) is surface-muted + text-muted.
   ok = groen (alleen git/PR/klaar) · warn = amber (wacht-op-jou)
   error = rood · info = accent-tint (bezig). */
.chefbar-stamp {{
  border-radius: 200px;
  padding: 2px 8px;
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  background-color: {surface_muted};
  color: {text_muted};
}}
.chefbar-stamp.ok    {{ background-color: {success_soft}; color: {success}; }}
.chefbar-stamp.warn  {{ background-color: {warn_soft};    color: {warn}; }}
.chefbar-stamp.error {{ background-color: {error_soft};   color: {error}; }}
.chefbar-stamp.info  {{ background-color: {info_soft};    color: {info}; }}

/* ============ Actierows (klikbare rijen in een zone) ============ */
.chefbar-row-btn {{
  background-color: transparent;
  border: none;
  border-bottom: 1px solid {line};
  border-radius: 0;
  min-height: 0;
  transition: background-color 140ms;
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
  border-left: 2px solid {focus};
  box-shadow: inset 0 0 0 1px {focus_soft};
}}

/* ============ Room — sidebar + main canvas ============ */
.chefbar-sidebar {{
  background-color: {surface_muted};
  border-right: 1px solid {line};
}}
.chefbar-sidebar-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 14px;
  font-weight: 500;
  color: {text};
}}
.chefbar-sidebar-sub {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-nav-item {{
  background-color: transparent;
  border: 1px solid transparent;
  border-left: 2px solid transparent;
  border-radius: 6px;
  color: {text_muted};
  font-size: 13px;
  font-weight: 500;
  padding: 6px 10px;
  min-height: 28px;
  transition: background-color 140ms, color 140ms;
}}
.chefbar-nav-item:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-nav-item:focus {{
  border-color: {focus};
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
.chefbar-sidebar-group-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_muted};
  padding: 6px 12px 2px 12px;
}}
.chefbar-sidebar-footer {{
  border-top: 1px solid {line};
  padding-top: 10px;
}}
.chefbar-sidebar-footer-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_muted};
}}
.chefbar-sidebar-footer-meta {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 11px;
  color: {text_muted};
}}
.chefbar-main {{
  background-color: {canvas};
}}

/* ============ Footer — gepind onder de scroller ============ */
.chefbar-footer {{
  background-color: {canvas};
  border-top: 1px solid {line};
  padding: 6px 16px;
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-footer-label {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
}}
.chefbar-footer-btn {{
  background-color: transparent;
  border: 1px solid {control_border};
  border-radius: 6px;
  color: {text_muted};
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 11px;
  font-weight: 500;
  padding: 2px 9px;
  transition: background-color 140ms, color 140ms;
}}
.chefbar-footer-btn:hover {{
  background-color: {hover};
  color: {text};
}}
.chefbar-footer-btn.on {{
  color: {brand};
  border-color: {focus_soft};
  background-color: {brand_soft};
}}

/* ============ Drawer (300px, hairline left, canvas bg) ============ */
.chefbar-drawer {{
  min-width: 300px;
  background-color: {canvas};
  border-left: 1px solid {line};
  transition: opacity 160ms ease-out;
}}
.chefbar-drawer-title {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 15px;
  font-weight: 600;
  color: {text};
}}
.chefbar-drawer-actions {{
  padding-top: 8px;
}}
.chefbar-drawer-hint {{
  font-family: "IBM Plex Mono", monospace;
  font-size: 10px;
  color: {text_muted};
  padding: 4px 12px 12px 12px;
}}

/* ============ Overlay (palette, center, 560px, radius 10, shadow) ============ */
.chefbar-overlay,
.chefbar-palette-overlay {{
  min-width: 560px;
  background-color: {surface};
  border: 1px solid {line};
  border-radius: 10px;
  box-shadow: 0 14px 20px rgba(0, 0, 0, 0.50);
  padding: 12px;
}}
.chefbar-palette-entry {{
  min-height: 36px;
}}
.chefbar-palette-results {{
  padding-top: 6px;
}}
.chefbar-palette-section {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: {text_muted};
  padding: 6px 8px 4px 8px;
}}
.chefbar-palette-row {{
  background-color: transparent;
  border: none;
  border-radius: 6px;
  padding: 6px 8px;
  transition: background-color 140ms;
}}
.chefbar-palette-row:hover {{
  background-color: {hover};
}}
.chefbar-palette-row:focus {{
  box-shadow: inset 0 0 0 1px {focus};
}}

/* ============ Zone header + card grid ============ */
.chefbar-zone-header {{
  font-family: "General Sans", system-ui, sans-serif;
  font-size: 10.5px;
  font-weight: 600;
  color: {text_muted};
  padding: 10px 12px 6px 12px;
}}
.chefbar-card-grid {{
  padding: 8px 12px;
}}

/* ============ Textdialog (acties met needs_text) — radius 12 ============ */
.chefbar-dialog {{
  background-color: {canvas};
  border: 1px solid {line};
  border-radius: 12px;
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

/* ============ Density token ============ */
/* comfortable (default) / compact via .density-compact op het window.
   Compact: header 8/12/8, statuslijn 6/10, section-padding gehalveerd,
   rows 5px, group-marges kleiner, titel -2px. Eén klas, geen tweede sheet. */
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
"#,
        canvas = t.canvas,
        surface = t.surface,
        surface_muted = t.surface_muted,
        surface_raised = t.surface_raised,
        line = t.line,
        control_border = t.control_border,
        text = t.text,
        text_muted = t.text_muted,
        brand = t.brand,
        brand_soft = t.brand_soft,
        warm = t.warm,
        warn = t.warn,
        warn_soft = t.warn_soft,
        info = t.info,
        info_soft = t.info_soft,
        focus = t.focus,
        focus_soft = t.focus_soft,
        success = t.success,
        success_soft = t.success_soft,
        error = t.error,
        error_soft = t.error_soft,
        hover = t.hover,
    )
}

/// v2-tokenwaarden per thema (tokens.css, skin devin).
struct Tokens {
    canvas: &'static str,
    surface: &'static str,
    surface_muted: &'static str,
    surface_raised: &'static str,
    line: &'static str,
    control_border: &'static str,
    text: &'static str,
    text_muted: &'static str,
    brand: &'static str,
    brand_soft: &'static str,
    warm: &'static str,
    warn: &'static str,
    warn_soft: &'static str,
    info: &'static str,
    info_soft: &'static str,
    focus: &'static str,
    focus_soft: &'static str,
    success: &'static str,
    success_soft: &'static str,
    error: &'static str,
    error_soft: &'static str,
    hover: &'static str,
}

impl Tokens {
    /// Donker (v2 devin-skin dark — standaard).
    fn dark() -> Self {
        Self {
            canvas: "#121111",
            surface: "#1B1A19",
            surface_muted: "#242322",
            surface_raised: "#242322",
            line: "rgba(255,255,255,0.09)",
            control_border: "rgba(255,255,255,0.16)",
            text: "#F0EEEB",
            text_muted: "rgba(240,238,235,0.55)",
            brand: "#5C97FF",
            brand_soft: "rgba(92,151,255,0.12)",
            warm: "#D9A038",
            warn: "#D9A038",
            warn_soft: "rgba(217,160,56,0.08)",
            info: "#8AB4FF",
            info_soft: "rgba(92,151,255,0.12)",
            focus: "#5C97FF",
            focus_soft: "rgba(92,151,255,0.12)",
            success: "#3FB950",
            success_soft: "rgba(63,185,80,0.12)",
            error: "#F85149",
            error_soft: "rgba(248,81,73,0.12)",
            hover: "rgba(255,255,255,0.05)",
        }
    }

    /// Licht (v2 devin-skin light — volledige pariteit).
    fn light() -> Self {
        Self {
            canvas: "#F7F6F5",
            surface: "#FFFFFF",
            surface_muted: "#EFEFEF",
            surface_raised: "#FFFFFF",
            line: "rgba(0,0,0,0.08)",
            control_border: "rgba(0,0,0,0.14)",
            text: "#191919",
            text_muted: "rgba(0,0,0,0.55)",
            brand: "#317CFF",
            brand_soft: "rgba(49,124,255,0.09)",
            warm: "#BF5B00",
            warn: "#BF5B00",
            warn_soft: "rgba(191,91,0,0.06)",
            info: "#1D5FD6",
            info_soft: "rgba(49,124,255,0.09)",
            focus: "#317CFF",
            focus_soft: "rgba(49,124,255,0.09)",
            success: "#1F883D",
            success_soft: "rgba(31,136,61,0.10)",
            error: "#CF222E",
            error_soft: "rgba(207,34,46,0.10)",
            hover: "rgba(0,0,0,0.045)",
        }
    }
}

/// Welk thema actief is. Signaal v2: donker (basalt-warm) is de standaard
/// voor ChefBar, zoals bij de Huly-lock was afgesproken. `gtk-application-prefer-dark-theme` is op GNOME vrijwel
/// altijd false (GTK3 krijgt dark-pref niet door), dus daarop defaulten zou
/// de app onbedoeld licht maken. Licht heeft pariteit en is bereikbaar via
/// CHEFBAR_THEME=light (dev/parity-checks); donker kan expliciet met
/// CHEFBAR_THEME=dark.
pub fn detect_theme(_settings: &gtk::Settings) -> String {
    if let Ok(force) = std::env::var("CHEFBAR_THEME") {
        match force.trim().to_ascii_lowercase().as_str() {
            THEME_LIGHT => return THEME_LIGHT.into(),
            THEME_DARK => return THEME_DARK.into(),
            _ => {}
        }
    }
    THEME_DARK.into()
}
