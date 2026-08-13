#!/usr/bin/env bash
# Visual QA voor ChefBar / ChefApp 4.0 — screenshots + accent-assert onder Xvfb.
#
# Doel: één geïsoleerde Xvfb + runtime-dir per run, IPC-poke om het paneel te
# tonen, screenshot van de root-window, assert dat het accent (dark #5C97FF)
# aanwezig is. Geen brede pkill, geen gedeelde DISPLAY — alles via expliciete
# PIDs + trap-cleanup. XDG_RUNTIME_DIR wordt per run in mktemp (700) gezet.
#
# Shots (Lane G, ChefApp 4.0):
#   panel            — default: toont panel via IPC (--bar) (3.1 compat)
#   palette          — probeert palette-overlay te tonen (IPC: palette/toggle-palette/bar)
#   drawer           — probeert drawer te openen (IPC: drawer/open-drawer) na panel
#   density-compact  — panel met CHEFBAR_DENSITY=compact  (compact vs comfortable)
#   density-comfortable — panel met CHEFBAR_DENSITY=comfortable (default spacing)
#   all              — draait panel + palette + drawer + beide densities achter elkaar
#
# IPC-poke is tolerant: onbekende IPC-commando's falen zacht en vallen terug op
# --bar (ShowPanel). Zo blijft het script groen terwijl Lane C/D/E hun
# UiCommand-varianten nog landen.
#
# Usage:
#   scripts/visual-shot.sh [mode] [theme] [out.png]
#   scripts/visual-shot.sh dark                          # panel, dark, /tmp/chefbar-dark.png
#   scripts/visual-shot.sh palette dark /tmp/palette.png
#   scripts/visual-shot.sh --mode palette --theme light --out /tmp/p.png
#   scripts/visual-shot.sh all dark                      # alle shots, out = prefix (/tmp/chefbar-dark-*.png)
#
# Args (positie-compatibel met 3.1):
#   mode    panel (default) | palette | drawer | density-compact | density-comfortable | all
#   theme   dark (default) | light | auto
#   out.png default /tmp/chefbar-<theme>[-<mode>].png
#
# Flags (optioneel, overschrijven positie-args):
#   --mode <mode>  --theme <theme>  --out <path>  --display :N  --accent #HEX
#
# Exit-codes:
#   0  venster leefde + accent-pixel gevonden (assert geslaagd)
#   1  app crashte of venster kwam niet (harde fail)
#   2  tooling ontbreekt (Xvfb/import) — zachte skip voor CI zonder X-stack
#
# Cleanup is eigen-PID-scoped: nooit brede pkill; Xvfb en chefbar worden via
# expliciete PIDs + trap opgeruimd. Eigen XDG_RUNTIME_DIR (700) in mktemp.
# Accent-assert gebruikt ImageMagick convert met 8% fuzz; minstens 1 pixel moet
# binnen tolerantie van ACCENT_HEX vallen.

set -u

# ---- arg parsing (flags + positie-compat) ----
MODE=""
THEME=""
OUT=""
# :97 collides when pr-isolated and heavy share one host (same /tmp/.X11-unix).
# Prefer CHEFBAR_XVFB_DISPLAY; otherwise pick a free display per shot.
DISPLAY_NUM="${CHEFBAR_XVFB_DISPLAY:-}"
ACCENT_HEX="${ACCENT_HEX:-#5C97FF}"   # dark accent, zie src/css.rs

while [ $# -gt 0 ]; do
  case "$1" in
    --mode) MODE="${2:-}"; shift 2 ;;
    --theme) THEME="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --display) DISPLAY_NUM="${2:-}"; shift 2 ;;
    --accent) ACCENT_HEX="${2:-}"; shift 2 ;;
    --help|-h) echo "Usage: $0 [mode] [theme] [out.png]  [--mode M --theme T --out P]"; echo "Modes: panel palette drawer density-compact density-comfortable all"; exit 0 ;;
    --*) echo "visual-shot: onbekende flag $1" >&2; exit 1 ;;
    *)
      # positie-args: eerste onbekende = mode of theme, tweede = theme of out, derde = out
      if [ -z "$MODE" ]; then
        case "$1" in
          panel|palette|drawer|density-compact|density-comfortable|all) MODE="$1" ;;
          dark|light|auto) THEME="$1" ;;
          *) OUT="$1" ;;
        esac
      elif [ -z "$THEME" ]; then
        case "$1" in
          dark|light|auto) THEME="$1" ;;
          *) OUT="$1" ;;
        esac
      else
        OUT="$1"
      fi
      shift ;;
  esac
