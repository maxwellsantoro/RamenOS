#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# S8 Phase 4: Data-Plane Foundry Gate
# Total assertions: 40

echo "=== S8 Phase 4: Data-Plane Foundry Gate ==="
echo ""

# Run unit tests (cargo test will build as needed)
echo "[1/7] Running unit tests..."
run_gate_test "UNIT_TEST_BITMAP" \
  cargo test --release -p kernel --lib mm::bitmap
run_gate_test "UNIT_TEST_ADDRESS_SPACE" \
  cargo test --release -p kernel --lib mm::address_space
run_gate_test "UNIT_TEST_MMU" \
  cargo test --release -p kernel --lib arch::mmu
echo "✓ Unit tests passed"
echo ""

# Run integration tests
echo "[2/7] Running integration tests..."
run_gate_test "INTEGRATION_TEST_SHMEM" \
  cargo test --release -p kernel --lib shmem
run_gate_test "INTEGRATION_TEST_IPC" \
  cargo test --release -p kernel --lib ipc_v0
echo "✓ Integration tests passed"
echo ""

# Run assertions
echo "[3/7] Running assertions..."
echo ""

# Module 1: BitmapAllocator (10 assertions)
echo "  BitmapAllocator assertions (10):"
run_gate_test "BITMAP_ALLOCATE_SINGLE" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_allocates_single_frame -- --exact
echo "    ✓ bitmap_allocator_allocates_single_frame"

run_gate_test "BITMAP_ALLOCATE_CONTIGUOUS" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_allocates_contiguous_range -- --exact
echo "    ✓ bitmap_allocator_allocates_contiguous_range"

run_gate_test "BITMAP_REJECT_INSUFFICIENT" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_rejects_insufficient_contiguous -- --exact
echo "    ✓ bitmap_allocator_rejects_insufficient_contiguous"

run_gate_test "BITMAP_DEALLOCATE_SINGLE" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_deallocates_single_frame -- --exact
echo "    ✓ bitmap_allocator_deallocates_single_frame"

run_gate_test "BITMAP_DEALLOCATE_RANGE" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_deallocates_range -- --exact
echo "    ✓ bitmap_allocator_deallocates_range"

run_gate_test "BITMAP_PREVENT_DOUBLE_ALLOC" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_prevents_double_allocation -- --exact
echo "    ✓ bitmap_allocator_prevents_double_allocation"

run_gate_test "BITMAP_PREVENT_DOUBLE_DEALLOC" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_prevents_double_deallocation -- --exact
echo "    ✓ bitmap_allocator_prevents_double_deallocation"

run_gate_test "BITMAP_TRACKS_FREE" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_tracks_free_frames -- --exact
echo "    ✓ bitmap_allocator_tracks_free_frames"

run_gate_test "BITMAP_HANDLES_FRAGMENTATION" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_handles_fragmentation -- --exact
echo "    ✓ bitmap_allocator_handles_fragmentation"

run_gate_test "BITMAP_EXHAUSTION" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_exhaustion_returns_none -- --exact
echo "    ✓ bitmap_allocator_exhaustion_returns_none"
echo ""

# Module 2: AddressSpaceTable (4 assertions)
echo "  AddressSpaceTable assertions (4):"
run_gate_test "ADDRSPACE_INIT_KERNEL_ROOT" \
  cargo test -p kernel mm::address_space::tests::address_space_table_initializes_kernel_root -- --exact
echo "    ✓ address_space_table_initializes_kernel_root"

run_gate_test "ADDRSPACE_SET_DOMAIN_ROOT" \
  cargo test -p kernel mm::address_space::tests::address_space_table_sets_domain_root -- --exact
echo "    ✓ address_space_table_sets_domain_root"

run_gate_test "ADDRSPACE_GET_DOMAIN_ROOT" \
  cargo test -p kernel mm::address_space::tests::address_space_table_gets_domain_root -- --exact
echo "    ✓ address_space_table_gets_domain_root"

run_gate_test "ADDRSPACE_REJECT_INVALID" \
  cargo test -p kernel mm::address_space::tests::address_space_table_rejects_invalid_domain -- --exact
