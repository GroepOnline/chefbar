# ChefApp QA — handmatige checklist (4.0)

> Voor elke PR naar `feat/chefapp-4.0` en vóór merge naar `main`.
> Afvinken: `[x]` = ok, `[ ]` = open, `[-]` = n.v.t. voor deze change.
> Automatische gates (fmt/clippy/test/visual) lopen in CI; dit dekt wat
> alleen een mens op een echte GNOME-sessie kan beoordelen.

## Launch & lifecycle

- [ ] `Super+Space` opent het panel binnen 300 ms (stopwatch of gevoel)
- [ ] Herhaald `Super+Space` (5× snel) levert precies één venster (geen duplicaten)
- [ ] `Esc` sluit drawer → palette → panel (gestapeld, niet alles tegelijk)
- [ ] Tweede instantie (`chefbar --bar`) toont bestaande panel, start geen tweede proces
- [ ] `chefbar --ipc quit` sluit netjes; socket `$XDG_RUNTIME_DIR/chefbar.sock` is weg of stale-cleanup werkt

## Panel & navigatie

- [ ] Panel toont 860×880, gecentreerd, niet resizable (`set_resizable(false)`)
- [ ] Sidebar 240 px: groepen + hairline-separators, actieve groep heeft accent-streep
- [ ] Boven: app-titel "ChefApp" + profielnaam (mono 10 px muted); onder: statusfooter 1 regel (zelfde als tray)
- [ ] Zones renderen per domein met header (titel · count · freshness · ›) + card-grid
- [ ] Cards: titel semibold 13 px, meta 11 px muted, pill BEZIG/HULP/KLAAR/STIL, shortcut rechts mono 10 px
- [ ] Empty-state toont `Leeg`-component met CTA, nooit leeg zonder uitleg
- [ ] Detail-drawer: klik card → slide 300 px rechts/onder, `Esc` sluit drawer vóór panel

## Palette overlay

- [ ] `Super+Space` toont palette-overlay (zelfde ranking als panel-header)
- [ ] Typen filtert direct over alle domeinen; ranking tier-invariant (contains > prefix > gappy)
- [ ] `↑`/`↓` navigeert, `↵` voert uit, `Esc` sluit overlay terug naar panel of desktop
- [ ] `/` focust zoekveld ook wanneer palette niet open is
- [ ] Alias-expansie werkt: `fleet` vindt Herdr, `vault` vindt Commerce, `share` vindt Sync

## Density toggle

- [ ] `comfortable` is default: card-padding 12 px, header 14 px, grid-gap 12 px
- [ ] `compact` halveert padding/gap via setting of `CHEFBAR_DENSITY=compact`
- [ ] Toggle is ogenblikkelijk (geen herstart/restart vereist waar Lane F dat belooft)
- [ ] `scripts/visual-shot.sh density-compact` vs `density-comfortable` levert twee screenshots met zichtbaar verschil

## Tray & glyphs

- [ ] Tray-icon staat in GNOME top-bar (ksni), menu opent, geen duplicaten na reload
- [ ] Glyph-states kloppen (forceer via IPC):
  - [ ] `chefbar --ipc "state stil"` → stil-glyph
  - [ ] `chefbar --ipc "state bezig"` → bezig-glyph
  - [ ] `chefbar --ipc "state hulp"` → hulp-glyph (oranje/amber)
  - [ ] `chefbar --ipc "state fout"` → fout-glyph
  - [ ] `chefbar --ipc "state offline"` → offline-glyph; na 10 s terug naar live
- [ ] Statuslijn max 5 live regels + profielregel; nieuwste eerst binnen priority-groep
- [ ] Menu-items werken: FocusDomain / Open Thuis / Pause / Doctor / Quit

## Offline & freshness

- [ ] Offline-banner toont boven de zones wanneer vault/ops onbereikbaar (`freshness-reason`)
- [ ] Stale kaart: "stale sinds <t> — reden" (amber banner), nooit leeg zonder uitleg
- [ ] Laatste goede snapshot blijft zichtbaar onder banner (geen wit scherm)
- [ ] `chefbar --doctor` exit 0 bij OK, 1 bij degraded, 2 bij fout; IPC-first wanneer service draait

## Zoeken dat kiest

- [ ] Recent geopende kaart krijgt `Recent` chip en lichte ranking-boost (frecency, 24 u)
- [ ] Pinned items ranken boven unpinned binnen dezelfde tier
- [ ] Actieve sidebar-groep boost (+150) is voelbaar maar breekt nooit tier-grens
- [ ] `Cmd+K` / `Ctrl+K` alias naast `/` (indien Lane D geland)

## Motion & toegankelijkheid

- [ ] Drawer-slide ~160 ms, palette-fade ~100 ms, geen poll-animatie tijdens render
- [ ] Focus-ring (2 px accent) zichtbaar bij keyboard-nav, `Tab` door zones
- [ ] `Enter` opent drawer, `Cmd+Enter` opent in browser, `Del` vraagt confirm waar destructief
- [ ] Geen animatie-jank bij snelle poll-cycli

