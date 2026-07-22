"""Ayatana AppIndicator tray host for ChefBar 2.0."""

from __future__ import annotations

import logging
import os
import subprocess
import threading
from pathlib import Path

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("AyatanaAppIndicator3", "0.1")
from gi.repository import AyatanaAppIndicator3 as AppIndicator  # noqa: E402
from gi.repository import GLib, Gtk  # noqa: E402

from . import api, ipc, ops
from .bar import CommandBar
from .panel import ChefBarPanel, open_url

log = logging.getLogger("chefbar.indicator")

ICONS_DIR = Path(__file__).resolve().parent / "icons"
BACKGROUND_REFRESH = int(os.environ.get("CHEFBAR_REFRESH", "60"))
DASHBOARD = os.environ.get("CHEFBAR_DASHBOARD", "http://127.0.0.1:8080")
DESKTOP_URL = os.environ.get("CHEFBAR_DESKTOP", "http://127.0.0.1:3000")

TRAY_STATES = ("stil", "bezig", "hulp", "fout", "offline")


def _run_bg(cmd: list[str]) -> None:
    def worker() -> None:
        try:
            subprocess.run(cmd, check=False, timeout=30, capture_output=True)
        except (OSError, subprocess.TimeoutExpired) as exc:
            log.warning("%s faalde: %s", cmd[0], exc)

    threading.Thread(target=worker, daemon=True).start()


