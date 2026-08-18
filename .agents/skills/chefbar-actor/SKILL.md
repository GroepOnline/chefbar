---
name: chefbar-actor
description: ChefBar poll-actor skill for src/state.rs and src/models.rs. Use when changing VAULT_POLL_MS, OPS_POLL_MS, FETCH_BUDGET_MS, last-good stale Snapshot, fetch_all fanout paths, coalesce_toasts, inbox builders, watchdog files, or adding a vault/ops/linear/kater JSON field. Use when a feature needs new network bytes without a second loop. Covers Shared RwLock, ActorCommand RefreshNow, and tolerant serde_json parsing.
---

# ChefBar poll-actor

Owns the rhythm. UI surfaces only read `Shared`.

Load [../chefbar-architecture/references/poll-map.md](../chefbar-architecture/references/poll-map.md) before adding a path.

## Instructions

1. Confirm the bytes are not already on `Snapshot` (inbox, crm_deals, linear_issues, kater_status, …).
2. Add a tolerant `build_*` in `src/models.rs`. Missing JSON → empty vec / Default, **never panic**.
3. Attach the path to `fetch_all` (5s vault), `fetch_vault_extra` (30s), `fetch_linear`, or `fetch_kater`. Do not add `thread::spawn` + `sleep`.
4. In the poll method: `let prev = snapshot.read().clone();` then merge successful keys into `prev`. If nothing succeeded, keep prev and record error/stale.
5. Stamp `last_poll_at` per source. Tray/panel use that for “stale sinds”.
6. Watcher suggestions: only on **transition**. Run `coalesce_toasts` so one poll cycle ≤ one toast. Honor `quiet.rs` / `mutes.rs` (already wired in state).
7. HTTP only via `self.vault` / `self.ops` / a policy `Client::new`. No raw `ureq::get`.
8. Tests: builders + coalesce + stale merge in `#[cfg(test)]`. No network.

### Shared

```
Shared { snapshot, ops, revision, vault_online, last_error }
spawn_actor(shared, vault_client, ops_client)
ActorCommand::{ RefreshNow, Shutdown }
```

`refresh_global()` sends `RefreshNow`. GTK must not fetch as a substitute for refresh.

### Fan-out

`fetch_all` spawns one thread per path, `recv_timeout` until `FETCH_BUDGET_MS`. Slow endpoints become `None` this cycle; previous snapshot fields remain. That is the freshness contract.

Linear is skipped when no `CHEFBAR_LINEAR_API` / `LINEAR_API`. Kater is skipped when `katerWorkspace` is empty — still may stamp last_poll_at if the profile says the origin exists (see `poll_kater`).

## Examples

**Example 1 — new extra path**

Input: `/observability/errors`

Output: add to `fetch_vault_extra` table, `build_obs_*` on `Snapshot.observability` (field already exists as `ObsSummary` — extend that type). No new tick.

**Example 2 — toast storm**

Input: five agents enter `HULP` in one cycle

Output: `coalesce_toasts` → `"ChefGroep · 5 meldingen"` with worst severity. Do not notify per agent.

**Example 3 — 401 vault**

Input: vault returns 401

Output: keep last providers/agents; `vault_online=false`; doctor/tray show offline/fout. Do not `Snapshot::default()`.

## Performance Notes

- 2s per endpoint, 8s budget. Adding 15 extra vault paths on the 5s tick will starve; put bulky 4.0 domains on `fetch_vault_extra`.
- Do not hold the write-lock during HTTP.
- `raw: Value` is a debug/escape hatch; prefer typed fields for UI.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Section always empty | path not in fetch_* or builder too strict |
| UI flickers to empty | poll assigned Default instead of prev |
| Double toast | coalescing skipped or quiet filter bypassed |
| Linear never fills | env URL unset or policy blocked |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- One actor. New paths join `fetch_all` / `fetch_vault_extra` / `fetch_linear` / `fetch_kater`.
- Last-good on failure. Stamp `last_poll_at`. Never `Snapshot::default()` on 401.
- `coalesce_toasts`: one toast per cycle. No write-lock during HTTP.
- No tokio, reqwest, webview, or Electron.

## Next

Policy/auth failures → `chefbar-policy-http`. Cards → `chefbar-gtk-panel`. Actions on new rows → `chefbar-actions-palette`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, or Electron — the poll-actor is sync `ureq` on one thread.
