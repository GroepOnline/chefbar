"""ChefBar command-bar: global hotkey → prompt → agents en acties.

Eén balk, midden bovenin. Typ wat er moet gebeuren en de bar doet het:
agent starten, herdr focussen, ploegpas wisselen, dashboard openen.
Bovenin hangen verse suggesties van de watcher (agent klaar, hulp nodig).
"""

from __future__ import annotations

import logging
import subprocess
import threading
from dataclasses import dataclass, field
from typing import Callable
from urllib.parse import urlparse

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk, Pango  # noqa: E402

from . import api, ops
from .panel import load_css, notify, open_url

log = logging.getLogger("chefbar.bar")

_os = __import__("os")
DASHBOARD = _os.environ.get("CHEFBAR_DASHBOARD", "http://127.0.0.1:8080")
DESKTOP_URL = _os.environ.get("CHEFBAR_DESKTOP", "http://127.0.0.1:3000")
OPS_URL = _os.environ.get("CHEFBAR_OPS_API", "http://127.0.0.1:10101")
BAR_WIDTH = int(_os.environ.get("CHEFBAR_BAR_WIDTH", "640"))
MAX_ROWS = 8


def _ops_port_label(ops_url: str = OPS_URL) -> str:
    """Leid poort af uit CHEFBAR_OPS_API/OPS_URL; fallback 10101 bij parse-fout."""
    try:
        parsed = urlparse(ops_url.strip())
        if parsed.port is not None:
            return str(parsed.port)
        if parsed.scheme == "https":
            return "443"
        if parsed.scheme == "http":
            return "80"
    except ValueError:
        pass
    return "10101"


OPS_PORT_LABEL = _ops_port_label()


STAMP_CLASS = {
    "BEZIG": "bezig",
    "KLAAR": "klaar",
    "HULP": "hulp",
    "FOUT": "fout",
    "STIL": "stil",
    "LIMIET": "hulp",
    "BON": "stil",
}


@dataclass
class Action:
    title: str
    meta: str
    stamp: str
    keywords: str
    run: Callable[[str], None]
    needs_text: bool = False
    pinned: bool = False

    def matches(self, query: str) -> bool:
        if not query:
            return True
        hay = f"{self.title} {self.meta} {self.keywords}".lower()
        return all(tok in hay for tok in query.lower().split())


@dataclass
class BarData:
    vault: api.Snapshot = field(default_factory=api.Snapshot)
    herdr: ops.OpsSnapshot = field(default_factory=ops.OpsSnapshot)


def _agent_stamp(status: str) -> str:
    return {
        "working": "BEZIG",
        "idle": "KLAAR",
        "blocked": "HULP",
    }.get(status, "STIL")


