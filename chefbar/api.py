"""Vault-API client + local OS health/eval helpers for ChefBar."""

from __future__ import annotations

import json
import logging
import os
import re
import urllib.error
import urllib.parse
import urllib.request
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any

from . import auth
from .endpoints import PROFILE
from . import security

log = logging.getLogger("chefbar.api")

HOME = Path.home()
DEFAULT_ENV = HOME / "Documents/Github/GroepOnline/chefgroep-vault/docker/.env"
VAULT_API = PROFILE.vault_api.rstrip("/")
ENV_FILE = Path(os.environ.get("CHEFBAR_ENV_FILE", str(DEFAULT_ENV)))
_POLICY = security.POLICY.with_profile_hosts(*PROFILE.all_urls())
WATCHDOG_STATE = Path(
    os.environ.get(
        "CHEFBAR_WATCHDOG_STATE",
        str(HOME / ".local/share/chefgroep-os/watchdog-state.json"),
    )
)
EVAL_DIR = Path(
    os.environ.get(
        "CHEFBAR_EVAL_DIR",
        str(HOME / ".local/share/chefgroep-os/reports"),
    )
)

# Soft daily budgets for usage bars (requests / tokens).
OCX_REQ_BUDGET = int(os.environ.get("CHEFBAR_OCX_REQ_BUDGET", "500"))
OCX_TOK_BUDGET = int(os.environ.get("CHEFBAR_OCX_TOK_BUDGET", "40000000"))

PROVIDER_ORDER = ("codex", "claude", "cursor", "ocx", "pi", "openai", "zai", "custom")
CORE_PROVIDERS = frozenset({"codex", "claude", "cursor", "ocx", "pi"})
PROVIDER_LABELS = {
    "codex": "Codex",
    "claude": "Claude",
    "cursor": "Cursor",
    "ocx": "OCX",
    "pi": "Pi",
    "openai": "OpenAI",
    "zai": "Z.ai",
    "custom": "Custom",
}


@dataclass
class HealthInfo:
    ok: int = 0
    warn: int = 0
    down: int = 0
    skip: int = 0
    total: int = 0
    level: str = "down"  # ok | warn | down
    updated_at: str | None = None

    @property
    def line(self) -> str:
        if self.total <= 0:
            return "OS health · onbekend"
        return f"OS health · {self.ok}/{self.total} ok"


@dataclass
class DayScore:
    letter: str | None = None
    score: int | None = None
    source: str | None = None

    @property
    def line(self) -> str:
        if self.letter and self.score is not None:
            return f"Dagscore {self.letter} ({self.score}/100)"
        if self.score is not None:
            return f"Dagscore {self.score}/100"
        if self.letter:
            return f"Dagscore {self.letter}"
        return "Dagscore · n.v.t."


@dataclass
class ProviderRow:
    provider: str
    label: str
    active_label: str | None
    active_id: str | None
    color: str
    source: str = "vault"
    driver: str | None = None
    accounts: list[dict[str, Any]] = field(default_factory=list)
    requests: int | None = None
    tokens: int | None = None
    usage_frac: float = 0.0
    usage_level: str = "ok"
    usage_text: str = ""


@dataclass
class AgentRow:
    key: str
    agent: str
    workspace: str
    status: str
    summary: str
    last_activity: str | None
    running: bool


@dataclass
class FleetInfo:
    online: int = 0
    total: int = 0
    host: str | None = None
    stale: bool = False


@dataclass
class Snapshot:
    fetched_at: datetime = field(default_factory=datetime.now)
    health: HealthInfo = field(default_factory=HealthInfo)
    day_score: DayScore = field(default_factory=DayScore)
    providers: list[ProviderRow] = field(default_factory=list)
    agents: list[AgentRow] = field(default_factory=list)
    fleet: FleetInfo = field(default_factory=FleetInfo)
    revision: int = 0
    tasks: list[dict[str, Any]] = field(default_factory=list)
    clipboard: list[dict[str, Any]] = field(default_factory=list)
    events: list[dict[str, Any]] = field(default_factory=list)
    desktop: dict[str, Any] = field(default_factory=dict)
    share_sync: dict[str, Any] = field(default_factory=dict)
    error: str | None = None
    raw: dict[str, Any] = field(default_factory=dict)


