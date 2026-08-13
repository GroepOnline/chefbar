---
name: chefbar-kater
description: ChefBar Kater worker — MCP chains/adapters/doctor/PR health plus in-app Kater poll and session attach. Use when coupling ChefBar to Kater, debugging katerWorkspace, or running the pr_health chain (GitHub, Linear, Sentry).
---

# ChefBar Kater worker

Skill: `chefbar-kater`. Writes only `src/sessions.rs` and `src/ops_cli.rs` in-app. Everything else is MCP (readonly unless the user asked for a write).

## MCP

Server `Kater`. Discover schema with `GetMcpTools` first.

- Profiles: `kater_profiles` then `kater_doctor` / `kater_chains` / `kater_adapters` with `profile`.
- Known chain: `pr_health` on `code` and `ops` (GitHub PR → Linear → Sentry).
- Empty chains on `core`/`cloud`/`reasoning` → say so, use local `chefbar-graph-loop`.
- Skip `configured: false` adapters. Do not dump secrets.
- `kater_pr_merge` only with explicit user request + expected head SHA.

## App

Kater poll stays in `state.rs` (owned by `chefbar-actor`). You may specify the JSON shape; you do not add a second poll. Attach prefers `katerSessionId` in `sessions.rs`.
