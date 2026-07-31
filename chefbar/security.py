"""Network safety policy for remote Chefbar clients.

Primary path: HTTPS to PROFILE hosts / *.chefgroep.online / explicit allowlist.
Optional: loopback, Tailscale CGNAT, *.ts.net (never required).
Bearer tokens never follow redirects. Same-origin join only.
"""

from __future__ import annotations

import ipaddress
import os
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from urllib.parse import urljoin, urlparse

TAILNET = ipaddress.ip_network("100.64.0.0/10")
LOOPBACK_NAMES = {"localhost", "localhost.localdomain"}
DEFAULT_ONLINE_SUFFIXES = (".chefgroep.online",)


def _host_set(env_name: str) -> frozenset[str]:
    return frozenset(
        item.strip().lower().rstrip(".")
        for item in os.environ.get(env_name, "").split(",")
        if item.strip()
    )


def _online_suffixes() -> tuple[str, ...]:
    raw = os.environ.get("CHEFBAR_ONLINE_SUFFIXES", "").strip()
    if not raw:
        return DEFAULT_ONLINE_SUFFIXES
    return tuple(
        item if item.startswith(".") else f".{item}"
        for item in (part.strip().lower() for part in raw.split(","))
        if item
    )


def _env_flag(name: str, default: bool = True) -> bool:
    raw = os.environ.get(name)
    if raw is None:
        return default
    return raw.strip() not in {"0", "false", "False", "no", "off"}


@dataclass(frozen=True)
class EndpointPolicy:
    https_allowlist: frozenset[str] = field(default_factory=lambda: _host_set("CHEFBAR_HTTPS_ALLOWLIST"))
    http_allowlist: frozenset[str] = field(default_factory=lambda: _host_set("CHEFBAR_HTTP_ALLOWLIST"))
    online_suffixes: tuple[str, ...] = field(default_factory=_online_suffixes)
    allow_tsnet_https: bool = field(default_factory=lambda: _env_flag("CHEFBAR_ALLOW_TSNET_HTTPS", True))
    allow_tailnet_http: bool = field(default_factory=lambda: _env_flag("CHEFBAR_ALLOW_TAILNET_HTTP", True))
    allow_profile_https_hosts: frozenset[str] = field(default_factory=frozenset)

    def with_profile_hosts(self, *urls: str | None) -> "EndpointPolicy":
        hosts: set[str] = set(self.allow_profile_https_hosts)
        for url in urls:
            if not url:
                continue
            host = (urlparse(url).hostname or "").lower().rstrip(".")
            if host:
                hosts.add(host)
        return EndpointPolicy(
            https_allowlist=self.https_allowlist,
            http_allowlist=self.http_allowlist,
            online_suffixes=self.online_suffixes,
            allow_tsnet_https=self.allow_tsnet_https,
            allow_tailnet_http=self.allow_tailnet_http,
            allow_profile_https_hosts=frozenset(hosts),
        )

    def _is_private_online(self, host: str) -> bool:
        return any(host == suffix.lstrip(".") or host.endswith(suffix) for suffix in self.online_suffixes)

    def allows(self, url: str) -> bool:
        parsed = urlparse(url)
        host = (parsed.hostname or "").lower().rstrip(".")
        if parsed.scheme not in {"http", "https"} or not host:
            return False
        if host in LOOPBACK_NAMES:
            return True
        try:
            address = ipaddress.ip_address(host)
            if address.is_loopback:
                return True
            if parsed.scheme == "http" and self.allow_tailnet_http and address in TAILNET:
                return True
            return False
        except ValueError:
            pass
        if parsed.scheme == "https":
            if host in self.https_allowlist:
                return True
            if host in self.allow_profile_https_hosts:
                return True
            if self._is_private_online(host):
                return True
            if self.allow_tsnet_https and host.endswith(".ts.net"):
                return True
            return False
        return host in self.http_allowlist

    def require(self, url: str) -> None:
        if not self.allows(url):
            raise ValueError(f"Chefbar blokkeert niet-toegestaan endpoint: {url}")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        raise urllib.error.HTTPError(req.full_url, code, "redirect geblokkeerd", headers, fp)


POLICY = EndpointPolicy()
OPENER = urllib.request.build_opener(NoRedirect())


def safe_urlopen(
    request: str | urllib.request.Request,
    *,
    timeout: float = 5.0,
    policy: EndpointPolicy | None = None,
):
    active = policy or POLICY
    url = request.full_url if isinstance(request, urllib.request.Request) else request
    active.require(url)
    return OPENER.open(request, timeout=timeout)


def safe_join(base: str, path: str, *, policy: EndpointPolicy | None = None) -> str:
    active = policy or POLICY
    active.require(base)
    joined = urljoin(base.rstrip("/") + "/", path.lstrip("/"))
    base_origin = urlparse(base)
    joined_origin = urlparse(joined)
    if (base_origin.scheme, base_origin.hostname, base_origin.port) != (
        joined_origin.scheme,
        joined_origin.hostname,
        joined_origin.port,
    ):
        raise ValueError("cross-origin endpoint join geblokkeerd")
    active.require(joined)
    return joined


def open_browser_url(url: str, *, policy: EndpointPolicy | None = None) -> None:
    """xdg-open only for policy-allowed http(s) URLs."""
    import subprocess

    active = policy or POLICY
    active.require(url)
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"}:
        raise ValueError(f"URL-schema niet toegestaan: {parsed.scheme!r}")
    subprocess.Popen(
        ["xdg-open", url],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
