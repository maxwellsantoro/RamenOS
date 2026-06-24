#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_HARDENING_WAVE_B_BATCH2: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# V-04 / SC-04: Unforgeable capability token model
run_gate_test "WBB2_V04_TOKEN_VALIDATION" \
  cargo test -p runtime_supervisor tests::rejects_invalid_token -- --exact
run_gate_test "WBB2_V04_TOKEN_ACCEPT" \
  cargo test -p runtime_supervisor tests::accepts_valid_token -- --exact
run_gate_test "WBB2_V04_TOKEN_FORGED" \
  cargo test -p runtime_supervisor tests::rejects_forged_token -- --exact
run_gate_test "WBB2_V04_TOKEN_IS_VALID" \
  cargo test -p runtime_supervisor tests::token_is_valid_checks_non_zero -- --exact
echo "FOUNDRY_HARDENING_WAVE_B_BATCH2: PASS control=V-04"

# V-05 / SC-05: Kernel capability table with generation counters
run_gate_test "WBB2_V05_HANDLE_ALLOCATE" \
  cargo test -p kernel cap_table::tests::allocate_returns_valid_handle -- --exact
run_gate_test "WBB2_V05_HANDLE_VALIDATE_INVALID" \
  cargo test -p kernel cap_table::tests::validate_rejects_invalid_handle -- --exact
run_gate_test "WBB2_V05_HANDLE_VALIDATE_CURRENT" \
  cargo test -p kernel cap_table::tests::validate_rejects_stale_handle -- --exact
echo "FOUNDRY_HARDENING_WAVE_B_BATCH2: PASS control=V-05"

# V-06 / SC-05: Stale handle rejection via generation mismatch
run_gate_test "WBB2_V06_STALE_REJECTED" \
  cargo test -p kernel cap_table::tests::stale_handle_rejected_via_generation_mismatch -- --exact
run_gate_test "WBB2_V06_DEALLOCATE_STALE" \
  cargo test -p kernel cap_table::tests::deallocate_rejects_stale_handle -- --exact
run_gate_test "WBB2_V06_GENERATION_INCREMENT" \
  cargo test -p kernel cap_table::tests::deallocate_increments_generation -- --exact
echo "FOUNDRY_HARDENING_WAVE_B_BATCH2: PASS control=V-06"

# Handle pack/unpack tests (V-05/V-06)
run_gate_test "WBB2_HANDLE_PACK_UNPACK" \
  cargo test -p kernel_api tests::handle_pack_unpack_roundtrip -- --exact
run_gate_test "WBB2_HANDLE_INVALID_ZERO" \
  cargo test -p kernel_api tests::handle_invalid_is_zero -- --exact
run_gate_test "WBB2_HANDLE_PACK_PRESERVES" \
  cargo test -p kernel_api tests::handle_pack_preserves_index_and_generation -- --exact
run_gate_test "WBB2_HANDLE_UNPACK_RECONSTRUCTS" \
  cargo test -p kernel_api tests::handle_unpack_reconstructs_original -- --exact

# IPC handle validation integration tests (V-05/V-06)
run_gate_test "WBB2_IPC_VALIDATE_INVALID" \
  cargo test -p kernel ipc_v0::tests::validate_handle_accepts_invalid_handle -- --exact
run_gate_test "WBB2_IPC_VALIDATE_CURRENT" \
  cargo test -p kernel ipc_v0::tests::validate_handle_accepts_current_handle -- --exact
run_gate_test "WBB2_IPC_VALIDATE_STALE" \
  cargo test -p kernel ipc_v0::tests::validate_handle_rejects_stale_handle -- --exact

echo "FOUNDRY_HARDENING_WAVE_B_BATCH2: ok"