class ChefBarApp:
    def __init__(self) -> None:
        self._state = "stil"
        self.panel = ChefBarPanel(
            on_quit=self.quit,
            on_health_change=self._on_snapshot_change,
        )
        self.watcher = ops.Watcher()
        self.watcher.start()
        self.bar = CommandBar(
            watcher=self.watcher,
            get_vault=lambda: self.panel.snapshot,
        )
        self.ipc = ipc.IpcServer(
            handlers={
                "bar": self.bar.toggle,
                "panel": self.panel.toggle,
            },
            dispatch=GLib.idle_add,
        )
        try:
            self.ipc.start()
        except OSError:
            log.exception("IPC-socket starten faalde")
        self.indicator = AppIndicator.Indicator.new(
            "chefbar",
            self._icon_path("stil"),
            AppIndicator.IndicatorCategory.APPLICATION_STATUS,
        )
        self.indicator.set_status(AppIndicator.IndicatorStatus.ACTIVE)
        self.indicator.set_title("ChefGroep · nog stil in de keuken")

        self.menu = self._build_menu()
        self.indicator.set_menu(self.menu)

        # Warm cache in background so first panel open is <300ms.
        threading.Thread(target=self._prefetch, daemon=True).start()
        GLib.timeout_add_seconds(BACKGROUND_REFRESH, self._bg_tick)
        log.info("ChefBar 2.0 gestart (bg refresh %ss)", BACKGROUND_REFRESH)

    # -- menu: de bonnenstrook -------------------------------------------------

    def _build_menu(self) -> Gtk.Menu:
        menu = Gtk.Menu()

        # Bovenin max 3 live bonregels; gevuld door _rebuild_tickets.
        self.ticket_items: list[Gtk.MenuItem] = []
        for _ in range(3):
            it = Gtk.MenuItem(label="")
            it.set_no_show_all(True)
            it.hide()
            menu.append(it)
            self.ticket_items.append(it)
        self.ticket_sep = Gtk.SeparatorMenuItem()
        self.ticket_sep.set_no_show_all(True)
        self.ticket_sep.hide()
        menu.append(self.ticket_sep)

        bar_item = Gtk.MenuItem(label="Bar (Super+Space)")
        bar_item.connect("activate", lambda *_: self.bar.show())
        menu.append(bar_item)

        thuis_item = Gtk.MenuItem(label="Open Thuis")
        thuis_item.connect("activate", lambda *_: open_url(DASHBOARD))
        menu.append(thuis_item)
        try:
            self.indicator.set_secondary_activate_target(thuis_item)
        except Exception:  # noqa: BLE001 — optional API
            pass

        ploeg_item = Gtk.MenuItem(label="Open Ploeg")
        ploeg_item.connect(
            "activate", lambda *_: open_url(f"{DASHBOARD.rstrip('/')}/#agents")
        )
        menu.append(ploeg_item)

        panel_item = Gtk.MenuItem(label="Open de pas (panel)")
        panel_item.connect("activate", lambda *_: self.panel.show())
        menu.append(panel_item)

        menu.append(Gtk.SeparatorMenuItem())

        self.accounts_item = Gtk.MenuItem(label="Pas")
        self.accounts_item.set_submenu(Gtk.Menu())
        menu.append(self.accounts_item)

        desktop_item = Gtk.MenuItem(label="Desktop starten")
        desktop_item.connect("activate", self._on_desktop)
        menu.append(desktop_item)

        menu.append(Gtk.SeparatorMenuItem())

        pause_item = Gtk.MenuItem(label="Notificaties pauzeren")
        pause_menu = Gtk.Menu()
        for label, args in (
            ("1 uur", ["pause", "1h"]),
            ("Vanavond", ["pause", "vanavond"]),
            ("Hervatten", ["resume"]),
        ):
            it = Gtk.MenuItem(label=label)
            it.connect("activate", lambda _w, a=args: _run_bg(["joep-notify", *a]))
            pause_menu.append(it)
        pause_item.set_submenu(pause_menu)
        menu.append(pause_item)

        self.autostart_item = Gtk.CheckMenuItem(label="Meelopen vanaf login")
        self.autostart_item.set_active(self._autostart_enabled())
        self.autostart_item.connect("toggled", self._on_autostart_toggle)
        menu.append(self.autostart_item)

        quit_item = Gtk.MenuItem(label="Afsluiten")
        quit_item.connect("activate", lambda *_: self.quit())
        menu.append(quit_item)

        menu.show_all()
        return menu

    def _on_desktop(self, *_a) -> None:
        _run_bg(["chefvault", "desktop", "start"])
        open_url(DESKTOP_URL)

    def _rebuild_tickets(self, snap: api.Snapshot) -> None:
        rows = snap.agents[:3]
        for it, agent in zip(self.ticket_items, rows):
            if agent.running:
                text = f"{agent.agent} werkt in {agent.workspace}"
            elif agent.status in api.FAILED_STATUSES:
                text = f"{agent.agent} hapert in {agent.workspace}"
            else:
                text = f"{agent.agent} is klaar in {agent.workspace}"
            it.set_label(text)
            try:
                it.disconnect_by_func(self._on_ticket)
            except TypeError:
                pass
            it.connect("activate", self._on_ticket, agent)
            it.show()
        for it in self.ticket_items[len(rows):]:
            it.hide()
        self.ticket_sep.set_visible(bool(rows))

    def _on_ticket(self, _item, agent: api.AgentRow) -> None:
        _run_bg(["joep-ops", "focus", "--agent", agent.agent])

    def _rebuild_accounts(self, snap: api.Snapshot) -> None:
        active = next(
            (r.active_label for r in snap.providers if r.active_label), None
        )
        self.accounts_item.set_label(f"Pas: {active}" if active else "Pas")
        sub = Gtk.Menu()
        rows = [r for r in snap.providers if r.accounts]
        if not rows:
            empty = Gtk.MenuItem(label="Nog geen ploegpassen")
            empty.set_sensitive(False)
            sub.append(empty)
        for row in rows:
            head = Gtk.MenuItem(label=row.label)
            head.set_sensitive(False)
            sub.append(head)
            for acc in row.accounts:
                active = acc.get("id") == row.active_id
                mark = "●" if active else "○"
                label = f"  {mark} {acc.get('label') or acc.get('id')}"
                it = Gtk.MenuItem(label=label)
                if active:
                    it.set_sensitive(False)
                else:
                    it.connect("activate", self._on_switch, acc)
                sub.append(it)
        sub.show_all()
        self.accounts_item.set_submenu(sub)

    def _on_switch(self, _item, acc: dict) -> None:
        acc_id = str(acc.get("id"))
        label = acc.get("label") or acc_id

        def worker() -> None:
            ok = api.switch_account(acc_id) is not None
            msg = ["-s", "ops", "-S", "ok", "Je werkt nu als " + label] if ok else [
                "-s", "ops", "-S", "error", "Wisselen naar " + label + " lukte niet",
            ]
            subprocess.run(
                ["joep-notify", *msg], check=False, timeout=15, capture_output=True
            )
            self._prefetch()

        threading.Thread(target=worker, daemon=True).start()

    # -- autostart -----------------------------------------------------------

    def _autostart_enabled(self) -> bool:
        try:
            out = subprocess.run(
                ["systemctl", "--user", "is-enabled", "chefbar.service"],
                check=False,
                timeout=10,
                capture_output=True,
                text=True,
            )
            return out.stdout.strip() == "enabled"
        except (OSError, subprocess.TimeoutExpired):
            return False

    def _on_autostart_toggle(self, item: Gtk.CheckMenuItem) -> None:
        verb = "enable" if item.get_active() else "disable"
        _run_bg(["systemctl", "--user", verb, "chefbar.service"])

    # -- icon / state ----------------------------------------------------------

    def _icon_path(self, state: str) -> str:
        candidates = (
            ICONS_DIR / f"tray-{state}.png",
            Path.home() / ".local/share/chefbar/app/chefbar/icons" / f"tray-{state}.png",
        )
        for path in candidates:
            if path.exists():
                return str(path)
        return "utilities-system-monitor"

    def _on_snapshot_change(self, snap: api.Snapshot) -> None:
        self.watcher.feed_vault(snap)
        state, tooltip = api.tray_state(snap)
        if state not in TRAY_STATES:
            state = "hulp"
        self._state = state
        icon = self._icon_path(state)
        try:
            self.indicator.set_icon_full(icon, tooltip)
            self.indicator.set_title(tooltip)
            if state in ("fout", "hulp"):
                self.indicator.set_attention_icon_full(icon, tooltip)
                self.indicator.set_status(AppIndicator.IndicatorStatus.ATTENTION)
            else:
                self.indicator.set_status(AppIndicator.IndicatorStatus.ACTIVE)
        except Exception:  # noqa: BLE001
            log.exception("icon update faalde")
        GLib.idle_add(self._rebuild_accounts, snap)
        GLib.idle_add(self._rebuild_tickets, snap)

    def _prefetch(self) -> None:
        snap = api.fetch_snapshot()
        GLib.idle_add(self.panel.seed_cache, snap)

    def _bg_tick(self) -> bool:
        # Keep tray icon health fresh even when panel is closed.
        if not self.panel.is_open():
            threading.Thread(target=self._prefetch, daemon=True).start()
        return True

    def quit(self) -> None:
        log.info("ChefBar stop")
        Gtk.main_quit()


def run() -> None:
    ChefBarApp()
    Gtk.main()