def read_api_token() -> str | None:
    return auth._read_bearer()


def api_request(
    path: str,
    method: str = "GET",
    body: dict | None = None,
    timeout: float = 5.0,
    headers: dict[str, str] | None = None,
) -> dict | list | None:
    url_path = path if path.startswith("/") else f"/{path}"
    data = None
    request_headers = auth.get_headers(json_body=body is not None)
    if headers:
        request_headers.update(headers)
    if body is not None:
        data = json.dumps(body).encode()
    try:
        url = security.safe_join(VAULT_API, url_path, policy=_POLICY)
    except ValueError as exc:
        log.warning("API URL geweigerd: %s", exc)
        return None
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers=request_headers,
    )
    try:
        with security.safe_urlopen(req, timeout=timeout, policy=_POLICY) as resp:
            raw = resp.read().decode()
            if not raw:
                return {}
            return json.loads(raw)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode(errors="replace")[:200]
        log.warning("API %s %s antwoordde %s: %s", method, url_path, exc.code, detail)
        return None
    except (urllib.error.URLError, json.JSONDecodeError, TimeoutError, ValueError) as exc:
        log.warning("API %s %s faalde (%s): %s", method, url_path, PROFILE.label("vaultApi"), exc)
        return None


def fetch_connector_events(connector_id: str = "ops", limit: int = 50) -> list[dict[str, Any]]:
    """Vault-first connector feed (share/ops/log/session). Empty if unsupported."""
    data = api_request(f"/connectors/{connector_id}/events?limit={int(limit)}")
    if isinstance(data, dict) and isinstance(data.get("events"), list):
        return [e for e in data["events"] if isinstance(e, dict)]
    if isinstance(data, list):
        return [e for e in data if isinstance(e, dict)]
    return []


def fetch_opencodex_status() -> dict[str, Any] | None:
    data = api_request("/opencodex/status")
    if isinstance(data, dict):
        return data
    data = api_request("/opencodex")
    return data if isinstance(data, dict) else None


def fetch_sessions() -> list[dict[str, Any]]:
    data = api_request("/connectors/session/events?limit=20")
    if isinstance(data, dict) and isinstance(data.get("events"), list):
        return [e for e in data["events"] if isinstance(e, dict)]
    data = api_request("/sessions")
    if isinstance(data, dict) and isinstance(data.get("sessions"), list):
        return [e for e in data["sessions"] if isinstance(e, dict)]
    if isinstance(data, list):
        return [e for e in data if isinstance(e, dict)]
    return []


def switch_account(
    account_id: str,
    source: str,
    expected_revision: int,
    driver: str | None = None,
) -> dict | None:
    body: dict[str, Any] = {
        "source": source,
        "accountId": account_id,
        "expectedRevision": expected_revision,
    }
    if driver:
        body["driver"] = driver
    return api_request(
        "/coding/accounts/switch",
        method="POST",
        body=body,
        headers={"Idempotency-Key": f"chefbar-{uuid.uuid4()}"},
    )  # type: ignore[return-value]


def cancel_commander_task(task_id: str) -> dict | None:
    return api_request(
        f"/commander/tasks/{urllib.parse.quote(task_id, safe='')}/cancel",
        method="POST",
    )  # type: ignore[return-value]


def clipboard_add(text: str) -> dict | None:
    return api_request("/clipboard", method="POST", body={"text": text})  # type: ignore[return-value]


def clipboard_delete(row: int) -> dict | None:
    return api_request(f"/clipboard/{row}", method="DELETE")  # type: ignore[return-value]


def desktop_action(action: str) -> dict | None:
    if action not in ("start", "stop"):
        raise ValueError(f"Onbekende desktopactie: {action}")
    return api_request(f"/desktop/{action}", method="POST")  # type: ignore[return-value]


def share_sync_action(action: str) -> dict | None:
    if action not in ("pull", "push"):
        raise ValueError(f"Onbekende syncactie: {action}")
    return api_request(f"/share-sync/{action}", method="POST")  # type: ignore[return-value]


