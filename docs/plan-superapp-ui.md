# Super-app UI-lane (visueel + 15-domein render)

Branch: `feat/superapp-visual` → base `feat/chefapp-5.0`. Eigenaar: Pi-sessie
"superapp visual". Dirigent-coördinatie: Cursor (Herdr w2R). Sibling: w2R:p2
(chefapp-herdr, echte herdr CLI) — **niet aangeraakt hier**: `src/ops_cli.rs`
en Executor-varianten `FleetExec`/`FleetDeploy`/`SendPrompt` in `src/actions.rs`.

## Doel

ChefBar = de Chef super app, volledig in de **Devin v2**-taal
(`GroepOnline/design-system`: warm basalt, hairlines, één accent #5C97FF,
General Sans + IBM Plex Mono, worked-row-streep, geen spinners/emoji/gradients).
Het gat dat bestond: typed snapshot (lane H, 18 /api/*-families) was er,
maar de panel renderde altijd dezelfde 6 generieke secties. Deze lane rendert
per domein zijn eigen data.

## File-scope

| File | Wat |
| --- | --- |
| `src/panel/domains.rs` | **nieuw** — 15 per-domein views uit typed snapshot-data |
| `src/panel/mod.rs` | domein-dispatch, gepinde footer, statuslijn running-accent, header toont actief domein |
| `src/css.rs` | Devin v2-poolse: density-regels, 2px-signature, stamps, kbd, footer-chips, live theme-provider |
| `src/panel/zones.rs` | `domain_row`, `status_dot_cls`, STIL neutraal, empty-state zonder dash-icoon |
| `src/panel/sidebar.rs` | gegroepeerde nav-volgorde, statische labels + live counts, `label_for` |
| `src/panel/header.rs` | titel = actief domein, tracking −0.02em, "ChefBar" |
| `src/panel/drawer.rs` | Annuleren, ↵-hint, Enter voert uit |
| `src/panel/overlay.rs` | palette-rijen: titel + meta + stamp, ACTIES-cap |
| `src/panel_state.rs` | `theme`-veld (dark/light, tolerant, persists) |
| `src/tray.rs` / `src/ipc.rs` / `src/main.rs` | `UiCommand::DrawerPreview` (`chefbar --ipc drawer`) voor visual-shot; css-provider init |
| `src/harness.rs` | `HarnessGroup::Fleet` label "Fleet" (was "Vastgoed") |

## Gedrag

- **Elke domein** (Inbox, Fleet, Herdr, Containers, Vault, Accounts, CRM,
  Share, Clipboard, Desktop, Taken, Linear, Secrets, Kater, Health) rendert
  zijn typed rijen: dot (statuskleur), titel, mono-meta, stamp rechts.
  Max 8 rijen, sectie-sub telt "{shown} van {total}".
- **Acties-zone** blijft bovenaan elk domein (interactie eerst, zelfde
  ranking als palette).
- **Signalen** (watcher-suggesties) en **Heeft jou nodig** alleen op
  werk-domeinen (inbox/fleet/herdr/health/eval/taken/linear).
- **Footer** is gepind onder de scroller: live counts + toggles
  Dichtheid (Rustig/Compact) en Thema (Donker/Licht) — beide persist.
- **Secrets** toont alleen meta, nooit plaintext. Clipboard-rij = kopiëren.
- Aliasen: IPC `focus-domain accounts|providers` → commerce, `taken` → tasks.

## Gates

Build/test alleen op `chef-runner-01-1` (`~/chefbar-superapp`):
fmt --check, clippy -D warnings, test --all-targets, build --release.
Visual: `scripts/visual-shot.sh --mode all` (Xvfb) — drawer-mode werkt nu via
de nieuwe IPC-preview. Review-loop: screenshots → mimo-v2.5 vision-subagent.

## Status

- [x] css-v2-basis + density + theme-toggle
- [x] 15 per-domein views + dispatch
- [x] gepinde footer, drawer/palette/sidebar-polish
- [ ] gates groen op runner (bezige build)
- [ ] visuele matrix + vision-review-ronde
- [ ] PR tegen feat/chefapp-5.0
