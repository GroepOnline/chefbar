#!/usr/bin/env bash
# Capture and assert ChefBar visual states under an isolated Xvfb display.
#
# Usage:
#   scripts/visual-shot.sh [mode] [theme] [out.png]
#   scripts/visual-shot.sh --mode all-domains --theme dark --out /tmp/chefbar-domains
#   scripts/visual-shot.sh --mode domain --domain inbox --theme dark --out /tmp/inbox.png
#
# Modes:
#   panel, overlay (palette alias), palette, drawer
#   density-compact, density-comfortable
#   domain:<name>, all-domains, all
#
# Exit codes:
#   0 = screenshot and accent assertion passed
#   1 = app/screenshot/assertion failure
#   2 = Xvfb or ImageMagick is unavailable (soft skip for CI)

set -u

MODE=""
THEME=""
OUT=""
# :97 collides when pr-isolated and heavy share one host (same /tmp/.X11-unix).
# Prefer CHEFBAR_XVFB_DISPLAY; otherwise pick a free display per shot.
DISPLAY_NUM="${CHEFBAR_XVFB_DISPLAY:-}"
ACCENT_HEX="${CHEFBAR_ACCENT:-${ACCENT_HEX:-#5C97FF}}"
DOMAIN=""

DOMAINS=(
  inbox fleet herdr vault commerce crm share clipboard desktop
  tasks linear containers secrets kater health
)

usage() {
  cat <<'USAGE'
Usage: scripts/visual-shot.sh [mode] [theme] [out.png]
       scripts/visual-shot.sh --mode MODE --theme THEME --out PATH

Modes: panel palette overlay drawer density-compact density-comfortable
       domain:<name> all-domains all
Flags: --domain NAME --display :N --accent #HEX
USAGE
}

canonical_domain() {
  case "$1" in
    taken) echo tasks ;;
    accounts|providers) echo commerce ;;
    instellingen|settings) echo health ;;
    *) echo "$1" ;;
  esac
}

is_domain() {
  local candidate
  candidate="$(canonical_domain "$1")"
  local domain
  for domain in "${DOMAINS[@]}" eval sync control; do
    if [ "$domain" = "$candidate" ]; then
      return 0
    fi
  done
  return 1
}

need_val() {
  if [ "$#" -lt 2 ] || [ -z "${2:-}" ] || [[ "${2}" == --* ]]; then
    echo "visual-shot: $1 vereist een waarde" >&2
    usage >&2
    exit 1
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --mode)
      need_val "$@"
      MODE="$2"
      shift 2
      ;;
    --theme)
      need_val "$@"
      THEME="$2"
      shift 2
      ;;
    --out)
      need_val "$@"
      OUT="$2"
      shift 2
      ;;
    --display)
      need_val "$@"
      DISPLAY_NUM="$2"
      shift 2
      ;;
    --accent)
      need_val "$@"
      ACCENT_HEX="$2"
      shift 2
      ;;
    --domain)
      need_val "$@"
      DOMAIN="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --*)
      echo "visual-shot: onbekende flag $1" >&2
      usage >&2
      exit 1
      ;;
    *)
      if [ -z "$MODE" ]; then
        case "$1" in
          panel|palette|overlay|drawer|density-compact|density-comfortable|all-domains|domains|all|domain|domain:*) MODE="$1" ;;
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
      shift
      ;;
  esac
done

MODE="${MODE:-panel}"
THEME="${THEME:-dark}"

case "$MODE" in
  overlay) MODE="palette" ;;
  domains) MODE="all-domains" ;;
  domain)
    if [ -z "$DOMAIN" ]; then
      echo "visual-shot: --mode domain vereist --domain NAME" >&2
      exit 1
    fi
    MODE="domain:$DOMAIN"
    ;;
esac

case "$MODE" in
  panel|palette|drawer|density-compact|density-comfortable|all-domains|all|domain:*) ;;
  *)
    echo "visual-shot: onbekende mode '$MODE'" >&2
    usage >&2
    exit 1
    ;;
