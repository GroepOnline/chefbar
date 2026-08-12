# ChefBar optimalisatie & uitbreiding — plan (2026-08-12)

Vervolg op het masterplan (`docs/plan-volledig-werkend.md`, grotendeels afgevinkt)
en de roadmap (`docs/roadmap.md`). Dit plan richt zich op **prestaties**,
**uitbreiding** en **QA & tooling**, met de bouw-gezondheid als prioriteit 0.

## 0. Context & uitgangspunten

- **Staat:** 3.1 verscheept; masterplan W1–W5 vrijwel klaar. Restpunten van het
  masterplan die hier meeliften: layer-shell (W1), chef-hud (W1),
  poll-gezondheid in de statuslijn (W4).
- **Bevinding bij opstellen:** HEAD compileerde niet. `cargo build` faalde met
  twee fouten in `src/tray.rs` (`menu()`):
  - `E0382` — `snap` verplaatst door `snap.map(...)` en daarna opnieuw gebruikt;
  - `E0597` — `desktop_running` (local) gevangen in een `'static`-closure.
  Fix in deze sessie doorgevoerd: `.as_ref()` i.p.v. `.map()` + `move`-closure.
- **Bouwregel (hard):** op de laptop is de toolchain bewust verwijderd (shim in
  `~/.local/bin/cargo`). **Nooit lokaal bouwen.** De officiële gate is CI op de
  self-hosted runner **`chef@chef-runner-01-1`** (checkout `~/chefbar-check`,
  `export PATH=$HOME/.cargo/bin:$PATH`). Elke werkstroom in dit plan valideert
  dus via CI, niet lokaal.
- **Architectuur blijft:** één poll-actor → één snapshot → tray/panel/ipc.
  Geen tweede daemon, geen tweede poll-loop, geen tweede socket, geen tweede
  waarheid.

---

## 1. Prioriteit 0 — bouw & CI groen (deel gedaan)

| # | Actie | Status |
|---|-------|--------|
| P0.1 | Compileer-breuk `src/tray.rs` fixen (E0382/E0597) | ✅ gedaan (`.as_ref()` ×2, `move`-closure) |
| P0.2 | CI op `chef-runner-01-1` over de branch — check/test/build + visual | ✅ groen (2026-08-12) |
| P0.3 | Borging: compile-breuk mag nooit meer ongemerkt de branch in — QA-gate `scripts/gate.sh` (fmt · clippy · tests) staat nu in CI | ✅ (Q1/Q2) |

