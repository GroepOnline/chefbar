---
name: chefbar-actor
description: ChefBar poll-actor and snapshot worker. Use when changing src/state.rs or src/models.rs — poll rhythm, Shared/RwLock, last-good stale data, vault/ops/linear/kater fetches, coalesced toasts, inbox builders.
---

# ChefBar actor

Owns **writes** to `src/state.rs` and `src/models.rs` only.

## Rules

- Keep a **single** actor thread. New sources get a tick in the existing loop (`VAULT_POLL_MS`, `OPS_POLL_MS`, `VAULT_EXTRA_POLL_MS`, `LINEAR_POLL_MS`, `KATER_POLL_MS`) and a budget (`FETCH_BUDGET_MS` 8s, per-endpoint 2s).
- Failed fetch → keep last-good, set `last_poll_at` / stale — never panic, never clear the UI to empty without a reason.
- Parsing is tolerant (`serde_json::Value` → structs with defaults).
- HTTP only through `http::Client` + `EndpointPolicy`. No extra `ureq::Agent` on the side.
- Toasts: `coalesce_toasts`, quiet/mutes already wired — do not fire on steady state.

## Tests

Inline `#[cfg(test)]` in `models.rs` / `state.rs` for builders, stale behavior, and coalescing. No network.

Read `chefbar-architecture` and `chefbar-rust`. Do not edit panel/tray/policy files — ask the orchestrator.