esac

if ! command -v Xvfb >/dev/null 2>&1; then
  echo "visual-shot: Xvfb ontbreekt (zachte skip)" >&2
  exit 2
fi
if ! command -v import >/dev/null 2>&1 || ! command -v convert >/dev/null 2>&1; then
  echo "visual-shot: ImageMagick import/convert ontbreekt (zachte skip)" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO_ROOT/target/release/chefbar"
if [ ! -x "$BIN" ]; then
  echo "visual-shot: $BIN ontbreekt — bouw eerst 'cargo build --release'" >&2
  exit 1
fi

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

ipc_try() {
  local command="$1"
  "$BIN" --ipc "$command" >/dev/null 2>&1 && return 0
  return 1
}

run_shot() {
  local mode="$1"
  local theme="$2"
  local out="$3"
  local rt_dir=""
  local xvfb_pid=""
  local app_pid=""
  local found=""
  local domain=""
  local focused=""
  local stderr_keep="${out%.png}-stderr.log"

  finish() {
    if [ -n "${rt_dir:-}" ] && [ -f "$rt_dir/app-stderr.log" ]; then
      cp "$rt_dir/app-stderr.log" "$stderr_keep" 2>/dev/null || true
    fi
    if [ -n "${app_pid:-}" ]; then kill "$app_pid" 2>/dev/null || true; fi
    if [ -n "${xvfb_pid:-}" ]; then kill "$xvfb_pid" 2>/dev/null || true; fi
    if [ -n "${rt_dir:-}" ]; then rm -rf "$rt_dir"; fi
  }

  if [[ "$mode" == domain:* ]]; then
    domain="$(canonical_domain "${mode#domain:}")"
    if ! is_domain "$domain"; then
      echo "visual-shot: onbekend domein '$domain'" >&2
      return 1
    fi
  fi

  rt_dir="$(mktemp -d /tmp/chefbar-visual.XXXXXX)"
  chmod 700 "$rt_dir"
  mkdir -p "$(dirname "$out")"

  local started
  started="$(start_xvfb "$rt_dir/xvfb.log")" || {
    echo "visual-shot [$mode]: Xvfb startte niet (geen vrije display :90-:169)" >&2
    if [ -s "$rt_dir/xvfb.log" ]; then
      echo "visual-shot [$mode]: Xvfb log:" >&2
      cat "$rt_dir/xvfb.log" >&2
    fi
    finish
    return 1
  }
  xvfb_pid="${started%% *}"
  DISPLAY_NUM="${started##* }"
  echo "visual-shot [$mode]: Xvfb $DISPLAY_NUM pid=$xvfb_pid"

  export DISPLAY="$DISPLAY_NUM"
  export XDG_RUNTIME_DIR="$rt_dir"
  # Panel-state isoleren per shot: anders laadt de app Joeps persisted
  # actieve domein en is de panel-baseline niet deterministisch.
  export CHEFBAR_PANEL_STATE="$rt_dir/panel-state.json"
  if [ "$theme" != "auto" ]; then
    export CHEFBAR_THEME="$theme"
  fi

  case "$mode" in
    density-compact) export CHEFBAR_DENSITY=compact ;;
    density-comfortable) export CHEFBAR_DENSITY=comfortable ;;
    *) unset CHEFBAR_DENSITY ;;
  esac

  "$BIN" >"$rt_dir/app-stdout.log" 2>"$rt_dir/app-stderr.log" &
  app_pid=$!
  sleep 4
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "visual-shot [$mode]: app overleed — stderr:" >&2
    cat "$rt_dir/app-stderr.log" >&2
    finish
    return 1
  fi

  case "$mode" in
    panel|density-compact|density-comfortable)
      ipc_try "bar" || "$BIN" --bar >/dev/null 2>&1 || true
      ;;
    palette)
      ipc_try "palette" || ipc_try "toggle-palette" || ipc_try "bar" || "$BIN" --bar >/dev/null 2>&1 || true
      ;;
    drawer)
      ipc_try "drawer" || ipc_try "open-drawer" || ipc_try "bar" || "$BIN" --bar >/dev/null 2>&1 || true
      ;;
    domain:*)
      focused=0
      for _ in 1 2 3; do
        if ipc_try "focus-domain $domain" || ipc_try "focus $domain"; then
          focused=1
          break
        fi
        sleep 1
      done
      if [ "$focused" -ne 1 ]; then
        echo "visual-shot [$mode]: domain-focus faalde (focus-domain/focus $domain)" >&2
        cat "$rt_dir/app-stderr.log" >&2
        finish
        return 1
      fi
      ipc_try "bar" || "$BIN" --bar >/dev/null 2>&1 || true
      ;;
  esac
  sleep 3

  if ! import -window root "$out" 2>/dev/null || [ ! -s "$out" ]; then
    echo "visual-shot [$mode]: screenshot mislukt ($out)" >&2
    finish
    return 1
  fi

  # 20% fuzz: GTK3 rendert de accentkleur met kleurbeheer/antialiasing, exact
  # matchen (8%) gaf false-negatives voor palette/drawer-streaks.
  found="$(convert "$out" -fuzz 20% -fill black +opaque "$ACCENT_HEX" -fill white -opaque "$ACCENT_HEX" -format "%[fx:mean*w*h]" info: 2>/dev/null | cut -d. -f1)"
  echo "visual-shot [$mode]: accent-pixels ≈ ${found:-0} (${ACCENT_HEX}) in $out"
  if [ -z "${found:-}" ] || [ "$found" -eq 0 ] 2>/dev/null; then
    echo "visual-shot [$mode]: GEEN accent-pixels gevonden — paneel waarschijnlijk niet zichtbaar" >&2
    echo "--- app stderr ---" >&2
    cat "$rt_dir/app-stderr.log" >&2
    finish
    return 1
  fi

  echo "visual-shot [$mode]: OK ($theme, $out)"
  finish
  sleep 1
  return 0
}

