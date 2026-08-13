#!/usr/bin/env python3
"""Create or reuse the ChefBar Daytona emergency sandbox.

Reads DAYTONA_API_KEY from the environment. Never prints the key.
"""

from __future__ import annotations

import argparse
import json
import os
import ssl
import sys
import time
import urllib.error
import urllib.request
from typing import Any

API_URL = os.environ.get("DAYTONA_API_URL", "https://app.daytona.io/api").rstrip("/")
SANDBOX_NAME = "chefbar-cursor-emergency"
LABELS = {"role": "cursor-cloud-emergency", "repo": "chefbar"}
AUTO_STOP_MINUTES = 60


def _die(message: str, code: int = 1) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def _api_key() -> str:
    key = os.environ.get("DAYTONA_API_KEY", "").strip()
    if not key:
        _die("DAYTONA_API_KEY is not set")
    if not key.startswith("dtn_"):
        _die("DAYTONA_API_KEY has an unexpected prefix")
    # Cursor's secret store previously truncated this value to 67 chars
    # (63 hex digits). A valid key is dtn_ + 64 hex = 68.
    if len(key) != 68:
        _die(
            f"DAYTONA_API_KEY length is {len(key)}, expected 68 "
            "(dtn_ + 64 hex). Re-save the full key in Cursor secrets."
        )
    return key


def _request(method: str, path: str, body: dict[str, Any] | None = None) -> tuple[int, Any]:
    payload = None if body is None else json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        f"{API_URL}{path}",
        data=payload,
        method=method,
        headers={
            "Authorization": f"Bearer {_api_key()}",
            "Accept": "application/json",
            "Content-Type": "application/json",
        },
    )
    ctx = ssl.create_default_context()
    try:
        with urllib.request.urlopen(req, timeout=60, context=ctx) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            parsed: Any = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            parsed = {"message": raw}
        return exc.code, parsed


def _state(data: dict[str, Any]) -> str:
    value = data.get("state") or data.get("status") or ""
    return str(value).lower()


def get_sandbox(name_or_id: str) -> dict[str, Any] | None:
    status, data = _request("GET", f"/sandbox/{name_or_id}")
    if status == 404:
        return None
    if status >= 400:
        _die(f"GET /sandbox/{name_or_id} failed: HTTP {status} {data}")
    if not isinstance(data, dict):
        _die(f"unexpected GET payload type: {type(data).__name__}")
    return data


def create_sandbox() -> dict[str, Any]:
    status, data = _request(
        "POST",
        "/sandbox",
        {
            "name": SANDBOX_NAME,
            "labels": LABELS,
            "autoStopInterval": AUTO_STOP_MINUTES,
        },
    )
    if status >= 400:
        _die(f"POST /sandbox failed: HTTP {status} {data}")
    if not isinstance(data, dict):
        _die(f"unexpected POST payload type: {type(data).__name__}")
    return data


def wait_started(sandbox: dict[str, Any], timeout: int = 120) -> dict[str, Any]:
    sandbox_id = str(sandbox.get("id") or "")
    if not sandbox_id:
        _die("sandbox payload has no id")
    deadline = time.time() + timeout
    current = sandbox
    while time.time() < deadline:
        state = _state(current)
        if state in {"started", "running"}:
            return current
        if state in {"error", "destroyed", "deleted"}:
            _die(f"sandbox {sandbox_id} entered terminal state {state}")
        time.sleep(2)
        fetched = get_sandbox(sandbox_id)
        if fetched is None:
            _die(f"sandbox {sandbox_id} disappeared while starting")
        current = fetched
    _die(f"timed out waiting for sandbox {sandbox_id} to start")
    return current


def smoke(sandbox_id: str) -> None:
    try:
        from daytona import Daytona, DaytonaConfig
    except ImportError:
        _die("daytona SDK is not installed; pip install daytona  (venv is fine)")

    client = Daytona(DaytonaConfig(api_key=_api_key(), api_url=API_URL))
    sandbox = client.get(sandbox_id)
    response = sandbox.process.code_run('print("Hello World from code!")', timeout=60)
    print(f"code_run exit={response.exit_code}")
    print(response.result.rstrip())
    if response.exit_code != 0:
        _die(f"smoke failed: {response.result}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="run the Daytona hello-world snippet inside the sandbox",
    )
    args = parser.parse_args()

    existing = get_sandbox(SANDBOX_NAME)
    if existing is None:
        sandbox = create_sandbox()
        action = "created"
    else:
        sandbox = existing
        action = "reused"

    sandbox = wait_started(sandbox)
    sandbox_id = sandbox.get("id")
    print(
        f"{action} sandbox name={SANDBOX_NAME} id={sandbox_id} "
        f"state={sandbox.get('state')}"
    )
    if args.smoke:
        if not isinstance(sandbox_id, str) or not sandbox_id:
            _die("sandbox id missing after start")
        smoke(sandbox_id)


if __name__ == "__main__":
    main()
