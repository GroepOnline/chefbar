# ChefBar 3.2

Mission control aan je menubalk. Eén venster, één actor, alle harnassen.

ChefBar is de native assistent voor ChefGroep OS. Rust, GTK3, ksni-tray. Één poll-actor voedt één snapshot dat tray, venster en command-bar delen. Geen floating hacks, geen Electron, geen tweede poll-loop.

Gebouwd in de Devin-v2 taal. Warme achtergrond, scherp accent, dark default. Strak genoeg voor dagelijks gebruik, rustig genoeg om te blijven staan.

## Installeren

Eén commando. Geen vragen.

```bash
./install.sh                 # binary naar ~/.local/bin/chefbar + voorbeeldprofiel
./install.sh --systemd       # + user-unit + Super+Space → chefbar --ipc bar
./install.sh /pad/naar/chefbar  # eigen build of CI-artifact
```

Wat `install.sh` doet:

* Kopieert de binary naar `~/.local/bin/chefbar` (root: `/usr/local/bin`).
* Plaatst `config/endpoints.example.json` naar `~/.config/chefbar/endpoints.json` als die nog niet bestaat. Bestaat hij al, dan blijft hij onaangeroerd.
* Met `--systemd`: installeert `chefbar.service` als user-unit, zet Super+Space als global hotkey en start de service. Herinstallatie is idempotent.

Vereisten: GTK3 runtime, display (`DISPLAY` of `WAYLAND_DISPLAY`), Rust toolchain alleen voor CI. Op de laptop bouw je niet lokaal.

Controleren:

```bash
chefbar --doctor
chefbar --show-config
```

## Wat het doet

| Vlak | Gedrag |
| --- | --- |
| **App-venster** | Sidebar met live harnas-navigatie (Fleet, Commerce, Evaluatie, Sync), header-zoekveld als enige bron van waarheid, content-paneel dat per harnas filtert en per poll opnieuw rendert. Undecorated, drag op header, Escape verbergt. |
| **Zoeken** | `/` focust, typen filtert de hele surface, geen aparte modi. Ranking kiest: recency-boost uit sessies die om jou vragen en lopende agents (`RankContext` in `src/palette.rs`). Raycast-geest, geen Raycast-kopie. |
| **Harnas-filtering** | Acties matchen op harnas via keyword-prefixen (`src/harness.rs`). Statuskleuren per harnas, geen generieke badges. |
| **Tray + IPC** | ksni-tray met command-menu. Externe commando's via Unix-socket op `$XDG_RUNTIME_DIR/chefbar.sock`. Hotkey en scripts praten tegen een draaiende instantie, niet tegen een tweede proces. |
| **Meldingen** | Watcher-transities gecoalesceerd tot hooguit één toast per poll-cyclus (`coalesce_toasts` in `src/models.rs`). Per-agent dempen via paneel-rij of tray-submenu (`src/mutes.rs`); rustige uren via `CHEFBAR_QUIET` dempen niet-kritieke toasts (`src/quiet.rs`). FOUT gaat altijd door, inbox blijft gevuld. Geen ticker, geen storm. |
| **Panel-state** | Laatste harnas + zoekterm bewaard in `~/.config/chefbar/panel-state.json` (`src/panel_state.rs`). Heropenen zonder verrassingen. |
| **Doctor** | `chefbar --doctor` beoordeelt profiel, policy, credentials (alleen fingerprints), watchdog, laatste poll én latency-probes per endpoint (vault/ops, ms). Exit 0 bij OK, anders 1. Ook als desktop-melding + tray-tooltip. |
| **Serve** | `chefbar --serve` draait alleen de actor. Poll-ritme vault 5s, ops 15s. Geen UI, zelfde snapshot. |

Acties zijn declaratieve data (`src/actions.rs`). Uitvoer loopt door één executor met policy-begrensde HTTP-clients (`src/http.rs` + `src/policy.rs`). UI-threads doen geen netwerk.

## CLI

