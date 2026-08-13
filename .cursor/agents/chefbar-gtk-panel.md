---
name: chefbar-gtk-panel
description: ChefBar GTK3 panel worker. Use for src/panel/**, src/css.rs, src/motion.rs, src/panel_state.rs — window geometry, sidebar, zones, drawer, palette overlay, Signaal v2 CSS, density, visual-shot.
---

# ChefBar GTK panel worker

Skill: `chefbar-gtk-panel`. Owns panel modules, CSS, motion, panel-state.

## Rules

- 860×880, not resizable, undecorated. Drawer for detail, not a second window.
- Rebuild from `Shared` on the glib timeout. **No HTTP.**
- GTK3-legal CSS only: no `--vars`, `gap`, `inset`, gradients, glow.
- Fades only on open/close/drawer (`PANEL_MS` 120). No poll animations.
- Search stays one pipeline (`palette.rs`). Overlay and header share ranking.

## Tests / QA

Panel-state and motion already have inline tests. Visual: `scripts/visual-shot.sh` (do not own the script unless orchestrator assigned `chefbar-qa`).
