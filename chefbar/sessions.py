"""CHEF-shaped injectable sessions.

A session is an attachable live work object, not an inbox item. Chefbar may
surface it contextually and hand control to Kater, herdr, browser, or evidence.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal

SessionSource = Literal["cursor", "kater", "herdr", "opencodex", "browser"]
SessionState = Literal["starting", "working", "waiting", "blocked", "done", "failed"]


@dataclass(frozen=True)
class AttachPoints:
    focus: str | None = None
    browser: str | None = None
    workspace_url: str | None = None
    kater_session_id: str | None = None
    evidence_url: str | None = None

    @classmethod
    def from_dict(cls, raw: dict[str, Any] | None) -> "AttachPoints":
        raw = raw or {}
        return cls(
            focus=raw.get("focus"),
            browser=raw.get("browser"),
            workspace_url=raw.get("workspaceUrl"),
            kater_session_id=raw.get("katerSessionId"),
            evidence_url=raw.get("evidenceUrl"),
        )


@dataclass(frozen=True)
class Session:
    id: str
    source: SessionSource
    state: SessionState
    title: str
    summary: str = ""
    updated_at: str | None = None
    attach: AttachPoints = field(default_factory=AttachPoints)
    capabilities: tuple[str, ...] = ()

    @classmethod
    def from_dict(cls, raw: dict[str, Any]) -> "Session":
        session_id = str(raw.get("id") or "").strip()
        if not session_id:
            raise ValueError("session mist id")
        source = str(raw.get("source") or "").lower()
        state = str(raw.get("state") or "").lower()
        if source not in {"cursor", "kater", "herdr", "opencodex", "browser"}:
            raise ValueError(f"onbekende session source: {source}")
        if state not in {"starting", "working", "waiting", "blocked", "done", "failed"}:
            raise ValueError(f"onbekende session state: {state}")
        return cls(
            id=session_id,
            source=source,  # type: ignore[arg-type]
            state=state,  # type: ignore[arg-type]
            title=str(raw.get("title") or session_id),
            summary=str(raw.get("summary") or ""),
            updated_at=raw.get("updatedAt"),
            attach=AttachPoints.from_dict(raw.get("attach")),
            capabilities=tuple(str(item) for item in raw.get("capabilities") or ()),
        )

    @property
    def needs_attention(self) -> bool:
        return self.state in {"waiting", "blocked", "failed"}

    @property
    def primary_action(self) -> tuple[str, str] | None:
        if self.attach.kater_session_id:
            return ("Open sessie", "kater")
        if self.attach.focus:
            return ("Neem over", "focus")
        if self.attach.workspace_url:
            return ("Open workspace", "workspace")
        if self.attach.browser:
            return ("Open browser", "browser")
        if self.attach.evidence_url:
            return ("Bekijk evidence", "evidence")
        return None


_STATE_RANK = {
    "blocked": 0,
    "waiting": 1,
    "failed": 2,
    "working": 3,
    "starting": 4,
    "done": 5,
}


def _session_from_event(raw: dict[str, Any]) -> Session | None:
    """Map vault connector/API payloads into Session objects."""
    try:
        if raw.get("source") and raw.get("state"):
            return Session.from_dict(raw)
        payload = raw.get("payload")
        if isinstance(payload, dict):
            merged = {**payload, **{k: raw[k] for k in ("id", "source", "state", "title") if k in raw}}
            return Session.from_dict(merged)
        if raw.get("id") and raw.get("kind") == "session":
            return Session.from_dict(
                {
                    "id": raw["id"],
                    "source": raw.get("source") or "kater",
                    "state": raw.get("state") or "working",
                    "title": raw.get("title") or raw.get("summary") or str(raw["id"]),
                    "summary": str(raw.get("summary") or ""),
                    "updatedAt": raw.get("updatedAt") or raw.get("ts"),
                    "attach": raw.get("attach") or {},
                }
            )
    except ValueError:
        return None
    return None


def load_ranked_sessions(_vault: Any = None) -> list[Session]:
    """Fetch injectable sessions and rank attention-first for the command bar."""
    from . import api

    rows: list[Session] = []
    for raw in api.fetch_sessions():
        if not isinstance(raw, dict):
            continue
        session = _session_from_event(raw)
        if session is not None:
            rows.append(session)

    rows.sort(
        key=lambda item: (
            _STATE_RANK.get(item.state, 9),
            0 if item.needs_attention else 1,
            item.title.lower(),
        )
    )
    return rows[:12]


def event_to_session(raw: dict) -> Session | None:
    """Map vault connector event or /api/sessions row to Session."""
    payload = raw.get("payload") if isinstance(raw.get("payload"), dict) else raw
    if not isinstance(payload, dict):
        return None
    try:
        return Session.from_dict(payload)
    except ValueError:
        return None


def load_ranked_sessions(_vault_snap: object | None = None) -> list[Session]:
    """Ranked injectable sessions for the command bar (Kater-first attach)."""
    from . import api

    rows: list[Session] = []
    for item in api.fetch_sessions():
        if not isinstance(item, dict):
            continue
        session = event_to_session(item)
        if session is not None:
            rows.append(session)

    def sort_key(session: Session) -> tuple[int, str]:
        if session.needs_attention:
            priority = 0
        elif session.state == "working":
            priority = 1
        else:
            priority = 2
        return (priority, session.updated_at or "")

    rows.sort(key=sort_key)
    return rows[:6]
