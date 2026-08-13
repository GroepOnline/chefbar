---
name: chefbar-gtk-panel
description: ChefBar GTK3 panel worker for src/panel, src/css.rs, src/motion.rs, and src/panel_state.rs. Use for the 860x880 undecorated window, sidebar, zones, drawer.rs, palette overlay, Signaal v2 CSS, density, PANEL_MS fades, or when CSS gap, inset, or custom properties --tokens appear. No HTTP on the GTK thread. No second GtkWindow. Skill chefbar-gtk-panel.
---

# ChefBar GTK panel

GTK 3.24 via `gtk 0.18` (`v3_24`). No GTK4, relm4, webview, Electron.

Skill: `chefbar-gtk-panel`.

## Identity

- Graph node: `gtk-panel`
- Writes: `src/panel/**`, `src/css.rs`, `src/motion.rs`, `src/panel_state.rs`
- Reads: `palette.rs` (rank, do not fork), `actions.rs` (`executor.run`), `Shared`

## Owns

| | |
| --- | --- |
| Writes | `src/panel/` (mod, header, sidebar, zones, drawer, overlay), `src/css.rs`, `src/motion.rs`, `src/panel_state.rs` |
| Reads | snapshot rows, `palette::rank_actions_with`, `Executor` |
| Never | `ureq` / `http::Client` on this thread; second window; `gap:` / `inset:` / `--vars` in emitted CSS |

## Playbook

1. Geometry: **860×880**, `set_resizable(false)`, undecorated, `keep_above`, centered (`panel/mod.rs`). Do not make it resizable “for accessibility” without an explicit product decision.
2. Keep the module split — do not re-merge a 1.5k-line `panel.rs`:
   - `mod.rs` — lifecycle, show/toggle, poll rebuild
   - `header.rs` — drag + **the only** query string
   - `sidebar.rs` — 240px, `NAV_IDS`, dots
   - `zones.rs` — cards, stale, empty
   - `drawer.rs` — detail; Esc closes drawer **before** hiding the window
   - `overlay.rs` — palette; **same** `palette::rank_actions_with` as the header
3. Rebuild from `Shared` on a glib timeout. **No HTTP.** Run user actions with `executor.run(&spec, query)`.
4. CSS in `src/css.rs`: interpolate `Tokens` into GTK3 properties. Forbidden in the emitted string: custom properties (`--accent`), `gap`, `inset`, `grid-gap`, gradients, glow filters. `box-shadow` is already used for focus/overlay — do not add blur glow. Child `margin`/`padding` instead of `gap`.
5. Motion: `PANEL_MS` 120ms fade on **open/close/drawer only**. Poll rebuilds are instant (fades on poll = flicker).
6. Density: `comfortable` | `compact` via padding token (`panel_state.rs`). One widget tree, not two layouts.
7. Persist only UI prefs in `panel_state.rs` (group, query, density, drawer, recent_domains cap 20). Atomic write, dirty-flag. `CHEFBAR_PANEL_STATE` for tests. **No secrets.**
8. Surfaces: tray = glance, overlay = speed (`Super+Space`), panel = control. All three consume one snapshot.
9. Visual: `scripts/visual-shot.sh` is owned by `chefbar-qa`. Dark accent assert `#5C97FF`. Exit 2 = no Xvfb (warning-only in CI).
10. Fonts/tokens: General Sans UI, IBM Plex Mono data. Accent `#317CFF` light / `#5C97FF` dark. Signature = 2px `chefbar-signature`. Green = git/PR/consent; amber = wait-on-you.

## Output

- Widgets/modules touched
- CSS tokens changed (and confirmation no `gap`/`inset`/`--var`)
- Esc stack still drawer → overlay → hide
- Overlay and header still share ranking
- What QA should screenshot

## Handoff

| Need | Worker |
| --- | --- |
| Ranking disagrees | `chefbar-actions-palette` |
| Empty/stale data | `chefbar-actor` |
| Show/toggle via hotkey | `chefbar-tray-ipc` |
| visual-shot / CI PNG | `chefbar-qa` |
| Illegal CSS clippy-adjacent | still you (`css.rs`) |

## Anti-patterns

- `gap: 8px` on `.chefbar-zones` (already burned CI once; fixed on `main`).
- `GtkWindow` for Linear detail — `drawer.rs` 300px.
- Button click `ureq` to vault — `RunSpec` + actor refresh; label reads snapshot.
- Duplicating fuzzy rank in overlay.
- Animating every poll.
- Putting tokens in `panel_state`.

## Definition of done

- 860×880, not resizable
- GTK3-legal CSS (bench scans `src/css.rs` for `gap` / `inset` / `--var`)
- No HTTP from GTK
- Drawer Esc order intact
- Inline tests for `panel_state` / `motion` if you changed them

## Benchmark

Routing ids: `gtk-css-gap`, `panel-drawer`. Skill pair: `chefbar-gtk-panel`.

Do not own `scripts/visual-shot.sh` (that is `chefbar-qa`). Do not rank actions in GTK (that is `chefbar-actions-palette`).
