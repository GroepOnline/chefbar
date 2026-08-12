# ChefBar volledig werkend — masterplan (2026-08-12)

Aanleiding: screenshots + melding "rare bugs, totaal niet werkend, rare jumps, mist veel".
Dit plan catalogueert eerst wat er ECHT mis is (met bewijs), daarna de werkstromen
om alles werkend te maken. Geen redesign — Signaal v2 blijft de visuele autoriteit.

## 1. Bevestigde defecten (met bewijs)

| # | Defect | Bewijs | Ernst |
|---|--------|--------|-------|
| D1 | **Super+Space deed niets.** De live GNOME-binding riep `chefbar --bar` aan; clap kende alleen `--ipc <cmd>` → exit 2, stil. | `chefbar --bar` → exit 2; binding uit oudere install.sh | Kritiek — gefixt in PR #10 + live binding gerepareerd |
| D2 | **Venster "jumps".** `GDK_BACKEND=x11` geforceerd (drop-in 40-gdk-backend.conf) + `set_position(Center)` + `set_keep_above` + volledige content-rebuild elke poll-cyclus → het venster her-centreert/resizet zichtbaar op XWayland. | `src/panel.rs:41-44`; drop-in; roadmap stelde layer-shell al uit | Hoog |
| D3 | **Dubbele-venster-ervaring.** Screenshots tonen twee panelen. Oorzaken: (a) het 10×10-verborgen venster dat bij rare show-paden zichtbaar kan worden, (b) `chef-hud` (Alt+Space, rofi-lookalike met vrijwel dezelfde header "agentische assistent"), (c) historisch FORCE_NEW-tweede-instantie-pad. | OCR van screenshots 07:25; `~/.local/bin/chef-hud`; window 10x10 op Xvfb | Hoog (perceptie) |
| D4 | **Doctor liegt buiten de service-env.** Kale `chefbar --doctor` gebruikt de profiel-default `https://vault-api.chefgroep.online` (heeft helemaal geen DNS-record) terwijl de service via drop-in `CHEFBAR_VAULT_API=http://127.0.0.1:18321/api` pollt → "vault offline" terwijl de app wél data heeft. | doctor-run vs drop-in env; `getent hosts` faalt | Midden |
| D5 | **Data-vlak hangt aan fragiele stukjes.** (a) vault-api bereikbaar via een handmatige ssh-forward (pid-gebonden, 127.0.0.1:18321) — valt weg bij reboot; (b) `ops.chefgroep.online` zit achter Cloudflare Access (302) — de app kan daar niet inloggen; (c) providers tonen STALE sinds 2026-08-12 (connector-data oud, upstream); (d) kosten `$0.0000` — niet gewired. | curl-probes 302/401; screenshot-rijen "STALE sinds 2026-08-12" | Hoog |
| D6 | **Tray heeft geen Wayland-anker.** AppIndicator kan geen positie doorgeven; het paneel opent gecentreerd i.p.v. bij de tray — voelt als een sprong. | ksni/Wayland-gedrag | Midden |

## 2. Werkstromen

