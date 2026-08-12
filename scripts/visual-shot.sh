#!/usr/bin/env bash
# Visual QA: start ChefBar in een geïsoleerde Xvfb, poke het paneel via IPC,
# screenshot de root-window en assert dat het accent (dark #5C97FF) aanwezig is.
#
# Usage: scripts/visual-shot.sh [theme] [out.png]
#   theme   dark (default) | light | auto
#   out.png default /tmp/chefbar-<theme>.png
#
# Exit-codes:
#   0  venster leefde + accent-pixel gevonden (assert geslaagd)
#   1  app crashte of venster kwam niet (harde fail)
#   2  tooling ontbreekt (Xvfb/import) — zachte skip voor CI zonder X-stack
#
# Cleanup is eigen-PID-scoped: nooit brede pkill; Xvfb en chefbar worden via
# expliciete PIDs + trap opgeruimd. Eigen XDG_RUNTIME_DIR (700) in mktemp.

set -u
THEME="${1:-dark}"
OUT="${2:-/tmp/chefbar-${THEME}.png}"
ACCENT_HEX="${ACCENT_HEX:-#5C97FF}"   # dark accent, zie src/css.rs

command -v Xvfb >/dev/null 2>&1 || { echo "visual-shot: Xvfb ontbreekt (zachte skip)"; exit 2; }
command -v import >/dev/null 2>&1 || { echo "visual-shot: imagemagick import ontbreekt (zachte skip)"; exit 2; }

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO_ROOT/target/release/chefbar"
if [ ! -x "$BIN" ]; then
  echo "visual-shot: $BIN ontbreekt — bouw eerst 'cargo build --release'" >&2
  exit 1
fi

DISPLAY_NUM="${CHEFBAR_XVFB_DISPLAY:-:97}"
RT_DIR="$(mktemp -d /tmp/chefbar-visual.XXXXXX)"
chmod 700 "$RT_DIR"
XVFB_PID=""
APP_PID=""

cleanup() {
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null
  [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null
  rm -rf "$RT_DIR"
}
trap cleanup EXIT

Xvfb "$DISPLAY_NUM" -screen 0 900x1000x24 >/dev/null 2>&1 &
XVFB_PID=$!
sleep 2
kill -0 "$XVFB_PID" 2>/dev/null || { echo "visual-shot: Xvfb startte niet" >&2; exit 1; }

export DISPLAY="$DISPLAY_NUM" XDG_RUNTIME_DIR="$RT_DIR"
if [ "$THEME" != "auto" ]; then export CHEFBAR_THEME="$THEME"; fi

"$BIN" >/dev/null 2>"$RT_DIR/app-stderr.log" &
APP_PID=$!
sleep 4
if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "visual-shot: app overleed — stderr:" >&2
  cat "$RT_DIR/app-stderr.log" >&2
  exit 1
fi

# Tweede instantie = IPC-poke: toont het paneel (single-instance e2e).
"$BIN" --bar >/dev/null 2>&1
sleep 3

import -window root "$OUT" 2>/dev/null
if [ ! -s "$OUT" ]; then
  echo "visual-shot: screenshot mislukt" >&2
  exit 1
fi

# Pixel-assert: minstens 1 pixel binnen tolerantie van het accent.
FOUND=$(convert "$OUT" -fuzz 8% -fill black +opaque "$ACCENT_HEX" -fill white -opaque "$ACCENT_HEX" -format "%[fx:mean*w*h]" info: 2>/dev/null | cut -d. -f1)
echo "visual-shot: accent-pixels ≈ ${FOUND:-0} (${ACCENT_HEX}) in $OUT"
if [ -z "${FOUND:-}" ] || [ "$FOUND" -eq 0 ] 2>/dev/null; then
  echo "visual-shot: GEEN accent-pixels gevonden — paneel waarschijnlijk niet zichtbaar" >&2
  exit 1
fi

echo "visual-shot: OK ($THEME, $OUT)"
