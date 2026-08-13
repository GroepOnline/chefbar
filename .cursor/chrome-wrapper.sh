#!/usr/bin/env bash
# Chrome launcher for Cloud Agent computer-use + Cloudflare browser kits.
# Keeps Cursor's CDP / SwiftShader flags and quiets known-noisy Chromium
# subsystems that fail on these VMs (GCM, crashpad cpufreq, on-device model).
set -euo pipefail

find_real_chrome() {
  local candidate
  for candidate in \
    "${CHEFBAR_CHROME_REAL:-}" \
    /usr/bin/google-chrome-stable \
    /opt/google/chrome/chrome \
    /usr/bin/chromium-browser \
    /usr/bin/chromium
  do
    if [[ -n "$candidate" && -x "$candidate" && "$candidate" != "$0" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  echo "error: no Chrome/Chromium binary found" >&2
  exit 127
}

REAL="$(find_real_chrome)"
USER_DATA_DIR="${CHEFBAR_CHROME_USER_DATA_DIR:-${HOME}/.config/google-chrome}"
mkdir -p "$USER_DATA_DIR"

# Quiet flags:
# - GCM DEPRECATED_ENDPOINT: Google's registration API is gone; disable GCM.
# - crashpad scaling_{cur,max}_freq: Cloud VMs have no cpufreq sysfs.
# - on_device_model service_cli: not available in this image.
exec "$REAL" \
  --no-sandbox \
  --test-type \
  --disable-dev-shm-usage \
  --use-gl=angle \
  --use-angle=swiftshader-webgl \
  --password-store=basic \
  --no-first-run \
  --no-default-browser-check \
  --remote-debugging-port=9222 \
  --user-data-dir="$USER_DATA_DIR" \
  --class=google-chrome \
  --window-size=1820,1100 \
  --window-position=50,50 \
  --disable-crash-reporter \
  --disable-breakpad \
  --disable-component-update \
  --disable-sync \
  --disable-default-apps \
  --disable-client-side-phishing-detection \
  --disable-background-networking \
  --metrics-recording-only \
  --disable-features=GCM,OnDeviceModel,OptimizationHints,TranslateUI,MediaRouter,DialMediaRouteProvider,AutofillServerCommunication,CertificateTransparencyComponentUpdater \
  "$@"
