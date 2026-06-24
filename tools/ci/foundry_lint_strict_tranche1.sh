#!/usr/bin/env bash
# Strict lint gate for tranche-1 low-noise crates.
#
# We use --no-deps so enforcement is scoped to the selected crate targets,
# avoiding immediate workspace-wide dependency warning failouts.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "FOUNDRY_LINT_STRICT_TRANCHE1: START"

check_crate() {
  local crate="$1"
  echo "FOUNDRY_LINT_STRICT_TRANCHE1: INFO checking crate=${crate}"
  cargo clippy -p "$crate" --all-targets --no-deps -- -D warnings
}

check_crate artifact_store_schema
check_crate store_cli
check_crate domain_manager
check_crate portals

echo "FOUNDRY_LINT_STRICT_TRANCHE1: PASS"
