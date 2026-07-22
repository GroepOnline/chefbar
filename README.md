# ChefBar 2.0

Mission-control tray voor ChefGroep OS (GNOME Shell / Wayland).

## Architectuurkeuze

**GTK3 + AyatanaAppIndicator3 in één proces**, met een rijk gestyled
`Gtk.Window`-panel i.p.v. een plat menu.

| Optie | Keuze |
|-------|--------|
| Tray | Ayatana AppIndicator (bewezen op deze host + AppIndicator-extensie) |
| Panel | GTK3 window + CSS provider (cards, statusdots, usage-bars) |
| GTK4 / libadwaita | **Niet** in-process — `gi.require_version` kan GTK3 en GTK4 niet mixen; een los GTK4-proces zou IPC + koude start kosten en het &lt;300 ms-budget schaden |

Klik op het tray-icoon opent het panel (menu-show wordt gehijackt). Quit zit
in de panel-footer; middle-click opent het panel via secondary-activate.

## Panel-secties

1. **Kop** — OS-health (`X/14 ok` uit `watchdog-state.json`) + dagscore uit laatste `chef-eval` rapport
2. **Providers** — actief account per provider + OCX usage-bar; **switch** → account-popover → `POST /api/accounts/<id>/switch`
3. **Agents** — lopende/laatste events uit `/api/agents` (running = pulse-dot)
4. **Fleet** — online nodes; klik → dashboard `#fleet`
5. **Quick actions** — Dashboard, Desktop `:3000`, HUD (`chef-hud`), Refresh, Agent task → `POST /api/commander/tasks`

Data: één parallel fetch-cyclus (`/status`, `/accounts/status`, `/agents`, `/fleet`, `/usage`). Cache voor snelle open; auto-refresh 30 s alleen terwijl het panel open is. Tray-health refresht op de achtergrond (60 s).

## Tray: de bon (De Pas)

Icoonstates volgen `.ulpi/design/chefbar-tray.md`: een gestileerde bon,
status via vorm + badge. Bron-SVG's in `~/.local/share/icons/chefgroep/`
(`cg-tray-<state>.svg`, gegenereerd door `build-icons.py`), PNG-renders in
`chefbar/icons/tray-<state>[-32|-48].png`.

| State | Betekenis | Tooltip |
|-------|-----------|---------|
| `stil` | lege bon | ChefGroep · nog stil in de keuken |
| `bezig` | bon met koraal regels | ChefGroep · {n} aan het werk |
| `hulp` | koraal dot (attention) | ChefGroep · even jou nodig |
| `fout` | wijn !-badge (attention) | ChefGroep · {dienst} hapert |
| `offline` | gestreepte rvs-bon | ChefGroep · keuken offline |

Het tray-menu is de bonnenstrook: max 3 live bonregels (klik → `joep-ops
focus`), Open Thuis / Open Ploeg / panel, `Pas: <account>`-submenu,
Desktop starten, Notificaties pauzeren (via `joep-notify pause`),
Meelopen vanaf login, Afsluiten. Meldingen lopen via `joep-notify`
(bron/status → icoon + urgency; pauze en GNOME-niet-storen gerespecteerd).

## Vereisten

- `gir1.2-ayatanaappindicator3-0.1` + AppIndicator GNOME-extension (Wayland)
- `CHEF_VAULT_API_TOKEN` in `docker/.env` (of `CHEFBAR_ENV_FILE` / env)
- Vault-API op `:8321` (override: `CHEFBAR_VAULT_API`)
- joep-ops op `:10101` (override: `CHEFBAR_OPS_API`, default `http://127.0.0.1:10101`)

## Omgeving (Ops)

| Env | Default | Rol |
|-----|---------|-----|
| `CHEFBAR_OPS_API` | `http://127.0.0.1:10101` | Basis-URL van joep-ops (snapshot + focus) |
| `CHEFBAR_VAULT_API` | `http://127.0.0.1:8321/api` | Vault API |
| `CHEFBAR_DASHBOARD` | `http://127.0.0.1:8080` | Dashboard-link |

Desktop (Tauri) gebruikt dezelfde canonieke poort maar de env-naam **`JOEP_OPS_BASE`**. Dat verschil is bewust surface-specifiek; zie `config/ops-url.json` (`envNames`) en `apps/desktop/README.md`.

## Installatie

```bash
./install.sh
```

Installeert naar `~/.local/share/chefbar/`, wrapper `~/.local/bin/chefbar`,
systemd user unit `chefbar.service`.

```bash
chefbar --show-panel   # testmodus zonder tray
chefbar --version
```

Log: `~/.local/share/chefbar/chefbar.log`.
