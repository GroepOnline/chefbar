"""Live connectivity probe for ChefBar PROFILE targets."""

from __future__ import annotations

import socket
import ssl
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Callable
from urllib.parse import urlparse

from . import auth
from .endpoints import EndpointProfile, PROFILE
from . import security


@dataclass(frozen=True)
class ProbeResult:
    key: str
    url: str
    label: str
    ok: bool
    detail: str
    latency_ms: int | None = None
    auth_code: int | None = None
    allowlist: bool | None = None


def _probe_dns(host: str) -> tuple[bool, str]:
    try:
        socket.getaddrinfo(host, None, type=socket.SOCK_STREAM)
        return True, "DNS ok"
    except OSError as exc:
        return False, f"DNS faalde: {exc}"


def _probe_tls(host: str, port: int) -> tuple[bool, str]:
    context = ssl.create_default_context()
    try:
        with socket.create_connection((host, port), timeout=5) as sock:
            with context.wrap_socket(sock, server_hostname=host) as tls:
                cert = tls.getpeercert()
                subject = dict(x[0] for x in cert.get("subject", ()))
                cn = subject.get("commonName", host)
                return True, f"TLS ok · {cn}"
    except OSError as exc:
        return False, f"TLS faalde: {exc}"


def probe_url(
    key: str,
    url: str,
    *,
    policy: security.EndpointPolicy,
    fetch_path: str | None = None,
    expect_auth: bool = True,
) -> ProbeResult:
    parsed = urlparse(url)
    host = parsed.hostname or ""
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    label = host if parsed.port in (None, 80, 443) else f"{host}:{parsed.port}"

    allowlist = policy.allows(url)
    if not allowlist:
        return ProbeResult(
            key=key,
            url=url,
            label=label,
            ok=False,
            detail="Niet op allowlist",
            allowlist=False,
        )

    dns_ok, dns_detail = _probe_dns(host)
    if not dns_ok:
        return ProbeResult(
            key=key,
            url=url,
            label=label,
            ok=False,
            detail=dns_detail,
            allowlist=True,
        )

    if parsed.scheme == "https":
        tls_ok, tls_detail = _probe_tls(host, port)
        if not tls_ok:
            return ProbeResult(
                key=key,
                url=url,
                label=label,
                ok=False,
                detail=tls_detail,
                allowlist=True,
            )

    path = fetch_path or parsed.path or "/"
    if not path.startswith("/"):
        path = f"/{path}"
    try:
        target = security.safe_join(url.rstrip("/"), path.lstrip("/"), policy=policy)
    except ValueError as exc:
        return ProbeResult(
            key=key,
            url=url,
            label=label,
            ok=False,
            detail=str(exc),
            allowlist=True,
        )

    headers = auth.get_headers()
    req = urllib.request.Request(target, headers=headers, method="GET")
    started = time.perf_counter()
    auth_code: int | None = None
    try:
        with security.safe_urlopen(req, timeout=8, policy=policy) as resp:
            auth_code = resp.status
            latency = int((time.perf_counter() - started) * 1000)
            ok = 200 <= resp.status < 400
            if expect_auth and auth_code == 401:
                ok = False
                detail = "401 Unauthorized · token ontbreekt of ongeldig"
            else:
                detail = f"HTTP {resp.status}"
            return ProbeResult(
                key=key,
                url=url,
                label=label,
                ok=ok,
                detail=detail,
                latency_ms=latency,
                auth_code=auth_code,
                allowlist=True,
            )
    except urllib.error.HTTPError as exc:
        latency = int((time.perf_counter() - started) * 1000)
        auth_code = exc.code
        if exc.code == 401 and expect_auth:
            bearer = auth.auth_status().get("bearer")
            cf = auth.auth_status().get("cloudflare_access_service_token")
            if not bearer and not cf:
                detail = "401 · geen bearer/CF Access token geconfigureerd"
            else:
                detail = "401 · token geweigerd"
            return ProbeResult(
                key=key,
                url=url,
                label=label,
                ok=False,
                detail=detail,
                latency_ms=latency,
                auth_code=auth_code,
                allowlist=True,
            )
        ok = 200 <= exc.code < 500
        return ProbeResult(
            key=key,
            url=url,
            label=label,
            ok=ok,
            detail=f"HTTP {exc.code}",
            latency_ms=latency,
            auth_code=auth_code,
            allowlist=True,
        )
    except (urllib.error.URLError, TimeoutError, OSError, ValueError) as exc:
        return ProbeResult(
            key=key,
            url=url,
            label=label,
            ok=False,
            detail=str(exc),
            allowlist=True,
        )


def run_doctor(profile: EndpointProfile | None = None) -> int:
    """Print PROFILE probe results; exit 0 only when all required targets pass."""
    active = profile or PROFILE
    policy = security.POLICY.with_profile_hosts(*active.all_urls())
    auth_info = auth.auth_status()

    print(f"ChefBar doctor · profiel {active.name!r}")
    print(
        f"Auth: mode={auth_info.get('mode')} "
        f"bearer={'ja' if auth_info.get('bearer') else 'nee'} "
        f"cf_access={'ja' if auth_info.get('cloudflare_access_service_token') else 'nee'}"
    )
    print()

    targets: list[tuple[str, str, str | None, bool]] = [
        ("vaultApi", active.vault_api, "/status", True),
        ("opsApi", active.ops_api, "/api/snapshot", False),
        ("dashboard", active.dashboard, "/", False),
        ("desktop", active.desktop, "/", False),
    ]
    if active.opencodex_dashboard:
        targets.append(("opencodexDashboard", active.opencodex_dashboard, "/", False))
    if active.kater_workspace:
        targets.append(("katerWorkspace", active.kater_workspace, "/", False))

    results: list[ProbeResult] = []
    for key, url, path, expect_auth in targets:
        result = probe_url(key, url, policy=policy, fetch_path=path, expect_auth=expect_auth)
        results.append(result)
        mark = "OK" if result.ok else "FAIL"
        latency = f" · {result.latency_ms}ms" if result.latency_ms is not None else ""
        allow = ""
        if result.allowlist is False:
            allow = " · allowlist=BLOCK"
        print(f"[{mark}] {key} ({result.label}) — {result.detail}{latency}{allow}")

    required = [r for r in results if r.key in ("vaultApi", "opsApi")]
    all_ok = all(r.ok for r in required)
    print()
    if all_ok:
        print("VERDICT: PROFILE bereikbaar (geen Tailscale vereist voor deze probe).")
        return 0
    print("VERDICT: minstens één verplicht doel faalde — controleer DNS/TLS/token/allowlist.")
    return 1
