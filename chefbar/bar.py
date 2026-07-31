"""ChefBar command-bar: global hotkey → prompt → agents en acties.

Eén balk, midden bovenin. Typ wat er moet gebeuren en de bar doet het:
agent starten, herdr focussen, account wisselen, dashboard openen.
Bovenin hangen verse suggesties van de watcher (agent klaar, hulp nodig).
"""

from __future__ import annotations

import logging
import os
import threading
from dataclasses import dataclass, field
from typing import Callable
from urllib.parse import urlparse

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk, Pango  # noqa: E402

from . import api, motion, ops, sessions
from .endpoints import PROFILE
from .palette import Action, rank_actions
from .panel import load_css, notify, open_url

log = logging.getLogger("chefbar.bar")

DASHBOARD = PROFILE.dashboard
DESKTOP_URL = PROFILE.desktop
OPS_URL = PROFILE.ops_api
OPS_LABEL = PROFILE.label("opsApi")

# SSOT (config/ops-url.json): canoniek Joep Ops-adres, fallback voor OPS_PORT_LABEL.
OPS_SSOT_BASE = "http://127.0.0.1:10101"


def _ops_port_label(ops_url: str = OPS_URL) -> str:
    """Leid poort af uit OPS_URL; SSOT-fallback bij parse-fout."""
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
    return OPS_SSOT_BASE.rsplit(":", 1)[-1]


OPS_PORT_LABEL = _ops_port_label()
BAR_WIDTH = int(os.environ.get("CHEFBAR_BAR_WIDTH", "640"))
MAX_ROWS = 8


