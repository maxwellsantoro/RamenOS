#!/usr/bin/env bash
# Strict lint gate for tranche-6 S10/S11 crates.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "FOUNDRY_LINT_STRICT_TRANCHE6: START"

check_crate() {
  local crate="$1"
  echo "FOUNDRY_LINT_STRICT_TRANCHE6: INFO checking crate=${crate}"
  cargo clippy -p "$crate" --all-targets --no-deps -- -D warnings
}

check_crate ramen_sdk
check_crate native_runner
check_crate semantic_state
check_crate execution_fabric
check_crate driver_foundry

echo "FOUNDRY_LINT_STRICT_TRANCHE6: PASS"
