# Poll map

Source of truth: `src/state.rs`. Fan-out uses `http::Client` (ureq, redirects 0, 2s timeout) and stops at `FETCH_BUDGET_MS` (8000).

## Rhythm

| Tick | Const | Default | Method |
| --- | --- | --- | --- |
| vault | `VAULT_POLL_MS` | 5000 | `poll_vault` / `fetch_all` |
| ops | `OPS_POLL_MS` | 15000 | `poll_ops` |
| vault-extra | `VAULT_EXTRA_POLL_MS` | 30000 | `poll_vault_extra` / `fetch_vault_extra` |
| linear | `LINEAR_POLL_MS` | 60000 | `poll_linear` / `fetch_linear` |
| kater | `KATER_POLL_MS` | 30000 | `poll_kater` / `fetch_kater` |
| watchdog files | 5s | local | `poll_watchdog_into_shared` |

`RefreshNow` (UI/IPC) re-runs vault + vault-extra immediately.

## Vault `fetch_all` paths

| Key | Path |
| --- | --- |
| status | `/status` |
| accounts/overview | `/accounts/overview` |
| agents | `/agents` |
| agent_events | `/agents/events?limit=8` |
| fleet | `/fleet` |
| tasks | `/commander/tasks?limit=12` |
| clipboard | `/clipboard` |
| desktop/status | `/desktop/status` |
| share-sync/status | `/share-sync/status` |
| sessions | `/sessions` |

## Vault extra

| Key | Path |
| --- | --- |
| vault_accounts | `/accounts` |
| crm_deals | `/crm/deals` |
| secrets_meta | `/secrets/meta` |
| containers | `/containers` |
| inbox | `/inbox` |
| fleet_nodes | `/fleet/nodes` |
| herdr_workspaces | `/herdr/workspaces` |
| commander_tasks | `/commander/tasks?limit=20` |
| clipboard_extra | `/clipboard` |
| observability | `/observability/summary` |

## Other

- Linear: `CHEFBAR_LINEAR_API` / `LINEAR_API` / `LINEAR_API_URL` + `/issues?limit=20` (skip if unset)
- Kater: profile `katerWorkspace`, strip trailing `/agents/`, try `/api/status` then `/status`
- Watchdog: `CHEFBAR_WATCHDOG_STATE` or `~/.local/share/chefgroep-os/watchdog-state.json`

New network data = new key on an existing tick, plus a `build_*` in `models.rs`. Not a new thread with `sleep`.
