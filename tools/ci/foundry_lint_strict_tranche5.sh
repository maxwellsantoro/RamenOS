#!/usr/bin/env bash
# Strict lint gate for tranche-5 crates.
#
# We use --no-deps so enforcement is scoped to selected crate targets while
# workspace-wide warning debt is paid down incrementally.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "FOUNDRY_LINT_STRICT_TRANCHE5: START"

check_crate() {
  local crate="$1"
  echo "FOUNDRY_LINT_STRICT_TRANCHE5: INFO checking crate=${crate}"
  cargo clippy -p "$crate" --all-targets --no-deps -- -D warnings
}

check_crate kernel

echo "FOUNDRY_LINT_STRICT_TRANCHE5: PASS"