### W1 — Interactie & venster-stabiliteit (hoogste impact)
- [x] D1 hotkey-alias (PR #10) + live binding gerepareerd
- [x] Vaste venster-geometrie: hoogte van de inhoud mag nooit de venstergrootte
      veranderen (scroller met vaste hoogte; min/max size lock). Eliminieert
      resize-jumps onafhankelijk van backend. (PR #17, gemerged)
- [x] Fades alleen op open/dicht, nooit tijdens render; `present()` alleen
      als het venster verborgen was (nu: elke show → her-positionering). (PR #17)
- [ ] **gtk-layer-shell evaluatie** (stond al op de roadmap als "bewust
      uitgesteld"): echte Wayland-laag met tray-anker (top-right, margin),
      geen XWayland meer. Beslis-tree: crate `gtk-layer-shell` aanwezig op
      GNOME/Ubuntu → layer-shell; anders fallback = huidige X11 met W1-fixes.
- [ ] Alt+Space (chef-hud) vs Super+Space (chefbar) expliciteren: één
      quick-command-overlay. Voorstel: chef-hud retireren of herstylen zodat
      het nooit als "tweede ChefBar" leest.
- [ ] Verborgen 10×10-venster: paneel start `visible=false` en krijgt pas
      grootte bij eerste show (nu mapt hij 10×10 op X11).

### W2 — Data-vlak robuust
- [ ] vault-api-route vervangen: ssh-forward (pid!) → duurzame route.
      Opties: (a) vault-api als user-service op laptop (draait nu niet),
      (b) publieke edge `vault-api.chefgroep.online` via CF (DNS-record +
      tunnel-ingress + Access service-token voor de app) — dit is de
      "CF first"-richting uit de Vault-docs, (c) Tailscale-adres als interim.
- [ ] `ops.chefgroep.online` achter Access: of service-token header
      (CF-Access-Client-Id/Secret via drop-in env), of ops-endpoints via
      vault-api proxyen. Beslissing hoort bij Vault-edge werk.
- [ ] Freshness-contract: elke sectie toont "stale sinds <t>" + reden
      (endpoint onbereikbaar / 401 / connector oud) i.p.v. alleen STALE.
- [ ] Kosten-wiring: `$0.0000` → echte bron (vault-api usage of
      provider-export); expliciet "n.v.t." als er geen bron is.

### W3 — Tray & notificaties volgens de brief
- [ ] Tray-menu = compacte statuslijn uit `chefbar-tray.md` (max 10 items,
      Plex Mono data, acties Open Thuis/Ploeg, account-submenu).
- [ ] v2-look voor mako/dunst (staat al in de herbonden tray-brief):
      hairline, radius 10, General Sans, amber rand bij hulp/critical.
- [ ] Glyph live-verificatie op echt GNOME-panel (AppIndicator draait niet
      onder Xvfb): stil/bezig/hulp/fout/offline doorprikken via ipc-testhook.

### W4 — Doctor, IPC & observability
- [x] `chefbar --doctor` bevraagt eerst de draaiende instantie via IPC
      (die heeft de echte env); pas zonder instantie zelf pollen.
      Geen "vault offline" meer door env-drift. (PR #18, gemerged)
- [x] Doctor exit-codes: 0 ok / 1 degraded / 2 down — bruikbaar in
      systemd en scripts. (PR #18)
- [ ] Poll-gezondheid zichtbaar in de statuslijn zelf ("laatste poll 4s
      geleden · vault ok · ops 302").

### W5 — QA-harnas (zodat dit niet terugkomt)
- [x] `/tmp/chefbar-shot3.sh` volwassen maken als `scripts/visual-shot.sh`
      in de repo (geïsoleerde Xvfb + runtime-dir, IPC-poke, pixel-assert
      op accent-kleur). (PR #19, gemerged)
- [x] CLI golden tests: elke gedocumenteerde vlag (`--bar`, `--ipc bar`,
      `doctor`, `serve`, `--show-config`) in een clap-test. (PR #19)
- [x] Single-instance e2e: twee processen starten → exact één venster.
      (in visual-shot.sh, PR #19)
- [x] Screenshot-diff tegen design-referentie in CI (zacht, warning-only).
      (CI visual-job met Xvfb + accent-assert, PR #19)

## 3. Acceptatiecriteria (done = dit)
1. Super+Space opent het paneel binnen ~300ms, **altijd precies één venster**,
   ongeacht hoe vaak en hoe snel gedrukt.
2. Het venster verspringt/resize't nooit tijdens poll of render; het opent
   op een vaste, voorspelbare plek.
3. Elke sectie toont verse data, of een expliciete reden + "stale sinds".
4. `chefbar --doctor` is consistent tussen kale shell en service-env.
5. Tray-glyph + menu matchen `chefbar-tray.md`; notificaties matchen v2.
6. W5-tests groen in CI (self-hosted runner).

## 4. Volgorde
PR #10 (D1) → W1 venster-stabiliteit (zonder layer-shell eerst) → W4 doctor
→ W2 data (afhankelijk van Vault-edge beslissing) → W3 tray/notificaties →
W5 CI-harnas → layer-shell als eigen PR met fallback-bewijs.