class CommandBar:
    """Spotlight-achtige bar, gevoed door joep-ops en de vault-API."""

    def __init__(
        self,
        watcher: ops.Watcher | None = None,
        get_vault: Callable[[], api.Snapshot] | None = None,
        on_hide: Callable[[], None] | None = None,
    ) -> None:
        load_css()
        self.watcher = watcher
        self.get_vault = get_vault
        self.on_hide = on_hide
        self.data = BarData()
        self._open = False
        self._actions: list[Action] = []
        self._rows: list[tuple[Gtk.Widget, Action]] = []
        self._selected = 0
        self._build_window()

    # -- window ---------------------------------------------------------------

    def _build_window(self) -> None:
        self.window = Gtk.Window(type=Gtk.WindowType.TOPLEVEL)
        self.window.set_title("ChefBar Bar")
        self.window.set_decorated(False)
        self.window.set_skip_taskbar_hint(True)
        self.window.set_skip_pager_hint(True)
        self.window.set_keep_above(True)
        self.window.set_resizable(False)
        self.window.set_type_hint(Gdk.WindowTypeHint.DIALOG)
        self.window.stick()
        self.window.get_style_context().add_class("chefbar-bar")
        self.window.connect("delete-event", self._on_delete)
        self.window.connect("key-press-event", self._on_key)
        self.window.connect("focus-out-event", self._on_focus_out)

        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.get_style_context().add_class("chefbar-bar-root")
        self.window.add(outer)

        # Suggestiestrook (bonnen van de watcher)
        self.suggestion_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.pack_start(self.suggestion_box, False, False, 0)

        # Prompt
        self.entry = Gtk.Entry()
        self.entry.set_placeholder_text("Wat gaan we doen?")
        self.entry.get_style_context().add_class("chefbar-bar-entry")
        self.entry.connect("changed", lambda *_: self._refilter())
        self.entry.connect("activate", lambda *_: self._run_selected())
        outer.pack_start(self.entry, False, False, 0)

        # Resultaten
        self.results_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        outer.pack_start(self.results_box, False, False, 0)

        # Statusstrook onderin
        self.status_lab = Gtk.Label(label="", xalign=0)
        self.status_lab.get_style_context().add_class("chefbar-bar-status")
        self.status_lab.set_ellipsize(Pango.EllipsizeMode.END)
        outer.pack_start(self.status_lab, False, False, 0)

        self.window.show_all()
        self.window.hide()

    def is_open(self) -> bool:
        return self._open

    def toggle(self) -> None:
        if self._open:
            self.hide()
        else:
            self.show()

    def show(self) -> None:
        if self.watcher is not None:
            self.data.herdr = self.watcher.ops_snapshot
        if self.get_vault is not None:
            self.data.vault = self.get_vault()
        self.entry.set_text("")
        self._rebuild_catalog()
        self._render_suggestions()
        self._refilter()
        self._render_status()
        self._position()
        self.window.present()
        self.entry.grab_focus()
        self._open = True
        self._refresh_async()

    def hide(self) -> None:
        self.window.hide()
        self._open = False
        if self.on_hide:
            self.on_hide()

    def _position(self) -> None:
        display = self.window.get_display()
        try:
            seat = display.get_default_seat()
            _screen, px, py, _m = seat.get_pointer().get_position()
            monitor = display.get_monitor_at_point(px, py)
        except Exception:  # noqa: BLE001
            monitor = None
        if monitor is None:
            monitor = display.get_primary_monitor() or display.get_monitor(0)
        self.window.set_size_request(BAR_WIDTH, -1)
        self.window.resize(BAR_WIDTH, 1)
        self.window.show_all()
        if monitor is None:
            return
        wa = monitor.get_workarea()
        x = wa.x + (wa.width - BAR_WIDTH) // 2
        y = wa.y + max(24, wa.height // 7)
        self.window.move(int(x), int(y))

    # -- events -----------------------------------------------------------------

    def _on_delete(self, *_a):
        self.hide()
        return True

    def _on_key(self, _w, event):
        if event.keyval == Gdk.KEY_Escape:
            self.hide()
            return True
        if event.keyval in (Gdk.KEY_Down, Gdk.KEY_Tab):
            self._move_selection(1)
            return True
        if event.keyval in (Gdk.KEY_Up, Gdk.KEY_ISO_Left_Tab):
            self._move_selection(-1)
            return True
        return False

    def _on_focus_out(self, *_a):
        GLib.timeout_add(160, self._maybe_hide)
        return False

    def _maybe_hide(self) -> bool:
        if self._open and not self.window.has_toplevel_focus():
            self.hide()
        return False

    # -- data -----------------------------------------------------------------

    def _refresh_async(self) -> None:
        def worker() -> None:
            herdr = ops.fetch_ops_snapshot()
            vault = self.get_vault() if self.get_vault else api.fetch_snapshot()
            GLib.idle_add(self._on_fresh_data, herdr, vault)

        threading.Thread(target=worker, daemon=True).start()

    def _on_fresh_data(self, herdr: ops.OpsSnapshot, vault: api.Snapshot) -> bool:
        if herdr.ok:
            self.data.herdr = herdr
        if vault.providers or vault.agents or not self.data.vault.providers:
            self.data.vault = vault
        if self._open:
            self._rebuild_catalog()
            self._refilter()
            self._render_status()
        return False

    # -- catalogus --------------------------------------------------------------

    def _rebuild_catalog(self) -> None:
        actions: list[Action] = []
        herdr = self.data.herdr
        vault = self.data.vault

        for agent in herdr.agents:
            stamp = _agent_stamp(agent.status)
            actions.append(
                Action(
                    title=f"Focus {agent.name} · {agent.workspace}",
                    meta=agent.cwd.replace(str(api.HOME), "~"),
                    stamp=stamp,
                    keywords=f"focus herdr spring {agent.name} {agent.workspace}",
                    run=lambda _q, t=agent.terminal_id: self._do_bg(
                        lambda: ops.focus_target(t), None
                    ),
                )
            )
            actions.append(
                Action(
                    title=f"Stuur naar {agent.name} · {agent.workspace}",
                    meta="typ je opdracht en kies deze regel",
                    stamp="BON",
                    keywords=f"stuur send prompt opdracht {agent.name} {agent.workspace}",
                    needs_text=True,
                    run=lambda q, a=agent: self._do_send(a, q),
                )
            )

        seen_ws: set[str] = set()
        for agent in herdr.agents:
            if agent.workspace_id in seen_ws:
                continue
            seen_ws.add(agent.workspace_id)
            actions.append(
                Action(
                    title=f"Nieuwe agent in {agent.workspace}",
                    meta="start een cursor-agent met jouw opdracht",
                    stamp="BON",
                    keywords=f"nieuwe start agent workspace {agent.workspace}",
                    needs_text=True,
                    run=lambda q, cwd=agent.cwd: self._do_task(q, cwd),
                )
            )

        for row in vault.providers:
            for acc in row.accounts:
                if acc.get("id") == row.active_id:
                    continue
                label = acc.get("label") or acc.get("id")
                actions.append(
                    Action(
                        title=f"Werk als {label}",
                        meta=f"{row.label} · ploegpas wisselen",
                        stamp="STIL",
                        keywords=f"account switch wissel pas {row.label} {label}",
                        run=lambda _q, aid=acc.get("id"), lab=label: self._do_switch(
                            str(aid), str(lab)
                        ),
                    )
                )

        actions.extend(
            [
                Action(
                    title="Open ops",
                    meta="joep-ops · herdr agents en pulse",
                    stamp="STIL",
                    keywords=f"open ops joep-ops {OPS_PORT_LABEL} herdr overzicht",
                    run=lambda _q: self._do_open(OPS_URL),

                ),
                Action(
                    title="Open dashboard (Thuis)",
                    meta="vault dashboard · keukenoverzicht",
                    stamp="STIL",
                    keywords="open dashboard thuis vault 8080",
                    run=lambda _q: self._do_open(DASHBOARD),
                ),
                Action(
                    title="Open desktop",
                    meta="webtop op :3000",
                    stamp="STIL",
                    keywords="open desktop webtop 3000",
                    run=lambda _q: self._do_open(DESKTOP_URL),
                ),
                Action(
                    title="Ververs status",
                    meta="haal verse data uit de keuken",
                    stamp="STIL",
                    keywords="ververs refresh status",
                    run=lambda _q: self._refresh_async(),
                ),
            ]
        )
        self._actions = actions

    # -- filteren en renderen ---------------------------------------------------

    def _query(self) -> str:
        return self.entry.get_text().strip()

    def _refilter(self) -> None:
        query = self._query()
        matched = [a for a in self._actions if a.matches(query)]

        # Vrije tekst: geen match of duidelijk een zin → agent starten met prompt.
        if query and (not matched or " " in query):
            free = Action(
                title=f"Start agent met: \u201c{query}\u201d",
                meta="cursor-agent thuis · via commander",
                stamp="BON",
                keywords="",
                pinned=True,
                run=lambda q: self._do_task(q, str(api.HOME)),
            )
            focused = next((a for a in self.data.herdr.agents if a.focused), None)
            extra: list[Action] = [free]
            if focused is not None:
                extra.append(
                    Action(
                        title=f"Stuur naar gefocuste agent ({focused.name})",
                        meta=f"{focused.workspace} · direct in de terminal",
                        stamp="BON",
                        keywords="",
                        pinned=True,
                        run=lambda q, a=focused: self._do_send(a, q),
                    )
                )
            matched = extra + matched if not matched else matched + extra

        self._render_results(matched[:MAX_ROWS], query)

    def _render_results(self, actions: list[Action], query: str) -> None:
        for child in self.results_box.get_children():
            self.results_box.remove(child)
        self._rows = []
        self._selected = 0
        for i, action in enumerate(actions):
            row = self._action_row(action, selected=(i == 0))
            self.results_box.pack_start(row, False, False, 0)
            self._rows.append((row, action))
        if not actions:
            empty = Gtk.Label(label="Nog stil in de keuken.", xalign=0)
            empty.get_style_context().add_class("chefbar-bar-empty")
            self.results_box.pack_start(empty, False, False, 0)
        self.results_box.show_all()

    def _action_row(self, action: Action, selected: bool) -> Gtk.Widget:
        ebox = Gtk.EventBox()
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        box.get_style_context().add_class("chefbar-bar-row")
        if selected:
            box.get_style_context().add_class("selected")

        stamp = Gtk.Label(label=action.stamp)
        ctx = stamp.get_style_context()
        ctx.add_class("chefbar-stamp")
        ctx.add_class(STAMP_CLASS.get(action.stamp, "stil"))
        box.pack_start(stamp, False, False, 0)

        col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1)
        title = Gtk.Label(label=action.title, xalign=0)
        title.get_style_context().add_class("chefbar-bar-row-title")
        title.set_ellipsize(Pango.EllipsizeMode.END)
        col.pack_start(title, False, False, 0)
        if action.meta:
            meta = Gtk.Label(label=action.meta, xalign=0)
            meta.get_style_context().add_class("chefbar-bar-row-meta")
            meta.set_ellipsize(Pango.EllipsizeMode.END)
            col.pack_start(meta, False, False, 0)
        box.pack_start(col, True, True, 0)

        ebox.add(box)
        ebox.connect("button-press-event", lambda *_a, a=action: self._run_action(a))
        return ebox

    def _render_suggestions(self) -> None:
        for child in self.suggestion_box.get_children():
            self.suggestion_box.remove(child)
        if self.watcher is None:
            return
        for sug in self.watcher.fresh_suggestions()[:2]:
            ebox = Gtk.EventBox()
            box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
            box.get_style_context().add_class("chefbar-bar-suggestion")
            stamp = Gtk.Label(label=sug.stamp)
            ctx = stamp.get_style_context()
            ctx.add_class("chefbar-stamp")
            ctx.add_class(STAMP_CLASS.get(sug.stamp, "stil"))
            box.pack_start(stamp, False, False, 0)
            col = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1)
            title = Gtk.Label(label=sug.title, xalign=0)
            title.get_style_context().add_class("chefbar-bar-row-title")
            title.set_ellipsize(Pango.EllipsizeMode.END)
            col.pack_start(title, False, False, 0)
            meta = Gtk.Label(label=sug.meta, xalign=0)
            meta.get_style_context().add_class("chefbar-bar-row-meta")
            col.pack_start(meta, False, False, 0)
            box.pack_start(col, True, True, 0)
            act = Gtk.Label(label=sug.action_label + " →")
            act.get_style_context().add_class("chefbar-bar-suggestion-act")
            box.pack_end(act, False, False, 0)
            ebox.add(box)
            ebox.connect(
                "button-press-event",
                lambda *_a, s=sug: (self.hide(), s.run()) and False,
            )
            self.suggestion_box.pack_start(ebox, False, False, 0)
        self.suggestion_box.show_all()

    def _render_status(self) -> None:
        vault = self.data.vault
        herdr = self.data.herdr
        parts: list[str] = []
        if vault.health.total:
            parts.append(f"OS {vault.health.ok}/{vault.health.total} ok")
        busy = sum(1 for a in herdr.agents if a.status == "working")
        if herdr.ok:
            parts.append(f"{busy} bezig" if busy else "keuken rustig")
        else:
            parts.append(f"joep-ops slaapt (poort {OPS_PORT_LABEL})")

        for row in vault.providers:
            if row.provider == "cursor" and row.active_label:
                parts.append(f"Cursor: {row.active_label}")
                break
        self.status_lab.set_text("  ·  ".join(parts) if parts else "sync…")

    # -- selectie ----------------------------------------------------------------

    def _move_selection(self, delta: int) -> None:
        if not self._rows:
            return
        old_row, _ = self._rows[self._selected]
        old_row.get_child().get_style_context().remove_class("selected")
        self._selected = (self._selected + delta) % len(self._rows)
        new_row, _ = self._rows[self._selected]
        new_row.get_child().get_style_context().add_class("selected")

    def _run_selected(self) -> None:
        if not self._rows:
            return
        _row, action = self._rows[self._selected]
        self._run_action(action)

    def _run_action(self, action: Action) -> None:
        query = self._query()
        if action.needs_text and not query:
            self.entry.set_placeholder_text("Typ eerst wat er moet gebeuren…")
            self.entry.grab_focus()
            return
        self.hide()
        try:
            action.run(query)
        except Exception:  # noqa: BLE001
            log.exception("actie faalde: %s", action.title)
            notify("Dat lukte niet", action.title, status="error")

    # -- uitvoerders --------------------------------------------------------------

    def _do_bg(self, fn: Callable[[], bool], ok_msg: str | None) -> None:
        def worker() -> None:
            ok = fn()
            if ok and ok_msg:
                notify(ok_msg, "", status="ok")
            elif not ok:
                notify("Dat lukte niet", "zie chefbar.log", status="error")

        threading.Thread(target=worker, daemon=True).start()

    def _do_send(self, agent: ops.HerdrAgent, text: str) -> None:
        def worker() -> None:
            ok = ops.send_prompt(agent, text)
            if ok:
                notify(
                    f"Bon ligt bij {agent.name}",
                    f"{agent.workspace} · {text[:60]}",
                    status="ok",
                )
                ops.focus_target(agent.terminal_id)
            else:
                notify("Sturen lukte niet", f"{agent.name} · zie chefbar.log", status="error")

        threading.Thread(target=worker, daemon=True).start()

    def _do_task(self, prompt: str, cwd: str) -> None:
        def worker() -> None:
            result = api.create_commander_task(prompt, agent_type="cursor", cwd=cwd)
            if result is None:
                notify("Taak starten lukte niet", "zie chefbar.log", status="error")
            else:
                tid = result.get("id") or "?"
                notify("Agent aan de slag", f"{tid} · {prompt[:60]}", status="ok")

        threading.Thread(target=worker, daemon=True).start()

    def _do_switch(self, account_id: str, label: str) -> None:
        def worker() -> None:
            ok = api.switch_account(account_id) is not None
            if ok:
                notify(f"Je werkt nu als {label}.", "", status="ok")
            else:
                notify("Wisselen lukte niet", f"{label} · probeer opnieuw", status="error")

        threading.Thread(target=worker, daemon=True).start()

    def _do_open(self, url: str) -> None:
        open_url(url)


def run_bar_only() -> None:
    """Losse modus: `chefbar --bar` zonder lopende tray (fallback)."""
    watcher = ops.Watcher()
    watcher.start()
    bar = CommandBar(watcher=watcher, on_hide=Gtk.main_quit)

    def boot() -> bool:
        bar.show()
        return False

    GLib.idle_add(boot)
    Gtk.main()
