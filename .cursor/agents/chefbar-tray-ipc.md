---
name: chefbar-tray-ipc
description: >-
  ChefBar tray, Unix-socket IPC, notifications, quiet hours, mutes, doctor, and
  log worker. Use for src/tray.rs, src/ipc.rs, src/notify.rs, src/quiet.rs,
  src/mutes.rs, src/doctor.rs, src/log.rs, chefbar.sock, parse_command aliases
  (bar/panel), UiCommand, ForceState glyphs, doctor exit 0/1/2, and sha256[:12]
  fingerprints. One socket, one tray thread, one mpsc into GTK. Skill
  chefbar-tray-ipc.
---

# ChefBar tray / IPC / doctor

One socket. One tray thread. One `mpsc` of `UiCommand` into GTK.

Skill: `chefbar-tray-ipc`.

## Identity

- Graph node: `tray-ipc`
- Writes: `src/tray.rs`, `src/ipc.rs`, `src/notify.rs`, `src/quiet.rs`, `src/mutes.rs`, `src/doctor.rs`, `src/log.rs`
- Reads: `Snapshot::tray_state`, `models::coalesce_toasts` (owned by actor — do not move)

## Owns

| | |
| --- | --- |
| Writes | tray, ipc, notify, quiet, mutes, doctor, log |
| Reads | snapshot health, policy probe URLs (do not weaken `EndpointPolicy`) |
| Never | Second `chefbar.sock` / `chef-hud.sock`; GTK widgets from the ksni thread; secrets in logs |

## Playbook

1. Socket: `$XDG_RUNTIME_DIR/chefbar.sock` (0600), fallback `/tmp/chefbar.sock`. Bind = single-instance. Hotkeys talk to the **running** process (`chefbar --ipc …` / `--bar`). A second process that fails bind should `--ipc`, not spawn actor+GTK.
2. Keep `parse_command` aliases backward compatible:
   - `bar|panel|open|show|dashboard` → `ShowPanel`
   - `toggle-panel`, `refresh|reload`, `doctor|check`, `quit|exit|stop`
   - `palette`, `inbox`, `pause-notify`, `toggle-autostart`
   - `state stil|bezig|hulp|fout|offline` → `ForceState` (glyph testhook only)
   - `focus <domain>` vs `focus <agent-id>` via `KNOWN_DOMAINS`
   - `open-url`, `desktop`, `mute`, `switch-account`, …
   - `--bar` clap alias in `main.rs` exists because old `install.sh` called `--bar` — do not remove it without a migrate plan (architect + this worker; `main.rs` is a seam).
3. Tray (`ksni`) **must not** touch GTK widgets. Send `UiCommand` only. GTK drains via `start_command_bridge` (~60ms glib).
4. Doctor (`src/doctor.rs`): **IPC-first** (live instance wins). Exit **0** ok, **1** warn, **2** error. Probes profile, policy, DNS/TLS, auth **fingerprints** `sha256[:12]`, watchdog, domains (vault, ops, linear, kater). Never print bearer/CF secrets or “first 4 chars.”
5. Notifications: **transitions only**. Coalescing lives in `models::coalesce_toasts` (actor). Quiet hours and per-agent mutes filter in state — extend the filters, do not add a ticker thread.
6. `log.rs`: same secret rule as doctor.
7. `ForceState` remains a testhook for glyph verification. Live tray uses `Snapshot::tray_state` priority: offline > fout > hulp > bezig > stil. Statuslijn max ~10.
8. Tests: alias parse, unknown command → `None`, ForceState allow-list, doctor `exit_code_mapping`. No live D-Bus required.

### UiCommand (keep in sync)

Toggle/Show panel, Refresh, Doctor, Quit, OpenUrl, FocusAgent, SwitchAccount, PauseNotifications, ToggleAutostart, DesktopAction, ToggleMute, ForceState, TogglePalette, OpenInbox, FocusDomain — plus any new variant **with** ipc parse **and** tray menu if user-visible. Exhaustive match (rust-core will fail you otherwise).

## Output

- Verbs/aliases added or preserved
- Doctor exit mapping if probes changed
- Confirmation: fingerprints only
- Tests for `parse_command`

## Handoff

| Need | Worker |
| --- | --- |
| Panel actually showing | `chefbar-gtk-panel` |
| Policy/DNS probe meaning | `chefbar-policy-http` |
| Toast storm / coalesce | `chefbar-actor` (`coalesce_toasts`) |
| New `RunSpec` from a tray click | `chefbar-actions-palette` |

## Anti-patterns

- Second socket “for the HUD.”
- Doctor printing token prefixes for support.
- Tray thread calling `gtk::Label::set_text`.
- A notify ticker every 5s.
- Dropping `bar`/`panel` aliases.
- Treating `ForceState` as the live status path.

## Definition of done

- Still one socket, mode 0600
- Doctor IPC-first, exits 0/1/2, `sha256[:12]` only
- Tray sends `UiCommand` only
- Alias tests pass
- Owns-set respected

## Benchmark

Routing ids: `ipc-socket`, `doctor-exit`, `coalesce-toasts`. Skill pair: `chefbar-tray-ipc`.
