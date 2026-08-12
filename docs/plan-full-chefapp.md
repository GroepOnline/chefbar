# ChefApp — van hulpje naar volwaardige app (plan 2026-08-12)

> Status: **in uitvoering — Fase 0 done (2026-08-12)**. Dit document is de SSOT voor de sprong van ChefBar 3.1 (dun, stabiel) naar **ChefApp 4.0** — de native mission-control app voor alles ChefGroep.
> Branch: `main` → `feat/chefapp-4.0` stack. Uitvoer: 7 file-disjointe lanes parallel, één merge-train.
> Fase 0: `feat/chefapp-4.0` stack root + `src/panel/` split + `4.0.0-dev` — gemerged.

---

## 1. Recap — waar we waren

### 1.1 Wat er staat (3.1, gemerged op `main`)

| Laag | Huidige realiteit |
|---|---|
| **Stack** | Rust, GTK 3 (`gtk 0.18`), `ksni 0.2` tray, `ureq 2` zonder redirects, `clap 4`, `url 2`. Geen Electron, geen webview. Build alleen op `chef-runner-01-1` (laptop heeft geen toolchain — stubs). |
| **Architectuur** | Één poll-actor (`src/state.rs`: vault 5 s, ops 15 s, budget 8 s) → één `Snapshot` + `OpsSnapshot` onder `RwLock` → tray/panel delen dat beeld. Geen tweede loop, geen tweede socket. |
| **Sources** | `vault-api` (`/api/status`, providers, accounts, share-sync, commander) + `joep-ops` (`:10101`, agents/herdr) + lokaal (`watchdog-state.json`, dagscore-md/json, share-clipboard). Tailscale-forward `/ vault-forward.service` vervangt ssh-forward; `ops.chefgroep.online` wordt lokaal geserveerd. |
| **Panel** | 760×840 fixed, `set_resizable(false)`, undecorated, `keep_above`, gecentreerd. Sidebar 220 px + search-header + card-zones + footer. `panel.rs` = 1504 regels monoliet. |
| **Zoeken** | `/` focust, `palette.rs` fuzzy (contains 1000, prefix 700, gappy 500) + `RankContext` boost (+150) uit recente sessies/lopende agents. Bewaart harnas + query in `panel-state.json`. |
| **Acties** | `actions.rs` (721 r): declaratieve `RunSpec` (OpenUrl, FocusAgent, SwitchAccount, ClipboardAdd/Delete, CreateTask, DesktopAction …) → één `Executor` met `http::Client` + `EndpointPolicy`. |
| **Harnassen** | `harness.rs` (405 r): 4 kinds — `fleet`, `commerce`, `sync`, `eval` — elk kleur + keyword-prefixen. `build_harnesses` leidt af uit `Snapshot`; panel filtert actions via prefix-match. |
| **Tray** | `tray.rs` (514 r): ksni, `UiCommand` (Show/Toggle/Refresh/Doctor/Quit/OpenUrl/FocusAgent/SwitchAccount/Pause/ToggleAutostart/DesktopAction/ForceState), `start_command_bridge` (60 ms glib-poll), `ForceState` testhook voor glyph-verificatie. |
| **Styling** | `css.rs` (514 r): Signaal v2 / Devin-skin gemapt op GTK3-subset. Tokens warm off-white `#F7F6F5` / basalt `#121111`, accent `#317CFF` / `#5C97FF`, `General Sans` + `IBM Plex Mono`, hairlines, radius 6/10/200, CG-statuslijn (2 px `chefbar-signature`). |
| **Doctor/IPC** | `doctor.rs` bevraagt eerst live instantie via IPC; `ipc.rs` Unix-socket op `$XDG_RUNTIME_DIR/chefbar.sock` (0600), `UiCommand` parse + `acquire`/`send_command`/`spawn_listener`. Single-instance via socket-bind + retry. |
| **Install/service** | `install.sh` → `~/.local/bin/chefbar` + `~/.config/chefbar/endpoints.json` + optioneel `chefbar.service` (user-unit, `PartOf=graphical-session`, `RuntimeDirectory=chefbar`). `Super+Space → chefbar --ipc bar`. `CHEFBAR_*` env wint per veld van profiel. |
| **QA** | `cargo fmt --check` + `clippy --all-targets -- -D warnings` als harde CI-gates (self-hosted `company-control`). `scripts/visual-shot.sh` (Xvfb + accent-assert), CLI golden tests, single-instance e2e. |

Verscheept in 3.1 (roadmap): *zoeken dat kiest* (ranking), *rustigere meldingen* (`coalesce_toasts`, één toast/cyclus), *panel dat onthoudt* (`panel_state.rs`).

### 1.2 Masterplan W1–W5 — wat af is, wat open staat

Bron: `docs/plan-volledig-werkend.md` (2026-08-12, 6 defecten D1–D6).

