---
name: chefbar-policy-http
description: ChefBar policy and HTTP skill for EndpointPolicy, ureq Client redirects(0), auth get_headers, and endpoint profile config.rs. Use when editing policy.rs, http.rs, auth.rs, config.rs, endpoints JSON, CHEFBAR_* env, Cloudflare Access CF_ACCESS_CLIENT_ID, vault bearer CHEFBAR_VAULT_TOKEN, allowlists, or safe_join. Use when a new origin, redirect, or token might leak into logs or Snapshot.
---

# ChefBar policy + HTTP

Every byte that leaves the machine goes through `EndpointPolicy` + `auth::get_headers` + `http::Client`.

## Instructions

1. Profile SSOT: camelCase JSON (`src/config.rs`) — `name`, `vaultApi`, `opsApi`, `dashboard`, `desktop`, `opencodexDashboard`, `katerWorkspace`, `linearApi`, `vaultwardenUrl`.
2. Resolution: `--profile` > `CHEFBAR_ENDPOINT_PROFILE` > `~/.config/chefbar/endpoints.json`. Then **per-field** env wins (`CHEFBAR_VAULT_API`, …). Empty env ignored. Invalid URLs → `clean_url` fallback.
3. `EndpointPolicy::allows`:
   - https to profile hosts, `*.chefgroep.online` (`CHEFBAR_ONLINE_SUFFIXES`), `CHEFBAR_HTTPS_ALLOWLIST`
   - loopback always
   - `*.ts.net` https if `CHEFBAR_ALLOW_TSNET_HTTPS` (default true)
   - Tailnet CGNAT `100.64.0.0/10` **http** only if `CHEFBAR_ALLOW_TAILNET_HTTP`
   - public http/https otherwise denied
4. `safe_join(base, path)`: reject `http://` paths that swap host; same origin (scheme, host, port); then `require` again.
5. `http::Client`: timeout 5s default, actor uses 2s. `ureq` **redirects(0)**. GET/POST/DELETE JSON. Attach `get_headers(json_body)`.
6. Auth seam (`docs/auth-remote.md`):
   - Bearer: `CHEF_VAULT_API_TOKEN` / `CHEFBAR_VAULT_TOKEN` / `CHEFBAR_VAULT_TOKEN_FILE` / legacy `~/ChefFactory/chefgroep-vault/docker/.env`
   - CF Access: `CF_ACCESS_CLIENT_ID` + `CF_ACCESS_CLIENT_SECRET` (or `CHEFBAR_CF_*`). Pair complete or omit both.
7. Never log Authorization, CF secrets, or raw tokens. Doctor: `sha256[:12]` fingerprints only. Never put tokens on `Snapshot`.
8. Tests: allow/deny matrix, `safe_join` cross-origin, env overlay, loopback vs example.com (already in `policy.rs`).

### New origin checklist

1. Optional field on `RawProfile` / `EndpointProfile` + env
2. Host allowed by policy (profile URL feeds `with_profile_hosts`)
3. Fetch only from actor
4. Tolerant parse in models
5. Doctor probe + `--show-config` host line

## Examples

**Example 1 — open redirect**

Input: `safe_join("https://vault-api.chefgroep.online/api", "https://evil.test/x")`

Output: error `absolute URL-pad mag nooit de host vervangen`. Do not “fix” by allowing absolute paths.

**Example 2 — half CF Access**

Input: only `CF_ACCESS_CLIENT_ID` set

Output: `cf_access()` returns None; do not send a lone header.

**Example 3 — public http**

Input: `http://example.com/api`

Output: `allows` false. Loopback and (optionally) `100.64/10` are the http exceptions.

## Performance Notes

- 2s endpoint timeout exists so one hung host cannot eat the 8s actor budget.
- `get_headers` is per-call (OIDC later). Do not cache bearer in a static that ignores rotation.
- `--show-config` prints hosts, never query strings with tokens.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Doctor: policy weigert URL | host not in profile / allowlist / `*.chefgroep.online` |
| 302 then empty body | redirects are 0 by design; fix the origin, do not enable redirects |
| Token in journal | strip notify/log paths; fingerprints only |
| Profile `name` not local but vault is 127.0.0.1 | env overlay or wrong JSON — `--show-config` |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- `ureq` `redirects(0)`. `safe_join` same-origin. CF headers both or neither.
- Per-field env overlay, not whole-file replace. No tokens on `Snapshot` or in logs.
- No tokio, reqwest, webview, or Electron.

## Next

Actor paths → `chefbar-actor`. Doctor UX → `chefbar-tray-ipc`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, or Electron; `ureq` stays `redirects(0)`.
