---
name: chefbar-gtk-panel
description: ChefBar GTK3 panel, palette overlay, drawer, CSS tokens, motion, density, and panel-state persistence. Use when editing src/panel/**, src/css.rs, src/motion.rs, src/panel_state.rs, visual-shot, Super+Space UX, sidebar, or Signaal v2 styling.
---

# ChefBar GTK panel

## Layout (4.0)

- Window: 860×880, undecorated, `keep_above`, not resizable, centered.
- `src/panel/mod.rs` — `Panel` lifecycle, show/toggle, poll-driven rebuild.
- `header.rs` — drag + search (single source of truth for the query).
- `sidebar.rs` — 240px groups, status dots, `NAV_IDS`.
- `zones.rs` — cards, stale rows, empty states.
- `drawer.rs` — detail slide, Esc closes drawer before window.
- `overlay.rs` — command palette; same `palette.rs` ranking as the header.

Do not merge these back into a 1.5k-line monolith.

## Data flow

`Panel` holds `Shared` + `Executor`. A glib timeout rebuilds from the snapshot. **No HTTP on this thread.**

Search: `palette::rank_actions_with` + `RankContext` (recency, running agents, active harness). Boosts stay inside a fuzzy tier.

Persist: `panel_state.rs` (harnas/group, query, density, drawer). Atomic JSON write, dirty-flag + ~2s timer. `CHEFBAR_PANEL_STATE` overrides the path for tests.

## CSS

`css.rs` interpolates `Tokens` into a GTK3 stylesheet string.

Forbidden in the emitted CSS (CI caught this once): `--custom-properties`, `gap`, `inset`. Also no gradients, glow, or web-only layout.

Fonts: General Sans (UI), IBM Plex Mono (data). Accent `#317CFF` / `#5C97FF`. Signature = 2px vertical `chefbar-signature`.

## Motion & density

`motion.rs`: `PANEL_MS` = 120. Fade only open/close/drawer. Poll rebuilds are instant.

Density: `comfortable` | `compact` — padding token, one widget tree.

## Visual QA

`scripts/visual-shot.sh <panel|palette|drawer|density-compact|density-comfortable> <dark|light> <out.png>`

Needs Xvfb + ImageMagick. Exit 2 = X-stack missing (soft skip). Accent assert is hard on panel-dark.

## Do not

- `set_resizable(true)` or free-size windows
- Animate every poll
- Open a second `gtk::Window` for detail (drawer only)
- Put `ureq` or `Executor` HTTP behind a button closure that captures widgets forever — call `executor.run(&spec)` with data
