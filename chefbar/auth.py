"""Auth headers for ChefBar → private *.online / vault-api.

Interim: Bearer API token (+ optional Cloudflare Access service-token pair).
Doel: Authentik OIDC / Access user tokens via the same get_headers() seam —
without redesigning clients.
"""

from __future__ import annotations

import os
from pathlib import Path


def _read_bearer() -> str | None:
    env_token = os.environ.get("CHEF_VAULT_API_TOKEN") or os.environ.get("CHEFBAR_VAULT_TOKEN")
    if env_token and env_token.strip():
        return env_token.strip()
    token_file = os.environ.get("CHEFBAR_VAULT_TOKEN_FILE", "").strip()
    if token_file:
        try:
            text = Path(token_file).read_text(encoding="utf-8").strip()
            if text.startswith("CHEF_VAULT_API_TOKEN="):
                text = text.split("=", 1)[1].strip().strip("'\"")
            elif text.startswith("CHEFBAR_VAULT_TOKEN="):
                text = text.split("=", 1)[1].strip().strip("'\"")
            return text or None
        except OSError:
            return None
    # Narrow legacy: only CHEF_* keys from optional env file (no full dotenv load).
    legacy = Path(
        os.environ.get(
            "CHEFBAR_ENV_FILE",
            str(Path.home() / "Documents/Github/OnlineChefGroep/chefgroep-vault/docker/.env"),
        )
    )
    try:
        found: dict[str, str] = {}
        for line in legacy.read_text(encoding="utf-8").splitlines():
            if line.startswith("CHEF_VAULT_API_TOKEN="):
                found["CHEF_VAULT_API_TOKEN"] = line.split("=", 1)[1].strip().strip("'\"")
            elif line.startswith("CHEFBAR_VAULT_TOKEN="):
                found["CHEFBAR_VAULT_TOKEN"] = line.split("=", 1)[1].strip().strip("'\"")
        return found.get("CHEF_VAULT_API_TOKEN") or found.get("CHEFBAR_VAULT_TOKEN")
    except OSError:
        return None


def get_headers(*, json_body: bool = False) -> dict[str, str]:
    """Return request headers for vault/ops HTTPS calls."""
    headers = {"Accept": "application/json"}
    bearer = _read_bearer()
    if bearer:
        headers["Authorization"] = f"Bearer {bearer}"
    # Cloudflare Access service tokens (machine → private .online).
    client_id = os.environ.get("CF_ACCESS_CLIENT_ID") or os.environ.get(
        "CHEFBAR_CF_ACCESS_CLIENT_ID"
    )
    client_secret = os.environ.get("CF_ACCESS_CLIENT_SECRET") or os.environ.get(
        "CHEFBAR_CF_ACCESS_CLIENT_SECRET"
    )
    if client_id and client_secret:
        headers["CF-Access-Client-Id"] = client_id.strip()
        headers["CF-Access-Client-Secret"] = client_secret.strip()
    if json_body:
        headers["Content-Type"] = "application/json"
    return headers


def auth_status() -> dict[str, bool | str]:
    """Compact status for --doctor (never echo secrets)."""
    bearer = bool(_read_bearer())
    cf = bool(
        (os.environ.get("CF_ACCESS_CLIENT_ID") or os.environ.get("CHEFBAR_CF_ACCESS_CLIENT_ID"))
        and (
            os.environ.get("CF_ACCESS_CLIENT_SECRET")
            or os.environ.get("CHEFBAR_CF_ACCESS_CLIENT_SECRET")
        )
    )
    mode = "none"
    if bearer and cf:
        mode = "bearer+cf-access"
    elif bearer:
        mode = "bearer"
    elif cf:
        mode = "cf-access"
    return {
        "bearer": bearer,
        "cloudflare_access_service_token": cf,
        "mode": mode,
    }
