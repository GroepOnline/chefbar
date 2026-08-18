---
name: chefbar-architecture
description: ChefBar architecture skill for the one poll-actor, one Snapshot, one Unix socket shape. Use when adding a domain or HarnessKind, changing Snapshot or OpsSnapshot, altering poll rhythm in state.rs, splitting modules, or asking where a feature belongs. Use before any second loop, second tray, GTK HTTP, tokio, webview, or Electron is tempting. Covers ChefApp 4.0 rooms, last-good stale data, and file-disjoint worker maps.
---

# ChefBar architecture

Load this skill when the description matches, or when a graph-loop chain includes the architect. Do not assume it is already in context.

Invariants in `.cursor/rules/chefbar-invariants.mdc` are also description/chain-triggered (not always-on). When loaded, they win over generic Rust skills.

## Instructions

1. Name the user-visible change in one sentence (what Super+Space or the tray should do afterwards).
2. Classify it with the table in [references/modules.md](references/modules.md). One row = one owning worker.
3. If the change needs **new bytes from the network**, add a `Snapshot` field + tolerant builder in `src/models.rs` and a path on the **existing** actor in `src/state.rs`. Do not spawn a timer in GTK or tray.
4. If it is **something the user runs**, add `RunSpec` + `build_actions` + `Executor` (skill `chefbar-actions-palette`). GTK only calls `executor.run`.
5. If it is **something a hotkey/script sends**, add `UiCommand` + `ipc::parse_command` (skill `chefbar-tray-ipc`).
6. If it is **a new origin**, add a camelCase profile field (skill `chefbar-policy-http`) before the first GET.
7. Write the file map (worker → writes). Overlapping writes = serialize those workers.
8. Stop and re-plan if the design needs tokio, a second `chefbar.sock`, WebKit, or a widget that calls `ureq`.

### Mental model

```
endpoints.json + CHEFBAR_*     EndpointProfile
        |
        v
  poll-actor (state.rs)  -- http::Client + policy -->  vault / ops / linear / kater
        |   rhythm: vault 5s, ops 15s, extra 30s, kater 30s, linear 60s
        |   fan-out threads OK inside FETCH_BUDGET_MS (8s); they are not a second actor
        v
  Shared { Snapshot, OpsSnapshot, revision }     RwLock, last-good on failure
        |
        +-- tray.rs     ksni thread  --mpsc UiCommand-->  GTK
        +-- ipc.rs      unix socket  --same mpsc------->  GTK
        +-- panel/      GTK window   reads snapshot, no HTTP
        +-- palette.rs  ranks Action data from actions.rs
```

Fan-out `std::thread::spawn` inside `Poller::fetch_all` / `fanout` is **in-budget parallel GETs**, still one actor. A glib timeout that GETs is a second loop — reject it.

Executor `spawn_bg` is a **one-shot** mutation thread (focus agent, create task). Also allowed. A `loop { ureq; sleep }` in that thread is not.

### Snapshot is the product

`Snapshot` (`src/models.rs`) already has 4.0 sections: inbox, fleet_nodes, herdr_workspaces, vault_accounts, commander_tasks, crm_deals, containers, secrets_meta, linear_issues, kater_status, observability, last_poll_at. Prefer filling an existing field over adding a parallel struct.

Failed polls keep the previous `Snapshot` clone and stamp `last_poll_at` / stale. The UI must render last-good plus a reason, never an empty pane that looks like “no data ever”.

### Harnassen

Groups: Fleet, Commerce, Sync, Work, System.

Kinds: Fleet, Commerce, Sync, Eval, Inbox, Herdr, Vault, Crm, Share, Clipboard, Desktop, Tasks, Linear, Containers, Secrets, Kater, Health.

Sidebar filter = keyword prefix on `Action.keywords` (`HarnessKind::prefixes`). Ranking stays in `palette.rs` (contains 1000 > prefix 700 > gappy 500). Boosts never jump a tier.

## Examples

**Example 1 — new CRM field on an existing poll**

Input: “toon deal-owner op de CRM-kaart”

Output plan:

- `models.rs`: extend `CrmDeal`, tolerant parse
- `state.rs`: already fetches `/crm/deals` in `fetch_vault_extra` — no new timer
- `panel/zones.rs`: display (gtk worker)
- `actions.rs` only if there is a new command

**Example 2 — reject a second daemon**

Input: “kleine rust-service naast chefbar die linear polt”

Output: refuse. Linear already ticks in the actor (`LINEAR_POLL_MS`, `/issues?limit=20`). Add fields on `linear_issues`, do not add a process.

**Example 3 — hotkey**

Input: “Super+I opent inbox”

Output: IPC alias or `UiCommand::OpenInbox` (already parsed as `inbox`) + install.sh keybinding. Not a new binary.

## Performance Notes

- Actor budget is 8s wall, 2s per endpoint. New paths join `fetch_all` or `fetch_vault_extra`, they do not get their own 5s glib timer.
- UI rebuilds are cheap if they clone small rows, not the whole `raw` JSON, on every poll.
- `last_poll_at` is the freshness contract; do not add a second clock in the panel.
- Keep SKILL.md lean: deep file map in [references/modules.md](references/modules.md), poll paths in [references/poll-map.md](references/poll-map.md).

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Empty UI after a 401 | poll replaced snapshot with Default | clone previous snapshot; mark stale |
| Two trays / two sockets | new binary or second bind | one `acquire` on `chefbar.sock` |
| GTK freeze | HTTP on main thread | move GET to actor or `Executor::spawn_bg` |
| Action missing in a room | keywords lack harness prefix | `HarnessKind::prefixes` |
| Agent wants tokio | ecosysteem rust-testing/patterns | this skill + `chefbar-rust` win |

## Next

- Network shape → `chefbar-actor` + `chefbar-policy-http`
- Command → `chefbar-actions-palette`
- Pixels → `chefbar-gtk-panel`
- Cross-module → `chefbar-graph-loop`
