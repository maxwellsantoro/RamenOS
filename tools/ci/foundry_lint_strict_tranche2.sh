#!/usr/bin/env bash
# Strict lint gate for tranche-2 crates.
#
# We use --no-deps so enforcement is scoped to selected crate targets while
# workspace-wide warning debt is paid down incrementally.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "FOUNDRY_LINT_STRICT_TRANCHE2: START"

check_crate() {
  local crate="$1"
  echo "FOUNDRY_LINT_STRICT_TRANCHE2: INFO checking crate=${crate}"
  cargo clippy -p "$crate" --all-targets --no-deps -- -D warnings
}

check_crate artifact_store_core
check_crate idl_codegen
check_crate capsule_relay
check_crate runtime_supervisor

echo "FOUNDRY_LINT_STRICT_TRANCHE2: PASS"
