---
name: chefbar-tray-ipc
description: ChefBar tray and IPC skill for ksni tray.rs, Unix socket chefbar.sock, parse_command aliases, UiCommand, doctor.rs exit 0/1/2, notify.rs, quiet.rs, mutes.rs, and log.rs. Use when Super+Space, --ipc bar, ForceState glyphs, coalesce toasts, doctor fingerprints sha256, or single-instance socket bind changes. Use when a second socket or tray is proposed.
---

# ChefBar tray + IPC + doctor

One socket. One tray thread. One `mpsc` into GTK.

## Instructions

1. Socket: `$XDG_RUNTIME_DIR/chefbar.sock` (0600), fallback `/tmp/chefbar.sock`. Bind = single-instance. Hotkeys talk to the running process (`chefbar --ipc …` / `--bar`).
2. `parse_command` aliases stay backward compatible:
   - `bar|panel|open|show|dashboard` → `ShowPanel`
   - `toggle-panel`, `refresh|reload`, `doctor|check`, `quit|exit|stop`
   - `palette`, `inbox`, `pause-notify`, `toggle-autostart`
   - `state stil|bezig|hulp|fout|offline` → `ForceState` (glyph testhook)
   - `focus <domain>` vs `focus <agent-id>` via `KNOWN_DOMAINS`
   - `open-url`, `desktop`, `mute`, `switch-account`, …
3. Tray (`ksni`) must not touch GTK widgets. Send `UiCommand` only. GTK drains via `start_command_bridge` (~60ms glib).
4. Doctor (`src/doctor.rs`): **IPC-first** (live instance wins). Exit **0** ok, **1** warn, **2** error. Probes profile, policy, DNS/TLS, auth **fingerprints** `sha256[:12]`, watchdog, domains (vault, ops, linear, kater).
5. Notifications: transitions only. `coalesce_toasts` in models; quiet hours and per-agent mutes already filter in state. Do not add a ticker.
6. `log.rs`: no secrets. Same rule as doctor.
7. Tests: alias parse, unknown command → None, ForceState allow-list, doctor exit mapping (`exit_code_mapping`).

### UiCommand (keep in sync)

Toggle/Show panel, Refresh, Doctor, Quit, OpenUrl, FocusAgent, SwitchAccount, PauseNotifications, ToggleAutostart, DesktopAction, ToggleMute, ForceState, TogglePalette, OpenInbox, FocusDomain — plus any new variant **with** ipc parse + tray menu if user-visible.

## Examples

**Example 1 — Super+Space does nothing**

Input: clap error on `--bar`

Output: `--bar` alias for `--ipc bar` already exists in `main.rs` because old install.sh called `--bar`. Do not remove it.

**Example 2 — second socket for HUD**

Input: `chef-hud.sock`

Output: refuse. One socket; add a `parse_command` verb instead.

**Example 3 — doctor prints token prefix**

Input: “first 4 chars of bearer for support”

Output: refuse. Fingerprint only.

## Performance Notes

- Command bridge is a short glib poll; do not sleep on GTK.
- Doctor should prefer IPC so it does not start a second actor.
- Tray statuslijn max ~10; `Snapshot::tray_state` priority offline > fout > hulp > bezig > stil.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Hotkey silent | instance down / socket missing / unknown verb |
| Two chefbar processes | bind failed, second process should `--ipc` not spawn actor+gtk |
| Glyph stuck | ForceState testhook; live path uses `tray_state()` |
| Toast storm | coalesce + quiet/mutes |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- One socket `$XDG_RUNTIME_DIR/chefbar.sock` (0600). Aliases `bar`/`panel`/`--bar` stay.
- Tray thread sends `UiCommand` only. GTK drains `start_command_bridge`.
- Doctor is IPC-first; exit **0 / 1 / 2**; fingerprints `sha256[:12]`; no secrets.
- Notifications are transitions. Coalesce lives in the actor (`coalesce_toasts`). Quiet/mutes are filters, not a ticker.
- `ForceState` is a glyph testhook. Live status is `Snapshot::tray_state`.
- No tokio, reqwest, webview, Electron, or a second socket.

## Next

Show panel pixels → `chefbar-gtk-panel`. Policy probes → `chefbar-policy-http`. Snapshot health → `chefbar-actor`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, Electron, or a second `chefbar.sock`.
