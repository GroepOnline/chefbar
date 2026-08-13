#!/usr/bin/env bash
# Cloud Agent install for ChefBar — Rust build/test environment.
# Idempotent and self-contained: works from Cursor's default image.
set -euo pipefail

export RUSTUP_HOME="${RUSTUP_HOME:-/usr/local/rustup}"
export CARGO_HOME="${CARGO_HOME:-/usr/local/cargo}"
export PATH="$CARGO_HOME/bin:$PATH"

# GTK3 development headers: the `gtk 0.18` crate links against GTK3 at compile
# time, so `cargo build` fails without them. No GUI runtime is required.
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends \
  libgtk-3-dev libglib2.0-dev libcairo2-dev libpango1.0-dev \
  libgdk-pixbuf-2.0-dev libatk1.0-dev

# Cargo.lock pins crates that require edition2024, so a stable toolchain
# >= 1.85 is needed (the default image ships an older Rust).
rustup toolchain install stable --profile minimal
rustup default stable

# Warm the build cache: release (shipped, LTO) + debug (fast iteration).
cargo build --release
cargo build
