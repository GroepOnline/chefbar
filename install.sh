#!/usr/bin/env bash
# Installeert ChefBar (Rust binary) voor de huidige gebruiker.
#
# Gebruik:
#   ./install.sh                        binary + endpoints-profiel
#   ./install.sh --systemd              + systemd-user-unit, start + hotkey
#   ./install.sh <pad/naar/binary>      eigen build of artifact gebruiken
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Doel-directories: ~/.local/bin (root: /usr/local/bin).
if [ "$(id -u)" -eq 0 ]; then
  BIN_DIR="/usr/local/bin"
else
  BIN_DIR="$HOME/.local/bin"
fi
UNIT_DIR="$HOME/.config/systemd/user"
CONFIG_DIR="$HOME/.config/chefbar"
mkdir -p "$BIN_DIR" "$UNIT_DIR" "$CONFIG_DIR"

# Argumenten.
# Ondersteunt beide volgordes: ./install.sh --systemd [binary]  en  ./install.sh [binary] --systemd
SYSTEMD=0
BINARY=""
for arg in "$@"; do
  case "$arg" in
    --systemd) SYSTEMD=1 ;;
    -h|--help)
      sed -n '2,7p' "$0"
      exit 0
      ;;
    --) break ;;
    -*) echo "fout: onbekende optie $arg (zie --help)" >&2; exit 2 ;;
    *) 
      if [ -n "$BINARY" ]; then
        echo "fout: meerdere binary-paden gegeven: $BINARY en $arg" >&2; exit 2
      fi
      BINARY="$arg" ;;
  esac
done

# Binary-bron: expliciet pad, target/release in deze checkout, of bestaande
# installatie. Anders stoppen met een duidelijke boodschap.
if [ -n "$BINARY" ] && [ ! -x "$BINARY" ]; then
  echo "fout: binary niet gevonden of niet uitvoerbaar: $BINARY" >&2
  exit 1
fi
if [ -z "$BINARY" ]; then
  if [ -x "$APP_DIR/target/release/chefbar" ]; then
    BINARY="$APP_DIR/target/release/chefbar"
  elif command -v chefbar >/dev/null 2>&1; then
    BINARY="$(command -v chefbar)"
  else
    echo "fout: geen chefbar-binary gevonden; bouw eerst (cargo build --release) of geef een pad." >&2
    exit 1
  fi
fi

# Als er al een user-service draait, stop die kort zodat ETXTBSY bij overschrijven vermeden wordt.
if systemctl --user cat chefbar.service >/dev/null 2>&1; then
  systemctl --user stop chefbar.service >/dev/null 2>&1 || true
fi

install -m 755 "$BINARY" "$BIN_DIR/chefbar"
echo "binary    $BIN_DIR/chefbar"

# Signaal v2 type: General Sans (UI) + IBM Plex Mono (data). No CDN.
# Missing faces fall back visibly — never silently to Cantarell-only.
if command -v fc-list >/dev/null 2>&1; then
  if fc-list : family | grep -Fqi "General Sans"; then
    echo "font      General Sans aanwezig"
  else
    echo "font      ONTBREEKT: General Sans — CSS noemt de face eerst; zonder installatie zie je system-ui/Cantarell. Fontshare-licentie: niet bundelen. Optioneel: $APP_DIR/config/fonts-signaal.conf → ~/.config/fontconfig/conf.d/" >&2
  fi
  if fc-list : family | grep -Fqi "IBM Plex Mono"; then
    echo "font      IBM Plex Mono aanwezig"
  else
    echo "font      ONTBREEKT: IBM Plex Mono — OFL, mag gebundeld of via distro. Zonder deze face valt data-mono terug." >&2
  fi
fi
if command -v gdk-pixbuf-query-loaders >/dev/null 2>&1; then
  if gdk-pixbuf-query-loaders 2>/dev/null | grep -Fqi svg; then
    echo "pixbuf   SVG-loader aanwezig"
  else
    echo "pixbuf   ONTBREEKT: SVG-loader (librsvg) — Lucide-iconen vallen terug op image-missing. Pakket: librsvg2-common" >&2
  fi
fi

# Waarschuw als BIN_DIR niet in PATH staat (on-click draait wel, terminal niet).
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "hint     $BIN_DIR staat niet in PATH — voeg toe aan ~/.profile of open een nieuwe shell" ;;
esac

# Endpoint-profiel, reflecteert de oude Python-profielen. CHEFBAR_* env-warden
# lopen direct door naar het binary en winnen per veld van het JSON-profiel.
if [ ! -f "$CONFIG_DIR/endpoints.json" ]; then
  install -m 644 "$APP_DIR/config/endpoints.example.json" "$CONFIG_DIR/endpoints.json"
  echo "profiel   $CONFIG_DIR/endpoints.json (voorbeeld geplaatst)"
