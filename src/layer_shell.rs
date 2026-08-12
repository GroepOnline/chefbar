//! Wayland layer-shell (E2): het paneel als echte compositor-laag (top-right)
//! in plaats van een X11/XWayland-venster.
//!
//! Feature-gated: `layer-shell` (default **uit**). De systeembibliotheek
//! `libgtk-layer-shell` (GTK3) is een build- én runtime-eis — alleen
//! inschakelen op machines waar die geïnstalleerd is (`apt install
//! libgtk-layer-shell0`). Zonder feature, zonder Wayland of zonder
//! protocol-ondersteuning → fallback = het bestaande X11-gedrag; de caller
//! verandert dan niets.

/// Pas layer-shell toe op het paneel-venster.
///
/// Returns `true` als het paneel als laag draait (de caller mag dan de
/// X11-positionering zoals `set_position(Center)` overslaan), `false` =
/// fallback naar het bestaande gedrag.
pub fn apply(window: &gtk::Window) -> bool {
    #[cfg(feature = "layer-shell")]
    {
        apply_layered(window)
    }
    #[cfg(not(feature = "layer-shell"))]
    {
        let _ = window;
        false
    }
}

#[cfg(feature = "layer-shell")]
fn apply_layered(window: &gtk::Window) -> bool {
    use gtk_layer_shell::LayerShell;
    // Alleen op echte Wayland-sessies met layer-shell-protocol; op X11/XWayland
    // (of zonder compositor-ondersteuning) netjes terugvallen.
    let on_wayland = std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !on_wayland || !gtk_layer_shell::is_supported() {
        return false;
    }
    // Moet vóór realisatie/mapping; de setters eronder configureren de laag.
    window.init_layer_shell();
    // Top-laag, rechtsboven geankerd met marge; exclusive zone zodat de laag
    // ruimte claimt; exclusive keyboard zodat Esc/slash blijven werken.
    window.set_layer(gtk_layer_shell::Layer::Top);
    window.set_anchor(gtk_layer_shell::Edge::Top, true);
    window.set_anchor(gtk_layer_shell::Edge::Right, true);
    window.set_layer_shell_margin(gtk_layer_shell::Edge::Top, 8);
    window.set_layer_shell_margin(gtk_layer_shell::Edge::Right, 8);
    window.auto_exclusive_zone_enable();
    window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::Exclusive);
    true
}
