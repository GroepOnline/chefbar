# Paste into chefbar-tray-ipc prompts

- One socket `$XDG_RUNTIME_DIR/chefbar.sock`, fallback `/tmp/chefbar.sock`. After bind, set and verify mode `0600`; chmod failure is a failed bind. No second socket.
- Tray ksni thread sends `UiCommand` only — no GTK widgets, no tokio.
- Doctor IPC-first; exit 0/1/2; fingerprints `sha256[:12]`; no secrets.
- Keep `parse_command` aliases (`bar`/`panel`/`--bar`).
- Notifications: transitions + coalesce (actor owns `coalesce_toasts`). No ticker.
- No webview, Electron, reqwest.
