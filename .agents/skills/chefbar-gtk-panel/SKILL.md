---
name: chefbar-gtk-panel
description: ChefBar GTK3 panel skill for src/panel, css.rs, motion.rs, and panel_state.rs. Use when changing the 860x880 window, sidebar, zones, drawer.rs, palette overlay, Signaal v2 GTK3 CSS, density comfortable/compact, fades PANEL_MS, or visual-shot screenshots. Use when CSS gap, inset, or custom properties --tokens appear, or when GTK might call ureq. Covers undecorated keep_above, no second window, search as single source of truth.
---

# ChefBar GTK panel

GTK 3.24 (`gtk 0.18`). No GTK4, relm4, webview, Electron.

## Instructions

1. Keep geometry: 860×880, `set_resizable(false)`, undecorated, `keep_above`, centered (`panel/mod.rs`).
2. Put widgets in the existing modules — do not re-merge a 1.5k-line `panel.rs`.
   - `mod.rs` lifecycle, show/toggle, poll rebuild
   - `header.rs` drag + search (the only query string)
   - `sidebar.rs` 240px, `NAV_IDS`, dots
   - `zones.rs` cards, stale, empty
   - `drawer.rs` detail; Esc closes drawer before window
   - `overlay.rs` palette; **same** `palette::rank_actions_with`
3. Rebuild from `Shared` on a glib timeout. **No `http::Client` on this thread.** Run actions with `executor.run(&spec, query)`.
4. CSS: interpolate `Tokens` in `src/css.rs` into GTK3 properties. Forbidden in the emitted string: `--custom-properties`, `gap`, `inset`, `grid-gap`, gradients, glow filters. `box-shadow` is used for focus ring / overlay only (already in file) — do not add blur glow.
5. Motion: `PANEL_MS` 280ms fade on open/close only. Drawer 160ms, overlay 100ms. Poll rebuilds are instant.
6. Density: `comfortable` | `compact` via padding token (`panel_state.rs`). One widget tree.
7. Persist only UI prefs in `panel_state.rs` (group, query, density, drawer, recent_domains cap 20). Atomic write, dirty-flag. `CHEFBAR_PANEL_STATE` for tests. No secrets.
8. Visual: `scripts/visual-shot.sh` (owned by qa). Dark accent assert `#5C97FF`. Exit 2 = no Xvfb.

### Surfaces

Tray = glance. Overlay = speed (`Super+Space`). Panel = control. All three consume one snapshot.

Search ranking is not duplicated in GTK. If overlay and header disagree, the overlay is wrong.

## Examples

**Example 1 — illegal CSS**

Input: `gap: 8px` on `.chefbar-zones`

Output: replace with child `margin` / `padding` on cards. CI already failed once on custom properties + gap + inset (`fix(css)` on main).

**Example 2 — detail view**

Input: “volledige Linear issue in een nieuw GtkWindow”

Output: `drawer.rs` 300px, not a second window. Esc stack: drawer → overlay → hide panel.

**Example 3 — live prices in a label**

Input: button click `ureq` to vault

Output: `RunSpec` + actor refresh. Label reads snapshot on next rebuild.

## Performance Notes

- Poll rebuild should update rows, not destroy the whole tree if avoidable; current code refills — do not add per-widget HTTP to “optimize”.
- Fades during poll cause flicker; keep them off the refresh path.
- Fonts: General Sans UI, IBM Plex Mono data. Accent `#317CFF` light / `#5C97FF` dark. Signature = 2px `chefbar-signature`.
- Green reserved for git/PR/consent; amber = wait-on-you.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Window resizable after patch | restore `set_resizable(false)` + size_request 860×880 |
| Search in overlay ≠ header | both must call `rank_actions_with` |
| CSS ignored | GTK3 subset; unknown properties are dropped silently |
| visual-shot exit 2 | X-stack missing — warning-only in CI, not a product bug |
| State lost on restart | persist via `panel_state`, not in-memory only |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- 860×880, not resizable, one window. Detail in `drawer.rs`.
- GTK3 CSS: no `--vars`, `gap`, `inset`, glow. Tokens interpolated in `src/css.rs`.
- No HTTP on this thread. Overlay and header share `rank_actions_with`.
- Fades only on open/close (`PANEL_MS` 280). Drawer 160ms, overlay 100ms. Poll rebuilds are instant.
- No tokio, reqwest, webview, or Electron.

## Next

Ranking bugs → `chefbar-actions-palette`. IPC to show panel → `chefbar-tray-ipc`. Snapshot empty → `chefbar-actor`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, or Electron on the GTK thread.
