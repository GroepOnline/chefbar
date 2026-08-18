//! Signaal motion helpers voor ChefBar (GTK3).
//!
//! Signaal v2 motion-spec: 140ms hover/press, 280ms paneel, 420ms expand.
//! Alleen opacity-fades: geen bounce, pulse-loops of scroll-theater.
//! Respecteert gtk-enable-animations (reduced-motion proxy).

use gtk::glib::ControlFlow;
use gtk::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

/// Devin v2 duur-ladder (`tokens.css`): `--dur-fast` / `--dur-med` /
/// `--dur-slow`. Elke timing hieronder moet één van deze drie zijn — geen
/// tussenwaarden, anders loopt de chrome uit de pas met de designfile.
pub const DUR_FAST_MS: u32 = 140;
pub const DUR_MED_MS: u32 = 280;
pub const DUR_SLOW_MS: u32 = 420;

pub const PRESS_MS: u32 = DUR_FAST_MS;
pub const HOVER_MS: u32 = DUR_FAST_MS;
pub const PANEL_MS: u32 = DUR_MED_MS;
/// Drawer is een pane-slide en volgt dus dezelfde duur als het paneel
/// (was 160ms — naast de ladder).
pub const DRAWER_MS: u32 = DUR_MED_MS;
/// Palette-overlay is een reveal, niet een pane (was 100ms — naast de ladder).
pub const OVERLAY_MS: u32 = DUR_FAST_MS;
const FADE_STEPS: u32 = 8;

/// Per-window animation generation: elke start-fade bumped de token zodat
/// timeouts van een eerdere fade de window niet meer aanraken (een fade-out
/// tick mag een heropend venster niet verbergen).
static GENERATIONS: std::sync::LazyLock<Mutex<HashMap<usize, u32>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn window_key(window: &gtk::Window) -> usize {
    window.as_ptr() as usize
}

fn begin(window: &gtk::Window) -> u32 {
    let mut map = GENERATIONS.lock().unwrap();
    let next = map.get(&window_key(window)).copied().unwrap_or(0) + 1;
    map.insert(window_key(window), next);
    next
}

fn is_current(window: &gtk::Window, generation: u32) -> bool {
    GENERATIONS
        .lock()
        .unwrap()
        .get(&window_key(window))
        .copied()
        == Some(generation)
}

pub fn motion_enabled() -> bool {
    gtk::Settings::default()
        .map(|settings| settings.property::<bool>("gtk-enable-animations"))
        .unwrap_or(true)
}

/// Toon window met een korte opacity-ramp (of instant als motion uit staat).
pub fn fade_in(window: &gtk::Window, duration_ms: u32) {
    let generation = begin(window);
    if !motion_enabled() || duration_ms == 0 {
        window.set_opacity(1.0);
        window.show();
        return;
    }
    window.set_opacity(0.0);
    window.show();
    let step_ms = std::cmp::max(1, duration_ms / FADE_STEPS);
    let window = window.clone();
    let mut step = 0u32;
    gtk::glib::timeout_add_local(
        std::time::Duration::from_millis(step_ms as u64),
        move || {
            if !is_current(&window, generation) {
                return ControlFlow::Break;
            }
            step += 1;
            let frac = (step as f64 / FADE_STEPS as f64).min(1.0);
            window.set_opacity(frac);
            if frac >= 1.0 {
                window.set_opacity(1.0);
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        },
    );
}

/// Verberg window na een korte opacity-ramp (of instant).
pub fn fade_out(window: &gtk::Window, duration_ms: u32) {
    let generation = begin(window);
    let window_clone = window.clone();

    fn finish(window: &gtk::Window) {
        window.hide();
        window.set_opacity(1.0);
    }

    if !motion_enabled() || duration_ms == 0 {
        finish(&window_clone);
        return;
    }
    let step_ms = std::cmp::max(1, duration_ms / FADE_STEPS);
    let mut step = 0u32;
    gtk::glib::timeout_add_local(
        std::time::Duration::from_millis(step_ms as u64),
        move || {
            if !is_current(&window_clone, generation) {
                return ControlFlow::Break;
            }
            step += 1;
            let frac = (1.0 - step as f64 / FADE_STEPS as f64).max(0.0);
            window_clone.set_opacity(frac);
            if frac <= 0.0 {
                finish(&window_clone);
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        },
    );
}

/// Slide drawer in/uit (`DRAWER_MS`). Wrapper om fade_in/fade_out.
/// TODO: echte translate-animatie als GTK reveal-infrastructuur beschikbaar is;
/// voor nu fade — compileerbaar en visueel consistent met panel.
pub fn slide_drawer(window: &gtk::Window, open: bool) {
    if open {
        fade_in(window, DRAWER_MS);
    } else {
        fade_out(window, DRAWER_MS);
    }
}

/// Fade palette-overlay in/uit (`OVERLAY_MS`). Wrapper om fade_in/fade_out.
pub fn fade_overlay(window: &gtk::Window, show: bool) {
    if show {
        fade_in(window, OVERLAY_MS);
    } else {
        fade_out(window, OVERLAY_MS);
    }
}

/// Bereken de zichtbare rij-breedte aan de hand van een fractie (0..1) en de
/// track-breedte in pixels — voor de usage-bars.
pub fn fill_width_px(frac: f64, track_width: i32) -> i32 {
    let frac = frac.clamp(0.0, 1.0);
    (track_width.max(0) as f64 * frac).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timings_blijven_op_de_devin_ladder() {
        let ladder = [DUR_FAST_MS, DUR_MED_MS, DUR_SLOW_MS];
        for (naam, ms) in [
            ("PRESS_MS", PRESS_MS),
            ("HOVER_MS", HOVER_MS),
            ("PANEL_MS", PANEL_MS),
            ("DRAWER_MS", DRAWER_MS),
            ("OVERLAY_MS", OVERLAY_MS),
        ] {
            assert!(
                ladder.contains(&ms),
                "{naam} = {ms}ms staat niet op de ladder 140/280/420"
            );
        }
    }

    #[test]
    fn ladder_matcht_tokens_css() {
        assert_eq!((DUR_FAST_MS, DUR_MED_MS, DUR_SLOW_MS), (140, 280, 420));
    }

    #[test]
    fn fill_width_clamps() {
        assert_eq!(fill_width_px(0.5, 100), 50);
        assert_eq!(fill_width_px(2.0, 100), 100);
        assert_eq!(fill_width_px(-1.0, 100), 0);
    }
}
