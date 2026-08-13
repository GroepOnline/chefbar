#!/usr/bin/env bash
# Cloud Agent install for ChefBar — Rust build/test environment.
# Idempotent and self-contained: works from Cursor's default image.
set -euo pipefail

writable_dir() {
  local d="$1"
  mkdir -p "$d" 2>/dev/null || return 1
  [[ -w "$d" ]]
}

# Prefer an existing writable rustup/cargo home. Cursor's default image uses
# /usr/local/{rustup,cargo}; fall back to user homes if those aren't writable.
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
export PATH="$CARGO_HOME/bin:/usr/local/cargo/bin:${PATH:-}"

# GTK3 development headers: the `gtk 0.18` crate links against GTK3 at compile
# time. Skip apt when headers are already present.
if ! pkg-config --exists gtk+-3.0; then
  if ! sudo -n true >/dev/null 2>&1; then
    echo "error: gtk+-3.0 headers missing and passwordless sudo is unavailable" >&2
    echo "install libgtk-3-dev (plus glib/cairo/pango/gdk-pixbuf/atk -dev) then retry" >&2
    exit 1
  fi
  export DEBIAN_FRONTEND=noninteractive
  sudo -n apt-get update -qq
  sudo -n apt-get install -y --no-install-recommends \
    libgtk-3-dev libglib2.0-dev libcairo2-dev libpango1.0-dev \
    libgdk-pixbuf-2.0-dev libatk1.0-dev
fi
pkg-config --exists gtk+-3.0

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup not on PATH" >&2
  exit 1
fi

# Cargo.lock pins crates that require edition2024, so a stable toolchain
# >= 1.85 is needed (the default image ships an older Rust).
rustup toolchain install stable --profile minimal
rustup default stable

# Warm the build cache against the committed lockfile.
cargo build --locked --release
cargo build --locked
