"""joep-ops (:10101) client + proactieve watcher voor ChefBar.

De watcher kijkt elke ~20s naar herdr-agents en de vault en signaleert
overgangen (klaar, hulp nodig, limiet in zicht) als notificatie met een
voorgestelde vervolgactie. De bar leest dezelfde suggesties.
"""

from __future__ import annotations

import json
import logging
import os
import subprocess
import threading
import time
import urllib.error
import urllib.request
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Callable

from . import api

log = logging.getLogger("chefbar.ops")

OPS_API = os.environ.get("CHEFBAR_OPS_API", "http://127.0.0.1:10101").rstrip("/")
WATCH_INTERVAL = int(os.environ.get("CHEFBAR_WATCH_REFRESH", "20"))
SUGGESTION_TTL = 15 * 60  # seconden dat een suggestie relevant blijft


@dataclass
class HerdrAgent:
    terminal_id: str
    name: str
    status: str  # working | idle | blocked | unknown
    workspace: str
    workspace_id: str
    cwd: str
    pane_id: str
    focused: bool


@dataclass
class HerdrWorkspace:
    workspace_id: str
    label: str
    agent_status: str
    focused: bool


@dataclass
class OpsSnapshot:
    ok: bool = False
    agents: list[HerdrAgent] = field(default_factory=list)
    workspaces: list[HerdrWorkspace] = field(default_factory=list)


@dataclass
class Suggestion:
    """Voorgestelde vervolgactie, getoond in de bar en als notificatie."""

    key: str
    title: str
    meta: str
    stamp: str  # KLAAR | HULP | FOUT | LIMIET
    action_label: str
    run: Callable[[], None]
    created: float = field(default_factory=time.time)

    @property
    def fresh(self) -> bool:
        return (time.time() - self.created) < SUGGESTION_TTL