echo "    ✓ address_space_table_rejects_invalid_domain"
echo ""

# Module 3: MMU Programming (6 assertions)
echo "  MMU Programming assertions (6):"
run_gate_test "MMU_MAP_SUCCEEDS" \
  cargo test -p kernel arch::mmu::tests::mmu_map_pages_succeeds_with_valid_params -- --exact
echo "    ✓ mmu_map_pages_succeeds_with_valid_params"

run_gate_test "MMU_MAP_REJECT_INVALID_DOMAIN" \
  cargo test -p kernel arch::mmu::tests::mmu_map_pages_rejects_invalid_domain -- --exact
echo "    ✓ mmu_map_pages_rejects_invalid_domain"

run_gate_test "MMU_MAP_REJECT_MISALIGNED" \
  cargo test -p kernel arch::mmu::tests::mmu_map_pages_rejects_misaligned_address -- --exact
echo "    ✓ mmu_map_pages_rejects_misaligned_address"

run_gate_test "MMU_UNMAP_REMOVES" \
  cargo test -p kernel arch::mmu::tests::mmu_unmap_pages_removes_mapping -- --exact
echo "    ✓ mmu_unmap_pages_removes_mapping"

run_gate_test "MMU_FLUSH_TLB" \
  cargo test -p kernel arch::mmu::tests::mmu_flush_tlb_clears_mappings -- --exact
echo "    ✓ mmu_flush_tlb_clears_mappings"

run_gate_test "MMU_RIGHTS_ENFORCEMENT" \
  cargo test -p kernel arch::mmu::tests::mmu_rights_enforcement_blocks_invalid_access -- --exact
echo "    ✓ mmu_rights_enforcement_blocks_invalid_access"
echo ""

# Module 4: Data-Plane Integration (12 assertions)
echo "  Data-Plane Integration assertions (12):"
run_gate_test "DATAPLANE_CREATE_ALLOCATES" \
  cargo test -p kernel shmem::tests::create_region_allocates_physical_frames -- --exact
echo "    ✓ create_region_allocates_physical_frames"

run_gate_test "DATAPLANE_CREATE_NO_MEMORY" \
  cargo test -p kernel shmem::tests::create_region_returns_no_memory_on_exhaustion -- --exact
echo "    ✓ create_region_returns_no_memory_on_exhaustion"

run_gate_test "DATAPLANE_CREATE_TOO_LARGE" \
  cargo test -p kernel shmem::tests::create_region_rejects_too_large_region -- --exact
echo "    ✓ create_region_rejects_too_large_region"

run_gate_test "DATAPLANE_MAP_PROGRAMS_MMU" \
  cargo test -p kernel shmem::tests::map_region_programs_mmu -- --exact
echo "    ✓ map_region_programs_mmu"

run_gate_test "DATAPLANE_MAP_MMU_ERROR" \
  cargo test -p kernel shmem::tests::map_region_returns_error_on_mmu_failure -- --exact
echo "    ✓ map_region_returns_error_on_mmu_failure"

run_gate_test "DATAPLANE_MAP_INCREMENTS_REFCOUNT" \
  cargo test -p kernel shmem::tests::map_region_increments_refcount -- --exact
echo "    ✓ map_region_increments_refcount"

run_gate_test "DATAPLANE_UNMAP_DECREMENTS" \
  cargo test -p kernel shmem::tests::unmap_region_decrements_refcount -- --exact
echo "    ✓ unmap_region_decrements_refcount"

run_gate_test "DATAPLANE_CLOSE_DEALLOCATES" \
  cargo test -p kernel shmem::tests::close_region_deallocates_frames -- --exact
echo "    ✓ close_region_deallocates_frames"

run_gate_test "DATAPLANE_CLOSE_ACTIVE_FAILS" \
  cargo test -p kernel shmem::tests::close_region_fails_with_active_mappings -- --exact
echo "    ✓ close_region_fails_with_active_mappings"

run_gate_test "DATAPLANE_CLOSE_AFTER_UNMAP" \
  cargo test -p kernel shmem::tests::close_region_succeeds_after_unmap -- --exact
echo "    ✓ close_region_succeeds_after_unmap"

