#!/usr/bin/env bash
# ChefBar QA-gate (Q1/Q2): draait precies de checks die CI draait — op een
# machine mét toolchain. Dat is de self-hosted runner (chef-runner-01-1),
# nooit de laptop: daar is cargo bewust afwezig (harde regel, zie README).
#
#   # op chef-runner-01-1, in een checkout/kopie van de repo:
#   scripts/gate.sh
#
# Exit 0 = gate groen (fmt · clippy · tests), anders 1.
set -euo pipefail

cd "$(dirname "$0")/.." || exit 1
export PATH="${HOME}/.cargo/bin:${PATH}"

command -v cargo >/dev/null 2>&1 || {
  echo "gate: cargo niet gevonden — draai dit op de runner (chef-runner-01-1)" >&2
  exit 1
}

echo "== fmt =="
cargo fmt --check
echo "== clippy (-D warnings) =="
cargo clippy --all-targets -- -D warnings
echo "== tests =="
cargo test --all-targets
echo "gate: OK"