**Waarom:** de break overleefde de laatste merge (PR #20) omdat er geen lokale
build en geen clippy/fmt-gate in CI zit. Q2 maakt dit onmogelijk.

---

## 2. Prestaties

Principe: **eerst meten, dan optimaliseren.** Per werkstroom een kleine
meetstap (timings loggen via doctor/`--debug`), daarna pas schroeven.

### P1 — poll-actor: thread-per-endpoint → vaste worker-pool
`src/state.rs::fetch_all()` spawns per poll (elke 5s) **10 `std::thread`s**
(één per endpoint) met een channel + 200ms-recv-loop binnen een 8s-budget.
Dat is ~2 threads/s thread-churn voor tien lokale HTTP-calls.

- Meetstap: per-endpoint timing loggen (ms) onder doctor/`--debug`.
- Voorstel: vaste kleine pool (bijv. 4 workers) of één fan-out-loop met
  per-request timeout i.p.v. thread-per-endpoint.
- Acceptatie: zelfde budget (`FETCH_BUDGET_MS`), geen meetbare vertraging in de
  riptijd, thread-churn → 0.

### P2 — HTTP: gedeelde `ureq::Agent` (connection reuse)
`src/http.rs::agent()` bouwt **elke call een nieuwe `ureq::Agent`**: geen
keep-alive, bij HTTPS opnieuw een TLS-handshake per request (relevant zodra we
via de CF-edge draaien, niet alleen loopback).

- Voorstel: één `Agent` per `Client` (gedeeld via `Arc`), config met
  `redirects(0)` + timeout blijft; headers blijven per call via
  `auth::get_headers()`.
- Meetstap: count/duur van verbindingen voor/na (doctor-latency per endpoint).
- Acceptatie: herhaalde polls re-connecten niet meer per request; latency per
  endpoint stabiel.

### P3 — panel-render: full rebuild → diff/update-in-place
`src/panel.rs::render_into()` verwijdert elke render **alle children** en bouwt
de hele inhoud opnieuw — elke 5s zolang het venster open is. Alleen zichtbaar
bij open paneel (goed), maar: herbouw kost allocaties en forceert herlayout.

- Meetstap: render-tijd per cyclus loggen (glib-timer).
- Voorstel, in oplopende zwaarte:
  1. **Revisie-check:** sla over als `snap.revision` + error + fetched_at
     onveranderd zijn (de goedkoopste winst); ✅ P3.1 gedaan (2026-08-12):
     handtekening over de zichtbare velden, statusregel blijft los tikken;
  2. per-sectie rebuild (alleen de secties die veranderden);
  3. row-widgets hergebruiken en tekstupdates i.p.v. vervangen.
- Randvoorwaarde: W1 blijft staan — vaste geometrie, geen resize-jumps,
  `present()` alleen bij verborgen→zichtbaar.

### P4 — opstart & footprint
- **Opstart:** `Panel::new` bouwt alles bij app-start; overweeg lazy build bij
  eerste `show()` (tray-only levens duiken sneller op). Doel: start→tray
  <500ms, start→panel <1s.
- **Snapshot:** `poll_vault` clonet de hele snapshot per cyclus (nodig voor de
  watcher) — ok, maar `raw`-JSON kan getrimd worden tot wat UI/doctor leest.
- **Release:** staat al op `lto = true` + `strip = true`; overweeg
  `panic = "abort"` (kleinere binary, minder unwind-code) zonder gedrag te
  veranderen.
- Doel-RSS < 80MB in rust, < 120MB bij open paneel.

---

## 3. Uitbreiding

### E1 — poll-gezondheid in de statuslijn (W4-staart)
Masterplan-restpunt: de statuslijn toont nu wel versheid van data, niet de
gezondheid van de poll zelf.
- `Shared` heeft al `last_error` + `vault_online`; voeg toe: laatste
  poll-tijdstip + ops-status (ok/HTTP-code) en toon bv.
  `laatste poll 4s geleden · vault ok · ops 302` in de statuslijn
  (`src/panel.rs`) én in `--doctor`.
- Klein, zelfstandig, hoge zichtbaarheidswaarde.

### E2 — Wayland layer-shell (roadmap: bewust uitgesteld)
✅ Implementatie + fallback-bewijs (2026-08-12):
- `gtk-layer-shell` 0.8 (gtk 0.18) als **optionele dependency** achter feature
  `layer-shell` (default **uit** — de systeem-lib `libgtk-layer-shell` is een
  build- én runtime-eis; de laptop heeft alleen de GTK4-variant).
- `src/layer_shell.rs`: top-laag, top-right-anchor, marge, exclusive zone,
  exclusive keyboard; alleen actief op echte Wayland-sessies
  (`WAYLAND_DISPLAY` + `is_supported()` + `init_for_window`), anders nette
  fallback naar het bestaande X11-gedrag.
- Dev-libs geïnstalleerd op `chef-runner-01-1`; CI heeft een pkg-config-
  guarded `--features layer-shell` build-stap; de X11-fallback blijft bewezen
  door het visual-harnas.
- **Inschakelen op de desktop:** `apt install libgtk-layer-shell0` + build met
  `--features layer-shell` (of CI-artifact met de feature).
- Beslis-tree (Wayland met lib → laag; anders X11-fallback) blijft.

### E3 — chef-hud vs chefbar expliciteren (W1-staart)
✅ Gedaan (2026-08-12): Alt+Space (chef-hud) en Super+Space (chefbar) mogen
nooit als "tweede ChefBar" lezen. `--doctor` waarschuwt zolang
`~/.local/bin/chef-hud` aanwezig is (alleen info, geen failure); README heeft
de sectie "Eén quick-command-overlay" met de retireren/herbinden-keuze.

### E4 — OIDC via de `get_headers`-seam
De seam staat (`src/auth.rs::get_headers()`); OIDC-access-tokens landen daar
zonder client-herbouw. **Wachten op** `auth.chefgroep.online` (Authentik) —
configuratievraagstuk, geen refactor. Blokkerend extern, dus laatste in
volgorde.

### E5 — nieuwe acties & oppervlakken
✅ Deel gedaan (2026-08-12): **Herdr-agents in het panel** — eigen Herdr-sectie
(naast Agents) met pane/focus-status en inline prompt-sturen (klik op een rij →
tekst-dialog → SendPrompt). Commander-queue, rustige meldingen en deep-links
blijven open (M5-staart).
- **Herdr-agents in het panel:** herdr-agent-rijen tonen pane/focus-status
  (`OpsSnapshot.agents` heeft `pane_id`/`focused` al) + inline prompt-sturen
  (nu alleen via de actie "Stuur naar …").
- **Commander-queue beheren:** lijst + prioriteit + cancel (CancelTask bestaat
  al in `src/actions.rs`); voeg per-taak acties toe.
- **Rustige meldingen uitbreiden:** per-agent mute / do-not-disturb-schema
  (nu alleen 1u-alles-pauze via joep-notify in `src/tray.rs`).
- **Deep-links benutten:** evidence-urls, kater-sessies en workspace-urls zitten
  al in `AttachPoints` (`src/sessions.rs`) — zichtbaarder maken in sessie-rijen.

### E6 — toetsenbord-first
✅ Gedaan (2026-08-12): pijltjes ↑/↓ door de actie-rijen van het actieve harnas
(wrap-around + `.selected`-stijl), Enter voert de geselecteerde rij uit,
Ctrl+K/Cmd+K focust zoeken; `/` en Esc stonden al. Selectielogica is een pure
`next_selection()` met unit-tests.
`/` focust zoeken, Esc verbergt (staat). Uitbreiden: pijltjes door de
resultaten van het actieve harnas, Enter = uitvoeren, Ctrl+K/Cmd+K = zoeken
focussen; focus-chain expliciet maken (komt deels al terug via GTK-traversal).

### E7 — config & versie
- Meer `CHEFBAR_*`-velden (poll-intervallen `VAULT_POLL_MS`/`OPS_POLL_MS`,
  notificatie-prefs) — warden-laag blijft per-veld, nooit per-bestand.
- Changelog + bump naar 3.2 met roadmap-update in de README.

---

## 4. QA & tooling

### Q1 — clippy in CI (met `-D warnings`) op `chef-runner-01-1`
Huidige CI (`ci.yml`) doet check/test/build/shellcheck/visual. Voeg toe:
`cargo clippy --all-targets -- -D warnings` (naast `cargo check`).

### Q2 — pre-commit/CI-gate tegen compile-breuken
De E0382/E0597-breuk overleefde een merge. Borging:
- `.git/hooks/pre-commit` (of `.githooks` in de repo) die `cargo check`
  draait op de runner — op de laptop is `cargo` bewust afwezig, dus de hook
  kan alleen een ssh-`cargo check` naar `chef-runner-01-1` triggeren;
- in elk geval: CI-clippy + een expliciete `cargo check --all-targets`-stap
  vóór de test-job (staat er al, wordt nu dus effectief — de break kwam
  binnen via een werkstroom die CI niet doorliep).

### Q3 — tests
✅ **Tray-menu refactoren** (2026-08-12): `menu()` is nu een dunne ksni-adapter;
alle inhoud komt uit de pure data-builder `menu_items(snap, profile, autostart)
-> Vec<MenuItemSpec>` (geen ksni-types/closures). 7 unit-tests dekken de
rijlogica (basisrijen, account-submenu, desktop-state, autostart-checkmark,
geen-events, char-veilige titel-truncatie) — precies waar de E0382/E0597-breuk zat.
- **state.rs testbaar maken:** `http::Client` achter een trait of mock-`Client`
  zodat `fetch_all`/`poll_vault` (fan-out, budget, coalescing) zonder netwerk
  getest kunnen worden (nog open).
- **Golden CLI-tests uitbreiden:** de bestaande clap-tests (`src/main.rs`)
  blijven; voeg `--ipc state <x>` en `switch-account`-varianten toe (nog open).

### Q4 — logging
✅ Gedaan (2026-08-12): `src/log.rs` — lichte bestand-logger (append) naar
`~/.local/state/chefbar/chefbar.log` (of `CHEFBAR_LOG`). Actor (vault-/ops-poll)
en executor (alle actie-fouten) loggen nu; `--doctor` toont het pad; README
wijst naar het echte bestand.

### Q5 — doctor & observability
- Latency-probes per endpoint (DNS/TLS/connect), ops-status, laatste poll-tijd
  (zie E1); exit-codes 0/1/2 bestaan al.
- Meldingen en tray-tooltips mogen het doctor-pad tonen ("doctor · alles ok").

---

## 5. Volgorde & milestones

| Milestone | Inhoud | Gate |
|-----------|--------|------|
| **M1 (nu)** | P0.1-fix (gedaan) + CI groen over de branch op `chef-runner-01-1` | CI check/test/build/visual |
| **M2** | Q1 (clippy) + Q2 (gate) → daarna P2 (Agent reuse) en P3.1 (revisie-check), elk met meetstap | CI + gemeten verbetering |
| **M3** | E1 (poll-gezondheid), E3 (chef-hud), E6 (toetsenbord) — klein en zelfstandig | CI + visual-shot |
| **M4** | E2 (layer-shell) als eigen PR met fallback-bewijs; P1 (worker-pool), P4 (opstart) | CI + visual-shot (X11-fallback) |
| **M5** | E5 (acties/oppervlakken), E7 (config/versie 3.2), Q3–Q5 | CI + golden tests |
| **E4** | OIDC — pas zodra `auth.chefgroep.online` live is | externe afhankelijkheid |

## 6. Acceptatiecriteria (3.2 done = dit)
1. CI op `chef-runner-01-1` is groen (check, clippy `-D warnings`, test,
   build, visual) — en blijft groen op elke PR.
2. Geen compile-breuk kan nog ongemerkt de branch in (Q2-gate).
3. Poll-render: gemeten her-render-tijd en thread-churn dalen (P1/P3-metingen
   in de PR beschreven).
4. Statuslijn toont poll-gezondheid (laatste poll · vault · ops) — panel én
   doctor.
5. Alle nieuwe oppervlakken/acties hebben unit- of golden-tests; tray-menu is
   uit de ksni-closures getrokken en getest.
6. `chefbar.log` bestaat en bevat actor/executor-fouten (Q4); README wijst
   naar het echte pad.

## 7. Niet-doelen
- Geen tweede daemon, poll-loop, socket of tray.
- Geen Electron/webview; GTK3 blijft de surface tot de layer-shell-beslissing.
- Geen lokale builds — validatie loopt altijd via CI op `chef-runner-01-1`.
- Geen scope-bloat in 3.2: elk item hierboven is een eigen, reviewbare PR.
