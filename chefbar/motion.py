"""Signaal motion helpers for ChefBar (GTK3).

DESIGN.md: 100ms press, 180ms hover/select, 280ms panel/dialog.
Only opacity fades here — no bounce, pulse loops, or scroll theater.
Honors GtkSettings gtk-enable-animations (reduced-motion proxy).
"""

from __future__ import annotations

import weakref
from typing import TYPE_CHECKING, Any, Callable

# Signaal motion scale (ms)
PRESS_MS = 100
HOVER_MS = 180
PANEL_MS = 280

_FADE_STEPS = 8

# Per-window animation generation. Starting any fade bumps the token so
# timeouts from an earlier fade become stale and stop touching the window
# (e.g. a fade-out tick must not hide a window reopened by fade_in).
_generations: weakref.WeakKeyDictionary[Any, int] = weakref.WeakKeyDictionary()

if TYPE_CHECKING:
    from gi.repository import Gtk


def _gtk() -> tuple[Any, Any]:
    import gi

    gi.require_version("Gtk", "3.0")
    from gi.repository import GLib, Gtk

    return GLib, Gtk


def motion_enabled() -> bool:
    """Return False when the desktop asks for reduced / no animations."""
    _GLib, Gtk = _gtk()
    settings = Gtk.Settings.get_default()
    if settings is None:
        return True
    try:
        return bool(settings.get_property("gtk-enable-animations"))
    except Exception:  # noqa: BLE001
        return True


def _begin(window: Any) -> int:
    """Invalidate any in-flight fade on ``window`` and return a fresh token."""
    try:
        generation = _generations.get(window, 0) + 1
        _generations[window] = generation
    except TypeError:  # window not weak-referenceable / hashable
        return 0
    return generation


def _is_current(window: Any, generation: int) -> bool:
    try:
        return _generations.get(window, 0) == generation
    except TypeError:
        return True


def fade_in(window: Gtk.Window, *, duration_ms: int = PANEL_MS) -> None:
    """Show window with a short opacity ramp (or instantly if motion off).

    Caller may already have called ``show_all`` for layout; we still ramp
    opacity from 0 → 1 when animations are enabled.
    """
    GLib, _Gtk = _gtk()
    generation = _begin(window)
    if not motion_enabled() or duration_ms <= 0:
        window.set_opacity(1.0)
        window.show()
        return

    window.set_opacity(0.0)
    window.show()
    step_ms = max(1, duration_ms // _FADE_STEPS)
    state = {"i": 0}

    def tick() -> bool:
        if not _is_current(window, generation):
            return False
        state["i"] += 1
        frac = min(1.0, state["i"] / _FADE_STEPS)
        window.set_opacity(frac)
        if frac >= 1.0:
            window.set_opacity(1.0)
            return False
        return True

    GLib.timeout_add(step_ms, tick)


def fade_out(
    window: Gtk.Window,
    *,
    duration_ms: int = PANEL_MS,
    on_hidden: Callable[[], None] | None = None,
) -> None:
    """Hide window after a short opacity ramp (or instantly if motion off)."""
    GLib, _Gtk = _gtk()
    generation = _begin(window)

    def finish() -> None:
        window.hide()
        window.set_opacity(1.0)
        if on_hidden is not None:
            on_hidden()

    if not motion_enabled() or duration_ms <= 0:
        finish()
        return

    step_ms = max(1, duration_ms // _FADE_STEPS)
    state = {"i": 0}

    def tick() -> bool:
        if not _is_current(window, generation):
            # A newer fade (usually a reopen) owns the window now.
            return False
        state["i"] += 1
        frac = max(0.0, 1.0 - state["i"] / _FADE_STEPS)
        window.set_opacity(frac)
        if frac <= 0.0:
            finish()
            return False
        return True

    GLib.timeout_add(step_ms, tick)
