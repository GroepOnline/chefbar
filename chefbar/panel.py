"""Rich GTK3 mission-control panel for ChefBar 2.0."""

from __future__ import annotations

import logging
import os
import subprocess
import threading
from pathlib import Path

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk, Pango  # noqa: E402

from . import api, security
from .endpoints import PROFILE

log = logging.getLogger("chefbar.panel")

PACKAGE_DIR = Path(__file__).resolve().parent
STYLES_PATH = PACKAGE_DIR / "styles.css"
DARK_STYLES_PATH = PACKAGE_DIR / "styles-dark.css"

# GTK3 negeert `@media (prefers-reduced-motion: reduce)`; het contract loopt via
# de GTK-instelling gtk-enable-animations.
REDUCED_MOTION_CSS = b"* { transition-duration: 1ms; animation-duration: 1ms; }"

_POLICY = security.POLICY.with_profile_hosts(*PROFILE.all_urls())
DASHBOARD = PROFILE.dashboard
DESKTOP_URL = PROFILE.desktop
PANEL_REFRESH = int(os.environ.get("CHEFBAR_PANEL_REFRESH", "30"))
PANEL_WIDTH = int(os.environ.get("CHEFBAR_PANEL_WIDTH", "380"))


def load_css() -> None:
    provider = Gtk.CssProvider()
    try:
        provider.load_from_path(str(STYLES_PATH))
    except GLib.Error as exc:
        log.warning("CSS laden faalde: %s", exc)
        return
    Gtk.StyleContext.add_provider_for_screen(
        Gdk.Screen.get_default(),
        provider,
        Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
    )
    settings = Gtk.Settings.get_default()
    prefer_dark = bool(
        settings
        and settings.get_property("gtk-application-prefer-dark-theme")
    )
    theme_name = str(settings.get_property("gtk-theme-name") or "") if settings else ""
    if prefer_dark or "dark" in theme_name.lower():
        dark_provider = Gtk.CssProvider()
        try:
            dark_provider.load_from_path(str(DARK_STYLES_PATH))
            Gtk.StyleContext.add_provider_for_screen(
                Gdk.Screen.get_default(),
                dark_provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            )
        except GLib.Error as exc:
            log.warning("Donker thema laden faalde: %s", exc)
    if settings and not settings.get_property("gtk-enable-animations"):
        motion_provider = Gtk.CssProvider()
        try:
            motion_provider.load_from_data(REDUCED_MOTION_CSS)
            Gtk.StyleContext.add_provider_for_screen(
                Gdk.Screen.get_default(),
                motion_provider,
                Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
            )
        except GLib.Error as exc:
            log.warning("Reduced motion laden faalde: %s", exc)


def notify(title: str, body: str, status: str = "info") -> None:
    home = str(Path.home())
    for cmd in (
        [f"{home}/.local/bin/joep-notify", "-s", "ops", "-S", status, "--", title, body],
        ["/usr/bin/notify-send", "--app-name=ChefBar", "--", title, body],
    ):
        try:
            subprocess.run(cmd, check=False, timeout=10, capture_output=True)
            return
        except (OSError, subprocess.TimeoutExpired):
            continue


def open_url(url: str) -> None:
    try:
        security.open_browser_url(url, policy=_POLICY)
    except (OSError, ValueError) as exc:
        log.warning("URL openen geweigerd: %s (%s)", url, exc)


def _dot(level: str = "ok", pulse: bool = False) -> Gtk.Label:
    lab = Gtk.Label(label="")
    lab.set_size_request(3, 18)
    ctx = lab.get_style_context()
    ctx.add_class("chefbar-dot")
    ctx.add_class(level if level in ("ok", "warn", "down", "info") else "ok")
    if pulse:
        ctx.add_class("pulse")
    return lab


def _label(text: str, css_class: str, ellipsize: bool = False) -> Gtk.Label:
    lab = Gtk.Label(label=text, xalign=0)
    lab.get_style_context().add_class(css_class)
    if ellipsize:
        lab.set_ellipsize(Pango.EllipsizeMode.END)
        lab.set_max_width_chars(36)
    return lab


