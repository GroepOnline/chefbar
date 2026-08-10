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
SYSTEMD=0
BINARY=""
for arg in "$@"; do
  case "$arg" in
    --systemd) SYSTEMD=1 ;;
    -h|--help)
      sed -n '2,7p' "$0"
      exit 0
      ;;
    *) BINARY="$arg" ;;
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

install -m 755 "$BINARY" "$BIN_DIR/chefbar"
echo "binary    $BIN_DIR/chefbar"

# Endpoint-profiel, reflecteert de oude Python-profielen. CHEFBAR_* env-warden
# lopen direct door naar het binary en winnen per veld van het JSON-profiel.
if [ ! -f "$CONFIG_DIR/endpoints.json" ]; then
  install -m 644 "$APP_DIR/config/endpoints.example.json" "$CONFIG_DIR/endpoints.json"
  echo "profiel   $CONFIG_DIR/endpoints.json (voorbeeld geplaatst)"
else
  echo "profiel   $CONFIG_DIR/endpoints.json (bestaand, ongewijzigd)"
fi
echo "env       CHEFBAR_ENDPOINT_PROFILE=$CONFIG_DIR/endpoints.json (of eigen pad via env)"

if [ "$SYSTEMD" -eq 1 ]; then
  install -m 644 "$APP_DIR/chefbar.service" "$UNIT_DIR/chefbar.service"

  # Global hotkey: Super+Space opent de command-bar (GNOME custom shortcut).
  # Super+Space is standaard geclaimd door input-source switching; met één
  # layout doet die niets, dus we geven de combinatie aan de bar.
  if command -v gsettings >/dev/null 2>&1; then
    KB="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/chefbar0/"
    gsettings set org.gnome.desktop.wm.keybindings switch-input-source "['XF86Keyboard']" || true
    CUR="$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings)"
    NEW="$(python3 - "$CUR" "$KB" <<'PY'
import ast, sys
cur = ast.literal_eval(sys.argv[1])
kb = sys.argv[2]
if kb not in cur:
    cur.append(kb)
print(repr(cur))
PY
)"
    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$NEW"
    gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" name 'ChefBar bar'
    gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" command "$BIN_DIR/chefbar --ipc bar"
    gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" binding '<Super>space'
    echo "hotkey    Super+Space → chefbar --ipc bar"
  fi

  systemctl --user daemon-reload
  systemctl --user enable --now chefbar.service
  systemctl --user restart chefbar.service
  systemctl --user status chefbar.service --no-pager | head -8
  echo "service   chefbar.service actief in de user-manager"
fi

echo "ChefBar (Rust) geïnstalleerd → $BIN_DIR/chefbar"
echo "Test: chefbar --doctor"