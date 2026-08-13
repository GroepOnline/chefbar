---
name: chefbar-tray-ipc
description: ChefBar tray, Unix-socket IPC, notifications, quiet hours, mutes, doctor, and log worker. Use for src/tray.rs, src/ipc.rs, src/notify.rs, src/quiet.rs, src/mutes.rs, src/doctor.rs, src/log.rs.
---

# ChefBar tray / IPC worker

Owns tray, ipc, notify, quiet, mutes, doctor, log.

## Rules

- One socket: `$XDG_RUNTIME_DIR/chefbar.sock` (0600). Aliases in `parse_command` stay backward compatible (`bar`/`panel`/`--bar`).
- Tray thread sends `UiCommand` over `mpsc`; GTK drains via `start_command_bridge`. Do not touch GTK widgets from the tray thread.
- Doctor is IPC-first; exit 0/1/2; fingerprints only (`sha256[:12]`).
- Notifications: transitions + coalesce. Quiet/mutes already exist — extend them, do not add a ticker.
- `ForceState` remains a testhook for glyph verification.

## Tests

`ipc` parse aliases, doctor domain status, mute/quiet filters. No live bus required for unit tests.