class UsageBar(Gtk.Box):
    def __init__(self) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        self.track = Gtk.Box()
        self.track.set_hexpand(True)
        self.track.get_style_context().add_class("chefbar-bar-track")
        self.fill = Gtk.Box()
        self.fill.get_style_context().add_class("chefbar-bar-fill")
        self.track.pack_start(self.fill, False, False, 0)
        self.pack_start(self.track, True, True, 0)
        self._level = "ok"

    def set_fraction(self, frac: float, level: str = "ok") -> None:
        frac = max(0.0, min(float(frac), 1.0))
        width = max(4, int((PANEL_WIDTH - 60) * frac)) if frac > 0 else 0
        self.fill.set_size_request(width, 6)
        ctx = self.fill.get_style_context()
        for cls in ("ok", "warn", "down"):
            ctx.remove_class(cls)
        ctx.add_class(level if level in ("ok", "warn", "down") else "ok")
        self._level = level
        self.track.show_all()


class ChefBarPanel:
    """Compact mission-control window opened from the tray indicator."""

    def __init__(self, on_quit=None, on_health_change=None) -> None:
        load_css()
        self.on_quit = on_quit
        self.on_health_change = on_health_change
        self.snapshot = api.Snapshot()
        self._refreshing = False
        self._open = False
        self._refresh_timer = None
        self._build_window()

    # -- window chrome -----------------------------------------------------

    def _build_window(self) -> None:
        self.window = Gtk.Window(type=Gtk.WindowType.TOPLEVEL)
        self.window.set_title("ChefBar")
        self.window.set_decorated(False)
        self.window.set_skip_taskbar_hint(True)
        self.window.set_skip_pager_hint(True)
        self.window.set_keep_above(True)
        self.window.set_resizable(False)
        self.window.set_default_size(PANEL_WIDTH, 520)
        self.window.set_type_hint(Gdk.WindowTypeHint.UTILITY)
        self.window.stick()
        self.window.get_style_context().add_class("chefbar-panel")
        self.window.connect("delete-event", self._on_delete)
        self.window.connect("key-press-event", self._on_key)
        self.window.connect("focus-out-event", self._on_focus_out)

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.get_style_context().add_class("chefbar-root")
        self.window.add(outer)

        # Header
        header = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        header.get_style_context().add_class("chefbar-header")
        title_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.health_dot = _dot("ok")
        title_row.pack_start(self.health_dot, False, False, 0)
        title_col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.title_lab = _label("ChefBar", "chefbar-title")
        self.subtitle_lab = _label("Nog niet ververst", "chefbar-subtitle")
        title_col.pack_start(self.title_lab, False, False, 0)
        title_col.pack_start(self.subtitle_lab, False, False, 0)
        title_row.pack_start(title_col, True, True, 0)
        header.pack_start(title_row, False, False, 0)
        outer.pack_start(header, False, False, 0)

        # Scrollable body
        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_min_content_height(280)
        scroll.set_max_content_height(420)
        scroll.set_propagate_natural_height(True)
        self.body = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        scroll.add(self.body)
        outer.pack_start(scroll, True, True, 0)

        # Sections containers (rebuilt on refresh)
        self.providers_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.agents_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.events_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.fleet_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.body.pack_start(_label("Accounts", "chefbar-section-label"), False, False, 0)
        self.body.pack_start(self.providers_box, False, False, 0)
        self.body.pack_start(_label("Agents", "chefbar-section-label"), False, False, 0)
        self.body.pack_start(self.agents_box, False, False, 0)
        self.body.pack_start(_label("Recent", "chefbar-section-label"), False, False, 0)
        self.body.pack_start(self.events_box, False, False, 0)
        self.body.pack_start(_label("Verbinding", "chefbar-section-label"), False, False, 0)
        self.body.pack_start(self.fleet_box, False, False, 0)

        # Quick actions
        outer.pack_start(_label("Direct doen", "chefbar-section-label"), False, False, 0)
        actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        actions.get_style_context().add_class("chefbar-actions")
        actions.set_homogeneous(True)
        for label, handler, primary in (
            ("Dashboard", self._act_dashboard, False),
            ("Desktop", self._act_desktop, False),
            ("HUD", self._act_hud, False),
            ("Ververs", self._act_refresh, False),
            ("Agent", self._act_agent_task, True),
        ):
            btn = Gtk.Button(label=label)
            if primary:
                btn.get_style_context().add_class("chefbar-primary")
            btn.connect("clicked", handler)
            actions.pack_start(btn, True, True, 0)
        outer.pack_start(actions, False, False, 0)

        footer_row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        self.footer_lab = _label("", "chefbar-footer")
        footer_row.pack_start(self.footer_lab, True, True, 0)
        quit_btn = Gtk.Button(label="Afsluiten")
        quit_btn.get_style_context().add_class("chefbar-switch-btn")
        quit_btn.connect("clicked", self._act_quit)
        footer_row.pack_end(quit_btn, False, False, 0)
        outer.pack_start(footer_row, False, False, 0)

        self.window.show_all()
        self.window.hide()

    # -- show / hide -------------------------------------------------------

    def is_open(self) -> bool:
        return self._open

    def toggle(self) -> None:
        if self._open:
            self.hide()
        else:
            self.show()

    def show(self, near_pointer: bool = True) -> None:
        # Paint cache immediately, refresh async.
        self._render(self.snapshot)
        self._position(near_pointer=near_pointer)
        self.window.present()
        self.window.grab_focus()
        self._open = True
        self.refresh_async()
        self._arm_auto_refresh(True)

    def hide(self) -> None:
        self.window.hide()
        self._open = False
        self._arm_auto_refresh(False)

    def _on_delete(self, *_args):
        self.hide()
        return True

    def _on_key(self, _w, event):
        if event.keyval == Gdk.KEY_Escape:
            self.hide()
            return True
        ctrl = bool(event.state & Gdk.ModifierType.CONTROL_MASK)
        if event.keyval == Gdk.KEY_F5 or (ctrl and event.keyval in (Gdk.KEY_r, Gdk.KEY_R)):
            self.refresh_async(force=True)
            return True
        return False

    def _on_focus_out(self, *_args):
        # Close on click-away; delay slightly so button clicks inside register.
        GLib.timeout_add(160, self._maybe_hide_on_focus)
        return False

    def _maybe_hide_on_focus(self) -> bool:
        if self._open and not self.window.has_toplevel_focus():
            # Keep open if a dialog/popover is up
            for w in Gtk.Window.list_toplevels():
                if w.get_visible() and w is not self.window and w.get_type_hint() in (
                    Gdk.WindowTypeHint.DIALOG,
                    Gdk.WindowTypeHint.POPUP_MENU,
                ):
                    return False
            self.hide()
        return False

    def _position(self, near_pointer: bool = True) -> None:
        """Anchor top-right against the GNOME bar, like a shell menu.

        Requires GDK_BACKEND=x11 (XWayland): Wayland toplevels cannot be
        positioned, X11 clients can. The AppIndicator tray icon lives in the
        top-right corner, so the panel drops directly beneath it. The
        monitor workarea already excludes the top bar, so workarea.y is the
        exact bar height.
        """
        display = self.window.get_display()
        try:
            seat = display.get_default_seat()
            pointer = seat.get_pointer()
            _screen, px_x, px_y, _mods = pointer.get_position()
            monitor = display.get_monitor_at_point(px_x, px_y)
        except Exception:  # noqa: BLE001
            monitor = None
        if monitor is None:
            monitor = display.get_primary_monitor() or display.get_monitor(0)
        self.window.set_size_request(PANEL_WIDTH, -1)
        self.window.resize(PANEL_WIDTH, 1)
        self.window.show_all()
        if monitor is None:
            return
        wa = monitor.get_workarea()
        w = PANEL_WIDTH
        px = wa.x + wa.width - w
        py = wa.y
        self.window.move(int(px), int(py))

    # -- data refresh ------------------------------------------------------

    def refresh_async(self, force: bool = False) -> None:
        if self._refreshing and not force:
            return
        self._refreshing = True
        self.footer_lab.set_text("Even ophalen")

        def worker() -> None:
            snap = api.fetch_snapshot()
            GLib.idle_add(self._on_snapshot, snap)

        threading.Thread(target=worker, daemon=True).start()

    def seed_cache(self, snap: api.Snapshot) -> None:
        self.snapshot = snap
        if self.on_health_change:
            self.on_health_change(snap)

    def _on_snapshot(self, snap: api.Snapshot) -> bool:
        self._refreshing = False
        self.snapshot = snap
        self._render(snap)
        if self.on_health_change:
            self.on_health_change(snap)
        return False

    def _arm_auto_refresh(self, enabled: bool) -> None:
        if self._refresh_timer is not None:
            GLib.source_remove(self._refresh_timer)
            self._refresh_timer = None
        if enabled:
            self._refresh_timer = GLib.timeout_add_seconds(
                PANEL_REFRESH, self._auto_refresh_tick
            )

    def _auto_refresh_tick(self) -> bool:
        if not self._open:
            self._refresh_timer = None
            return False
        self.refresh_async()
        return True

    # -- render ------------------------------------------------------------

    def _render(self, snap: api.Snapshot) -> None:
        health = snap.health
        score = snap.day_score
        self.title_lab.set_text(health.line)
        self.subtitle_lab.set_text(score.line)
        for cls in ("ok", "warn", "down", "info", "pulse"):
            self.health_dot.get_style_context().remove_class(cls)
        self.health_dot.get_style_context().add_class("chefbar-dot")
        self.health_dot.get_style_context().add_class(health.level)

        self._clear(self.providers_box)
        if not snap.providers:
            meta = snap.error or "Nog geen providerdata · Ververs probeert opnieuw"
            self.providers_box.pack_start(
                self._card_text("Geen contact met de Vault-API", meta)
                if snap.error
                else self._card_text("Nog geen providers", meta),
                False,
                False,
                0,
            )
        for row in snap.providers:
            self.providers_box.pack_start(self._provider_card(row), False, False, 0)

        self._clear(self.agents_box)
        if not snap.agents:
            self.agents_box.pack_start(
                self._card_text("Nog niks gebeurd vandaag", "Start een agent via de knop hieronder"),
                False,
                False,
                0,
            )
        for agent in snap.agents:
            self.agents_box.pack_start(self._agent_card(agent), False, False, 0)

        self._clear(self.events_box)
        if not snap.events:
            self.events_box.pack_start(
                self._card_text("Nog geen recente updates", "Agentgebeurtenissen verschijnen hier"),
                False,
                False,
                0,
            )
        for event in snap.events[:5]:
            agent = str(event.get("agent") or "Agent")
            summary = str(event.get("summary") or event.get("kind") or "update")
            workspace = str(event.get("workspace") or "")
            self.events_box.pack_start(
                self._card_text(f"{agent} · {summary[:64]}", workspace or "recente update"),
                False,
                False,
                0,
            )

        self._clear(self.fleet_box)
        fleet = snap.fleet
        stale = " · verouderd" if fleet.stale else ""
        fleet_card = Gtk.EventBox()
        inner = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        inner.get_style_context().add_class("chefbar-card")
        if fleet.total:
            level = "ok" if fleet.online == fleet.total else (
                "warn" if fleet.online else "down"
            )
            title = f"{fleet.online}/{fleet.total} nodes online{stale}"
        else:
            level = "info"
            title = "Nog geen fleet-data"
        inner.pack_start(_dot(level), False, False, 0)
        col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1)
        col.pack_start(_label(title, "chefbar-card-title"), False, False, 0)
        host = fleet.host or "tailnet"
        col.pack_start(_label(f"{host} · klik → dashboard Fleet", "chefbar-card-meta"), False, False, 0)
        inner.pack_start(col, True, True, 0)
        fleet_card.add(inner)
        fleet_card.connect("button-press-event", lambda *_: self._act_fleet())
        self.fleet_box.pack_start(fleet_card, False, False, 0)

        ts = snap.fetched_at.strftime("%H:%M") if snap.fetched_at else "—"
        if snap.error:
            self.footer_lab.set_text(f"{snap.error} · {ts}")
        else:
            self.footer_lab.set_text(f"Vers · {ts}")
        self.window.show_all()
        if not self._open:
            self.window.hide()

    def _clear(self, box: Gtk.Box) -> None:
        for child in box.get_children():
            box.remove(child)

    def _card_text(self, title: str, meta: str) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        box.get_style_context().add_class("chefbar-card")
        box.pack_start(_label(title, "chefbar-card-title"), False, False, 0)
        box.pack_start(_label(meta, "chefbar-card-meta", ellipsize=True), False, False, 0)
        return box

    def _provider_card(self, row: api.ProviderRow) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.get_style_context().add_class("chefbar-card")
        top = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        top.pack_start(_dot("ok" if row.active_label else "warn"), False, False, 0)
        title = f"{row.label}"
        if row.active_label:
            title += f" · {row.active_label}"
        top.pack_start(_label(title, "chefbar-card-title", ellipsize=True), True, True, 0)
        if row.accounts:
            sw = Gtk.Button(label="wissel")
            sw.get_style_context().add_class("chefbar-switch-btn")
            sw.connect("clicked", self._on_switch_clicked, row)
            top.pack_end(sw, False, False, 0)
        box.pack_start(top, False, False, 0)
        if row.usage_text:
            box.pack_start(
                _label(row.usage_text, "chefbar-card-meta", ellipsize=True),
                False,
                False,
                0,
            )
        if row.requests is not None or row.tokens is not None:
            bar = UsageBar()
            bar.set_fraction(row.usage_frac, row.usage_level)
            box.pack_start(bar, False, False, 0)
        return box

    def _agent_card(self, agent: api.AgentRow) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        box.get_style_context().add_class("chefbar-card")
        level = "ok" if agent.running else ("warn" if agent.status != "done" else "info")
        box.pack_start(_dot(level, pulse=agent.running), False, False, 0)
        col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1)
        col.pack_start(
            _label(f"{agent.agent} · {agent.workspace}", "chefbar-card-title", ellipsize=True),
            False,
            False,
            0,
        )
        meta = agent.summary or agent.status
        col.pack_start(_label(meta, "chefbar-card-meta", ellipsize=True), False, False, 0)
        box.pack_start(col, True, True, 0)
        return box

    # -- actions -----------------------------------------------------------

    def _on_switch_clicked(self, btn: Gtk.Button, row: api.ProviderRow) -> None:
        pop = Gtk.Popover()
        pop.set_relative_to(btn)
        pop.get_style_context().add_class("chefbar-popover")
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
        vbox.set_margin_top(6)
        vbox.set_margin_bottom(6)
        vbox.set_margin_start(6)
        vbox.set_margin_end(6)
        for acc in row.accounts:
            mark = "■" if acc.get("id") == row.active_id else "□"
            label = f"{mark} {acc.get('label') or acc.get('id')}"
            b = Gtk.Button(label=label)
            b.set_relief(Gtk.ReliefStyle.NONE)
            b.set_halign(Gtk.Align.START)
            b.connect("clicked", self._do_switch, acc, row, pop)
            vbox.pack_start(b, False, False, 0)
        pop.add(vbox)
        vbox.show_all()
        pop.popup()

    def _do_switch(
        self,
        _btn,
        acc: dict,
        row: api.ProviderRow,
        pop: Gtk.Popover,
    ) -> None:
        pop.popdown()
        acc_id = acc.get("id")
        label = acc.get("label") or acc_id

        def worker() -> None:
            result = api.switch_account(
                str(acc_id),
                row.source,
                self.snapshot.revision,
                driver=row.driver,
            )
            ok = result is not None
            GLib.idle_add(self._after_switch, label, ok)

        threading.Thread(target=worker, daemon=True).start()

    def _after_switch(self, label, ok: bool) -> bool:
        if ok:
            notify("Je werkt nu als " + str(label), "", status="ok")
        else:
            notify("Wisselen lukte niet", f"{label} · probeer opnieuw", status="error")
        self.refresh_async(force=True)
        return False

    def _act_dashboard(self, *_a) -> None:
        open_url(DASHBOARD)
        self.hide()

    def _act_desktop(self, *_a) -> None:
        open_url(DESKTOP_URL)

    def _act_hud(self, *_a) -> None:
        try:
            subprocess.Popen(
                ["chef-hud"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except OSError as exc:
            notify("ChefBar", f"chef-hud startte niet: {exc}")
        self.hide()

    def _act_refresh(self, *_a) -> None:
        self.refresh_async(force=True)

    def _act_fleet(self, *_a) -> None:
        open_url(f"{DASHBOARD.rstrip('/')}/#fleet")
        self.hide()
        return False

    def _act_quit(self, *_a) -> None:
        if self.on_quit:
            self.on_quit()
        else:
            Gtk.main_quit()

    def _act_agent_task(self, *_a) -> None:
        dialog = Gtk.Dialog(
            title="Nieuwe agent-taak",
            transient_for=self.window,
            modal=True,
            flags=0,
        )
        dialog.get_style_context().add_class("chefbar-dialog")
        dialog.add_buttons(
            "Annuleer",
            Gtk.ResponseType.CANCEL,
            "Start",
            Gtk.ResponseType.OK,
        )
        dialog.set_default_response(Gtk.ResponseType.OK)
        box = dialog.get_content_area()
        box.set_spacing(8)
        box.set_margin_top(12)
        box.set_margin_bottom(12)
        box.set_margin_start(12)
        box.set_margin_end(12)
        box.add(Gtk.Label(label="Wat moet er gebeuren?", xalign=0))
        entry = Gtk.Entry()
        entry.set_activates_default(True)
        entry.set_placeholder_text("bijv. check CI op chefgroep-vault PR")
        box.add(entry)
        agent_store = Gtk.ListStore(str)
        for name in ("cursor", "codex", "cline"):
            agent_store.append([name])
        combo = Gtk.ComboBox.new_with_model(agent_store)
        renderer = Gtk.CellRendererText()
        combo.pack_start(renderer, True)
        combo.add_attribute(renderer, "text", 0)
        combo.set_active(0)
        box.add(Gtk.Label(label="Agent type:", xalign=0))
        box.add(combo)
        dialog.show_all()
        # Avoid focus-out closing the panel while dialog is open
        prev_open = self._open
        response = dialog.run()
        prompt = entry.get_text().strip()
        iter_ = combo.get_active_iter()
        agent_type = "cursor"
        if iter_ is not None:
            agent_type = agent_store[iter_][0]
        dialog.destroy()
        self._open = prev_open
        if response != Gtk.ResponseType.OK or not prompt:
            return

        def worker() -> None:
            result = api.create_commander_task(prompt, agent_type=agent_type)
            GLib.idle_add(self._after_task, prompt, result)

        threading.Thread(target=worker, daemon=True).start()

    def _after_task(self, prompt: str, result) -> bool:
        if result is None:
            notify("Taak starten lukte niet", "zie chefbar.log", status="error")
        else:
            tid = result.get("id") or "?"
            notify("Agent aan de slag", f"{tid} · {prompt[:60]}", status="ok")
        self.refresh_async(force=True)
        return False


def run_panel_only() -> None:
    """Testmodus: alleen het panel tonen (`chefbar --show-panel`)."""
    load_css()
    panel = ChefBarPanel(on_quit=Gtk.main_quit)

    def boot() -> bool:
        panel.show(near_pointer=False)
        return False

    GLib.idle_add(boot)
    Gtk.main()
