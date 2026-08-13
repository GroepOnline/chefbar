#!/usr/bin/env bash
# Shared Cloud Agent helpers. Sourced by install.sh and start.sh — not executed.
# shellcheck shell=bash

writable_dir() {
  local d="$1"
  mkdir -p "$d" 2>/dev/null || return 1
  [[ -w "$d" ]]
}

# Set when this file is sourced so callers can find chrome-wrapper.sh.
_CHEFBAR_CURSOR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

chefbar_cloud_cursor_dir() {
  printf '%s\n' "$_CHEFBAR_CURSOR_DIR"
}

# GTK3 headers for the gtk 0.18 crate, plus Chromium/Playwright OS libs used by
# Cloudflare computer-use / @cloudflare/playwright / browser-kit flows.
chefbar_cloud_apt_packages() {
  cat <<'EOF'
libgtk-3-dev
libglib2.0-dev
libcairo2-dev
libpango1.0-dev
libgdk-pixbuf-2.0-dev
libatk1.0-dev
libnss3
libnspr4
libatk-bridge2.0-0t64
libcups2t64
libdrm2
libxkbcommon0
libxcomposite1
libxdamage1
libxfixes3
libxrandr2
libgbm1
libasound2t64
libxshmfence1
fonts-liberation
ca-certificates
curl
unzip
EOF
}

chefbar_cloud_setup_env() {
  if [[ -z "${RUSTUP_HOME:-}" ]]; then
    if writable_dir /usr/local/rustup; then
      export RUSTUP_HOME=/usr/local/rustup
    else
      export RUSTUP_HOME="$HOME/.rustup"
      mkdir -p "$RUSTUP_HOME"
    fi
  fi
  if [[ -z "${CARGO_HOME:-}" ]]; then
    if writable_dir /usr/local/cargo; then
      export CARGO_HOME=/usr/local/cargo
    else
      export CARGO_HOME="$HOME/.cargo"
      mkdir -p "$CARGO_HOME"
    fi
  fi
  export PATH="$HOME/.bun/bin:$CARGO_HOME/bin:/usr/local/cargo/bin:${PATH:-}"
}

chefbar_cloud_have_sudo() {
  sudo -n true >/dev/null 2>&1
}

chefbar_cloud_pkg_installed() {
  local pkg="$1"
  dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q 'install ok installed' && return 0
  # Ubuntu 24.04 time64 transition: libfoo -> libfoo t64.
  if [[ "$pkg" != *t64 ]]; then
    dpkg-query -W -f='${Status}' "${pkg}t64" 2>/dev/null | grep -q 'install ok installed' && return 0
  fi
  return 1
}