```
chefbar                              # app (GTK)
chefbar --doctor                     # checks en afsluiten
chefbar --serve                      # alleen actor
chefbar --ipc panel|bar|refresh|doctor|quit
chefbar --show-config                # profiel + policy-summary, geen secrets
chefbar --profile /pad/endpoints.json
chefbar --version
```

IPC-aliasen: `panel`, `bar`, `toggle-panel`, `dashboard`, `open`, `show` → TogglePanel. `refresh` / `reload` → Refresh. `doctor` / `check` → Doctor. `quit` / `exit` / `stop` → Quit. `mute <agent-key>` → per-agent dempen (nogmaals = ontdempen).

## Eén quick-command-overlay

**Super+Space** opent ChefBar. **Alt+Space** (`chef-hud`) is een oudere
rofi-lookalike met vrijwel dezelfde header — hij leest als een "tweede
ChefBar". Keuze: chef-hud **retireren** (binding verwijderen) of herbinden.
`chefbar --doctor` wijst je erop zolang hij nog aanwezig is.

## Configuratie

### Endpoint-profiel is SSOT

Één JSON-bestand bezit elk netwerkvlak. Velden in `camelCase`:

```json
{
  "name": "chefgroep-online",
  "vaultApi": "https://vault-api.chefgroep.online/api",
  "opsApi": "https://ops.chefgroep.online",
  "dashboard": "https://vault.chefgroep.online",
  "desktop": "https://desktop.chefgroep.online",
  "opencodexDashboard": "https://opencodex.chefgroep.online",
  "katerWorkspace": "https://kater.chefgroep.online/agents/"
}
```

Voorbeelden: `config/endpoints.example.json` (productie via `*.chefgroep.online`) en `config/endpoints.tailnet.example.json` (lab of bypass, expliciet als optioneel gemarkeerd).

Pad-resolutie:

1. `--profile /pad.json` wint altijd.
2. `CHEFBAR_ENDPOINT_PROFILE=/pad.json` als fallback.
3. Anders `~/.config/chefbar/endpoints.json`.

Zie `src/config.rs:load_profile`, `default_profile_path`.

### CHEFBAR_* wint per veld

Env overschrijft het profiel per veld, niet als geheel. Dit is de warden-laag voor fleet, CI en lokale overrides zonder het JSON-bestand te herschrijven.

| Profielveld | Env | Default (local) |
| --- | --- | --- |
| `name` | `CHEFBAR_PROFILE_NAME` | `local` |
| `vaultApi` | `CHEFBAR_VAULT_API` | `http://127.0.0.1:8321/api` |
| `opsApi` | `CHEFBAR_OPS_API` | `http://127.0.0.1:10101` |
| `dashboard` | `CHEFBAR_DASHBOARD` | `http://127.0.0.1:8080` |
| `desktop` | `CHEFBAR_DESKTOP` | `http://127.0.0.1:3000` |
| `opencodexDashboard` | `CHEFBAR_OPENCODEX_DASHBOARD` | — |
| `katerWorkspace` | `CHEFBAR_KATER_WORKSPACE` | — |
| poll vault | `CHEFBAR_VAULT_POLL_MS` | `5000` |
| poll ops | `CHEFBAR_OPS_POLL_MS` | `15000` |
| demp-lijst | `CHEFBAR_MUTED_AGENTS` | `~/.config/chefbar/muted-agents.json` |
| rustige uren | `CHEFBAR_QUIET` | uit (bijv. `22:00-07:00`) |

Lege env-waarden worden genegeerd. Ongeldige URL's vallen terug op de default (zie `clean_url` in `src/config.rs`). Optionele velden blijven `None` als ze leeg zijn.

Profiel inspecteren zonder secrets te tonen:

```bash
chefbar --show-config
# profiel  chefgroep-online
# vault    vault-api.chefgroep.online
# ops      ops.chefgroep.online
# kater    kater.chefgroep.online
```

## Remote en auth

ChefBar praat met private `*.chefgroep.online` origins over HTTPS achter Cloudflare Access. Tailscale is bypass voor lab, nooit vereist voor product.

