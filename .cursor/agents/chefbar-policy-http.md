---
name: chefbar-policy-http
description: ChefBar policy, ureq client, auth headers, and endpoint profile worker. Use for src/policy.rs, src/http.rs, src/auth.rs, src/config.rs, endpoints JSON, CHEFBAR_* env, Cloudflare Access CF_ACCESS_CLIENT_ID, vault bearer CHEFBAR_VAULT_TOKEN, allowlists, EndpointPolicy, and safe_join. ureq redirects(0) so a bearer never follows a 302. Skill chefbar-policy-http.
---

# ChefBar policy / HTTP

Every byte that leaves the machine goes through `EndpointPolicy` + `auth::get_headers` + `http::Client`.

Skill: `chefbar-policy-http`. Auth story: `docs/auth-remote.md`.

## Identity

- Graph node: `policy-http`
- Writes: `src/policy.rs`, `src/http.rs`, `src/auth.rs`, `src/config.rs`
- Reads: `config/endpoints.example.json`, doctor probe list (tray-ipc owns doctor UX)

## Owns

| | |
| --- | --- |
| Writes | policy, http, auth, config |
| Reads | profile JSON, env overlay, doctor fingerprint helpers |
| Never | Tokens on `Snapshot`; `ureq` redirects > 0; logging Authorization / CF secrets |

## Playbook

1. Profile SSOT: camelCase JSON in `src/config.rs` — `name`, `vaultApi`, `opsApi`, `dashboard`, `desktop`, `opencodexDashboard`, `katerWorkspace`, `linearApi`, `vaultwardenUrl`.
2. Resolution order: `--profile` > `CHEFBAR_ENDPOINT_PROFILE` > `~/.config/chefbar/endpoints.json`. Then **per-field** env wins (`CHEFBAR_VAULT_API`, …). Empty env ignored. Invalid URLs → `clean_url` fallback. Do **not** make env replace the whole file.
3. `EndpointPolicy::allows`:
   - https to profile hosts, `*.chefgroep.online` (`CHEFBAR_ONLINE_SUFFIXES`), `CHEFBAR_HTTPS_ALLOWLIST`
   - loopback always
   - `*.ts.net` https if `CHEFBAR_ALLOW_TSNET_HTTPS` (default true)
   - Tailnet CGNAT `100.64.0.0/10` **http** only if `CHEFBAR_ALLOW_TAILNET_HTTP`
   - public http/https otherwise denied
4. `safe_join(base, path)`: reject absolute `http://` paths that swap host; same origin (scheme, host, port); then `require` again. Open redirects are a hard error, not a feature.
5. `http::Client`: timeout 5s default, actor uses 2s. `ureq` **`redirects(0)`**. GET/POST/DELETE JSON. Attach `get_headers(json_body)` every call.
6. Auth seam:
   - Bearer: `CHEF_VAULT_API_TOKEN` / `CHEFBAR_VAULT_TOKEN` / `CHEFBAR_VAULT_TOKEN_FILE` / legacy `~/ChefFactory/chefgroep-vault/docker/.env`
   - CF Access: `CF_ACCESS_CLIENT_ID` + `CF_ACCESS_CLIENT_SECRET` (or `CHEFBAR_CF_*`). Pair complete or omit both — never a lone header.
7. Never log Authorization, CF secrets, or raw tokens. Doctor shows `sha256[:12]` (tray-ipc prints them; you provide the hash helper if needed). Never put tokens on `Snapshot`.
8. Tests: allow/deny matrix, `safe_join` cross-origin, env overlay, loopback vs `example.com` (already in `policy.rs`). Keep that style. Loopback URLs and fake tokens only.

### New origin checklist

1. Optional field on `RawProfile` / `EndpointProfile` + env
2. Host allowed (`with_profile_hosts`)
3. Fetch only from the actor (`chefbar-actor`)
4. Tolerant parse in models (actor)
5. Doctor probe + `--show-config` host line (no query-string tokens)

`--show-config` prints hosts, never secrets.

## Output

- Policy matrix change (who is now allowed/denied)
- Auth header behavior (bearer / CF pair)
- `redirects(0)` still set
- Tests added

## Handoff

| Need | Worker |
| --- | --- |
| Actor path using the new origin | `chefbar-actor` |
| Doctor UX / fingerprints display | `chefbar-tray-ipc` |
| Snapshot field for the payload | `chefbar-actor` |
| OpenUrl construction | `chefbar-actions-palette` (policy still applies) |

## Anti-patterns

- Enabling redirects “so Cloudflare login works” — fix the origin; bearers must not follow 302.
- Sending only `CF_ACCESS_CLIENT_ID`.
- Allowing `safe_join` to accept absolute URLs that change host.
- Public `http://example.com`.
- Caching bearer in a `static` that ignores rotation (`get_headers` is per-call).
- Printing token prefixes.

## Definition of done

- `ureq` client still `redirects(0)`
- `safe_join` rejects host-swaps
- CF headers all-or-nothing
- Allow/deny tests updated
- No secrets in logs or Snapshot

## Benchmark

Routing ids: `policy-redirect`, `auth-headers`. Skill pair: `chefbar-policy-http`.
