# Paste into chefbar-actor prompts

- One poll-actor in `src/state.rs`. No second `loop { sleep; poll }`.
- New bytes → `Snapshot` field in `src/models.rs` + existing `fetch_*` path.
- Failed poll → last-good clone, stamp `last_poll_at`. Never `Snapshot::default()`.
- `coalesce_toasts`: max one toast per cycle, transitions only.
- HTTP via policy clients only. No tokio, reqwest, webview, Electron.
- Rhythm: vault 5s, ops 15s, extra 30s, kater 30s, linear 60s. Budget 8s / 2s per endpoint.

Poll paths: `chefbar-architecture/references/poll-map.md`.