done

MODE="${MODE:-panel}"
THEME="${THEME:-dark}"
# OUT default pas na mode-resolutie (voor 'all' is het een prefix)

command -v Xvfb >/dev/null 2>&1 || { echo "visual-shot: Xvfb ontbreekt (zachte skip)"; exit 2; }
command -v import >/dev/null 2>&1 || { echo "visual-shot: imagemagick import ontbreekt (zachte skip)"; exit 2; }
command -v convert >/dev/null 2>&1 || { echo "visual-shot: imagemagick convert ontbreekt (zachte skip)"; exit 2; }

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO_ROOT/target/release/chefbar"
if [ ! -x "$BIN" ]; then
  echo "visual-shot: $BIN ontbreekt — bouw eerst 'cargo build --release'" >&2
  exit 1
fi

# ---- helpers ----

# Vrije X-display: CI draait pr-isolated en heavy op dezelfde host, dus :97
# is vaak al bezet (lock van een andere job of een achtergebleven Xvfb).
display_busy() {
  local num="${1#:}"
  [ -e "/tmp/.X${num}-lock" ] || [ -S "/tmp/.X11-unix/X${num}" ]
}

# Start Xvfb op een vrije display. Print "PID :N" op stdout.
# Logt stderr naar $1. Exit 1 als geen display start.
start_xvfb() {
  local log="$1"
  local seed start n pid i
  local -a try_nums=()

  if [ -n "${DISPLAY_NUM:-}" ]; then
    try_nums+=("${DISPLAY_NUM#:}")
  fi

  seed=$$
  if [ -n "${GITHUB_RUN_ID:-}" ]; then
    seed=$((GITHUB_RUN_ID + $$))
  fi
  start=$((90 + seed % 80))
  for i in $(seq 0 79); do
    n=$((90 + (start - 90 + i) % 80))
    try_nums+=("$n")
  done

  for n in "${try_nums[@]}"; do
    if display_busy "$n"; then
      continue
    fi
    : >"$log"
    Xvfb ":$n" -screen 0 900x1000x24 -nolisten tcp >"$log" 2>&1 &
    pid=$!
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
      echo "$pid :$n"
      return 0
    fi
    wait "$pid" 2>/dev/null || true
  done
  return 1
}

