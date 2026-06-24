#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_SHMEM_CONTROL_S8_PHASE2: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# Control-plane contract tests: CreateRegion, MapRegion, UnmapRegion, CloseRegion
run_gate_test "S8P2_CREATE_SUCCESS" \
  cargo test -p kernel shmem::tests::create_region_succeeds_with_valid_parameters -- --exact

run_gate_test "S8P2_CREATE_REJECTS_ZERO_SIZE" \
  cargo test -p kernel shmem::tests::create_region_rejects_zero_size -- --exact

run_gate_test "S8P2_CREATE_REJECTS_INVALID_PAGE_SIZE" \
  cargo test -p kernel shmem::tests::create_region_rejects_invalid_page_size -- --exact

run_gate_test "S8P2_CREATE_REJECTS_UNKNOWN_FLAGS" \
  cargo test -p kernel shmem::tests::create_region_rejects_unknown_flags -- --exact

run_gate_test "S8P2_MAP_INCREMENTS_REFCOUNT" \
  cargo test -p kernel shmem::tests::map_region_increments_refcount -- --exact

run_gate_test "S8P2_MAP_MULTIPLE_INCREMENTS" \
  cargo test -p kernel shmem::tests::map_region_multiple_times_increments_refcount -- --exact

run_gate_test "S8P2_UNMAP_DECREMENTS_REFCOUNT" \
  cargo test -p kernel shmem::tests::unmap_region_decrements_refcount -- --exact

run_gate_test "S8P2_CLOSE_REJECTS_ACTIVE_MAPPINGS" \
  cargo test -p kernel shmem::tests::close_region_fails_with_active_mappings -- --exact

run_gate_test "S8P2_CLOSE_SUCCEEDS_AFTER_UNMAP" \
  cargo test -p kernel shmem::tests::close_region_succeeds_after_all_unmaps -- --exact

run_gate_test "S8P2_GENERATION_COUNTER" \
  cargo test -p kernel shmem::tests::close_region_increments_generation_counter -- --exact

run_gate_test "S8P2_GENERATION_WRAPS" \
  cargo test -p kernel shmem::tests::generation_counter_wraps_avoids_zero -- --exact

# Capability validation tests
run_gate_test "S8P2_MAP_REJECTS_INVALID_CAP" \
  cargo test -p kernel shmem::tests::map_region_rejects_invalid_capability -- --exact

run_gate_test "S8P2_MAP_REJECTS_UNKNOWN_REGION" \
  cargo test -p kernel shmem::tests::map_region_rejects_unknown_region -- --exact

run_gate_test "S8P2_MAP_CHECKS_RIGHTS" \
  cargo test -p kernel shmem::tests::map_region_checks_rights_against_flags -- --exact

run_gate_test "S8P2_VALIDATE_CAP_REJECTS_STALE" \
  cargo test -p kernel shmem::tests::validate_cap_rejects_stale_handle -- --exact

# IPC integration tests
run_gate_test "S8P2_IPC_CREATE_SUCCESS" \
  cargo test -p kernel ipc_v0::tests::handle_create_region_succeeds_with_valid_params -- --exact

run_gate_test "S8P2_IPC_CREATE_REJECTS_INVALID_PAGE_SIZE" \
  cargo test -p kernel ipc_v0::tests::handle_create_region_rejects_invalid_page_size -- --exact

run_gate_test "S8P2_IPC_MAP_REJECTS_INVALID_CAP" \
  cargo test -p kernel ipc_v0::tests::handle_map_region_without_capability_fails -- --exact

run_gate_test "S8P2_IPC_MAP_AND_UNMAP_SUCCESS" \
  cargo test -p kernel ipc_v0::tests::handle_map_and_unmap_region_succeeds_with_capability -- --exact

run_gate_test "S8P2_IPC_CLOSE_REJECTS_ACTIVE_MAPPINGS" \
  cargo test -p kernel ipc_v0::tests::handle_close_region_fails_with_active_mappings -- --exact

# Kernel capability table tests (SC-05/V-05/V-06)
run_gate_test "S8P2_CAP_ALLOCATE" \
  cargo test -p kernel cap_table::tests::allocate_returns_valid_handle -- --exact

run_gate_test "S8P2_CAP_VALIDATE_INVALID" \
  cargo test -p kernel cap_table::tests::validate_rejects_invalid_handle -- --exact

run_gate_test "S8P2_CAP_VALIDATE_STALE" \
  cargo test -p kernel cap_table::tests::validate_rejects_stale_handle -- --exact

run_gate_test "S8P2_CAP_STALE_REJECTED" \
  cargo test -p kernel cap_table::tests::stale_handle_rejected_via_generation_mismatch -- --exact

run_gate_test "S8P2_CAP_DEALLOCATE_STALE" \
  cargo test -p kernel cap_table::tests::deallocate_rejects_stale_handle -- --exact

run_gate_test "S8P2_CAP_GENERATION_INCREMENT" \
  cargo test -p kernel cap_table::tests::deallocate_increments_generation -- --exact

run_gate_test "S8P2_IPC_VALIDATE_INVALID" \
  cargo test -p kernel ipc_v0::tests::validate_handle_rejects_invalid_handle -- --exact

run_gate_test "S8P2_IPC_VALIDATE_CURRENT" \
  cargo test -p kernel ipc_v0::tests::validate_handle_accepts_current_handle -- --exact

run_gate_test "S8P2_IPC_VALIDATE_STALE" \
  cargo test -p kernel ipc_v0::tests::validate_handle_rejects_stale_handle -- --exact

echo "FOUNDRY_SHMEM_CONTROL_S8_PHASE2: ok"
