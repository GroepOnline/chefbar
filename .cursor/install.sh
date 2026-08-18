#!/usr/bin/env bash
# Cloud Agent install for ChefBar — Rust build/test environment plus
# Cloudflare computer-use / Playwright browser-kit OS deps.
# Idempotent and self-contained: works from Cursor's default image.
set -euo pipefail

# shellcheck source=lib.sh disable=SC1091
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

chefbar_cloud_setup_env
chefbar_cloud_ensure_apt_packages
chefbar_cloud_install_chrome_wrapper
chefbar_cloud_ensure_rust
chefbar_cloud_ensure_bun
chefbar_cloud_ensure_daytona_sdk

# Warm the build cache against the committed lockfile.
cargo build --locked --release
cargo build --locked
