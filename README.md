# ChefBar 3.0

Mission-control assistant voor ChefGroep OS — **Rust native** (GTK3 + ksni tray), in de
Devin v2 designtaal (`~/design-system/tokens.css`, warme bg, accent `#317CFF`/`#5C97FF`,
GTK3-geschoonde CSS-subset, dark default).

## Wat het doet

Eén poll-actor (`state.rs`) voedt een gedeeld snapshot; tray, app-window en command-bar
delen dat beeld. Acties zijn declaratieve data (`actions.rs`), uitvoer loopt door één
executor met policy-begrensde HTTP-clients (`http.rs` + `policy.rs`).

- **App-window** — sidebar met live harness-nav (Fleet / Commerce / Sync / Evaluatie),
  header-zoekveld (één source of truth), content-paneel dat per harness filtert en
  per poll opnieuw rendert.
- **Harness-filtering** — acties matchen op harnas via keyword-prefixes (`harness.rs`),
  met statuskleuren per harnas.
- **Zoeken** — `/` focust het veld, `Esc` verbergt het venster, typen filtert de hele
  surface (Raycast-geest).
- **Tray + IPC** — ksni-tray met command-menu; externe commando's naar een draaiende
  instantie via Unix-socket (`--ipc panel|bar|refresh|doctor|quit`).
- **Doctor** — `chefbar --doctor`: profiel, policy, secrets (alleen fingerprints),
  watchdog; resultaat ook als desktop-melding.
- **Serve** — `chefbar --serve`: actor-only poll-loop (vault 5s, ops 15s), geen UI.

## CLI

```
chefbar                              app (GTK)
chefbar --doctor                     doctor-checks en afsluiten
chefbar --serve                      actor-only (poll-loop)
chefbar --ipc panel|bar|refresh|doctor|quit
chefbar --show-config                profiel + policy-summary (geen secrets)
chefbar --profile <pad>.json         endpoint-profiel (of CHEFBAR_ENDPOINT_PROFILE)
chefbar --version
```

## Configuratie

Endpoint-profiel = SSOT (`config.rs`): `name`, `vaultApi`, `opsApi`, `dashboard`,
`desktop`, `opencodexDashboard`, `katerWorkspace`. Veld voorbeelden in
`config/endpoints.example.json` en `config/endpoints.tailnet.example.json`.

- `CHEFBAR_ENDPOINT_PROFILE=<pad>` — profielpad (default `~/.config/chefbar/endpoints.json`).
- `CHEFBAR_*` env-warden — overschrijven per veld (bijv. `CHEFBAR_VAULT_API`); env wint.

## Installatie

```bash
./install.sh                 # binary → ~/.local/bin/chefbar + endpoints-profiel
./install.sh --systemd       # + systemd-user-unit, Super+Space-hotkey (→ chefbar --ipc bar)
./install.sh <pad/binary>    # eigen build/artifact
```

De unit (`chefbar.service`) draait als user-unit voor de ingelogde GUI-gebruiker;
IPC-socket op `$XDG_RUNTIME_DIR/chefbar.sock`.

## Development

- **Geen lokale Rust-builds op de laptop** (learned rule) — CI is notify-first:
  `.github/workflows/ci.yml` draait `cargo test --all-targets` + `cargo build --release`
  op de self-hosted runner en uploadt het release-artifact.
- `#[test]`-modules inline in `config`, `palette`, `models`, `motion`, `harness`, `ipc`,
  `policy`, `sessions`.
- Stack: GTK3 (`gtk 0.18`), `ksni` tray, `ureq` HTTP, declaratieve actions.