Seam: `chefbar.auth.get_headers()` bouwt request-headers per call. Zie `docs/auth-remote.md` voor de volledige matrix.

Kort:

* **Vault bearer** via `CHEF_VAULT_API_TOKEN` of `CHEFBAR_VAULT_TOKEN`, of via bestand `CHEFBAR_VAULT_TOKEN_FILE`. Fallback leest `~/ChefFactory/chefgroep-vault/docker/.env`.
* **Cloudflare Access service token** via `CF_ACCESS_CLIENT_ID` + `CF_ACCESS_CLIENT_SECRET` (of `CHEFBAR_CF_*` aliassen). Beide headers alleen als het paar compleet is.
* Toekomstige OIDC access tokens landen op dezelfde seam. Clients hoeven niet opnieuw ontworpen te worden.

Policy (`src/policy.rs`): alleen HTTPS naar profiel-hosts, `*.chefgroep.online`, expliciete allowlist of `*.ts.net`. Loopback altijd toegestaan. Tailnet CGNAT `100.64.0.0/10` alleen voor HTTP als `CHEFBAR_ALLOW_TAILNET_HTTP` aan staat. Bearer-tokens volgen nooit redirects. `safe_join` blijft same-origin.

Diagnose per profiel-target: `chefbar --doctor` rapporteert DNS, TLS, allowlist en auth zonder secrets te echoen. Alleen `sha256[:12]` fingerprints waar nodig.

## Service

`chefbar.service` is een systemd user-unit. Geen `User=` (user-units weigeren die), `PartOf=graphical-session.target`, `Restart=always`.

```
ExecStart=%h/.local/bin/chefbar
Environment=CHEFBAR_ENDPOINT_PROFILE=%h/.config/chefbar/endpoints.json
RuntimeDirectory=chefbar          # → $XDG_RUNTIME_DIR/chefbar
IPC-socket:  $XDG_RUNTIME_DIR/chefbar.sock (0600)
```

Logs: `journalctl --user -u chefbar.service -f` \
File-log: `~/.local/state/chefbar/chefbar.log` (of `CHEFBAR_LOG`, getoond door `chefbar --doctor`)

## Development

ChefBar bouwt niet lokaal op de laptop. CI is notify-first en draait op de self-hosted runner. Zie `.github/workflows/ci.yml`.

```yaml
runs-on: [self-hosted, Linux, X64, company-control]
steps:
  - cargo test --all-targets
  - cargo build --release
  - upload artifact: target/release/chefbar
```

Lokaal telt alleen `cargo test` als je de toolchain al hebt (de laptop heeft die
bewust niet — bouw/valideer op de runner). De QA-gate draait op de self-hosted
runner (`chef-runner-01-1`) en bestaat uit `scripts/gate.sh`: fmt · clippy
(`-D warnings`) · tests. Release-artifacts komen uit CI.

| Onderdeel | Waar |
| --- | --- |
| Poll-actor | `src/state.rs` (vault 5s, ops 15s, budget 8s, shared `Snapshot`) |
| Acties + executor | `src/actions.rs`, `src/palette.rs` |
| Harnas-filtering | `src/harness.rs` |
| HTTP + policy | `src/http.rs`, `src/policy.rs` |
| Auth-headers | `src/auth.rs` |
| IPC | `src/ipc.rs` (Unix-socket, `UiCommand`) |
| Tray | `src/tray.rs` (ksni, `UiCommand::TogglePanel` enz.) |
| Panel | `src/panel.rs` (GTK3, sidebar 220px, search, cards) |
| Styling | `src/css.rs` (Devin-tokens → GTK3-subset, dark/light) |
| Doctor | `src/doctor.rs` |
| Panel-state | `src/panel_state.rs` (harnas + zoekterm, atomair JSON) |

Tests zitten inline per module (`#[cfg(test)]` in `actions`, `config`, `palette`, `models`, `motion`, `harness`, `ipc`, `policy`, `sessions`, `panel_state`, `state`, `tray`, `mutes`).