def create_commander_task(
    prompt: str,
    agent_type: str = "cursor",
    cwd: str | None = None,
) -> dict | None:
    body: dict[str, Any] = {"prompt": prompt, "agentType": agent_type}
    if cwd:
        body["cwd"] = cwd
    else:
        body["cwd"] = str(HOME)
    return api_request("/commander/tasks", method="POST", body=body)  # type: ignore[return-value]


def load_health() -> HealthInfo:
    info = HealthInfo()
    try:
        data = json.loads(WATCHDOG_STATE.read_text())
    except (OSError, json.JSONDecodeError):
        return info
    comps = data.get("components") or {}
    info.total = len(comps)
    info.updated_at = data.get("updated_at")
    for comp in comps.values():
        status = (comp.get("last_status") or "").lower()
        if status in ("ok", "up", "healthy", "running"):
            info.ok += 1
        elif status in ("warn", "warning", "degraded", "flapping"):
            info.warn += 1
        elif status in ("skip", "skipped", "disabled"):
            info.skip += 1
        else:
            info.down += 1
    if info.down > 0:
        info.level = "down"
    elif info.warn > 0:
        info.level = "warn"
    elif info.total > 0 and info.ok + info.skip == info.total:
        info.level = "ok"
    else:
        info.level = "warn" if info.total else "down"
    return info


_SCORE_RE = re.compile(
    r"Score:\s*\*\*(?P<letter>[A-F][+-]?)\*\*\s*\((?P<score>\d+)\s*/\s*100\)",
    re.IGNORECASE,
)
_SCORE_RE_PLAIN = re.compile(
    r"State of the OS[^:]*:\s*(?P<letter>[A-F][+-]?)\s*\((?P<score>\d+)\)",
    re.IGNORECASE,
)


