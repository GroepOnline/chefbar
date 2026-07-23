# ChefBar 3.0

Raycast-stijl command palette + tray voor ChefGroep OS (GNOME Shell / Wayland),
in de "Signaal, warm"-designtaal (`.ulpi/design/DESIGN.md`).

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
4. **Signaal tokens** — Archivo display, Source Sans 3 interface, IBM Plex Mono data; radius 0/6/10; lichte basis + donker via systeemvoorkeur.

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
chefbar --show-panel   # panel zonder tray
chefbar --bar          # command palette (Super+Space)
chefbar --version
```

## Tests

```bash
cd apps/chefbar
PYTHONPATH=. python3 -m unittest discover -s tests -v
python3 -m py_compile chefbar/*.py
```
