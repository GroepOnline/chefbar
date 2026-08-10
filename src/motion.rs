//! Signaal motion helpers voor ChefBar (GTK3).
//!
//! DESIGN.md: 100ms press, 180ms hover/select, 280ms panel/dialog.
//! Alleen opacity-fades: geen bounce, pulse-loops of scroll-theater.
//! Respecteert gtk-enable-animations (reduced-motion proxy).

use gtk::prelude::*;
use glib::ControlFlow;
use std::collections::HashMap;
use std::sync::Mutex;

pub const PRESS_MS: u32 = 100;
pub const HOVER_MS: u32 = 180;
pub const PANEL_MS: u32 = 280;
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
    GENERATIONS.lock().unwrap().get(&window_key(window)).copied() == Some(generation)
}

pub fn motion_enabled() -> bool {
    gtk::Settings::default()
        .and_then(|settings| Some(settings.property::<bool>("gtk-enable-animations")))
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
    glib::timeout_add_local(std::time::Duration::from_millis(step_ms as u64), move || {
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
    });
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
    glib::timeout_add_local(std::time::Duration::from_millis(step_ms as u64), move || {
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
    });
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
    fn fill_width_clamps() {
        assert_eq!(fill_width_px(0.5, 100), 50);
        assert_eq!(fill_width_px(2.0, 100), 100);
        assert_eq!(fill_width_px(-1.0, 100), 0);
    }
}