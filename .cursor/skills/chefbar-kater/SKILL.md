---
name: chefbar-kater
description: ChefBar ↔ Kater coupling — MCP chains, adapters, profiles, PR health chain, and the in-app Kater poll. Use when touching Kater status in ChefBar, sessions attach, ops CLI, or when orchestrating workers via Kater chains/adapters. Also use before a PR when Kater profile code/ops is available.
---

# ChefBar × Kater

ChefBar **talks to** Kater; it does not replace the gateway. Poll Kater from the existing actor only.

## In-app poll

`src/state.rs`: `KATER_POLL_MS = 30_000`. `fetch_kater` uses `katerWorkspace` from the profile, strips a trailing `/agents/`, then tries `/api/status` and `/status`. Result → `build_kater_status` on `Snapshot`. Missing profile → skip, no extra thread.

Sessions: `src/sessions.rs` prefers `katerSessionId` for attach when present.

## MCP (this Cursor session)

Server id: `Kater` (capital K). Tools:

| Tool | Use |
| --- | --- |
| `kater_profiles` | List profiles (`cloud`, `code`, `content`, `core`, `docs`, `email`, `image`, `ops`, `reasoning`, `research`, `utils`, `web`) |
| `kater_doctor` | Gateway health for a profile |
| `kater_chains` | Named step lists (profile-scoped) |
| `kater_adapters` | MCP adapters + missing env + risk |
| `kater_config` | Rendered profile config |
| `kater_pr_list` / `kater_pr_status` / `kater_pr_gate` / `kater_pr_policy` / `kater_pr_audit` | PR gate (read) |
| `kater_pr_merge` | Write — only if the user explicitly asks to merge |

Always `GetMcpTools` for schema before `CallMcpTool`.

## Live chains (discovered)

Profile `code` and `ops` currently expose:

**`pr_health`** — `github_pr_status` → `linear_issue_status` → `sentry_issue_search`

Profiles `core`, `cloud`, `reasoning` may return `chains: []`. Empty means: use the **local** ChefBar graph in `chefbar-graph-loop`, do not invent remote steps.

## Adapters (discovered)

- **code:** github (high), sqlite, filesystem, context7, deepwiki
- **ops:** github, linear, sentry, cloudflare; upstash/postgres/notion often unconfigured (`missing_env`)

Skip adapters with `configured: false`. Do not print secret values from `missing_env` names beyond the variable name.

## Coupling rules

1. App-side Kater traffic stays on the actor + policy client.
2. Agent-side Kater traffic stays on MCP tools — do not scrape `kater.chefgroep.online` in a second scrape-loop.
3. Before opening/updating a PR, the orchestrator may run chain `pr_health` on profile `code` or `ops`.
4. `kater_pr_merge` is user-gated. Default is report, not merge.
5. High-risk adapters (github, cloudflare) are for the task at hand, not inventory dumps into chat.

## When implementing ChefBar features for Kater

- Status cards: `models::build_kater_status` + harness kind `Kater`
- Actions: `OpenUrl` to `katerWorkspace`, no embedded gateway
- Doctor: domain probe already in `doctor.rs`

## Next

Multi-worker implementation → `chefbar-graph-loop` (local graph + this MCP chain as an edge).
