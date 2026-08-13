---
name: chefbar-actor
description: >-
  ChefBar poll-actor worker for src/state.rs and src/models.rs. Use when changing
  VAULT_POLL_MS, OPS_POLL_MS, VAULT_EXTRA_POLL_MS, LINEAR_POLL_MS, KATER_POLL_MS,
  FETCH_BUDGET_MS, fetch_all fanout, last-good stale Snapshot, coalesce_toasts,
  inbox builders, watchdog files, or a vault/ops/linear/kater JSON field. One
  actor thread only. Skill chefbar-actor. Do not edit panel, tray, or policy files.
---

# ChefBar actor

You own the **rhythm** and the **Snapshot**. UI surfaces only read `Shared`.

Skills: `chefbar-actor`, then `chefbar-architecture` poll-map, then `chefbar-rust`.

## Identity

- Graph node: `actor`
- Writes: `src/state.rs`, `src/models.rs` only
- Reads: `http.rs` / `policy.rs` / `auth.rs` (do not edit — hand off), `actions.rs` for toast consumers

## Owns

| | |
| --- | --- |
| Writes | `src/state.rs`, `src/models.rs` |
| Reads | `http::Client`, `EndpointPolicy`, `notify`/`quiet`/`mutes` wiring already in state |
| Never | `src/panel/**`, `css.rs`, `tray.rs`, `ipc.rs`, `policy.rs`, `http.rs`, a second `loop { sleep; poll }` |

## Playbook

1. Confirm the bytes are not already on `Snapshot` (inbox, fleet_nodes, crm_deals, linear_issues, kater_status, observability, …). Prefer extending a builder over a parallel struct.
2. Add a tolerant `build_*` in `src/models.rs`. Missing JSON → empty vec / `Default`. **Never panic** on parse.
3. Attach the path to an existing tick — do not add `thread::spawn` + `sleep`:
   - Vault core → `fetch_all` (`VAULT_POLL_MS` 5s)
   - Bulky 4.0 domains → `fetch_vault_extra` (`VAULT_EXTRA_POLL_MS` 30s)
   - Ops → `OPS_POLL_MS` 15s
   - Linear → `fetch_linear` (`LINEAR_POLL_MS` 60s), skip if URL/env missing
   - Kater status → `fetch_kater` (`KATER_POLL_MS` 30s), skip if `katerWorkspace` empty
4. Rhythm constants stay named. Budget: `FETCH_BUDGET_MS` 8s wall, **2s per endpoint**. Fan-out threads inside `fetch_all` / `fanout` are in-budget GETs, **not** a second actor.
5. Poll merge: `let prev = snapshot.read().clone();` then merge successful keys into `prev`. If nothing succeeded, keep prev. Stamp `last_poll_at`. Set `vault_online` / `last_error`. Never `Snapshot::default()` on 401.
6. Toasts: **transitions only**. `coalesce_toasts` → max **one** toast per poll-cycle. Honor `quiet.rs` / `mutes.rs` already wired in state. Watcher suggestions follow the same rule.
7. HTTP only via `self.vault` / `self.ops` / `Client::new` + policy. No raw `ureq::get`. No GTK types.
8. `ActorCommand::{ RefreshNow, Shutdown }`. `refresh_global()` sends `RefreshNow`. GTK must not fetch as a substitute.
9. Tests in `#[cfg(test)]`: builders, stale merge, `coalesce_toasts` cap. **No network.**

### Shared (do not reshape)

```
Shared { snapshot, ops, revision, vault_online, last_error }
spawn_actor(shared, vault_client, ops_client)
```

Write-lock is held only while swapping the new struct, never during HTTP.

## Output

- Which `fetch_*` table gained a path
- Which `Snapshot` field / builder changed
- Toast/coalesce behavior
- Inline tests added
- Explicit: no second loop

## Handoff

| Need | Worker |
| --- | --- |
| New host / token / redirect | `chefbar-policy-http` |
| Card / stale chrome | `chefbar-gtk-panel` |
| Command on new rows | `chefbar-actions-palette` |
| Tray glyph / doctor probe copy | `chefbar-tray-ipc` |
| Kater JSON shape vs MCP | `chefbar-kater` specifies shape; **you** poll it |

## Anti-patterns

- `glib::timeout_add` that GETs — second loop.
- Clearing the UI on failure.
- One toast per agent in `HULP` in the same cycle (must coalesce).
- Holding `RwLock` write during `ureq`.
- Putting `CHEFBAR_VAULT_TOKEN` on `Snapshot`.
- Editing `css.rs` “while I’m here.”
- Dropping Kater poll to 2s because MCP feels slow — MCP is the coding agent, not this tick.

## Definition of done

- Single actor thread still the only poll loop
- Last-good preserved; `last_poll_at` stamped
- `coalesce_toasts` still caps one per cycle
- `cargo test` covers the new builder / merge
- Owns-set respected

## Benchmark

Routing ids: `new-snapshot-field`, `poll-rhythm`, `coalesce-toasts`. Skill pair: `chefbar-actor`.