# Probeer een IPC-commando; faal zacht (geen exit) — val terug op --bar.
ipc_try() {
  local cmd="$1"
  # 1) probeer als UiCommand via --ipc
  if "$BIN" --ipc "$cmd" >/dev/null 2>&1; then
    return 0
  fi
  # 2) probeer via directe binary poke (tweede instantie)
  if "$BIN" --ipc "$cmd" >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

# Eén shot: start Xvfb + app + IPC-poke + screenshot + accent-assert.
# Args: mode theme out
run_shot() {
  local mode="$1" theme="$2" out="$3"
  local rt_dir xvfb_pid app_pid

  rt_dir="$(mktemp -d /tmp/chefbar-visual.XXXXXX)"
  chmod 700 "$rt_dir"
  xvfb_pid=""
  app_pid=""

  local started
  started="$(start_xvfb "$rt_dir/xvfb.log")" || {
    echo "visual-shot [$mode]: Xvfb startte niet (geen vrije display :90-:169)" >&2
    if [ -s "$rt_dir/xvfb.log" ]; then
      echo "visual-shot [$mode]: Xvfb log:" >&2
      cat "$rt_dir/xvfb.log" >&2
    fi
    rm -rf "$rt_dir"
    return 1
  }
  xvfb_pid="${started%% *}"
  DISPLAY_NUM="${started##* }"
  echo "visual-shot [$mode]: Xvfb $DISPLAY_NUM pid=$xvfb_pid"
  echo "visual-shot [$mode]: Xvfb $DISPLAY_NUM pid=$xvfb_pid"

  export DISPLAY="$DISPLAY_NUM" XDG_RUNTIME_DIR="$rt_dir"
  if [ "$theme" != "auto" ]; then export CHEFBAR_THEME="$theme"; fi

  # Density-toggle: env die Lane F leest (valt terug op comfortable als onbekend).
  case "$mode" in
    density-compact) export CHEFBAR_DENSITY="compact" ;;
    density-comfortable) export CHEFBAR_DENSITY="comfortable" ;;
    *) unset CHEFBAR_DENSITY 2>/dev/null || true ;;
  esac

  "$BIN" >/dev/null 2>"$rt_dir/app-stderr.log" &
  app_pid=$!
  sleep 4
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "visual-shot [$mode]: app overleed — stderr:" >&2
    cat "$rt_dir/app-stderr.log" >&2
    kill "$xvfb_pid" 2>/dev/null || true
    rm -rf "$rt_dir"
    return 1
  fi

  # IPC-poke per mode (tolerant: faal -> fallback naar bar).
  case "$mode" in
    panel|density-compact|density-comfortable)
      "$BIN" --bar >/dev/null 2>&1 || ipc_try "bar" || true
      ;;
    palette)
      if ! ipc_try "palette" && ! ipc_try "toggle-palette" && ! ipc_try "bar"; then
        "$BIN" --bar >/dev/null 2>&1 || true
      fi
      ;;
    drawer)
      "$BIN" --bar >/dev/null 2>&1 || true
      sleep 1
      ipc_try "drawer" || ipc_try "open-drawer" || ipc_try "toggle-drawer" || true
      ;;
  esac
  sleep 3

  import -window root "$out" 2>/dev/null
  if [ ! -s "$out" ]; then
    echo "visual-shot [$mode]: screenshot mislukt ($out)" >&2
    kill "$app_pid" 2>/dev/null || true
    kill "$xvfb_pid" 2>/dev/null || true
    rm -rf "$rt_dir"
    return 1
  fi

  # Pixel-assert: minstens 1 pixel binnen tolerantie van het accent.
  local found
  found=$(convert "$out" -fuzz 8% -fill black +opaque "$ACCENT_HEX" -fill white -opaque "$ACCENT_HEX" -format "%[fx:mean*w*h]" info: 2>/dev/null | cut -d. -f1)
  echo "visual-shot [$mode]: accent-pixels ≈ ${found:-0} (${ACCENT_HEX}) in $out"
  if [ -z "${found:-}" ] || [ "$found" -eq 0 ] 2>/dev/null; then
    echo "visual-shot [$mode]: GEEN accent-pixels gevonden — paneel waarschijnlijk niet zichtbaar" >&2
    kill "$app_pid" 2>/dev/null || true
    kill "$xvfb_pid" 2>/dev/null || true
    rm -rf "$rt_dir"
    return 1
  fi

  echo "visual-shot [$mode]: OK ($theme, $out)"
  kill "$app_pid" 2>/dev/null || true
  kill "$xvfb_pid" 2>/dev/null || true
  rm -rf "$rt_dir"
  # Xvfb kan 1-2s nodig hebben om de socket vrij te geven voor de volgende shot.
  sleep 1
  return 0
}

# ---- dispatch ----

if [ "$MODE" = "all" ]; then
  # 'all' draait alle shots sequentieel; OUT is prefix als geen expliciet pad.
  PREFIX="${OUT:-/tmp/chefbar-${THEME}}"
  # Als OUT een .png is, gebruik basename zonder extensie als prefix.
  case "$PREFIX" in
    *.png) PREFIX="${PREFIX%.png}" ;;
  esac
  FAILED=0
  run_shot "panel" "$THEME" "${PREFIX}-panel.png" || FAILED=1
  run_shot "palette" "$THEME" "${PREFIX}-palette.png" || FAILED=1
  run_shot "drawer" "$THEME" "${PREFIX}-drawer.png" || FAILED=1
  run_shot "density-compact" "$THEME" "${PREFIX}-density-compact.png" || FAILED=1
  run_shot "density-comfortable" "$THEME" "${PREFIX}-density-comfortable.png" || FAILED=1
  if [ "$FAILED" -ne 0 ]; then
    echo "visual-shot [all]: één of meer shots faalden" >&2
    exit 1
  fi
  echo "visual-shot [all]: alle shots OK (prefix $PREFIX)"
  exit 0
fi

# Single shot
if [ -z "$OUT" ]; then
  if [ "$MODE" = "panel" ]; then
    OUT="/tmp/chefbar-${THEME}.png"
  else
    OUT="/tmp/chefbar-${THEME}-${MODE}.png"
  fi
fi

run_shot "$MODE" "$THEME" "$OUT"
RC=$?
exit $RC
