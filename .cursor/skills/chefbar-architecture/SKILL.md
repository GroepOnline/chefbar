---
name: chefbar-architecture
description: ChefBar architecture skill — one actor, one snapshot, one socket, harness rooms, ChefApp 4.0 domains. Use whenever adding a domain, changing Snapshot/poll rhythm, splitting modules, or asking where a feature belongs. Also use before any second loop, tray, or HTTP from GTK is tempting.
---

# ChefBar architecture

Load this skill before changing data flow, adding a harness, or touching more than one of `{state, models, actions, panel, tray, ipc}`.

## Mental model

```
endpoints.json + CHEFBAR_*  →  EndpointProfile
        │
        ▼
  poll-actor (state.rs)  --ureq+policy-->  vault / ops / linear / kater
        │
        ▼
  Shared { Snapshot, OpsSnapshot, revision }   # RwLock, last-good on failure
        │
        ├── tray.rs     ksni thread  --mpsc UiCommand-->  GTK
        ├── ipc.rs      unix socket  --same mpsc------->  GTK
        ├── panel/      GTK window   reads snapshot, no HTTP
        └── palette.rs  ranks Action data from actions.rs
```

Non-negotiables live in `.cursor/rules/chefbar-invariants.mdc`. Do not violate them to “make it simpler.”

## Where new work goes

| Need | Place |
| --- | --- |
| New JSON field from vault/ops | tolerant parse + struct on `Snapshot` in `src/models.rs`, poll path in `src/state.rs` |
| New sidebar room | `HarnessKind` / `HarnessGroup` in `src/harness.rs` + keywords on actions |
| New user command | `RunSpec` variant + `build_actions` + `Executor` in `src/actions.rs`; GTK only calls `executor.run` |
| New hotkey/script | `UiCommand` + `ipc::parse_command` aliases |
| New network origin | profile field in `src/config.rs` (camelCase JSON) + policy allow |
| New visual token | `Tokens` in `src/css.rs`, then GTK3-legal properties |
| Persist UI | `src/panel_state.rs` only (harnas, query, density, drawer) |

## Poll rhythm (do not invent extra timers)

| Source | Interval | File |
| --- | --- | --- |
| vault | 5s | `VAULT_POLL_MS` |
| ops | 15s | `OPS_POLL_MS` |
| vault-extra | 30s | `VAULT_EXTRA_POLL_MS` |
| kater | 30s | `KATER_POLL_MS` |
| linear | 60s | `LINEAR_POLL_MS` |
| per-fetch budget | 8s | `FETCH_BUDGET_MS` |
| per-endpoint timeout | 2s | `PER_ENDPOINT_TIMEOUT_MS` |

UI refresh is a glib timeout that *reads* `Shared`, it does not fetch.

## Harnassen (4.0)

Groups: Fleet, Commerce, Sync, Work, System.

Kinds include Fleet, Commerce, Sync, Eval, Inbox, Herdr, Vault, Crm, Share, Clipboard, Desktop, Tasks, Linear, Containers, Secrets, Kater, Health.

Filtering is keyword-prefix match, not a second search mode. Ranking stays in `palette.rs`: contains 1000 > prefix 700 > gappy 500; boosts never jump a tier.

## Surfaces

Tray = glance. Palette overlay = speed (`Super+Space`). Panel = control (860×880 + drawer). All three consume the same snapshot.

## Anti-patterns

- `thread::spawn` that polls HTTP outside `state.rs`
- `gtk::timeout` that calls `ureq` / `Client`
- WebKit/webview “just for this dashboard”
- Second socket next to `chefbar.sock`
- Storing secrets or tokens on `Snapshot`
- Duplicating `RunSpec` logic inside panel widgets

## Docs

- `README.md` — product + CLI
- `docs/plan-full-chefapp.md` — 8 domains, UX, lane contracts
- `docs/roadmap.md` — 3.1 shipped / deferred
- `docs/agent-harness.md` — workers, chains, graph loops

## Next

- Implementation in one module → matching domain skill + worker
- Cross-module → `chefbar-graph-loop`
- Kater status/chains → `chefbar-kater`