run_gate_test "DATAPLANE_GENERATION_PREVENTS_STALE" \
  cargo test -p kernel shmem::tests::generation_counter_prevents_stale_access -- --exact
echo "    ✓ generation_counter_prevents_stale_access"

run_gate_test "DATAPLANE_CAP_VALIDATION" \
  cargo test -p kernel shmem::tests::capability_validation_required_for_map_unmap_close -- --exact
echo "    ✓ capability_validation_required_for_map_unmap_close"
echo ""

# Module 5: IPC Handlers (4 assertions)
echo "  IPC Handlers assertions (4):"
run_gate_test "IPC_CREATE_TRACE" \
  cargo test -p kernel ipc_v0::tests::ipc_create_region_emits_trace_event -- --exact
echo "    ✓ ipc_create_region_emits_trace_event"

run_gate_test "IPC_MAP_TRACE" \
  cargo test -p kernel ipc_v0::tests::ipc_map_region_emits_trace_event -- --exact
echo "    ✓ ipc_map_region_emits_trace_event"

run_gate_test "IPC_CLOSE_TRACE" \
  cargo test -p kernel ipc_v0::tests::ipc_close_region_emits_trace_event -- --exact
echo "    ✓ ipc_close_region_emits_trace_event"

run_gate_test "IPC_ERROR_CODES" \
  cargo test -p kernel ipc_v0::tests::ipc_handlers_return_correct_error_codes -- --exact
echo "    ✓ ipc_handlers_return_correct_error_codes"
echo ""

# Module 6: Boot Integration (4 assertions)
echo "  Boot Integration assertions (4):"
run_gate_test "BOOT_BITMAP_INIT" \
  cargo test -p kernel mm::bitmap::tests::bitmap_allocator_initialized_from_boot_map -- --exact
echo "    ✓ bitmap_allocator_initialized_from_boot_map"

run_gate_test "BOOT_ADDRSPACE_INIT" \
  cargo test -p kernel mm::address_space::tests::address_space_table_initialized_on_boot -- --exact
echo "    ✓ address_space_table_initialized_on_boot"

run_gate_test "BOOT_KERNEL_ROOT" \
  cargo test -p kernel mm::address_space::tests::kernel_page_table_root_registered -- --exact
echo "    ✓ kernel_page_table_root_registered"

run_gate_test "BOOT_MEMORY_EXCLUDED" \
  cargo test -p kernel mm::bitmap::tests::boot_memory_regions_excluded_from_allocation -- --exact
echo "    ✓ boot_memory_regions_excluded_from_allocation"
echo ""

# Module 7: End-to-End Scenarios (4 assertions)
echo "  End-to-End Scenarios assertions (4):"
run_gate_test "E2E_CREATE_MAP_UNMAP_CLOSE" \
  cargo test -p kernel shmem::tests::end_to_end_create_map_unmap_close -- --exact
echo "    ✓ end_to_end_create_map_unmap_close"

run_gate_test "E2E_MULTIPLE_DOMAINS_SHARE" \
  cargo test -p kernel shmem::tests::multiple_domains_share_region -- --exact
echo "    ✓ multiple_domains_share_region"

run_gate_test "E2E_REGION_REUSE" \
  cargo test -p kernel shmem::tests::region_reuse_after_close -- --exact
echo "    ✓ region_reuse_after_close"

run_gate_test "E2E_ERROR_RECOVERY" \
  cargo test -p kernel shmem::tests::error_recovery_on_allocation_failure -- --exact
echo "    ✓ error_recovery_on_allocation_failure"
echo ""

# Summary
echo "[4/7] Summary:"
echo "  BitmapAllocator: 10/10 assertions passed"
echo "  AddressSpaceTable: 4/4 assertions passed"
echo "  MMU Programming: 6/6 assertions passed"
echo "  Data-Plane Integration: 12/12 assertions passed"
echo "  IPC Handlers: 4/4 assertions passed"
echo "  Boot Integration: 4/4 assertions passed"
echo "  End-to-End Scenarios: 4/4 assertions passed"
echo "  Total: 40/40 assertions passed"
echo ""

echo "=== S8 Phase 4: All Tests Passed ==="
echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4: ok"
