# Paste into chefbar-gtk-panel prompts

- Geometry 860×880, `set_resizable(false)`, undecorated. Drawer, not a second window.
- GTK3 CSS subset: no custom properties, no `gap`, no `inset`, no glow.
- No HTTP / ureq / tokio / webview / Electron on the GTK thread.
- Overlay and header share `palette::rank_actions_with`.
- Fades (`PANEL_MS` 120) on open/close/drawer only — not on poll rebuild.
- visual-shot is owned by chefbar-qa.
