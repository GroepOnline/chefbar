"""Remote-ready endpoint profile for Chefbar.

One configuration object owns every network surface. Local development remains
loopback by default; production uses HTTPS *.chefgroep.online (Cloudflare).
Tailnet profiles are optional, never required.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse

DEFAULT_PROFILE_PATH = Path.home() / ".config/chefbar/endpoints.json"


@dataclass(frozen=True)
class EndpointProfile:
    name: str = "local"
    vault_api: str = "http://127.0.0.1:8321/api"
    ops_api: str = "http://127.0.0.1:10101"
    dashboard: str = "http://127.0.0.1:8080"
    desktop: str = "http://127.0.0.1:3000"
    opencodex_dashboard: str | None = None
    kater_workspace: str | None = None

    def endpoint(self, key: str) -> str | None:
        aliases = {
            "vaultApi": "vault_api",
            "opsApi": "ops_api",
            "opencodexDashboard": "opencodex_dashboard",
            "katerWorkspace": "kater_workspace",
        }
        return getattr(self, aliases.get(key, key), None)

    def label(self, key: str) -> str:
        value = self.endpoint(key)
        if not value:
            return "niet ingesteld"
        parsed = urlparse(value)
        host = parsed.hostname or value
        if parsed.port and parsed.port not in (80, 443):
            return f"{host}:{parsed.port}"
        return host

    def all_urls(self) -> tuple[str, ...]:
        return tuple(
            url
            for url in (
                self.vault_api,
                self.ops_api,
                self.dashboard,
                self.desktop,
                self.opencodex_dashboard,
                self.kater_workspace,
            )
            if url
        )


def _clean_url(value: object, fallback: str | None = None) -> str | None:
    if not isinstance(value, str) or not value.strip():
        return fallback
    parsed = urlparse(value.strip())
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ValueError(f"ongeldig Chefbar endpoint: {value!r}")
    return value.rstrip("/")


def load_profile(path: Path | None = None) -> EndpointProfile:
    profile_path = path or Path(os.environ.get("CHEFBAR_ENDPOINT_PROFILE", DEFAULT_PROFILE_PATH))
    raw: dict[str, object] = {}
    try:
        raw = json.loads(profile_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        pass
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"endpoint-profiel kon niet worden gelezen: {profile_path}: {exc}") from exc

    local = EndpointProfile()
    return EndpointProfile(
        name=str(raw.get("name") or os.environ.get("CHEFBAR_PROFILE_NAME") or local.name),
        vault_api=_clean_url(os.environ.get("CHEFBAR_VAULT_API") or raw.get("vaultApi"), local.vault_api)
        or local.vault_api,
        ops_api=_clean_url(os.environ.get("CHEFBAR_OPS_API") or raw.get("opsApi"), local.ops_api)
        or local.ops_api,
        dashboard=_clean_url(os.environ.get("CHEFBAR_DASHBOARD") or raw.get("dashboard"), local.dashboard)
        or local.dashboard,
        desktop=_clean_url(os.environ.get("CHEFBAR_DESKTOP") or raw.get("desktop"), local.desktop)
        or local.desktop,
        opencodex_dashboard=_clean_url(
            os.environ.get("CHEFBAR_OPENCODEX_DASHBOARD") or raw.get("opencodexDashboard")
        ),
        kater_workspace=_clean_url(
            os.environ.get("CHEFBAR_KATER_WORKSPACE") or raw.get("katerWorkspace")
        ),
    )


PROFILE = load_profile()
