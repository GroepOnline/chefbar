# ChefBar roadmap 3.1

Drie features verscherpen 3.0 zonder de architectuur open te breken. Scope is
strak: geen tweede bar, geen tweede daemon, geen tweede waarheid. Eén profiel,
één actor, één venster.

## Verscheept in 3.1

### Zoeken dat kiest

Ranking op recency, geen platte filter. `RankContext` (`src/palette.rs`) krijgt
boost-termen uit sessies die om jou vragen en agents die nu draaien; het panel
bouwt ze per render uit de snapshot (`src/panel.rs`). De boost is tier-bewust:
herordenen binnen de contains-tier mag, prefix- en gappy-matches passeren een
contains-match nooit. Pinned acties krijgen een eigen kleine boost en
gebruiken hetzelfde mechanisme.

### Rustigere meldingen

Watcher-transities gaven per agent een toast; vijf agents die tegelijk
blokkeren betekenden vijf toasts. `coalesce_toasts` (`src/models.rs`) bundelt
verse suggesties tot hooguit één toast per poll-cyclus: één suggestie krijgt
haar eigen tekst, meerdere smelten tot "ChefGroep · N meldingen" met de
ergste ernst en de eerste drie titels. Transities blijven de enige trigger —
de watcher vuurt niet op toestand, alleen op verandering.

### Panel dat onthoudt

Laatste harnas en zoekterm worden bewaard in
`~/.config/chefbar/panel-state.json` (`src/panel_state.rs`), atomair
geschreven, tolerant geladen. Wijzigingen zetten een dirty-flag; één 2s-timer
schrijft alleen bij verandering. `CHEFBAR_PANEL_STATE` overschrijft het pad
(tests en warden-laag). Heropenen toont exact wat je zag — geen gefilterd
veld met ongefilterde inhoud, geen harnas-reset.

## Bewust uitgesteld

### Wayland zonder spijt

Layer-shell waar het kan, fallback waar het moet. Vereist `gtk-layer-shell`
(0.8, gekoppeld aan gtk3) plus een CI-afhankelijkheid op de self-hosted
runner, en een gedragsmatrix voor GNOME Wayland vs. X11-sessies. De huidige
undecorated-window + `keep_above` werkt op beide; de upgrade is een eigen
change met eigen testcyclus, geen bijvangst in 3.1. Beslissing: pas oppakken
als de CI-runner de layer-shell dev-libs heeft en er een X11-fallback-test is.

### Auth zonder wrijving

OIDC access tokens via de bestaande `auth::get_headers` seam. De seam staat
(service tokens werken, headers worden per call gebouwd), dus OIDC is een
configuratievraagstuk, geen refactor. Wacht op het identity-plane doel
`auth.chefgroep.online` (Authentik); tot dan blijven CF Access service tokens
de enige remote-auth voor productie-endpoints.

## Niet-doelen

* Geen tweede poll-loop, geen tweede socket, geen tweede tray.
* Geen Electron- of webview-afslag; GTK3 blijft de surface tot de
  layer-shell-beslissing.
* Geen meldingen zonder transitie. Stilte is een feature.
