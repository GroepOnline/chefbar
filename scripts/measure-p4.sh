#!/usr/bin/env bash
# P4-metingen: meet op de release-build — binary-grootte, start→gereed,
# start→panel (lazy-bouwtijd uit chefbar.log) en RSS (VmHWM) vóór en na het
# openen van het paneel. Objectief bewijs voor het P4-acceptatiecriterium:
# start→tray <500ms, start→panel <1s, RSS <80MB rust / <120MB met paneel.
#
# Zelfde isolatie als visual-shot.sh (eigen Xvfb + eigen XDG_RUNTIME_DIR,
# PID-scoped cleanup). Headless is er geen D-Bus → de tray skipt (P4-guard),
# dus "start→gereed" meet actor+css+socket-ready, en de lazy-bouwtijd is de
# echte start→panel-tijd.
#
# Usage: scripts/measure-p4.sh [out-dir]
#   out-dir  default /tmp/chefbar-p4-meting
# Exit-codes:
#   0  metingen afgerond
#   1  release-binary ontbreekt of app overleed
#   2  Xvfb ontbreekt — zachte skip voor CI zonder X-stack

set -u
OUT_DIR="${1:-/tmp/chefbar-p4-meting}"
mkdir -p "$OUT_DIR"

command -v Xvfb >/dev/null 2>&1 || { echo "measure-p4: Xvfb ontbreekt (zachte skip)"; exit 2; }

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO_ROOT/target/release/chefbar"
if [ ! -x "$BIN" ]; then
  echo "measure-p4: $BIN ontbreekt — bouw eerst 'cargo build --release'" >&2
  exit 1
fi

SIZE_BYTES=$(stat -c%s "$BIN")
echo "binary       $((SIZE_BYTES / 1024)) KiB ($SIZE_BYTES bytes)"

DISPLAY_NUM="${CHEFBAR_XVFB_DISPLAY:-:96}"
RT_DIR="$(mktemp -d "$OUT_DIR/rt.XXXXXX")"
chmod 700 "$RT_DIR"
LOG="$OUT_DIR/chefbar.log"
: > "$LOG"
XVFB_PID=""
APP_PID=""
cleanup() {
  [ -n "$APP_PID" ] && kill "$APP_PID" 2>/dev/null
  [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null
}
trap cleanup EXIT

Xvfb "$DISPLAY_NUM" -screen 0 900x1000x24 >/dev/null 2>&1 &
XVFB_PID=$!
sleep 2
kill -0 "$XVFB_PID" 2>/dev/null || { echo "measure-p4: Xvfb startte niet" >&2; exit 1; }

export DISPLAY="$DISPLAY_NUM" XDG_RUNTIME_DIR="$RT_DIR" CHEFBAR_LOG="$LOG" CHEFBAR_THEME=dark
START_MS=$(date +%s%3N)
"$BIN" >/dev/null 2>"$OUT_DIR/app-stderr.log" &
APP_PID=$!

# start→gereed: meteen pollen (geen vaste sleep) tot de "gestart"-logregel
# (actor + css + socket up). Bestand bestaat nog niet → grep faalt → doorpoll.
for _ in $(seq 1 100); do
  grep -q "gestart" "$LOG" 2>/dev/null && break
  sleep 0.05
done
READY_MS=$(( $(date +%s%3N) - START_MS ))
echo "start->gereed $READY_MS ms"

# RSS rust (tray-only leven, paneel nog niet gebouwd) — kort laten settelen.
sleep 1
RSS_RUST=$(grep VmHWM "/proc/$APP_PID/status" 2>/dev/null | awk '{print $2}')
echo "rss-rust     ${RSS_RUST:-?} KiB"

# start→panel: IPC-poke toont het paneel; meet de latentie vanaf de poke
# (de echte UX-tijd). De lazy-bouwtijd (absoluut sinds app-start) staat in de
# log ter referentie.
POKE_MS=$(date +%s%3N)
"$BIN" --bar >/dev/null 2>&1
for _ in $(seq 1 100); do
  grep -q "panel opgebouwd" "$LOG" 2>/dev/null && break
  sleep 0.05
done
PANEL_LATENCY=$(( $(date +%s%3N) - POKE_MS ))
PANEL_LOG=$(grep -o 'panel opgebouwd na [0-9]*ms (lazy)' "$LOG" | head -1)
if [ -n "$PANEL_LOG" ]; then
  echo "start->panel  ${PANEL_LATENCY}ms (poke→klaar; log: ${PANEL_LOG})"
else
  echo "start->panel  niet binnen 5s (paneel niet gebouwd?)"
fi

# RSS met paneel open.
sleep 2
RSS_PANEL=$(grep VmHWM "/proc/$APP_PID/status" 2>/dev/null | awk '{print $2}')
echo "rss-paneel   ${RSS_PANEL:-?} KiB"

if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "measure-p4: app overleed — stderr:" >&2
  cat "$OUT_DIR/app-stderr.log" >&2
  exit 1
fi
echo "measure-p4: OK"