def load_day_score(agents_payload: dict | None = None) -> DayScore:
    # Prefer latest markdown eval report.
    if EVAL_DIR.is_dir():
        md_files = sorted(
            EVAL_DIR.glob("*.md"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        for path in md_files:
            try:
                text = path.read_text(errors="replace")
            except OSError:
                continue
            m = _SCORE_RE.search(text)
            if m:
                return DayScore(
                    letter=m.group("letter"),
                    score=int(m.group("score")),
                    source=str(path),
                )
        # JSON nightly samples
        json_files = sorted(
            EVAL_DIR.glob("*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        for path in json_files:
            try:
                data = json.loads(path.read_text())
            except (OSError, json.JSONDecodeError):
                continue
            raw = data.get("score")
            if isinstance(raw, (int, float)):
                score = int(raw * 100) if raw <= 1 else int(raw)
                return DayScore(score=score, source=str(path))

    # Fallback: chef-eval agent summary from /api/agents
    if agents_payload:
        for agent in agents_payload.get("agents") or []:
            if (agent.get("agent") or "") != "chef-eval":
                continue
            summary = agent.get("summary") or ""
            m = _SCORE_RE_PLAIN.search(summary)
            if m:
                return DayScore(
                    letter=m.group("letter"),
                    score=int(m.group("score")),
                    source="api/agents",
                )
    return DayScore()


def _usage_frac(requests: int | None, tokens: int | None) -> tuple[float, str, str]:
    if requests is None and tokens is None:
        return 0.0, "ok", ""
    req = requests or 0
    tok = tokens or 0
    frac = max(req / max(OCX_REQ_BUDGET, 1), tok / max(OCX_TOK_BUDGET, 1))
    frac = max(0.0, min(frac, 1.0))
    if frac >= 0.9:
        level = "down"
    elif frac >= 0.7:
        level = "warn"
    else:
        level = "ok"
    text = f"{req} req · {tok:,} tok"
    return frac, level, text


def _build_providers(overview: dict | None) -> list[ProviderRow]:
    rows: list[ProviderRow] = []
    for provider in (overview or {}).get("providers") or []:
        accounts = list(provider.get("accounts") or [])
        active_id = provider.get("activeAccountId")
        active = next((item for item in accounts if item.get("id") == active_id), None)
        usage = provider.get("usage") or {}
        requests = usage.get("requests")
        tokens = usage.get("tokens")
        req = int(requests) if isinstance(requests, (int, float)) else None
        tok = int(tokens) if isinstance(tokens, (int, float)) else None
        frac, level, usage_text = _usage_frac(req, tok)
        source = provider.get("source") or "vault"
        provider_id = provider.get("id") or "custom"
        driver = provider_id.removeprefix("cpm:") if source == "cpm" else None
        refresh = provider.get("refresh") or {}
        if provider.get("availability") == "unavailable" or provider.get("error"):
            level = "down"
        elif provider.get("stale") or refresh.get("error"):
            level = "warn"
        rows.append(
            ProviderRow(
                provider=provider_id,
                label=provider.get("label") or PROVIDER_LABELS.get(provider_id, provider_id.title()),
                active_label=(active or {}).get("label"),
                active_id=active_id,
                color="#8A877C",
                source=source,
                driver=driver,
                accounts=accounts,
                requests=int(requests) if isinstance(requests, (int, float)) else None,
                tokens=int(tokens) if isinstance(tokens, (int, float)) else None,
                usage_frac=frac,
                usage_level=level,
                usage_text=usage_text
                or ((active or {}).get("label") or f"{len(accounts)} accounts"),
            )
        )
    return rows


def _build_agents(agents_payload: dict | None) -> list[AgentRow]:
    rows: list[AgentRow] = []
    if not agents_payload:
        return rows
    items = list(agents_payload.get("agents") or [])
    # running first, then by lastActivity desc
    def sort_key(a: dict) -> tuple:
        running = 0 if (a.get("status") or "").lower() == "running" else 1
        return (running, -( _parse_ts(a.get("lastActivity")) ))

    items.sort(key=sort_key)
    for a in items[:8]:
        status = (a.get("status") or "unknown").lower()
        rows.append(
            AgentRow(
                key=a.get("key") or f"{a.get('agent')}::{a.get('workspace')}",
                agent=a.get("agent") or "?",
                workspace=a.get("workspace") or "?",
                status=status,
                summary=(a.get("summary") or "")[:80],
                last_activity=a.get("lastActivity"),
                running=status == "running",
            )
        )
    return rows


def _parse_ts(value: Any) -> float:
    if not value or not isinstance(value, str):
        return 0.0
    text = value.strip()
    # Normalize +0200 → +02:00 for fromisoformat
    if len(text) >= 5 and (text[-5] in "+-") and text[-3] != ":":
        text = text[:-2] + ":" + text[-2:]
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return 0.0


def _build_fleet(fleet_payload: dict | None) -> FleetInfo:
    info = FleetInfo()
    if not fleet_payload:
        return info
    info.stale = bool(fleet_payload.get("stale"))
    info.host = fleet_payload.get("host")
    nodes: list[dict] = []
    self_node = fleet_payload.get("self")
    if isinstance(self_node, dict):
        nodes.append(self_node)
    peers = fleet_payload.get("peers") or []
    if isinstance(peers, list):
        nodes.extend(p for p in peers if isinstance(p, dict))
    info.total = len(nodes)
    info.online = sum(1 for n in nodes if n.get("online"))
    return info


NEEDS_YOU_STATUSES = frozenset({"blocked", "waiting", "needs_input", "input", "attention"})
FAILED_STATUSES = frozenset({"failed", "error", "crashed"})


def tray_state(snap: Snapshot) -> tuple[str, str]:
    """Snapshot -> (tray-iconstate, NL tooltip) volgens de statuslijn-spec.

    Prioriteit: offline > fout > hulp > bezig > stil.
    """
    if snap.error:
        return "offline", "ChefGroep · alles offline"
    # health.total == 0 betekent "nog geen data", geen storing.
    if snap.health.level == "down" and snap.health.total > 0:
        status = snap.raw.get("status") if isinstance(snap.raw, dict) else None
        services = (status or {}).get("services") or [] if isinstance(status, dict) else []
        down = next(
            (
                svc.get("name")
                for svc in services
                if isinstance(svc, dict)
                and (svc.get("state") or "").lower() not in ("running", "ok", "healthy")
            ),
            None,
        )
        return "fout", f"ChefGroep · {down or 'een dienst'} hapert"
    failed = next((a for a in snap.agents if a.status in FAILED_STATUSES), None)
    if failed:
        return "fout", f"ChefGroep · {failed.agent} hapert"
    if any(a.status in NEEDS_YOU_STATUSES for a in snap.agents):
        return "hulp", "ChefGroep · even jou nodig"
    running = sum(1 for a in snap.agents if a.running)
    if running:
        return "bezig", f"ChefGroep · {running} aan het werk"
    if snap.health.level == "warn":
        return "hulp", "ChefGroep · even jou nodig"
    return "stil", "ChefGroep · nog niks gebeurd vandaag"


def fetch_snapshot() -> Snapshot:
    """One parallel fetch-cycle; never raises."""
    snap = Snapshot()
    paths = {
        "status": "/status",
        "accounts_overview": "/accounts/overview",
        "agents": "/agents",
        "agent_events": "/agents/events?limit=8",
        "fleet": "/fleet",
        "tasks": "/commander/tasks?limit=12",
        "clipboard": "/clipboard",
        "desktop": "/desktop/status",
        "share_sync": "/share-sync/status",
    }
    results: dict[str, Any] = {}
    try:
        with ThreadPoolExecutor(max_workers=len(paths)) as pool:
            futs = {pool.submit(api_request, path): key for key, path in paths.items()}
            for fut in as_completed(futs):
                key = futs[fut]
                try:
                    results[key] = fut.result()
                except Exception as exc:  # noqa: BLE001
                    log.warning("fetch %s crashed: %s", key, exc)
                    results[key] = None
    except Exception as exc:  # noqa: BLE001
        snap.error = str(exc)
        log.exception("fetch_snapshot faalde")

    snap.raw = results
    if all(results.get(k) is None for k in paths):
        snap.error = "Vault-API reageert niet"

    snap.health = load_health()
    # If watchdog missing, degrade from /api/status services
    if snap.health.total == 0 and isinstance(results.get("status"), dict):
        services = results["status"].get("services") or []
        snap.health.total = len(services)
        for svc in services:
            state = (svc.get("state") or "").lower()
            if state in ("running", "ok", "healthy"):
                snap.health.ok += 1
            elif state in ("degraded", "warn", "unknown"):
                snap.health.warn += 1
            else:
                snap.health.down += 1
        if snap.health.down:
            snap.health.level = "down"
        elif snap.health.warn:
            snap.health.level = "warn"
        elif snap.health.total:
            snap.health.level = "ok"

    agents = results.get("agents") if isinstance(results.get("agents"), dict) else None
    snap.day_score = load_day_score(agents)
    accounts_overview = (
        results.get("accounts_overview")
        if isinstance(results.get("accounts_overview"), dict)
        else None
    )
    snap.revision = int((accounts_overview or {}).get("revision") or 0)
    snap.providers = _build_providers(accounts_overview)
    snap.agents = _build_agents(agents)
    fleet = results.get("fleet") if isinstance(results.get("fleet"), dict) else None
    snap.fleet = _build_fleet(fleet)
    tasks = results.get("tasks")
    snap.tasks = list(tasks.get("tasks") or []) if isinstance(tasks, dict) else []
    clipboard = results.get("clipboard")
    snap.clipboard = list(clipboard.get("items") or []) if isinstance(clipboard, dict) else []
    agent_events = results.get("agent_events")
    snap.events = list(agent_events.get("events") or []) if isinstance(agent_events, dict) else []
    connector_ops = fetch_connector_events("ops", limit=8)
    if connector_ops:
        snap.events = connector_ops + snap.events
    snap.desktop = results.get("desktop") if isinstance(results.get("desktop"), dict) else {}
    snap.share_sync = (
        results.get("share_sync") if isinstance(results.get("share_sync"), dict) else {}
    )
    snap.fetched_at = datetime.now()
    return snap