STAMP_CLASS = {
    "BEZIG": "bezig",
    "KLAAR": "klaar",
    "HULP": "hulp",
    "FOUT": "fout",
    "STIL": "stil",
    "LIMIET": "hulp",
    "TAAK": "stil",
}


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

        # Suggestiestrook (verse signalen van de watcher)
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
            # Fetch vault data off the UI thread; render with what we have and
            # refresh the catalog when the snapshot arrives (remote profiles
            # can take up to the 5s API timeout).
            def _fetch() -> None:
                try:
                    vault = self.get_vault()
                except Exception:
                    vault = None
                if vault is not None:
                    GLib.idle_add(self._apply_vault, vault)

            threading.Thread(target=_fetch, daemon=True).start()
        self.entry.set_text("")
        self._rebuild_catalog()
        self._render_suggestions()
        self._refilter()
        self._render_status()
        if motion.motion_enabled():
            self.window.set_opacity(0.0)
        self._position()
        self._open = True

    def _apply_vault(self, vault: api.Snapshot) -> None:
        self.data.vault = vault
        if self._open:
            self._rebuild_catalog()
            self._refilter()
            self._render_status()
        motion.fade_in(self.window, duration_ms=motion.PANEL_MS)
        self.window.present()
        self.entry.grab_focus()
        self._refresh_async()

    def hide(self) -> None:
        if not self._open:
            return
        self._open = False

        def _after() -> None:
            if self.on_hide:
                self.on_hide()

        motion.fade_out(
            self.window,
            duration_ms=motion.HOVER_MS,
            on_hidden=_after,
        )

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
                    stamp="TAAK",
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
                    stamp="TAAK",
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
                        meta=f"{row.label} · account wisselen",
                        stamp="STIL",
                        keywords=f"account switch wissel {row.label} {label}",
                        run=lambda _q, aid=acc.get("id"), lab=label, r=row: self._do_switch(
                            str(aid), str(lab), r.source, r.driver
                        ),
                    )
                )

        for task in vault.tasks:
            task_id = str(task.get("id") or "")
            status = str(task.get("status") or "queued")
            prompt = str(task.get("prompt") or "Taak zonder omschrijving")
            if status in ("queued", "running"):
                actions.append(
                    Action(
                        title=f"Stop taak · {prompt[:52]}",
                        meta=f"{task_id} · {status}",
                        stamp="HULP",
                        keywords=f"commander taak stop annuleer cancel {task_id}",
                        destructive=True,
                        run=lambda _q, tid=task_id: self._do_api(
                            lambda: api.cancel_commander_task(tid),
                            "Taak gestopt",
                        ),
                    )
                )

        for row, item in enumerate(vault.clipboard[:6]):
            text = str(item.get("text") or "").replace("\n", " ")
            actions.append(
                Action(
                    title=f"Kopieer · {text[:56]}",
                    meta=f"clipboard-rij {row}",
                    stamp="STIL",
                    keywords=f"clipboard klembord kopieer plak {text}",
                    run=lambda _q, value=str(item.get("text") or ""): self._copy_text(value),
                )
            )
            actions.append(
                Action(
                    title=f"Verwijder clipboard-rij {row}",
                    meta=text[:64],
                    stamp="HULP",
                    keywords=f"clipboard klembord verwijder delete {row}",
                    destructive=True,
                    run=lambda _q, index=row: self._do_api(
                        lambda: api.clipboard_delete(index),
                        "Clipboard-rij verwijderd",
                    ),
                )
            )

        for event in vault.events[:5]:
            agent = str(event.get("agent") or "Agent")
            workspace = str(event.get("workspace") or "")
            summary = str(event.get("summary") or event.get("kind") or "update")
            actions.append(
                Action(
                    title=f"{agent} · {summary[:54]}",
                    meta=f"{workspace} · recente agentupdate".strip(" ·"),
                    stamp="KLAAR" if event.get("kind") == "done" else "BEZIG",
                    keywords=f"recent event agent feed {agent} {workspace} {summary}",
                    run=lambda _q: self._do_open(f"{DASHBOARD.rstrip('/')}/#agents"),
                )
            )

        for session in sessions.load_ranked_sessions(vault):
            action = session.primary_action
            if action and action[1] == "kater" and not PROFILE.kater_workspace:
                action = None
            stamp = "HULP" if session.needs_attention else "BEZIG"
            title = session.title
            meta = session.summary or session.source
            if action:
                label, kind = action
                actions.append(
                    Action(
                        title=f"{label} · {title[:48]}",
                        meta=meta,
                        stamp=stamp,
                        keywords=f"sessie session {session.source} {session.id} {title}",
                        run=lambda _q, s=session, k=kind: self._do_session_action(s, k),
                    )
                )

        ocx = api.fetch_opencodex_status()
        if ocx and not ocx.get("error"):
            active = ocx.get("activeAccount") or {}
            email = active.get("email") or active.get("id") or "OpenCodex"
            actions.append(
                Action(
                    title=f"OpenCodex · {email}",
                    meta="dashboard en providerstatus",
                    stamp="STIL",
                    keywords="opencodex ocx codex dashboard",
                    run=lambda _q: self._do_open_ocx(),
                )
            )
            actions.append(
                Action(
                    title="Ververs OpenCodex status",
                    meta="via vault-api",
                    stamp="STIL",
                    keywords="opencodex refresh ocx",
                    run=lambda _q: self._do_api(
                        lambda: api.api_request("/opencodex/refresh", method="POST"),
                        "OpenCodex ververst",
                    ),
                )
            )

        desktop_running = str(vault.desktop.get("state") or "") == "running"
        actions.extend(
            [
                Action(
                    title="Stuur taak naar Commander",
                    meta="typ je opdracht en druk op Enter",
                    stamp="TAAK",
                    keywords="commander agent opdracht taak start",
                    needs_text=True,
                    run=lambda q: self._do_task(q, str(api.HOME)),
                ),
                Action(
                    title="Voeg toe aan clipboard",
                    meta="typ tekst en kies deze actie",
                    stamp="TAAK",
                    keywords="clipboard klembord toevoegen add tekst",
                    needs_text=True,
                    run=lambda q: self._do_api(
                        lambda: api.clipboard_add(q),
                        "Toegevoegd aan clipboard",
                    ),
                ),
                Action(
                    title="Stop desktop" if desktop_running else "Start desktop",
                    meta="webtop · remote desktop",
                    stamp="BEZIG" if desktop_running else "STIL",
                    keywords="desktop webtop start stop",
                    run=lambda _q, verb="stop" if desktop_running else "start": self._do_api(
                        lambda: api.desktop_action(verb),
                        "Desktop gestopt" if verb == "stop" else "Desktop gestart",
                    ),
                ),
                Action(
                    title="Haal gedeelde bestanden op",
                    meta=f"{vault.share_sync.get('pendingFiles', 0)} wijzigingen wachten",
                    stamp="STIL",
                    keywords="share sync pull ophalen bestanden",
                    run=lambda _q: self._do_api(
                        lambda: api.share_sync_action("pull"),
                        "Gedeelde bestanden opgehaald",
                    ),
                ),
                Action(
                    title="Deel lokale bestanden",
                    meta="push naar de gedeelde map",
                    stamp="STIL",
                    keywords="share sync push delen bestanden",
                    run=lambda _q: self._do_api(
                        lambda: api.share_sync_action("push"),
                        "Lokale bestanden gedeeld",
                    ),
                ),
                Action(
                    title="Open ops",
                    meta=f"joep-ops · {OPS_LABEL}",
                    stamp="STIL",
                    keywords=f"open ops joep-ops {OPS_LABEL} herdr overzicht",
                    run=lambda _q: self._do_open(OPS_URL),
                ),
                Action(
                    title="Open dashboard (Thuis)",
                    meta="vault dashboard · alles in één oogopslag",
                    stamp="STIL",
                    keywords="open dashboard thuis vault",
                    run=lambda _q: self._do_open(DASHBOARD),
                ),
                Action(
                    title="Open desktop",
                    meta="webtop · remote desktop",
                    stamp="STIL",
                    keywords="open desktop webtop",
                    run=lambda _q: self._do_open(DESKTOP_URL),
                ),
                Action(
                    title="Ververs status",
                    meta="haal de nieuwste status op",
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
        matched = rank_actions(self._actions, query, MAX_ROWS)

        # Vrije tekst: geen match of duidelijk een zin → agent starten met prompt.
        if query and (not matched or " " in query):
            free = Action(
                title=f'Start agent met: "{query}"',
                meta="cursor-agent thuis · via commander",
                stamp="TAAK",
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
                        stamp="TAAK",
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
            empty = Gtk.Label(label="Nog niks gebeurd vandaag.", xalign=0)
            empty.get_style_context().add_class("chefbar-bar-empty")
            self.results_box.pack_start(empty, False, False, 0)
        self.results_box.show_all()

    def _action_row(self, action: Action, selected: bool) -> Gtk.Widget:
        ebox = Gtk.EventBox()
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        box.get_style_context().add_class("chefbar-bar-row")
        if selected:
            box.get_style_context().add_class("selected")
        if action.destructive:
            box.get_style_context().add_class("destructive")

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
        shortcut = Gtk.Label(label=action.shortcut)
        shortcut.get_style_context().add_class("chefbar-shortcut")
        box.pack_end(shortcut, False, False, 0)

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
        if vault.error:
            parts.append(vault.error)
        if vault.health.total:
            parts.append(f"OS {vault.health.ok}/{vault.health.total} ok")
        busy = sum(1 for a in herdr.agents if a.status == "working")
        if herdr.ok:
            parts.append(f"{busy} bezig" if busy else "alles rustig")
        else:
            parts.append(f"joep-ops slaapt (poort {OPS_PORT_LABEL})")

        for row in vault.providers:
            if row.provider == "cursor" and row.active_label:
                parts.append(f"Cursor: {row.active_label}")
                break
        self.status_lab.set_text("  ·  ".join(parts) if parts else "Nog niet ververst")

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
                    f"Opdracht ligt bij {agent.name}",
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

    def _do_api(self, fn: Callable[[], dict | None], success: str) -> None:
        def worker() -> None:
            result = fn()
            if result is None:
                notify("Dat lukte niet", "Vault-API gaf geen bruikbaar antwoord", status="error")
            else:
                notify(success, "", status="ok")

        threading.Thread(target=worker, daemon=True).start()

    def _copy_text(self, text: str) -> None:
        clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
        clipboard.set_text(text, -1)
        clipboard.store()
        notify("Gekopieerd", text[:60], status="ok")

    def _do_switch(
        self,
        account_id: str,
        label: str,
        source: str,
        driver: str | None,
    ) -> None:
        def worker() -> None:
            ok = api.switch_account(
                account_id,
                source,
                self.data.vault.revision,
                driver=driver,
            ) is not None
            if ok:
                notify(f"Je werkt nu als {label}.", "", status="ok")
            else:
                notify("Wisselen lukte niet", f"{label} · probeer opnieuw", status="error")

        threading.Thread(target=worker, daemon=True).start()

    def _do_open(self, url: str) -> None:
        open_url(url)

    def _do_open_ocx(self) -> None:
        url = PROFILE.opencodex_dashboard
        if url:
            self._do_open(url)
            return
        self._do_open(f"{DASHBOARD.rstrip('/')}/#opencodex")

    def _do_session_action(self, session: sessions.Session, kind: str) -> None:
        if kind == "kater":
            base = PROFILE.kater_workspace
            kid = session.attach.kater_session_id
            if base and kid:
                self._do_open(f"{base.rstrip('/')}/{kid}")
                return
            return
        if kind == "focus" and session.attach.focus:
            self._do_bg(lambda: ops.focus_target(session.attach.focus or ""), None)
            return
        if kind == "workspace" and session.attach.workspace_url:
            self._do_open(session.attach.workspace_url)
            return
        if kind == "browser" and session.attach.browser:
            self._do_open(session.attach.browser)
            return
        if kind == "evidence" and session.attach.evidence_url:
            self._do_open(session.attach.evidence_url)
            return
        notify("Sessie openen lukte niet", session.title, status="error")


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