| Werkstroom | Done | Open |
|---|---|---|
| **W1 venster-stabiliteit** | D1 hotkey-alias (`--bar`, PR #10), vaste geometrie (scroller-lock, PR #17), fades alleen open/dicht, `present()` alleen bij hidden (PR #17) | `gtk-layer-shell` evaluatie (Wayland tray-anker, top-right), `chef-hud` (Alt+Space) retireren/herstylen, 10×10 hidden window → `visible=false` start |
| **W2 data-vlak** | ssh-forward → `vault-forward.service` + Tailnet-fallback (PR #20), ops lokaal via `joep-ops-serve` (PR #20), freshness-contract + kosten-wiring (fractie-balk i.p.v. `$0.0000`, PR #20) | Poll-gezondheid in statuslijn ("laatste poll 4 s · vault ok") — klein restant W4 |
| **W3 tray/notificaties** | Compacte statuslijn (max 10, `chefbar-tray.md`, PR #20), mako/dunst v2-templates, glyph-testhook `--ipc state …` (PR #20) | Glyph-verificatie op echt GNOME-panel (live-run, niet Xvfb) |
| **W4 doctor/IPC** | Doctor via IPC + exit-codes 0/1/2 (PR #18) | Poll-gezondheid zichtbaar in UI |
| **W5 QA-harnas** | `visual-shot.sh`, CLI golden tests, single-instance e2e, CI visual-job warning-only (PR #19) | Screenshot-diff hardening (nu warning-only) |

De app is **stabiel maar dun**: één venster, vier harnassen, een handvol acties. De schil klopt; de inhoud is nog een sampler.

### 1.3 Waarom dit niet genoeg is

ChefBar toont nu een *samenvatting* van Vault + Ops. Voor dagelijks werk moet je nog naar: Vault-dashboard `:8080` (frontend), Vaultwarden, Kater `:9090`, Herdr, Linear, Docker/containers, Clipboard/share-sync, Commander/tasks, provider-rekeningen, Neon/CRM. Dat zijn nu losse tabs en CLI's. De laptop heeft geen één plek waar alles samenkomt — precies waar Raycast/Linear/Alfred wél winnen: één shortcut, alles binnen handbereik, geen context-switch.

---

## 2. North star — wat ChefApp is

### 2.1 Elevator

> **ChefApp is de native desktop-app voor ChefGroep — één venster, één shortcut, alle domeinen.** Open met `Super+Space`, zoek of klik, handel af, sluit met `Esc`. Tray toont in één oogopslag of er iets om jou vraagt. Vault, Fleet, Kater, Linear, containers, secrets, clipboard en observability leven in dezelfde app, met dezelfde zoekbalk, dezelfde kaarten, hetzelfde beleid als de web-surfaces — maar sneller, offline-tolerant en zonder browser-tab-chaos.

### 2.2 Principes (non-negotiables)

1. **Eén waarheid.** Eén `Snapshot`, één actor, één socket, één profiel. Geen tweede poll-loop. Nieuwe domeinen zijn nieuwe *secties* in dezelfde snapshot, geen tweede app.
2. **Geen browser in een jasje.** Native GTK, geen webview-wrapper. Wil iemand het volle Vault-dashboard, dan opent ChefApp de URL — niet een iframe. Snel, licht, geen Electron-schuld.
3. **Zoek is de app.** Elk domein is doorzoekbaar via dezelfde balk. Ranking is overal gelijk: recency + lopende agents + pinned + frecency. Geen domein-specifieke zoekmodi.
4. **Acties zijn data.** Alles wat je doet is een `RunSpec` + `Executor`. Geen closures in de UI, geen netwerk op de main-thread, testbaar als pure data.
5. **Stilte is een feature.** Alleen transities melden. Geen ticker, geen badge-spam. Eén toast per cyclus, coalesced. Tray-glyph is de rustige indicator.
6. **Signaal, warm — geen bling.** Devin/Signal v2 blijft de visuele autoriteit: warm canvas, scherp accent, hairlines, mono voor data. Geen gradients, geen glassmorphism, geen bento-farm.
7. **Offline-tolerant.** Elk pane toont laatst-bekende data + freshness-reason ("stale sinds 08:12 — vault 401 / ops 302 / offline"). Nooit een leeg scherm.
8. **Beleid wint altijd.** `EndpointPolicy` + `auth::get_headers` blijven de enige uitgang. Geen URL zonder allowlist-check, geen bearer op redirect.

### 2.3 Wat het níét is

- Geen tweede Vault-dashboard (geen React in GTK).
- Geen tweede Kater-gateway (praat *met* Kater, vervangt hem niet).
- Geen full IDE / Herdr-vervanger (focus = besturen, niet editen).
- Geen mobiele app (laptop `joep` is de primaire surface).

---

## 3. Requirement atlas — 8 domeinen

Elk domein = één harnas-groep + één of meer snapshot-secties + een set acties. Volgorde = prioriteit voor 4.0.

### D1 — Inbox & Meldingen (verdiepen van bestaand)

*Doel: je opent de app en ziet in <1 s wat om aandacht vraagt.*

- Unified inbox: watcher-suggesties + blocked agents + fleet-health `warn/down` + Linear-assigned-to-me + share-sync errors, gesorteerd op urgentie (blocked > hulp > verslechtering).
- Coalescing blijft (1 toast/cyclus) + badge op tray + subtiele dot op sidebar-item (geen getal-spam).
- Snooze / "later" (1 u / tot morgen) via `joep-notify` pause, per item.
- Acties: `FocusAgent`, `OpenUrl` (naar juiste tab), `MarkDone`, `Snooze`, `PauseNotifications`.

### D2 — Fleet & Herdr (nu dun, wordt rijk)

*Doel: vloot zien en aansturen zonder terminal te openen.*

- Fleet-overzicht: alle tailnet-nodes (control-01, runner-01, sofie, jan, bc-scans) met status (online/ping/ssh/chef health), capaciteit, lopende workloads, drift-vs-desired.
- Per node: containers (uit `containers-observed.json`), systemd-services, disk/mem, laatste deploy.
- Herdr: workspaces + agents + panes, focus/attach, logs tailen (read-only in 4.0, exec in 4.1).
- Acties: `FleetExec` (template-commando's), `FleetDeploy`, `OpenKaterWorkspace`, `FocusAgent`, `TailLogs`.

### D3 — Vault Commerce: accounts, providers, CRM

*Doel: rekeningen en klanten bedienen zonder dashboard-tab.*

- Accounts: vault-accounts + provider-insights (kosten/verbruik, fractie-balk, freshness), balance/wallet indien beschikbaar.
- Commander: `commander` + `commander-hmac` tasks — lijst, filter op status, aanmaken (cwd-prompt), annuleren.
- CRM/Neon: deals/contacts uit vault-`crm`/`neon` — zoeken, recente deals, deal-detail via `OpenUrl`.
- Acties: `CreateTask`, `CancelTask`, `CopyText` (keys — met waarschuwing), `OpenUrl` (vault detail), `Refresh` per sectie.

### D4 — Containers & Workloads

*Doel: zien wat draait en wat gewenst is.*

- Observed vs desired: `catalog` vs `observed_host`, diff-view (te veel / te weinig / drift).
- Prune/migrate-hints (alleen tonen, niet auto-prunen in 4.0).
- Per container: image, host, status, logs-link, restart-hint.
- Acties: `OpenUrl` (naar vault docker-detail), `CopyText` (image ref), `PrunePreview` (read-only diff).

### D5 — Secrets & Vaultwarden

*Doel: secret vinden en veilig kopiëren.*

- Vaultwarden-collecties doorzoekbaar (titel + notities, geen secrets in snapshot), copy met clipboard-timeout, audit (alleen fingerprint in logs).
- Account-switcher: `SwitchAccount` (bestaand) uitbreiden met source/driver-keuze + recente accounts.
- Acties: `CopyText` (met auto-clear), `SwitchAccount`, `OpenUrl` (vaultwarden vault), `ClipboardAdd`.

### D6 — Clipboard, Share & Desktop

*Doel: klembord en gedeelde schijven als first-class.*

- Clipboard-geschiedenis (CopyQ-bridge): zoeken, plakken, verwijderen, sync-status.
- Share-sync: status per mount, pull/push, conflict-reason, "stale sinds" + actie.
- Desktop/webtop: start/stop/status, `DesktopAction` bestaande verb uitbreiden.
- Acties: `ClipboardAdd/Delete/CopyText`, `ShareSync`, `DesktopAction`, `OpenUrl` (desktop `:3000`).

### D7 — Linear & Tasks

*Doel: eigen werk zonder Linear-tab.*

- Mijn issues: assigned-to-me, recente, per project/sprint, status (todo/in-progress/done). Read-only in 4.0; mutaties via `OpenUrl` naar Linear.
- Commander-taken en Linear in één "Taken"-zone (twee bronnen, één sortering).
- Acties: `OpenUrl` (Linear issue), `CreateTask` (titel → Linear beschrijving in 4.1), `CopyText` (issue-id).

### D8 — Kater, Observability & Developer

*Doel: dev heeft alles bij de hand.*

- Kater: profielen, gateway-status, proxy-backends, docs-links, profile-switch.
- Observability: catalog-events (uit `catalog/observability-event-catalog.yaml`), recente errors/logs (read-only, sampling), health-detail per component.
- Developer: `doctor` inline (niet alleen CLI), policy-inspect, `show-config`, logs tail (`journalctl --user -u chefbar`), versie/sha.
- Acties: `OpenUrl` (kater dashboard, PostHog, Grafana), `Doctor`, `Refresh`, `OpenConfig`.

---

## 4. UX-architectuur

### 4.1 Drie surfaces, één snapshot

```
┌─────────────────────────────────────────────────────┐
│  Tray (ksni)  ──►  compacte statuslijn (max 10)     │  glanceable, altijd zichtbaar
│     │                 glyph: stil/bezig/hulp/fout   │
│     ▼                                               │
│  Command Palette  ──►  overlay, Super+Space, Esc    │  snelste pad: type → ↵
│     │                 fuzzy over alle acties          │
│     ▼                                               │
│  Panel (app-venster) ──►  full mission control      │  860×880, sidebar + canvas
│                           zones + detail-drawer      │
└─────────────────────────────────────────────────────┘
                    ▲
              Snapshot (Shared)
              poll-actor → RwLock
```

- **Tray** = *glance*: 3 live regels (BEZIG/KLAAR/JOUW) + Open Thuis/Ploeg + account + desktop + pause + autostart. Geen paginatie, geen modals. Klik op regel → focus.
- **Palette** = *speed*: `Super+Space` opent overlay (of panel als geen overlay — één codepath), typen filtert alle acties, `↵` voert uit, `Esc` sluit. Altijd <300 ms tot interactief.
- **Panel** = *control*: de ware app. Sidebar nav + search-header + gegroepeerde zones (per domein) + footer. Cards zijn compact, detail opent in drawer (rechts of onder, geen nieuw venster).

In 3.1 is palette == panel (zoekbalk ín het panel). In 4.0 krijgt palette een eigen overlay-mode (zelfde ranking, zelfde actions) — panel blijft voor browsen, palette voor doen.

### 4.2 Informatie-architectuur (sidebar)

```
Inbox  ·  3 om aandacht  ── Dot bij aantal >0
Fleet
Herdr              ── Fleet+Herdr gegroepeerd (vastgoed)
Vault
  Accounts
  Providers
  CRM / Neon         ── Commerce-cluster
Share
Clipboard
Desktop              ── Sync-cluster
Taken
Linear               ── Work-cluster (Commander + Linear)
Containers
Secrets
Kater
Health
Instellingen         ── System-cluster
```

- Sidebar 240 px (was 220), groepen met hairline-separator, geen icon-zoo — label + status-dot + count. Actief item heeft accent-streep (CG-statuslijn hergebruikt).
- Boven de sidebar: compacte app-titel ("ChefApp" + profielnaam `chefgroep-online` 10 px mono muted). Onder: statusfooter (zelfde als tray-statuslijn, 1 regel, Plex Mono 10 px).
- Harnas-filter blijft, maar wordt *group-filter*: kiezen in sidebar filtert canvas + palette-boost (boost_terms uit actieve groep).

### 4.3 Canvas — zones

Per domein één zone (herbruikbare `Zone`-component):

```
┌─ Zone header ─────────────────────────────────┐
│ Titel  ·  count  ·  freshness  ·  › open      │  hairline onder, 12 px header
├─ Cards (max 8 zichtbaar, rest via "toon meer")│  2-koloms grid waar het past
│  Card: titel · meta · status-stamp · shortcut│  hover → accent-border, geen shadow
└───────────────────────────────────────────────┘
```

- Cards: titel (semibold 13 px), meta (regular 11 px muted), stamp (mono 9 px pill: BEZIG/HULP/KLAAR/STIL), shortcut rechts (Plex Mono 10 px).
- Empty: `Leeg`-component ("Geen taken · maak er een" + CTA). Error/stale: amber/oranje banner + "stale sinds … — reden" (hergebruik W2 freshness-contract). Nooit leeg zonder uitleg.
- Detail-drawer: klik card → slide-drawer (300 px) met alle velden + acties (knoppen rij). `Esc` sluit drawer voor venster. Geen modal-overlay die focus steelt.
- Ferecency: recent geopende cards krijgen lichte boost + "Recent" chip (alleen visueel, geen extra sortering buiten ranking).

### 4.4 Search & ranking (één plek)

- Input behoudt single-source-of-truth (header). Palette-overlay hergebruikt dezelfde `palette.rs` pipeline.
- Ranking tiers (blijft): `contains (1000) > prefix (700) > gappy (500)`, binnen tier wint score + boost. Boosts nu: actieve groep (+150), pinned (+80), recent binnen 24 u (+60), lopende agents (+150). Nooit tier-doorbrekend.
- Synoniemen: `fleet=herdr|nodes`, `vault=accounts|commerce`, `share=sync|desktop` — kleine alias-map, geen LLM.
- Shortcuts: `/` focust (bestaand), `Cmd+K` / `Ctrl+K` alias, `↑↓` nav, `↵` run, `Esc` = drawer > palette > panel (stacked).

### 4.5 Toetsenbord & toegankelijkheid

- Volledig bedienbaar zonder muis. Focus-ring (2 px accent) altijd zichtbaar bij keyboard-nav.
- `Tab` door zones, `Enter` opent drawer, `Cmd+Enter` opent in browser, `Del` waar destructief (met confirm).
- Screen-reader: labels op zones/cards, status als tekst ("hulp nodig"), geen aria-hidden op interactieve elementen.

### 4.6 Beweging & density

- Fades 120 ms (`motion.rs` `PANEL_MS`), alleen open/dicht/drawer. Geen animatie tijdens poll-render (bestaande regel blijft).
- Density: default *comfortable* (bestaand). Compacte rij-modus (opt-in, setting) halveert card-padding voor power users — geen twee layouts onderhouden, alleen padding-token wisselt.

---

## 5. Tech-architectuur

### 5.1 Stack — blijft Rust + GTK

Geen herschrijving. GTK 3 blijft tot Wayland-beslissing rijp is (layer-shell als eigen PR). Wel: `panel.rs` uit elkaar trekken — de 1504 r monoliet wordt 5 modules.

```
src/
  main.rs            CLI + bootstrap (blijft dun)
  lib.rs             re-exports
  config.rs          EndpointProfile (uitbreiden: linearApi, vaultwardenUrl optioneel)
  policy.rs          EndpointPolicy (hosts uit profiel + allowlist)
  auth.rs            get_headers seam (bestaand, + OIDC-ready)
  http.rs            Client (ureq, geen redirects, bearer nooit op redirect)
  state.rs           actor (ritme uitbreiden: vault 5s, ops 15s, vault-extra 30s, linear 60s)
  models.rs          Snapshot + submodellen (8 domeinen → 8 secties)
  harness.rs         HarnessKind (4 → 9, met grouping)
  actions.rs         build_actions + RunSpec (uitbreiden per domein)
  palette.rs         fuzzy + RankContext (synoniemen, frecency)
  panel/
    mod.rs           Panel struct + lifecycle (was panel.rs)
    sidebar.rs       Sidebar widget (240px, groepen, dots)
    header.rs        SearchHeader (input + shortcuts)
    zones.rs         Zone<T> generiek + card-grid
    drawer.rs        DetailDrawer (slide, focus-trap)
    overlay.rs       PaletteOverlay (Super+Space fast-path)
  tray.rs            ksni (statuslijn + menu, ForceState blijft)
  ipc.rs             socket + UiCommand (uitbreiden: FocusDomain, PaletteToggle)
  css.rs             Signaal v2 tokens → GTK3-CSS (density-token)
  motion.rs          fade/timing
  doctor.rs          checks (per domein)
  panel_state.rs     persist active-group + query + drawer-state
  sessions.rs        recente sessies (boost-bron)
  notify.rs          coalesce + toast (mako/dunst-agnostisch)
  ops_cli.rs         joep-ops helpers
  // nieuw, klein:
  frecency.rs        recent-opened store ( capped 64, JSON, 30d TTL )
  aliases.rs         zoek-synoniemen map
```

Reden voor `panel/` split: lanes kunnen parallel zonder merge-conflict op één bestand.

### 5.2 State — snapshot groeit, ritme blijft simpel

```rust
// models.rs — Snapshot wordt domein-breed
pub struct Snapshot {
    // bestaand
    pub share_sync: HashMap<String, Value>,
    pub fleet: FleetInfo,
    pub health: HealthInfo,
    pub day_score: DayScore,
    pub providers: Vec<ProviderRow>,
    pub watcher_events: Vec<WatcherEvent>,
    // nieuw in 4.0 (alles Option/Vec met Default, tolerant parse)
    pub inbox: Vec<InboxItem>,           // unified D1
    pub fleet_nodes: Vec<FleetNode>,     // D2
    pub herdr_workspaces: Vec<HerdrWorkspace>, // D2
    pub vault_accounts: Vec<VaultAccount>,     // D3
    pub commander_tasks: Vec<CommanderTask>,   // D3+D7
    pub crm_deals: Vec<CrmDeal>,         // D3
    pub containers: ContainerDiff,       // D4  { observed, desired, drift }
    pub secrets_meta: Vec<SecretMeta>,   // D5  (alleen meta, geen values)
    pub clipboard: Vec<ClipboardEntry>,  // D6
    pub linear_issues: Vec<LinearIssue>, // D7
    pub kater_status: KaterStatus,       // D8
    pub observability: ObsSummary,       // D8
    pub last_poll_at: HashMap<String, String>, // freshness per sectie
}
```

Poll-ritme: vault (5 s, kritisch) + ops/herdr (15 s) blijven. Nieuw: `vault-extra` (accounts/providers/crm/containers/clipboard, 30 s) + `linear` (60 s, 429-vriendelijk) + `kater` (30 s). Alles fan-out met `FETCH_BUDGET_MS = 8_000` (bestaand), per endpoint timeout 2 s — mislukte sectie behoudt laatste goede waarde + zet `last_poll_at[sectie] = stale`.

`state.rs` `Poller` krijgt per-bron functie (`fetch_vault`, `fetch_ops`, `fetch_vault_extra`, `fetch_linear`, `fetch_kater`) — zelfde thread, sequentieel binnen budget, geen nieuwe threads.

### 5.3 Harness & actions — domeinen pluggen in

```rust
// harness.rs — 4 → 9 kinds, met group
pub enum HarnessKind {
    Inbox, Fleet, Herdr,
    Vault, Commerce, Crm,
    Share, Clipboard, Desktop,
    Tasks, Linear,
    Containers, Secrets,
    Kater, Health,
}
impl HarnessKind {
    pub fn group(&self) -> HarnessGroup { /* Fleet, Commerce, Sync, Work, System */ }
    pub fn prefixes(&self) -> Vec<&str> { /* per domein 3-5 prefixes */ }
}
```

`actions.rs`: per domein een `fn build_*_actions(snap, profile) -> Vec<Action>` + één `build_actions` die ze concat. Nieuwe `RunSpec` varianten:

```rust
pub enum RunSpec {
    // bestaand …
    // nieuw, telkens één variant per user-intent (geen god-variant)
    OpenLinearIssue(String),
    CopySecretMeta { id: String }, // alleen meta, value via vault-api copy-endpoint
    FleetDeploy { node: String },
    FleetExec { node: String, template: String },
    PrunePreview,
    FocusDomain(String),  // sidebar-nav via IPC/palette
    TogglePalette,        // overlay
}
```

Executor blijft één plek; `http::Client` policy per profiel-host. Clipboard-add met `policy.allow_clipboard_write()`-check (bestaand patroon).

### 5.4 IPC & hotkeys

Nieuwe `UiCommand` varianten: `FocusDomain(String)`, `TogglePalette`, `OpenInbox`. `Super+Space` blijft `TogglePanel` (panel) — palette-overlay is binnen panel (geen tweede venster, geen tweede socket). Optioneel tweede binding `Super+Shift+Space` → `TogglePalette` direct in palette-mode (zelfde venster, andere focus). `install.sh` registreert beide als GNOME custom-keybindings (`chefapp0`, `chefapp1`) idempotent.

### 5.5 Theming, motion, density

- `css.rs` krijgt `--chefbar-density` token (`comfortable` 14/16/12, `compact` 8/10/8). Setting in `panel-state.json` (`density: "comfortable"|"compact"`). Geen tweede stylesheet, alleen token-wissel.
- `motion.rs` ongewijzigd (fade 120 ms). Drawer slide 160 ms `ease-out`, alleen bij open/dicht.
- Dark default blijft; light via `detect_theme` (bestaand). Geen extra thema's.

### 5.6 Persistence & offline

- `panel_state.rs`: `active_group` (was `active_harness`), `query`, `drawer_open`, `density`, `recent_domains` (frecency, capped 20). Atomair JSON, tolerant load, 2 s debounce (bestaand).
- `frecency.rs`: `~/.local/share/chefbar/frecency.json` — `{ id, last_opened, open_count }` per action-id (hash van `title+run`), max 64, TTL 30 d, alleen lokaal (nooit naar server).
- Offline: elke zone leest `snap.last_poll_at[zone]` → "vers · 4 s" / "stale sinds 08:12 — vault 401". Nooit spinner zonder data.

### 5.7 Security & policy

- `EndpointProfile` uitbreiden met optioneel `linearApi`, `vaultwardenUrl` (env `CHEFBAR_LINEAR_API`, `CHEFBAR_VAULTWARDEN_URL`). Leeg = zone verborgen, geen error.
- `EndpointPolicy::with_profile_hosts` voedt automatisch nieuwe hosts. `auth::get_headers` blijft seam — Linear-token via `LINEAR_API_KEY` of `CHEFBAR_LINEAR_TOKEN` (zelfde pair-check als CF).
- Secrets: nooit waarden in snapshot, alleen meta. Copy via vault-api `POST /api/secrets/copy` (bestaand pattern) met audit-log. Geen plaintext in `~/.config`.

---

## 6. Swarm-plan — 7 lanes parallel, één merge-train

### 6.1 Branch & worktrees

- Base: `main` @ `d1a99b4`. Stack-root: `feat/chefapp-4.0`.
- 7 child-branches, elk vanaf `feat/chefapp-4.0`, elk in eigen worktree onder `~/ChefFactory/chefbar-worktrees/<lane>` of `/tmp` (vluchtig):
  - `feat/chefapp-4.0-lane-a-state`
  - `feat/chefapp-4.0-lane-b-actions`
  - `feat/chefapp-4.0-lane-c-panel-shell`
  - `feat/chefapp-4.0-lane-d-search`
  - `feat/chefapp-4.0-lane-e-tray-system`
  - `feat/chefapp-4.0-lane-f-style-motion`
  - `feat/chefapp-4.0-lane-g-tooling`
- Elke lane opent eigen PR met base `feat/chefapp-4.0` (stacked PRs). Merge-volgorde = dependency-volgorde (zie 6.3). Squash per lane.

### 6.2 File-disjoint matrix (harde grens — geen overlap)

| Lane | Titel | Bezit (mag editen) | Raakt níet |
|---|---|---|---|
| **A — State** | Snapshot & actor | `src/models.rs`, `src/state.rs`, `src/sessions.rs` (+ nieuw `src/frecency.rs` aanmaken) | panel, actions, tray, css, palette |
| **B — Actions** | Domein-acties | `src/actions.rs`, `src/harness.rs`, `src/policy.rs`, `src/auth.rs`, `src/http.rs`, `src/ops_cli.rs`, nieuw `src/aliases.rs` | panel, tray, css, state |
| **C — Panel shell** | Venster & zones | `src/panel.rs` → `src/panel/*` (mod/sidebar/header/zones/drawer/overlay), `src/panel_state.rs` | models, actions, palette, tray, css (alleen class-namen gebruiken) |
| **D — Search** | Ranking & palette | `src/palette.rs`, `src/panel/header.rs` (alleen search-deel, gecoördineerd met C via `header.rs` ownership — D bezit ranking-logica, C bezit widget-layout) | models, actions, tray, css |
| **E — Tray & systeem** | Tray/IPC/service | `src/tray.rs`, `src/ipc.rs`, `src/notify.rs`, `src/doctor.rs`, `install.sh`, `config/*.service`, `config/mako/*`, `config/dunst/*` | panel, palette, css, models |
| **F — Style & motion** | Tokens & animatie | `src/css.rs`, `src/motion.rs`, `assets/` indien nodig | panel-logica, state, actions, tray (alleen CSS-klassen consumeren) |
| **G — Tooling** | QA & docs | `scripts/*`, `.github/workflows/ci.yml`, `docs/*`, `README.md`, `CONTRIBUTING.md`, `Cargo.toml` (alleen dev-deps/scripts), `tests/` | src/ behalve via review-fixes |

Coördinatie-regels:
- `src/lib.rs` en `src/main.rs` zijn **bevroren** tijdens lanes — alleen stack-root mag ze aanraken (één commit: nieuwe modules registreren). Lanes importeren via `crate::` en falen niet als module nog niet geregistreerd is — ze schrijven de file, root wiret hem.
- `Cargo.toml` alleen via G (deps) of met expliciete ping in PR-body.
- `panel/header.rs` is gedeeld tussen C en D → contract: C bezit widget-boom (`SearchEntry`, layout, signals), D bezit `RankContext`/`fuzzy_score`/`aliases` en levert `fn rank(query, ctx) -> Vec<Action>`. D raakt geen GTK-widgets.

### 6.3 Dependency-volgorde (merge-train)

```
Fase 0 — Root (1 commit, direct op feat/chefapp-4.0)
  └─ Maak src/panel/ dir + mod.rs stub, registreer leeg in lib.rs/main.rs,
     zet feature-flag `chefapp_4_0 = true` (compile-gate voor nieuwe snapshot-velden).

Fase 1 — Parallel (geen onderlinge deps, direct na Fase 0):
  Lane A (State) ─┐
  Lane F (Style) ─┤ alle 7 kunnen starten zodra Fase 0 groen is
  Lane G (Tooling)┘ (G hoeft niet te wachten; schrijft alleen scripts/docs)

Fase 2 — Hangt van A af:
  Lane B (Actions) wacht tot A zijn Snapshot-velden heeft gepubliceerd
    (B kan wel al harness-kinds + alias-map voorbereiden op A::draft-types).

Fase 3 — Hangt van A+B af:
  Lane C (Panel shell) wacht tot A+B types er zijn (rendert zones/cards/drawer).
  Lane D (Search) wacht tot B zijn alias-map + nieuwe keywords heeft.
  Lane E (Tray) wacht tot B zijn nieuwe UiCommand varianten heeft.

Merge-train: A → F,G → B → C,D,E  (F en G kunnen tussendoor, B blokkeert C/D/E)
Queue: lokale test-gate per lane vóór push (zie 6.5).
```

Praktisch: start alle 7 lanes tegelijk, maar C/D/E doen eerst hun file-splits zonder domein-data (skeleton) en vullen pas na A+B green. Geen idle wachten.

### 6.4 Per-lane contract (wat "done" betekent)

**Lane A — State** — `~400 r` toegevoegd, geen UI.
- `Snapshot` uitgebreid met 9 nieuwe velden (Default, tolerant parse, geen panic).
- `Poller` met `fetch_vault_extra` (30 s), `fetch_linear` (60 s), `fetch_kater` (30 s) + `last_poll_at` map + freshness-reason.
- `sessions.rs` + nieuw `frecency.rs` (64 cap, 30 d TTL, atomair JSON, tests).
- Tests: parse-tolerantie per nieuw veld, offline-fallback, TTL-eviction. `cargo test` groen.

**Lane B — Actions** — `~500 r` toegevoegd.
- `HarnessKind` 4→9 (+ group), `prefixes()` per domein, `group()` helper.
- `build_actions` split per domein (`build_inbox_actions`, `build_fleet_actions`, …) + concat, elk pure functie.
- 6 nieuwe `RunSpec` varianten + executor-armen (policy-checked, geen netwerk op UI-thread).
- Nieuw `aliases.rs` (synoniem-map 20 regels, tests).
- Tests: harness-prefix matching, alias-expansion, RunSpec-determinisme, policy-weigering.

**Lane C — Panel shell** — grootste lane, `panel.rs` 1504 r → `panel/` 5 files.
- `panel/mod.rs` (lifecycle), `sidebar.rs` (240 px, groepen, dots), `header.rs` (search-entry), `zones.rs` (generieke Zone + card-grid + empty/error), `drawer.rs` (slide + focus-trap), `overlay.rs` (palette-overlay).
- Vaste geometrie behouden (860×880, `set_resizable(false)`), density-token via css-klas.
- Frecency + panel_state wiring (active_group, drawer-state).
- Tests: widget-smoke in Xvfb (bestaand `visual-shot.sh` hergebruikt), geen netwerk.

**Lane D — Search** — `~200 r`.
- `palette.rs`: `RankContext` uitgebreid (active_group boost, frecency boost, pinned), `aliases::expand_query`, tier-invariant behouden.
- `header.rs` ranking-call blijft dun (D levert `rank()`, C roept aan).
- Tests: tier-invariant (contains nooit onder prefix), alias-expansion, frecency-boost, determinisme.

**Lane E — Tray & systeem** — `~300 r` + shell.
- `tray.rs`: statuslijn 3→5 regels (Inbox-count), group-dots, menu uitgebreid (FocusDomain).
- `ipc.rs`: `FocusDomain`, `TogglePalette`, `OpenInbox` + parse + `Acquire` ongewijzigd.
- `doctor.rs`: per-domein checks (vault/ops/linear/kater), exit-codes 0/1/2, IPC-first.
- `notify.rs`: inbox-coalescing (bestaand) + per-domein ernst.
- `install.sh` + units: tweede hotkey `Super+Shift+Space` → palette, idempotent, `shellcheck` groen.

**Lane F — Style & motion** — `~150 r` + tokens.
- `css.rs`: density-token (`comfortable`/`compact`), drawer/overlay klassen, sidebar-group styles, zone-header/card-grid styles. Geen nieuwe kleuren — alleen spacing/radius hergebruik.
- `motion.rs`: drawer-slide 160 ms, palette-fade 100 ms, geen poll-animatie.
- Visueel proof: `visual-shot.sh` accent-assert blijft groen, nieuwe screenshot-diff warning-only.

**Lane G — Tooling** — geen src-logica, wel gates.
- `scripts/visual-shot.sh` uitgebreid (palette-overlay shot, drawer shot, density-toggle shot).
- `.github/workflows/ci.yml`: harde `fmt`+`clippy` gates blijven, `visual` job met Xvfb (warning-only → hard na 2 groene runs), `shellcheck` voor `install.sh`.
- `docs/plan-full-chefapp.md` (dit doc) + `docs/chefapp-qa.md` (handmatige checklist: Super+Space <300 ms, één venster, offline-banner, tray-glyph).
- `README.md` + `CONTRIBUTING.md` bijgewerkt (no-local-build regel blijft).

### 6.5 Gates (per lane, vóór merge)

Elke lane moet lokaal groen zijn — **op de runner**, nooit laptop:

```bash
ssh chef@chef-runner-01-1 'cd ~/chefbar-check && git fetch && git checkout feat/chefapp-4.0-lane-<x> && export PATH=$HOME/.cargo/bin:$PATH && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets && cargo build --release'
```

Plus lane-specifiek: E → `shellcheck install.sh`, C/F → `scripts/visual-shot.sh` (Xvfb), G → `scripts/ci-local.sh` (schema/compat waar van toepassing).

CI is notify-first (geen poll-loops). PR's squash-mergen naar `feat/chefapp-4.0`; `feat/chefapp-4.0` squash-merget naar `main` zodra alle 7 lanes groen + P1 latch clear + independent review op head SHA.

### 6.6 Swarm-prompt (voor agents)

Bestand: `~/.agents/prompts/chefapp-4.0-lane.md` (of `.cursor/prompts/` mirror). Inhoud per lane-agent:

```
Je bent Lane <X> van ChefApp 4.0. Repo: GroepOnline/chefbar (checkout ~/ChefFactory/chefbar).
Base: feat/chefapp-4.0 (pull --rebase eerst). Jouw branch: feat/chefapp-4.0-lane-<x>.
Worktree: ~/ChefFactory/chefbar-worktrees/chefapp-<x> (maak met git worktree add).
Scope: alleen files uit jouw matrix (zie docs/plan-full-chefapp.md §6.2). Raak niets anders aan.
Regels: geen Rust-build op laptop joep — alles op chef@chef-runner-01-1. fmt+clippy hard.
Push SHAs terug. Eén PR met base feat/chefapp-4.0. Geen force-push naar main.
Done = gates groen + file-disjoint gerespecteerd + tests + visual waar relevant.
```

Dirigent start 7 agents met die prompt + lane-specifieke subprompt (contract uit 6.4). Voortgang via `swarm status` + `fleet-ssh` naar runner voor build-logs.

---

## 7. Acceptatie — wanneer 4.0 "af" is

### 7.1 Functioneel

1. `Super+Space` opent binnen 300 ms, precies één venster, ongeacht hoe vaak/snel. `Esc` sluit drawer → palette → panel (stacked).
2. Zoeken vindt in elk domein; ranking voelt als "wat ik net aanraakte staat bovenaan" (tier-invariant bewezen in tests).
3. Elke zone toont verse data of "stale sinds <t> — reden" (nooit leeg zonder uitleg). Offline = laatste goede waarde + banner.
4. Tray: max 5 live regels + statuslijn, glyph `stil/bezig/hulp/fout/offline` klopt, menu-items werken (incl. FocusDomain).
5. Inbox bundelt blocked/hulp/down in één geordende lijst; snooze/pause werken.
6. Fleet toont nodes + containers diff; Vault toont accounts/providers/CRM; Linear toont assigned-to-me — alles read-only behalve waar RunSpec expliciet schrijft (clipboard/task-create).
7. Secrets alleen meta; copy via vault-api met audit, auto-clear.
8. `chefbar --doctor` consistent tussen kale shell en service-env, per domein, exit 0/1/2.

### 7.2 Technisch

- `cargo fmt --check` + `clippy --all-targets -- -D warnings` groen op runner + CI.
- `cargo test --all-targets` groen (incl. nieuwe domein-parsers, ranking-tiers, harness-prefixes, frecency-TTL).
- `scripts/visual-shot.sh` groen (accent-assert) + screenshot-diff warning-only (geen regressie >2% pixels zonder review).
- `shellcheck install.sh` groen.
- Single-instance e2e: twee processen → één venster.
- Geen tweede poll-loop, geen tweede socket, geen Electron, geen plaintext secrets in `~/.config`.

### 7.3 Volgorde na 4.0 (bewust níet in 4.0)

- **4.1 Schrijven**: Herdr exec/logs tail, Linear mutate, container prune/restart (met confirm), share push/pull.
- **Wayland layer-shell**: eigen PR met fallback-bewijs, pas als runner `libgtk-layer-shell-dev` heeft.
- **OIDC**: via `auth::get_headers` seam zodra `auth.chefgroep.online` (Authentik) live is.
- **WebKit-embed** (optioneel): alleen als native GTK echt tekort schiet voor één domein — expliciet besluit, geen sluiproute.
- **Mobile companion**: pas na laptop 4.0 stabiel.

---

## 8. Volgende stap — go of bijsturen

Dit plan is **uitvoer-klaar**. Met groen licht:

1. Maak `feat/chefapp-4.0` vanaf `main` + Fase-0 stub-commit.
2. Start 7 lane-agents parallel (worktrees + swarm-prompt).
3. Merge-train A → F,G → B → C,D,E, elk squash, gates groen, review op head SHA.
4. `feat/chefapp-4.0` → `main` squash-merge, tag `4.0.0`, CI artifact `chefbar-release`, `install.sh --systemd` op `joep`.

Zeg **"go"** en de dirigent start de swarm. Wil je eerst één domein snijden (bijv. alleen Inbox+Fleet in 4.0), dan schrappen we D4/D5/D7 uit Fase 1 zonder het plan te herschrijven — de lanes blijven file-disjoint, alleen B/C krijgen minder `build_*_actions`.

---

*Bijlage — bronnen: `README.md` (3.1), `docs/plan-volledig-werkend.md` (W1–W5, D1–D6), `docs/roadmap.md` (verscheept vs uitgesteld), `src/*.rs` (7 454 r), `Cargo.toml`, `install.sh`, `FLEET.md`, `chefgroep-vault` monorepo (packages/core+server, frontend), `chefgroep-os` (fleet/docker), `kater-dev-tools` (gateway).*
