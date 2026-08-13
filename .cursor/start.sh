#!/usr/bin/env bash
# Per-boot Cloud Agent refresh. Runs every start after a snapshot boot
# (install is not re-run). Must terminate; do not start long-lived servers.
set -euo pipefail

# shellcheck source=lib.sh disable=SC1091
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

chefbar_cloud_setup_env
chefbar_cloud_upgrade_apt_packages
chefbar_cloud_install_chrome_wrapper
chefbar_cloud_update_rust
chefbar_cloud_upgrade_bun
chefbar_cloud_ensure_daytona_sandbox

echo "chefbar cloud start: rustc=$(rustc --version 2>/dev/null || echo missing) bun=$(bun --version 2>/dev/null || echo missing)"