## Visual QA (scripts)

- [ ] `scripts/visual-shot.sh panel dark` → accent-pixels >0, screenshot in `/tmp/chefbar-dark-panel.png`
- [ ] `scripts/visual-shot.sh palette dark` → screenshot bestaat (warning-only tot overlay stabiel)
- [ ] `scripts/visual-shot.sh drawer dark` → screenshot bestaat (warning-only tot drawer stabiel)
- [ ] `scripts/visual-shot.sh all dark` → 5 screenshots met prefix `/tmp/chefbar-dark-*.png`

## ChefApp x10 — visueel (V0–V3)

Runner-only (`chef-runner-01-1` of CI). Geen `cargo` op laptop `joep`.

- [ ] `design-system.json` pin + `assets/design-tokens.snapshot.css` matchen `css.rs` light/dark
- [ ] General Sans + IBM Plex Mono zichtbaar (of fail-zichtbaar, niet stil Cantarell)
- [ ] Lucide-rail, geen Adwaita header/drawer-iconen; palette 560px + scrim
- [ ] Geen `to_uppercase()`-shout op sectiekoppen; illegal-CSS (gap/inset/--vars/gradient) leeg
- [ ] Motion: panel 280ms, drawer 160ms, overlay 100ms; poll-rebuild zonder fade; reduced-motion instant
- [ ] `scripts/visual-shot.sh --mode all --theme light` en `--theme dark` groen
- [ ] Agents/Flows/Goedkeuringen blijven gated empty tot ACP / Kater M2

## Install & doctor

- [ ] `shellcheck install.sh scripts/*.sh` groen
- [ ] `./install.sh` idempotent (2× achter elkaar zonder fout)
- [ ] `./install.sh --systemd` zet `Super+Space → chefbar --ipc bar` + user-unit `chefbar.service`
- [ ] `systemd-analyze verify chefbar.service` groen (of non-fatal skip op runner)

## Opmerkingen / bevindingen

- Datum: 2026-08-20
- Tester: chef-runner-01-1 (`~/chefapp-x10-visual`, `dbus-run-session`)
- Branch: `feat/chefapp-x10-visual` (GRO-1425)
- Visual-shot: `--mode all` dark `ALL_DARK=0` (`#5C97FF`) en light `ALL_LIGHT=0` (`#317CFF`)
- Open: StatusNotifier-tray blijft freedesktop-symbolic (host lookup, geen GTK-pixbuf). General Sans/IBM Plex Mono fail-zichtbaar via `install.sh`, niet stil Cantarell. P2 Goedkeuringen wacht op Kater M2; P3 Agents op ACP/`CHEFBAR_AGENTS_API`; P4 Brain-insight op mTLS; P5 Flows op Agents. Geen 5.0 Super App-claim.

## ChefApp 5.0 — acceptatiechecklist (§7)

Voer deze acht punten handmatig uit op de runner/service-installatie en noteer datum, SHA en display. Een lege kaart zonder freshness-reden is een fout.

- [ ] **1. Launch & stacked close:** `Super+Space` opent binnen 300 ms precies één venster; `Esc` sluit drawer → overlay → panel.
- [ ] **2. Search:** zoeken vindt resultaten in alle 15 domeinen; recent aangeraakt staat bovenaan, onafhankelijk van tier.
- [ ] **3. Freshness/offline:** elke zone toont verse data of `stale sinds <tijd> — <reden>`; offline behoudt de laatste goede snapshot met banner.
- [ ] **4. Tray:** zeven live regels, statuslijn en glyph (`stil/bezig/hulp/fout/offline`) kloppen; `FocusDomain` opent elk domein.
- [ ] **5. Inbox & quiet:** blocked/hulp/stale/limited worden op urgentie gebundeld; snooze/pause en quiet hours werken.
- [ ] **6. Read-first domains:** Fleet/containers drift, Vault accounts/providers/usage, CRM deals/Neon health en Commander tasks/work zijn zichtbaar; writes zijn beperkt tot policy-gedekte acties.
- [ ] **7. Secrets:** alleen metadata is zichtbaar; copy gaat via vault-api met audit en automatische clear na 45 seconden.
- [ ] **8. Doctor & single instance:** `chefbar --doctor` is consistent tussen shell en service-env (exit 0/1/2); twee processen houden één socket/venster.

### Lane-G visual matrix

Op `chef-runner-01-1` na een release-build:

```bash
shellcheck install.sh scripts/*.sh
scripts/visual-shot.sh --mode all --theme dark --out /tmp/chefbar-dark
```

Dit maakt panel, overlay, drawer, beide density-varianten en vijftien domeinshots. `Xvfb`/ImageMagick ontbreken betekent exit 2 (soft skip); een ontbrekend accentpixel of gestorven app is exit 1. CI uploadt de PNG-artefacten ook wanneer een warning-only shot faalt.
