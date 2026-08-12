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
✅ Gedaan (2026-08-12): `WorkerPool` in `src/state.rs` — 4 vaste workers op een
Mutex/VecDeque + Condvar wachten op jobs en leven zolang de actor leeft
(thread-churn → 0). Elke job draagt een eigen results-sender, zodat batches
geïsoleerd blijven. `fetch_all` stuurt de 10 endpoint-jobs naar de pool en
wacht met hetzelfde `FETCH_BUDGET_MS`-budget; unit-test bewijst dat de pool
nooit meer dan N jobs tegelijk draait.
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
✅ Gedaan (2026-08-12):
- **Lazy panel** — `Panel::new` bouwde alles bij app-start; nu bouwt
  `LazyPanel` het venster pas bij de eerste `show()` (tray-only levens duiken
  sneller op). Bouwtijd wordt gelogd in `chefbar.log` ("panel opgebouwd na
  Xms (lazy)"). Doel: start→tray <500ms, start→panel <1s.
- **Release:** `panic = "abort"` toegevoegd naast `lto = true` + `strip =
  true` (kleinere binary, minder unwind-code). Bijzonderheid ontdekt in CI:
  ksni paniekt zonder D-Bus session-bus (headless/Xvfb) — met abort zou die
  paniek de hele app uitschakelen, dus de tray start alleen als er een bus is
  (`DBUS_SESSION_BUS_ADDRESS` of `$XDG_RUNTIME_DIR/bus`) en skipt netjes met
  een logregel anders (alle UI-paden guarden al op TRAY_HANDLE).
- **Snapshot-`raw`-trim** ✅ gedaan (2026-08-12): `poll_vault` bewaart nu
  alleen nog `raw["status"]` (de enige consument is `tray_state`, leest
  `raw.status.services`); providers/agents/fleet/sessions worden niet meer
  in `raw` geassigned — de geparsede velden (ProviderRow/AgentRow/…) zijn de
  echte UI-bron. De snapshot-clone per cyclus (nodig voor de watcher-diff)
  wordt daarmee lichter. (Geverifieerd: `grep '\.raw' src/` → alleen
  models.rs:736 + state.rs.)
- **P4-metingen** ✅ gedaan (2026-08-12) — `scripts/measure-p4.sh` (Xvfb,
  zelfde isolatie als visual-shot) meet op de release-build: binary-grootte,
  start→gereed (logregel "gestart", gepollt vanaf launch), start→panel
  (latentie vanaf de IPC-poke tot de "panel opgebouwd"-logregel), RSS
  vóór/na paneel (`VmHWM` uit /proc). Resultaten op chef-runner-01-1
  (release 3.2.0, headless Xvfb → tray skipt door de D-Bus-guard):

  | Meting | Waarde | Doel | Status |
  |---|---|---|---|
  | Binary | 3770 KiB | kleiner | ✅ |
  | start→gereed | **57 ms** | <500 ms | ✅ |
  | start→panel (poke→klaar) | **64 ms** | <1 s | ✅ |
  | RSS rust | 78,2 MiB | <80 MiB | ✅ |
  | RSS met paneel | 93,2 MiB | <120 MiB | ✅ |
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
tekst-dialog → SendPrompt). **Commander-queue** ✅ — eigen sectie met taak-lijst
(positie + korte id), status-stamps en per-taak **Stop** (CancelTask via de
existing executor-route; queued/running). *Prioriteit verzetten* blijft open:
de vault-API exposeert geen priority-endpoint (geen verzonnen endpoint).
Rustige meldingen en deep-links blijven open (M5-staart).
- **Herdr-agents in het panel:** herdr-agent-rijen tonen pane/focus-status
  (`OpsSnapshot.agents` heeft `pane_id`/`focused` al) + inline prompt-sturen
  (nu alleen via de actie "Stuur naar …").
- **Commander-queue beheren:** lijst + prioriteit + cancel (CancelTask bestaat
  al in `src/actions.rs`); voeg per-taak acties toe.
- **Rustige meldingen uitbreiden:** ✅ per-agent mute gedaan (2026-08-12) —
  `src/mutes.rs` (demp-lijst per agent-key, atomair JSON, env-override
  `CHEFBAR_MUTED_AGENTS`); de watcher slaat gedempte agents over bij toasts
  (en haalt hun oude inbox-suggesties weg); paneel-rij met Demp/Ontdemp-togg
  en een tray-submenu "Demp agenten" (pure builder); `--ipc "mute <key>"` +
  golden test.