else
  echo "profiel   $CONFIG_DIR/endpoints.json (bestaand, ongewijzigd)"
fi
echo "env       CHEFBAR_ENDPOINT_PROFILE=$CONFIG_DIR/endpoints.json (of eigen pad via env)"

# v2-look notificaties (chefbar-tray.md): plaats de template alleen als de
# daemon aanwezig is — start nooit zelf een nieuwe daemon.
if command -v mako >/dev/null 2>&1; then
  mkdir -p "$HOME/.config/mako"
  if [ ! -f "$HOME/.config/mako/config" ]; then
    install -m 644 "$APP_DIR/config/mako/config" "$HOME/.config/mako/config"
    echo "notify    ~/.config/mako/config (v2-look geplaatst)"
  else
    echo "notify    ~/.config/mako/config (bestaand, ongewijzigd)"
  fi
fi
if command -v dunst >/dev/null 2>&1; then
  mkdir -p "$HOME/.config/dunst"
  if [ ! -f "$HOME/.config/dunst/dunstrc" ]; then
    install -m 644 "$APP_DIR/config/dunst/dunstrc" "$HOME/.config/dunst/dunstrc"
    echo "notify    ~/.config/dunst/dunstrc (v2-look geplaatst)"
  else
    echo "notify    ~/.config/dunst/dunstrc (bestaand, ongewijzigd)"
  fi
fi

if [ "$SYSTEMD" -eq 1 ]; then
  install -m 644 "$APP_DIR/chefbar.service" "$UNIT_DIR/chefbar.service"

  # Global hotkeys:
  #   chefbar0 = Super+Space -> chefbar --ipc bar (panel)
  #   chefapp1 = Super+Shift+Space -> chefbar --ipc palette (palette-overlay, Lane E)
  # Super+Space is standaard geclaimd door input-source switching; met één
  # layout doet die niets, dus we geven de combinatie aan de bar.
  if command -v gsettings >/dev/null 2>&1 && [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
    KB="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/chefbar0/"
    KB1="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/chefapp1/"
    # gsettings kan "@as []" (GVariant) teruggeven; python-snippet is daar robuust tegen.
    if CUR="$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null)"; then
      NEW="$(python3 - "$CUR" "$KB" "$KB1" <<'PY' 2>/dev/null || echo "['$KB', '$KB1']"
import ast, sys
raw = sys.argv[1].strip()
kbs = sys.argv[2:]
# "@as []" → []
if raw.startswith("@as"):
    raw = raw[3:].strip()
try:
    cur = ast.literal_eval(raw)
    if not isinstance(cur, list):
        cur = []
except Exception:
    cur = []
for kb in kbs:
    if kb not in cur:
        cur.append(kb)
print(repr(cur))
PY
)"
      gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$NEW" 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" name 'ChefBar bar' 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" command "$BIN_DIR/chefbar --ipc bar" 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" binding '<Super>space' 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB1" name 'ChefApp palette' 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB1" command "$BIN_DIR/chefbar --ipc palette" 2>/dev/null || true
      gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB1" binding '<Super><Shift>space' 2>/dev/null || true
      gsettings set org.gnome.desktop.wm.keybindings switch-input-source "['XF86Keyboard']" 2>/dev/null || true
      echo "hotkey    Super+Space → chefbar --ipc bar"
      echo "hotkey    Super+Shift+Space → chefbar --ipc palette"
    fi
  else
    echo "hotkey    overgeslagen (geen gsettings of geen display)"
  fi

  if systemctl --user daemon-reload 2>/dev/null; then
    systemctl --user enable --now chefbar.service 2>/dev/null || systemctl --user enable chefbar.service 2>/dev/null || true
    systemctl --user restart chefbar.service 2>/dev/null || systemctl --user start chefbar.service 2>/dev/null || true
    systemctl --user status chefbar.service --no-pager 2>/dev/null | head -8 || true
    echo "service   chefbar.service actief in de user-manager (indien beschikbaar)"
  else
    echo "service   user-manager niet bereikbaar (geen systemd --user); binary wel geïnstalleerd — start handmatig: $BIN_DIR/chefbar &"
  fi
fi

echo "ChefBar (Rust) geïnstalleerd → $BIN_DIR/chefbar"
# Non-blocking quick-check (faalt niet de install als vault offline is).
if [ -x "$BIN_DIR/chefbar" ]; then
  echo "doctor    $("$BIN_DIR/chefbar" --doctor 2>&1 | head -6 | tr '\n' '; ')"
fi
echo "Test: $BIN_DIR/chefbar --doctor  ·  Super+Space opent de bar (na --systemd)"
