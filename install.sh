#!/usr/bin/env bash
# Installeert ChefBar 2.0 als systemd user service voor de huidige gebruiker.
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$HOME/.config/systemd/user"
SHARE_DIR="$HOME/.local/share/chefbar"

mkdir -p "$BIN_DIR" "$UNIT_DIR" "$SHARE_DIR"

# gir-dependency check
python3 - <<'PY'
import gi
gi.require_version("Gtk", "3.0")
gi.require_version("AyatanaAppIndicator3", "0.1")
print("Gtk3 + AyatanaAppIndicator3: OK")
PY

# Sync app tree under share/app/ so logs in share/ survive --delete.
APP_SHARE="$SHARE_DIR/app"
mkdir -p "$APP_SHARE"
rsync -a --delete \
  --exclude '.git' \
  --exclude '__pycache__' \
  --exclude '*.pyc' \
  --exclude '*.log' \
  "$APP_DIR/" "$APP_SHARE/"

# Wrapper so PATH resolves `chefbar` without depending on repo checkout.
cat >"$BIN_DIR/chefbar" <<EOF
#!/usr/bin/env bash
exec /usr/bin/python3 "$APP_SHARE/chefbar.py" "\$@"
EOF
chmod 755 "$BIN_DIR/chefbar"

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
  gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" command "$BIN_DIR/chefbar --bar"
  gsettings set "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KB" binding '<Super>space'
  echo "Hotkey: Super+Space → chefbar --bar"
fi

systemctl --user daemon-reload
systemctl --user enable --now chefbar.service
systemctl --user restart chefbar.service
systemctl --user status chefbar.service --no-pager | head -8
echo "ChefBar 2.0 geïnstalleerd → $SHARE_DIR"
echo "Log: ~/.local/share/chefbar/chefbar.log"
echo "Test panel: chefbar --show-panel"
