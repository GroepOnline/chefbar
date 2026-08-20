# Paste into chefbar-gtk-panel prompts

- Geometry 860×880, `set_resizable(false)`, undecorated. Drawer, not a second window.
- GTK3 CSS subset: no custom properties, no `gap`, no `inset`, no glow.
- No HTTP / ureq / tokio / webview / Electron on the GTK thread.
- Overlay and header share `palette::rank_actions_with`.
- Fades (`PANEL_MS` 280) on open/close only — not on poll rebuild. Drawer 160ms, overlay 100ms.
- visual-shot is owned by chefbar-qa.