chefbar_cloud_ensure_apt_packages() {
  local pkg
  local -a missing=()
  local -a wanted=()

  mapfile -t wanted < <(chefbar_cloud_apt_packages)
  for pkg in "${wanted[@]}"; do
    [[ -z "$pkg" ]] && continue
    if ! chefbar_cloud_pkg_installed "$pkg"; then
      missing+=("$pkg")
    fi
  done

  if [[ ${#missing[@]} -eq 0 ]]; then
    pkg-config --exists gtk+-3.0
    return 0
  fi

  if ! chefbar_cloud_have_sudo; then
    echo "error: missing packages (${missing[*]}) and passwordless sudo is unavailable" >&2
    exit 1
  fi

  export DEBIAN_FRONTEND=noninteractive
  sudo -n apt-get update -qq
  # Ubuntu 24.04 uses t64 names; fall back to the unversioned package on older images.
  if ! sudo -n apt-get install -y --no-install-recommends "${missing[@]}"; then
    local -a fallback=()
    for pkg in "${missing[@]}"; do
      if [[ "$pkg" == *t64 ]]; then
        fallback+=("${pkg%t64}")
      else
        fallback+=("$pkg")
      fi
    done
    sudo -n apt-get install -y --no-install-recommends "${fallback[@]}"
  fi
  pkg-config --exists gtk+-3.0
}

chefbar_cloud_upgrade_apt_packages() {
  if ! chefbar_cloud_have_sudo; then
    echo "skip apt refresh: no passwordless sudo"
    return 0
  fi
  export DEBIAN_FRONTEND=noninteractive
  sudo -n apt-get update -qq
  chefbar_cloud_ensure_apt_packages
  local -a wanted=()
  mapfile -t wanted < <(chefbar_cloud_apt_packages)
  # Upgrade only the Cloud Agent toolchain packages, not a full dist-upgrade.
  # A world `apt-get upgrade` is slow and can break the snapshot image.
  sudo -n apt-get install -y --only-upgrade --no-install-recommends "${wanted[@]}" || true
}

chefbar_cloud_install_chrome_wrapper() {
  local src dest
  src="$(chefbar_cloud_cursor_dir)/chrome-wrapper.sh"
  if [[ ! -f "$src" ]]; then
    echo "error: chrome wrapper missing: $src" >&2
    exit 1
  fi
  chmod +x "$src"

  mkdir -p "$HOME/.local/bin"
  cp "$src" "$HOME/.local/bin/chefbar-chrome"
  chmod +x "$HOME/.local/bin/chefbar-chrome"

  if ! chefbar_cloud_have_sudo; then
    echo "skip system chrome wrapper: no passwordless sudo"
    return 0
  fi

  for dest in /usr/local/bin/google-chrome /usr/local/bin/chrome; do
    if [[ -e "$dest" && ! -e "${dest}.cursor-orig" ]]; then
      sudo -n cp -a "$dest" "${dest}.cursor-orig" || true
    fi
    sudo -n cp "$src" "$dest"
    sudo -n chmod 0755 "$dest"
  done
}

chefbar_cloud_ensure_bun() {
  if command -v bun >/dev/null 2>&1; then
    return 0
  fi
  if ! command -v curl >/dev/null 2>&1; then
    echo "warn: curl missing; skip bun install" >&2
    return 0
  fi
  curl -fsSL https://bun.sh/install | BUN_INSTALL="${HOME}/.bun" bash
  export PATH="$HOME/.bun/bin:${PATH}"
}

chefbar_cloud_upgrade_bun() {
  if ! command -v bun >/dev/null 2>&1; then
    chefbar_cloud_ensure_bun
    return 0
  fi
  bun upgrade || true
}

chefbar_cloud_ensure_rust() {
  if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup not on PATH" >&2
    exit 1
  fi
  rustup toolchain install stable --profile minimal
  rustup default stable
}

chefbar_cloud_update_rust() {
  chefbar_cloud_ensure_rust
  rustup update stable
}

chefbar_cloud_daytona_venv() {
  printf '%s\n' "${HOME}/.local/share/chefbar/daytona-venv"
}

chefbar_cloud_ensure_daytona_sdk() {
  local venv
  venv="$(chefbar_cloud_daytona_venv)"
  if [[ -x "$venv/bin/python" ]] && "$venv/bin/python" -c "import daytona" >/dev/null 2>&1; then
    return 0
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "warn: python3 missing; skip daytona SDK" >&2
    return 0
  fi
  python3 -m venv "$venv"
  "$venv/bin/pip" install -U pip daytona
}

chefbar_cloud_ensure_daytona_sandbox() {
  local repo python helper
  if [[ -z "${DAYTONA_API_KEY:-}" ]]; then
    echo "skip daytona sandbox: DAYTONA_API_KEY not set"
    return 0
  fi
  repo="$(cd "$(chefbar_cloud_cursor_dir)/.." && pwd)"
  helper="$repo/scripts/daytona-emergency.py"
  if [[ ! -f "$helper" ]]; then
    echo "warn: missing $helper" >&2
    return 0
  fi
  chefbar_cloud_ensure_daytona_sdk || true
  python="$(chefbar_cloud_daytona_venv)/bin/python"
  if [[ ! -x "$python" ]]; then
    python="python3"
  fi
  # Lifecycle is best-effort: a Daytona outage must not block the Cloud Agent.
  if ! "$python" "$helper" --ensure --refresh; then
    echo "warn: daytona ensure/refresh failed (Cloud Agent continues)" >&2
  fi
}
