#!/usr/bin/env python3
"""Lifecycle helper for the ChefBar Daytona emergency sandbox.

Create or reuse chefbar-cursor-emergency, start it if it is stopped/archived,
refresh packages inside it, and keep idle auto-stop + auto-archive armed.

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
SANDBOX_NAME = os.environ.get("DAYTONA_SANDBOX_NAME", "chefbar-cursor-emergency")
LABELS = {"role": "cursor-cloud-emergency", "repo": "chefbar"}
# Idle (no Daytona events) → stop. Stopped → archive. Never auto-delete the named box.
AUTO_STOP_MINUTES = int(os.environ.get("DAYTONA_AUTO_STOP_MINUTES", "15"))
AUTO_ARCHIVE_MINUTES = int(os.environ.get("DAYTONA_AUTO_ARCHIVE_MINUTES", "1440"))

REFRESH_SCRIPT = r"""
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
export PATH="$HOME/.bun/bin:/usr/local/share/nvm/current/bin:/usr/local/bin:/usr/bin:${PATH:-}"
sudo -n dpkg --configure -a || true
sudo -n apt-get -o Dpkg::Options::=--force-confold -y --fix-broken install || true
sudo -n apt-get update -qq
# Tiny snapshot (1 CPU / 1 GB): do not dist-upgrade the whole image.
sudo -n apt-get install -y --only-upgrade --no-install-recommends ca-certificates curl || true
python3 -m pip install -U pip
if ! bun upgrade >/dev/null 2>&1; then
  curl -fsSL https://bun.sh/install | BUN_INSTALL="$HOME/.bun" bash