Stack: `gtk 0.18`, `ksni 0.2`, `ureq 2` (json, geen redirects), `clap 4`, `url 2`, `serde_json 1`.

## Problemen oplossen

```bash
chefbar --doctor          # profiel, policy, secrets, watchdog, IPC, netwerk
chefbar --show-config     # welk profiel echt actief is
journalctl --user -u chefbar.service -n 100 --no-pager
ls -l $XDG_RUNTIME_DIR/chefbar.sock
```

Veelvoorkomend:

* Profiel heet niet `local` maar vault staat nog op `127.0.0.1` → doctor faalt. Zet `name` of endpoints recht.
* Policy weigert URL → host staat niet in profiel, allowlist of `*.chefgroep.online`. Controleer `CHEFBAR_HTTPS_ALLOWLIST` of profiel.
* Tray toont niets → actor offline. Kijk of `vault_online` false is en of credentials aanwezig zijn (`auth_status` in doctor).
* Hotkey doet niets → `gsettings` custom-keybinding `chefbar0` controleren, `Super+Space` is vrijgegeven van input-source switching door `install.sh`.

## Roadmap 3.1 — verscheept

Zie `docs/roadmap.md` voor detail. Samenvatting van wat 3.1 bracht:

* **Zoeken dat kiest** — ranking op recente sessies en lopende agents, geen platte filter. Snel naar wat je net aanraakte.
* **Rustige meldingen** — coalescing tot één toast per cyclus, alleen bij echte statusovergang (blocked, hulp nodig). Geen ticker.
* **Panel dat onthoudt** — laatste harnas en zoekterm bewaard over herstart heen, geen verrassingen bij heropenen.

## Roadmap 3.2 — verscheept

Zie `docs/plan-optimalisatie-uitbreiding.md` voor detail. Wat 3.2 bracht:

* **Sneller bij de les** — vaste worker-pool (geen thread-churn meer, `src/state.rs`), revisie-check die dure re-renders overslaat, en een **lazy panel**: de GTK-UI wordt pas bij de eerste `Super+Space` gebouwd (start→tray <500ms; bouwtijd staat in `chefbar.log`). Release-build met `panic = abort` + `strip` + `lto`.
* **Meer oppervlak** — Herdr-agents met pane/focus + inline prompt, Commander-queue met per-taak Stop, toetsenbord-first (↑/↓ + Enter + Ctrl+K).
* **Per-agent dempen + rustige uren** — demp één agent in het paneel of tray-submenu (`mute` via `--ipc`); `CHEFBAR_QUIET="22:00-07:00"` dempt buiten werktijd alle niet-kritieke toasts (FOUT gaat door; `--doctor`/`--show-config` tonen het venster).
* **Deep-links in sessie-rijen** — evidence/workspace/browser/kater-links als knoppen naast de primaire actie in de aandacht-sectie.
* **P4-metingen** — `scripts/measure-p4.sh` meet binary, start→gereed (**57ms**), start→panel (**64ms**) en RSS (78–93 MiB) op de release-build; alle doelen gehaald.
* **Doctor kijkt mee** — latency-probes per endpoint (vault/ops), draait in de achtergrond vanaf tray, resultaat ook in de tray-tooltip.
* **Config 3.2** — poll-intervallen per env (`CHEFBAR_VAULT_POLL_MS`/`CHEFBAR_OPS_POLL_MS`), demp-lijst via `CHEFBAR_MUTED_AGENTS`; `chefbar.log` voor actor/executor-fouten.
* **QA** — tray-menu uit de ksni-closures naar een pure testbare builder, state-mock-tests zonder netwerk, golden CLI-tests (waaronder `--ipc mute`), 80+ tests groen op de runner en in CI.

Bewust uitgesteld: Wayland layer-shell (eigen change met CI-afhankelijkheid) en OIDC via de `get_headers` seam (wacht op `auth.chefgroep.online`).

Scope blijft strak. Geen tweede bar, geen tweede daemon, geen tweede waarheid. Eén profiel, één actor, één venster.

## Licentie

MIT. Zie `Cargo.toml`.
