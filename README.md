# ChefBar 3.0

Mission-control tray voor ChefGroep OS (GNOME Shell / Wayland), in de
"Signaal · Huly"-designtaal (`.ulpi/design/DESIGN.md`).

## Stackbeslissing

**GTK3 + AyatanaAppIndicator3 in één proces**, met een rijk gestyled
`Gtk.Window` als command-bar en panel.

| Optie | Keuze |
|-------|--------|
| Tray | Ayatana AppIndicator (bewezen op deze host + AppIndicator-extensie) |
| Palette | GTK3 window + CSS provider, fuzzy action registry |
| Panel | Compacte status + accounts + agents + recente feed |
| GTK4 / libadwaita | **Niet** in-process — `gi.require_version` kan GTK3 en GTK4 niet mixen |

De bestaande tray-integratie, IPC-hotkey (`Super+Space`) en systemd-unit blijven.
Een parallelle Electron/Tauri-app zou koude start en dubbele indicators kosten.

## Wat de redesign doet

1. **Command palette core** — één input, fuzzy ranking (`palette.py`), pijltjes/Enter/Escape, mono shortcuts.
2. **Vault-API als control plane** op `127.0.0.1:8321`:
   - accounts: `GET /api/accounts/overview` + `POST /api/coding/accounts/switch` (Idempotency-Key + expectedRevision)
   - commander: create / list / cancel
   - clipboard: list / add / delete-row
   - desktop: status / start / stop
   - share-sync: status / pull / push
   - status / fleet / agents + `GET /api/agents/events` als recente feed
3. **Agent-interactief** — watcher-suggesties, recente events, snelle "stuur taak naar Commander".
4. **Signaal · Huly tokens** — Inter interface, IBM Plex Mono display/data; radius 12/8/14; void OLED donker als standaard.

CRM/deals blijft bewust buiten deze app.

## Tray

Icoonstates volgen `.ulpi/design/chefbar-tray.md`. Regenereer PNG's met:

```bash
python3 build_icons.py
python3 build_icons.py --check
```

## Vereisten

- `gir1.2-ayatanaappindicator3-0.1` + AppIndicator GNOME-extension (Wayland)
- Optioneel `CHEF_VAULT_API_TOKEN` (verplicht voor accountswitch)
- Vault-API op `:8321` (override: `CHEFBAR_VAULT_API`)
- joep-ops op `:10101` (override: `CHEFBAR_OPS_API`)

## Installatie

```bash
./install.sh
```

```bash
chefbar                # app starten (tray, panel, command-bar)
chefbar --show-config  # profiel tonen
chefbar --ipc bar      # command-bar toggle in de draaiende instantie
```

De vlaggen hierboven gelden voor het Rust binary; de oude Python-vlaggen
(`--show-panel`, `--bar`) bestaan niet meer. Zie "Rust build" hieronder.

## Tests

```bash
cd apps/chefbar
PYTHONPATH=. python3 -m unittest discover -s tests -v
python3 -m py_compile chefbar/*.py
```

## Rust build

ChefBar draait op een Rust-kern (`src/`, `Cargo.toml`); de Python-versie in
`chefbar/` blijft als referentie. Bouwen en testen gebeurt op chef-runner-01
of in CI (`.github/workflows/ci.yml`), niet op de laptop:

```bash
cargo test
cargo build --release
```

Het binary staat na een release-build in `target/release/chefbar`. CI draait
op een self-hosted company-control runner en uploadt het artifact
`chefbar-release`.

### Installatie

```bash
./install.sh            # binary + endpoints-profiel
./install.sh --systemd  # + systemd-user-unit, start + Super+Space hotkey
```

Het binary gaat naar `~/.local/bin/chefbar` (bij root naar `/usr/local/bin`).
Endpoints staan in `~/.config/chefbar/endpoints.json` (voorbeeld:
`config/endpoints.example.json`) of via `CHEFBAR_ENDPOINT_PROFILE`.
`CHEFBAR_*` omgevingsvariabelen lopen direct door naar het binary en winnen
per veld van het JSON-profiel.

### CLI

```bash
chefbar                   # app: tray, panel, command-bar
chefbar --doctor          # gezondheidscheck (exit 0 bij ok)
chefbar --show-config     # profiel + policy-samenvatting, geen secrets
chefbar --ipc <cmd>       # commando naar de draaiende instantie
                          #   cmd: panel, bar, refresh, doctor, quit
```

### IPC socket

De draaiende instantie luistert op `$XDG_RUNTIME_DIR/chefbar.sock` (zonder
`XDG_RUNTIME_DIR`: `/tmp/chefbar.sock`). Externe commando's gebruiken
`chefbar --ipc <cmd>`. De systemd-unit draait met `RuntimeDirectory=chefbar`;
de socket zelf ligt naast die runtime-map.