fi
export PATH="$HOME/.bun/bin:$PATH"
echo "daytona refresh: python=$(python3 --version 2>/dev/null | tr -d '\n') bun=$(bun --version 2>/dev/null || echo missing)"
"""


def _die(message: str, code: int = 1) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)


def _api_key() -> str:
    key = os.environ.get("DAYTONA_API_KEY", "").strip()
    if not key:
        _die("DAYTONA_API_KEY is not set")
    if not key.startswith("dtn_"):
        _die("DAYTONA_API_KEY has an unexpected prefix")
    if len(key) != 68:
        _die(
            f"DAYTONA_API_KEY length is {len(key)}, expected 68 "
            "(dtn_ + 64 hex). Re-save the full key in Cursor secrets."
        )
    return key


def _request(method: str, path: str, body: dict[str, Any] | None = None) -> tuple[int, Any]:
    payload = None if body is None else json.dumps(body).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {_api_key()}",
        "Accept": "application/json",
    }
    if payload is not None:
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(
        f"{API_URL}{path}",
        data=payload,
        method=method,
        headers=headers,
    )
    ctx = ssl.create_default_context()
    try:
        with urllib.request.urlopen(req, timeout=90, context=ctx) as resp:
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
            "autoArchiveInterval": AUTO_ARCHIVE_MINUTES,
        },
    )
    if status >= 400:
        _die(f"POST /sandbox failed: HTTP {status} {data}")
    if not isinstance(data, dict):
        _die(f"unexpected POST payload type: {type(data).__name__}")
    return data


def start_sandbox(name_or_id: str) -> dict[str, Any]:
    status, data = _request("POST", f"/sandbox/{name_or_id}/start")
    if status >= 400:
        recover_status, recover_data = _request("POST", f"/sandbox/{name_or_id}/recover")
        if recover_status >= 400:
            _die(f"POST /sandbox/{name_or_id}/start failed: HTTP {status} {data}")
        data = recover_data
    if data and not isinstance(data, dict):
        _die(f"unexpected start payload type: {type(data).__name__}")
    return data if isinstance(data, dict) else {}


def stop_sandbox(name_or_id: str) -> None:
    status, data = _request("POST", f"/sandbox/{name_or_id}/stop")
    if status >= 400:
        _die(f"POST /sandbox/{name_or_id}/stop failed: HTTP {status} {data}")


def arm_lifecycle(name_or_id: str) -> None:
    stop_status, stop_data = _request("POST", f"/sandbox/{name_or_id}/autostop/{AUTO_STOP_MINUTES}")
    if stop_status >= 400:
        _die(f"autostop failed: HTTP {stop_status} {stop_data}")
    archive_status, archive_data = _request(
        "POST", f"/sandbox/{name_or_id}/autoarchive/{AUTO_ARCHIVE_MINUTES}"
    )
    if archive_status >= 400:
        _die(f"autoarchive failed: HTTP {archive_status} {archive_data}")


def wait_state(
    sandbox_id: str,
    wanted: set[str],
    fatal: set[str],
    timeout: int,
) -> dict[str, Any]:
    deadline = time.time() + timeout
    current = get_sandbox(sandbox_id)
    if current is None:
        _die(f"sandbox {sandbox_id} disappeared")
    while time.time() < deadline:
        state = _state(current)
        if state in wanted:
            return current
        if state in fatal:
            _die(f"sandbox {sandbox_id} entered terminal state {state}")
        time.sleep(2)
        fetched = get_sandbox(sandbox_id)
        if fetched is None:
            _die(f"sandbox {sandbox_id} disappeared while waiting")
        current = fetched
    _die(f"timed out waiting for sandbox {sandbox_id} states={sorted(wanted)}")
    return current


def sdk_client() -> Any:
    try:
        from daytona import Daytona, DaytonaConfig
    except ImportError:
        _die("daytona SDK is not installed; pip install daytona  (venv is fine)")
    return Daytona(DaytonaConfig(api_key=_api_key(), api_url=API_URL))


def refresh(sandbox_id: str) -> None:
    import shlex

    command = "bash -lc " + shlex.quote(REFRESH_SCRIPT.strip())
    last_error = ""
    for attempt in range(1, 3):
        current = get_sandbox(sandbox_id)
        if current is None:
            _die(f"sandbox {sandbox_id} disappeared before refresh")
        if _state(current) not in {"started", "running"}:
            print(f"refresh: sandbox state={current.get('state')}; starting (attempt {attempt})")
            start_sandbox(sandbox_id)
            wait_state(
                sandbox_id,
                {"started", "running"},
                {"error", "destroyed", "deleted", "build_failed"},
                timeout=180,
            )
            time.sleep(3)
        try:
            client = sdk_client()
            sandbox = client.get(sandbox_id)
            response = sandbox.process.exec(command, timeout=180)
        except Exception as exc:  # noqa: BLE001 — SDK wraps transport errors
            last_error = str(exc)
            print(f"refresh attempt {attempt} failed: {last_error}", file=sys.stderr)
            time.sleep(2)
            continue
        print(response.result.rstrip())
        if response.exit_code == 0:
            return
        last_error = f"exit {response.exit_code}: {response.result}"
        print(f"refresh attempt {attempt} failed: {last_error}", file=sys.stderr)
    _die(f"refresh failed: {last_error}")


def smoke(sandbox_id: str) -> None:
    client = sdk_client()
    sandbox = client.get(sandbox_id)
    response = sandbox.process.code_run('print("Hello World from code!")', timeout=60)
    print(f"code_run exit={response.exit_code}")
    print(response.result.rstrip())
    if response.exit_code != 0:
        _die(f"smoke failed: {response.result}")


def ensure_started() -> dict[str, Any]:
    existing = get_sandbox(SANDBOX_NAME)
    if existing is None:
        sandbox = create_sandbox()
        action = "created"
    else:
        sandbox = existing
        action = "reused"

    sandbox_id = str(sandbox.get("id") or "")
    if not sandbox_id:
        _die("sandbox payload has no id")

    state = _state(sandbox)
    if state not in {"started", "running"}:
        print(f"{action} sandbox id={sandbox_id} state={state}; starting")
        start_sandbox(sandbox_id)
        action = "started"
    sandbox = wait_state(
        sandbox_id,
        {"started", "running"},
        {"error", "destroyed", "deleted", "build_failed"},
        timeout=180,
    )
    arm_lifecycle(sandbox_id)
    print(
        f"{action} sandbox name={SANDBOX_NAME} id={sandbox_id} "
        f"state={sandbox.get('state')} autostop={AUTO_STOP_MINUTES}m "
        f"autoarchive={AUTO_ARCHIVE_MINUTES}m"
    )
    return sandbox


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--ensure",
        action="store_true",
        help="create/start the sandbox if needed (default when no other action is given)",
    )
    parser.add_argument(
        "--refresh",
        action="store_true",
        help="apt/pip/bun upgrade inside the running sandbox",
    )
    parser.add_argument(
        "--stop",
        action="store_true",
        help="stop the sandbox now (idle auto-stop still applies otherwise)",
    )
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="run the Daytona hello-world snippet inside the sandbox",
    )
    args = parser.parse_args()

    if not (args.ensure or args.refresh or args.stop or args.smoke):
        args.ensure = True

    if args.stop and (args.refresh or args.smoke):
        _die("--stop cannot be combined with --refresh or --smoke")

    if args.stop:
        existing = get_sandbox(SANDBOX_NAME)
        if existing is None:
            print(f"sandbox {SANDBOX_NAME} does not exist")
            return
        sandbox_id = str(existing.get("id") or "")
        state = _state(existing)
        if state in {"stopped", "archived", "destroyed", "deleted"}:
            print(f"sandbox id={sandbox_id} already {state}")
            return
        stop_sandbox(sandbox_id)
        wait_state(
            sandbox_id,
            {"stopped", "archived", "destroyed"},
            {"error", "build_failed"},
            timeout=120,
        )
        print(f"stopped sandbox name={SANDBOX_NAME} id={sandbox_id}")
        return

    sandbox = ensure_started()
    sandbox_id = str(sandbox.get("id") or "")
    if args.refresh:
        refresh(sandbox_id)
    if args.smoke:
        smoke(sandbox_id)


if __name__ == "__main__":
    main()