def _ops_get(path: str, timeout: float = 4.0) -> dict | None:
    try:
        with urllib.request.urlopen(f"{OPS_API}{path}", timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except (urllib.error.URLError, json.JSONDecodeError, TimeoutError, OSError) as exc:
        log.debug("ops GET %s faalde: %s", path, exc)
        return None


def fetch_ops_snapshot() -> OpsSnapshot:
    snap = OpsSnapshot()
    data = _ops_get("/api/snapshot")
    if not isinstance(data, dict):
        return snap
    snap.ok = True
    for a in data.get("agents") or []:
        snap.agents.append(
            HerdrAgent(
                terminal_id=a.get("terminal_id") or "",
                name=a.get("agent") or "agent",
                status=(a.get("agent_status") or "unknown").lower(),
                workspace=a.get("terminal_title_stripped") or a.get("cwd") or "?",
                workspace_id=a.get("workspace_id") or "",
                cwd=a.get("cwd") or str(api.HOME),
                pane_id=a.get("pane_id") or "",
                focused=bool(a.get("focused")),
            )
        )
    for w in data.get("workspaces") or []:
        snap.workspaces.append(
            HerdrWorkspace(
                workspace_id=w.get("workspace_id") or "",
                label=w.get("label") or "?",
                agent_status=(w.get("agent_status") or "").lower(),
                focused=bool(w.get("focused")),
            )
        )
    return snap


def _run_herdr(args: list[str], timeout: float = 10.0) -> bool:
    try:
        proc = subprocess.run(
            ["herdr", *args], check=False, capture_output=True, timeout=timeout
        )
        return proc.returncode == 0
    except (OSError, subprocess.TimeoutExpired) as exc:
        log.warning("herdr %s faalde: %s", args[:2], exc)
        return False


def focus_target(target: str) -> bool:
    """Focus een herdr agent/terminal; eerst via joep-ops, dan CLI."""
    try:
        body = json.dumps({"target": target}).encode()
        req = urllib.request.Request(
            f"{OPS_API}/api/focus",
            data=body,
            method="POST",
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=6) as resp:
            payload = json.loads(resp.read().decode() or "{}")
            if payload.get("ok"):
                return True
    except (urllib.error.URLError, json.JSONDecodeError, TimeoutError, OSError):
        pass
    return _run_herdr(["agent", "focus", target])


def send_prompt(agent: HerdrAgent, text: str) -> bool:
    """Typ een prompt in de TUI van een lopende agent en verstuur met Enter."""
    if not _run_herdr(["agent", "send", agent.terminal_id or agent.pane_id, text]):
        return False
    pane = agent.pane_id
    if pane:
        _run_herdr(["pane", "send-keys", pane, "Enter"])
    return True


def notify_action(
    title: str,
    body: str,
    status: str,
    action_label: str,
    on_click: Callable[[], None],
) -> None:
    """joep-notify met actieknop; klik voert on_click uit (aparte thread)."""

    def worker() -> None:
        try:
            proc = subprocess.run(
                [
                    "joep-notify",
                    "-s", "agent",
                    "-S", status,
                    "-A", f"chefbar={action_label}",
                    "-w",
                    title,
                    body,
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=180,
            )
            if proc.stdout.strip() == "chefbar":
                on_click()
        except (OSError, subprocess.TimeoutExpired):
            pass

    threading.Thread(target=worker, daemon=True).start()


class Watcher:
    """Achtergrondwacht: ziet agent-overgangen en vault-problemen aankomen."""

    def __init__(self, on_suggestion: Callable[[Suggestion], None] | None = None) -> None:
        self.on_suggestion = on_suggestion
        self.suggestions: deque[Suggestion] = deque(maxlen=8)
        self.ops_snapshot = OpsSnapshot()
        self._statuses: dict[str, str] = {}
        self._seen_keys: dict[str, float] = {}
        self._vault: api.Snapshot | None = None
        self._started = False

    # -- input --------------------------------------------------------------

    def feed_vault(self, snap: api.Snapshot) -> None:
        prev = self._vault
        self._vault = snap
        self._check_vault(prev, snap)

    def start(self) -> None:
        if self._started:
            return
        self._started = True
        threading.Thread(target=self._loop, daemon=True).start()

    def fresh_suggestions(self) -> list[Suggestion]:
        return [s for s in self.suggestions if s.fresh]

    # -- intern ---------------------------------------------------------------

    def _loop(self) -> None:
        while True:
            try:
                snap = fetch_ops_snapshot()
                if snap.ok:
                    self._check_agents(snap)
                    self.ops_snapshot = snap
            except Exception:  # noqa: BLE001 — watcher mag nooit sterven
                log.exception("watcher tick faalde")
            time.sleep(WATCH_INTERVAL)

    def _dedupe(self, key: str, window: float = 300.0) -> bool:
        """True als deze suggestie recent al gemeld is."""
        now = time.time()
        last = self._seen_keys.get(key, 0)
        if now - last < window:
            return True
        self._seen_keys[key] = now
        return False

    def _push(self, sug: Suggestion, notify_status: str | None = None) -> None:
        if self._dedupe(sug.key):
            return
        self.suggestions.appendleft(sug)
        if notify_status:
            notify_action(sug.title, sug.meta, notify_status, sug.action_label, sug.run)
        if self.on_suggestion:
            try:
                self.on_suggestion(sug)
            except Exception:  # noqa: BLE001
                log.exception("on_suggestion callback faalde")

    def _check_agents(self, snap: OpsSnapshot) -> None:
        current: dict[str, str] = {}
        first_run = not self._statuses
        for agent in snap.agents:
            tid = agent.terminal_id
            if not tid:
                continue
            current[tid] = agent.status
            if first_run:
                continue
            prev = self._statuses.get(tid)
            if prev == agent.status:
                continue
            if prev == "working" and agent.status == "idle":
                self._push(
                    Suggestion(
                        key=f"done:{tid}",
                        title=f"{agent.name} is klaar in {agent.workspace}",
                        meta="Bekijk het resultaat of stuur een vervolg",
                        stamp="KLAAR",
                        action_label="Bekijken",
                        run=lambda t=tid: focus_target(t),
                    ),
                    notify_status="ok",
                )
            elif agent.status == "blocked":
                self._push(
                    Suggestion(
                        key=f"blocked:{tid}",
                        title=f"Even jou nodig: {agent.name} wacht in {agent.workspace}",
                        meta="Spring erin en geef antwoord",
                        stamp="HULP",
                        action_label="Erheen",
                        run=lambda t=tid: focus_target(t),
                    ),
                    notify_status="warn",
                )
        self._statuses = current

    def _check_vault(self, prev: api.Snapshot | None, snap: api.Snapshot) -> None:
        if snap.error and not (prev and prev.error):
            self._push(
                Suggestion(
                    key="vault:down",
                    title="Vault-API reageert niet (poort 8321)",
                    meta="Kijk op het dashboard wat er aan de hand is",
                    stamp="FOUT",
                    action_label="Dashboard",
                    run=lambda: subprocess.Popen(
                        ["xdg-open", os.environ.get("CHEFBAR_DASHBOARD", "http://127.0.0.1:8080")],
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    ),
                ),
                notify_status="error",
            )
        for row in snap.providers:
            if row.usage_level == "down" and row.accounts and len(row.accounts) > 1:
                self._push(
                    Suggestion(
                        key=f"limit:{row.provider}",
                        title=f"{row.label} zit bijna aan de limiet",
                        meta="Wissel van account via de bar",
                        stamp="LIMIET",
                        action_label="Oké",
                        run=lambda: None,
                    ),
                    notify_status="warn",
                )
        # total == 0 betekent "nog geen healthdata", geen storing.
        # Een eerdere no-data snapshot (level=down, down==0) mag de eerste
        # echte storing niet onderdrukken — check daarom ook prev.down > 0.
        if (
            snap.health.level == "down"
            and snap.health.down > 0
            and not (
                prev
                and prev.health.level == "down"
                and prev.health.down > 0
            )
        ):
            down = snap.health.down
            self._push(
                Suggestion(
                    key="health:down",
                    title=f"{down} dienst{'en' if down != 1 else ''} down",
                    meta="Kijk even op het dashboard wat er stilstaat",
                    stamp="FOUT",
                    action_label="Dashboard",
                    run=lambda: subprocess.Popen(
                        ["xdg-open", os.environ.get("CHEFBAR_DASHBOARD", "http://127.0.0.1:8080")],
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    ),
                ),
                notify_status="error",
            )