run_domains() {
  local theme="$1"
  local prefix="$2"
  local failed=0
  local domain
  for domain in "${DOMAINS[@]}"; do
    run_shot "domain:$domain" "$theme" "${prefix}-${domain}.png" || failed=1
  done
  return "$failed"
}

if [ "$MODE" = "all-domains" ]; then
  PREFIX="${OUT:-/tmp/chefbar-${THEME}-domain}"
  if run_domains "$THEME" "$PREFIX"; then
    echo "visual-shot [all-domains]: alle 15 domeinen OK"
    exit 0
  fi
  echo "visual-shot [all-domains]: één of meer domeinen faalden" >&2
  exit 1
fi

if [ "$MODE" = "all" ]; then
  PREFIX="${OUT:-/tmp/chefbar-${THEME}}"
  FAILED=0
  run_shot panel "$THEME" "${PREFIX}-panel.png" || FAILED=1
  run_shot palette "$THEME" "${PREFIX}-overlay.png" || FAILED=1
  run_shot drawer "$THEME" "${PREFIX}-drawer.png" || FAILED=1
  run_shot density-compact "$THEME" "${PREFIX}-density-compact.png" || FAILED=1
  run_shot density-comfortable "$THEME" "${PREFIX}-density-comfortable.png" || FAILED=1
  run_domains "$THEME" "${PREFIX}-domain" || FAILED=1
  if [ "$FAILED" -ne 0 ]; then
    echo "visual-shot [all]: één of meer shots faalden" >&2
    exit 1
  fi
  echo "visual-shot [all]: panel, overlay, drawer, density en 15 domeinen OK"
  exit 0
fi

if [[ "$MODE" == domain:* ]]; then
  DOMAIN_NAME="${MODE#domain:}"
  OUT="${OUT:-/tmp/chefbar-${THEME}-${DOMAIN_NAME}.png}"
else
  OUT="${OUT:-/tmp/chefbar-${THEME}-${MODE}.png}"
fi
run_shot "$MODE" "$THEME" "$OUT"