- **Do-not-disturb-schema (rustige uren)** ✅ gedaan (2026-08-12):
  - **Doel:** buiten werktijd geen niet-kritieke toasts. FOUT (error) gaat
    altijd door; KLAAR/HULP/LIMIET zwijgen. De inbox blijft gewoon gevuld
    (panel toont de suggesties), alleen de toast-route dempt.
  - **Config:** env `CHEFBAR_QUIET="HH:MM-HH:MM"` — één venster, overnight
    toegestaan (22:00-07:00). Warden-laag per veld, geen nieuw bestand.
    Geen venster = uit.
  - **Bouw:** `src/quiet.rs` — `quiet_window()` (parse, minuten mogen
    ontbreken), pure `in_quiet_hours_at(window, h, m)` (overnight-wrap en
    grensgevallen getest: `from` exact = actief, `to` exact = niet meer,
    gelijke grenzen = veilig uit), dunne `in_quiet_hours()` met lokale tijd
    via `libc::localtime_r` (libc al in de lock; geen chrono-dep).
  - **Waar:** `state.rs::poll_vault` (toast-route filtert `status != error`
    in rustige uren), tray-menu info-regel ("Rustige uren 22:00–07:00 ·
    actief/stil", puur in `menu_items`), doctor-regel en `--show-config`-regel.
  - **Acceptatie:** 7 unit-tests voor parse/wrap/grenzen + tray-test; rustige
    uren dempen KLAAR/HULP maar nooit FOUT.
- **Deep-links benutten** ✅ gedaan (2026-08-12): evidence-urls, kater-
  sessies en workspace-urls zaten al in `AttachPoints` (`src/sessions.rs`)
  maar alleen de primaire actie (één CTA per sessie) was zichtbaar.
  - Pure `session_deep_links(session, profile) -> Vec<(String, RunSpec)>` in
    `src/actions.rs`: alle aanhechtbare links in vaste volgorde (evidence >
    workspace > browser > kater-sessie > focus), max 3; kater-URL uit het
    profiel, focus als `FocusAgent`-run.
  - In de "Heeft jou nodig"-sectie per sessie max 2 kleine knoppen ("Bewijs
    ↗" enz.) voor de links die níet óók de primaire CTA zijn; tooltip toont
    het doel (URL of focus-id).
  - Unit-tests: volgorde (alle vijf attach-punten → top-3), kater+focus,
    lege attach → geen links.

### E6 — toetsenbord-first
✅ Gedaan (2026-08-12): pijltjes ↑/↓ door de actie-rijen van het actieve harnas
(wrap-around + `.selected`-stijl), Enter voert de geselecteerde rij uit,
Ctrl+K/Cmd+K focust zoeken; `/` en Esc stonden al. Selectielogica is een pure
`next_selection()` met unit-tests.
`/` focust zoeken, Esc verbergt (staat). Uitbreiden: pijltjes door de
resultaten van het actieve harnas, Enter = uitvoeren, Ctrl+K/Cmd+K = zoeken
focussen; focus-chain expliciet maken (komt deels al terug via GTK-traversal).

### E7 — config & versie
✅ Gedaan (2026-08-12):
- Poll-intervallen per env: `CHEFBAR_VAULT_POLL_MS` / `CHEFBAR_OPS_POLL_MS`
  (ondergrens 500ms; defaults blijven de constanten) — de actor-loop én de
  panel-refresh-loop volgen ze. Demp-lijst via `CHEFBAR_MUTED_AGENTS`.
  Warden-laag blijft per-veld, nooit per-bestand.
- Changelog + bump naar **3.2** (`Cargo.toml`) met roadmap-update in de
  README.

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
✅ **state.rs testbaar maken** (2026-08-12): `http::Client` achter de
`HttpClient`-trait (get_json); `Poller<C>` is er generiek over. Mock-`Client`
(Arc<Mutex>-stubs per pad) test fan-out-succes, totale/partiële foutafhandeling
("vault offline" / "gedeeltelijk"), watcher-coalescing (transitie → één
suggestie), ops-poll-status (ok/HTTP-code) en de pool-concurrency — 7 nieuwe
tests, zonder netwerk.
- **Golden CLI-tests uitbreiden:** de bestaande clap-tests (`src/main.rs`)
  blijven; voeg `--ipc state <x>` en `switch-account`-varianten toe (nog open).

### Q4 — logging
✅ Gedaan (2026-08-12): `src/log.rs` — lichte bestand-logger (append) naar
`~/.local/state/chefbar/chefbar.log` (of `CHEFBAR_LOG`). Actor (vault-/ops-poll)
en executor (alle actie-fouten) loggen nu; `--doctor` toont het pad; README
wijst naar het echte bestand.

### Q5 — doctor & observability
✅ Gedaan (2026-08-12):
- Latency-probes per endpoint (vault `/status`, ops `/api/snapshot`) met
  round-trip-ms in de doctor-uitvoer; transportfouten tellen als failure,
  HTTP-codes als info (401 zonder token is een secrets-zaak). Laatste
  poll-tijd stond al (E1).
- Doctor draait vanaf tray/panel in de achtergrond (`run_checks_background` —
  probes mogen de UI-thread nooit laten wachten op timeouts) en toont het
  resultaat óók in de tray-tooltip ("doctor · alles ok · Xms", 12s).

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
