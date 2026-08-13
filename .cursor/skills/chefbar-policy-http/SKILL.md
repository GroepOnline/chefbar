---
name: chefbar-policy-http
description: ChefBar network policy, ureq client, auth headers, endpoint profile, and doctor probes. Use when editing src/policy.rs, src/http.rs, src/auth.rs, src/config.rs, src/doctor.rs, endpoints JSON, CHEFBAR_* env, Cloudflare Access, vault tokens, or allowlists.
---

# ChefBar policy + HTTP

## Profile SSOT

`EndpointProfile` from JSON camelCase (`src/config.rs`):

`name`, `vaultApi`, `opsApi`, `dashboard`, `desktop`, `opencodexDashboard`, `katerWorkspace`, `linearApi`, `vaultwardenUrl`.

Resolution: `--profile` > `CHEFBAR_ENDPOINT_PROFILE` > `~/.config/chefbar/endpoints.json`. Then **per-field** env (`CHEFBAR_VAULT_API`, …) wins. Empty env is ignored. Invalid URLs fall back via `clean_url`.

Examples: `config/endpoints.example.json`, `config/endpoints.tailnet.example.json`.

## Policy

`EndpointPolicy` (`src/policy.rs`):

- HTTPS to profile hosts, `*.chefgroep.online` (or `CHEFBAR_ONLINE_SUFFIXES`), `CHEFBAR_HTTPS_ALLOWLIST`
- Loopback always allowed
- `*.ts.net` HTTPS optional; Tailnet CGNAT `100.64.0.0/10` HTTP only if `CHEFBAR_ALLOW_TAILNET_HTTP`
- `safe_join` stays same-origin

## Client

`http::Client`: timeout 5s default, `ureq` **redirects(0)**. GET/POST JSON. Actor uses 2s per-endpoint timeout inside an 8s budget.

Never follow redirects with a bearer. Never log Authorization or CF Access secrets.

## Auth seam

`auth::get_headers()` per call (`docs/auth-remote.md`):

- Vault bearer: `CHEF_VAULT_API_TOKEN` / `CHEFBAR_VAULT_TOKEN` / `CHEFBAR_VAULT_TOKEN_FILE` / fallback `~/ChefFactory/chefgroep-vault/docker/.env`
- Cloudflare Access: `CF_ACCESS_CLIENT_ID` + `CF_ACCESS_CLIENT_SECRET` (or `CHEFBAR_CF_*`). Pair must be complete.

OIDC lands on the same seam later (`auth.chefgroep.online`). Do not invent a second header builder.

## Doctor

`src/doctor.rs`: IPC-first (live instance wins). Exit 0 ok, 1 warn, 2 error. Probes DNS/TLS/allowlist/auth **fingerprints** (`sha256[:12]`). Also `chefbar --show-config` (hosts only).

## Checklist for a new origin

1. Add optional field on `RawProfile` / `EndpointProfile` + env override
2. Confirm policy allows the host
3. Fetch only from `state.rs` actor
4. Parse in `models.rs` (tolerant)
5. Doctor probe + `--show-config` line
6. No token on `Snapshot`
