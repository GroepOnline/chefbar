---
name: chefbar-kater
description: ChefBar Kater skill for MCP kater_chains, kater_adapters, kater_doctor, kater_profiles, pr_health on profile code and ops, and in-app KATER_POLL_MS poll in state.rs. Use when coupling ChefBar to Kater, debugging katerWorkspace, session katerSessionId attach in sessions.rs, ops_cli.rs, or before a PR health check across GitHub Linear Sentry. Use when empty chains on core/cloud should fall back to chefbar-graph-loop.
---

# ChefBar × Kater

ChefBar talks to Kater. It does not replace the gateway. App poll stays on the one actor.

## Instructions

### In-app

1. `KATER_POLL_MS = 30_000` in `src/state.rs`. `fetch_kater` uses profile `katerWorkspace`, strips trailing `/agents/`, tries `/api/status` then `/status`.
2. Result → `build_kater_status` on `Snapshot.kater_status`. Missing profile → skip (no extra thread).
3. Sessions: `src/sessions.rs` prefers `katerSessionId` for attach when present.
4. Actions: `OpenUrl` to the workspace. No embedded gateway, no second poll.

### MCP (Cursor)

Server id **`Kater`** (capital K). Call `GetMcpTools` before `CallMcpTool`.

| Tool | Use |
| --- | --- |
| `kater_profiles` | `cloud`, `code`, `content`, `core`, `docs`, `email`, `image`, `ops`, `reasoning`, `research`, `utils`, `web` |
| `kater_doctor` | gateway health |
| `kater_chains` | named steps, profile-scoped |
| `kater_adapters` | configured / missing_env / risk |
| `kater_config` | rendered config |
| `kater_pr_*` | list/status/gate/policy/audit (read) |
| `kater_pr_merge` | write — only if the user asked, `kater_pr_gate` is PASS, and expected head SHA matches |

### Live chain

`pr_health` on **code** and **ops**: `github_pr_status` → `linear_issue_status` → `sentry_issue_search`.

`core` / `cloud` / `reasoning` often return `chains: []`. That is normal. Use local `chefbar-graph-loop`, do not invent tool names.

### Adapters

- code: github (high), sqlite, filesystem, context7, deepwiki
- ops: github, linear, sentry, cloudflare; upstash/postgres/notion often `configured: false`

Skip unconfigured adapters. Log **variable names** from `missing_env`, never values.

## Examples

**Example 1 — empty chains**

Input: `kater_chains` profile `core` → `[]`

Output: say empty; run local graph (`feature` / `review`). Do not fake `github_pr_status` on core.

**Example 2 — merge**

Input: “merge the PR”

Output: only with explicit user intent + `kater_pr_gate` PASS + expected SHA. Default is report.

**Example 3 — status card**

Input: show gateway online in Health/Kater room

Output: `kater_status` from actor poll + harness kind `Kater`. MCP is for the coding agent, not for the GTK main loop.

## Performance Notes

- 30s tick is enough for a status card. Do not drop it to 2s.
- MCP chains are sequential tools; do not also scrape `kater.chefgroep.online` in a browser loop.
- High-risk adapters (github, cloudflare) are for the current task, not inventory dumps.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Snapshot kater empty | profile field unset or policy blocked host |
| MCP server `kater` 404 | use server `Kater` |
| pr_health missing | wrong profile — use `code` or `ops` |
| Agent scrape-loop | stop; actor + MCP only |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- MCP server id `Kater`. `pr_health` on `code`/`ops`. Empty other profiles → local graph.
- App poll stays in `state.rs` (actor). This skill writes `sessions.rs` / `ops_cli.rs`.
- No `kater_pr_merge` unless asked + `kater_pr_gate` PASS + expected SHA. No tokio, reqwest, webview, Electron, or scrape-loop.

## Next

Orchestrate workers after `pr_health` → `chefbar-graph-loop`. Poll implementation → `chefbar-actor`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, Electron, or invented MCP tool names when `chains: []`.
