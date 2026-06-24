#!/usr/bin/env bash
# S8 Phase 3: Physical Frame Allocator Foundry Gate
#
# This gate validates the physical memory frame allocator implementation
# including type-safe address handling, FrameAllocator trait, and BumpAllocator.
#
# Total assertions: 25

set -euo pipefail

# ANSI escape codes for colored output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NOCOLOR='\033[0m'

# Gate assertion counters
GATE_TOTAL=0
GATE_PASSED=0
GATE_FAILED=0

# Print gate header
gate_header() {
    local gate_name="$1"
    local description="$2"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
    echo -e "${MAGENTA}Gate: ${gate_name}${NOCOLOR}"
    echo -e "${description}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
}

# Run a single gate test
run_gate_test() {
    local test_name="$1"
    local test_command="$2"

    GATE_TOTAL=$((GATE_TOTAL + 1))
    echo -ne "${BLUE}[${GATE_TOTAL}/${GATE_TOTAL}]${NOCOLOR} ${test_name} ... "

    if eval "${test_command}" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASSED${NOCOLOR}"
        GATE_PASSED=$((GATE_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED${NOCOLOR}"
        GATE_FAILED=$((GATE_FAILED + 1))
        return 1
    fi
}

# Main gate execution
main() {
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════════╗${NOCOLOR}"
    echo -e "${CYAN}║  S8 Phase 3: Physical Frame Allocator Foundry Gate                 ║${NOCOLOR}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════════╝${NOCOLOR}"
    echo ""

    # Module 1: Type-safe physical addresses (PhysAddr)
    gate_header "S8P3_PHYS_ADDR" "Physical address type safety and alignment"

    run_gate_test "S8P3_PHYS_ADDR_NEW" \
        "cargo test -p kernel mm::address::tests::phys_addr_new_wraps_value -- --exact"

    run_gate_test "S8P3_PHYS_ADDR_ALIGN_DOWN" \
        "cargo test -p kernel mm::address::tests::phys_addr_align_down_rounds_to_page -- --exact"

    run_gate_test "S8P3_PHYS_ADDR_ALIGN_UP" \
        "cargo test -p kernel mm::address::tests::phys_addr_align_up_rounds_to_page -- --exact"

    run_gate_test "S8P3_PHYS_ADDR_IS_ALIGNED" \
        "cargo test -p kernel mm::address::tests::phys_addr_is_aligned_detects_alignment -- --exact"

    run_gate_test "S8P3_PHYS_ADDR_OFFSET" \
        "cargo test -p kernel mm::address::tests::phys_addr_offset_calculates_distance -- --exact"

    run_gate_test "S8P3_PHYS_ADDR_OFFSET_PANIC" \
        "cargo test -p kernel mm::address::tests::phys_addr_offset_panics_on_underflow -- --exact"

    # Module 2: Physical frame type (PhysFrame)
    gate_header "S8P3_PHYS_FRAME" "Physical frame type safety"

    run_gate_test "S8P3_PHYS_FRAME_FROM_START" \
        "cargo test -p kernel mm::address::tests::phys_frame_from_start_address_requires_alignment -- --exact"

    run_gate_test "S8P3_PHYS_FRAME_PANIC_MISALIGNED" \
        "cargo test -p kernel mm::address::tests::phys_frame_from_start_address_panics_on_misalignment -- --exact"

    run_gate_test "S8P3_PHYS_FRAME_NUMBER" \
        "cargo test -p kernel mm::address::tests::phys_frame_frame_number_divides_by_page_size -- --exact"

    run_gate_test "S8P3_PHYS_FRAME_FROM_NUMBER" \
        "cargo test -p kernel mm::address::tests::phys_frame_from_frame_number_multiplies_by_page_size -- --exact"

    run_gate_test "S8P3_PHYS_FRAME_INDEX" \
        "cargo test -p kernel mm::address::tests::phys_frame_index_from_calculates_offset -- --exact"

    run_gate_test "S8P3_PHYS_FRAME_INDEX_PANIC" \
        "cargo test -p kernel mm::address::tests::phys_frame_index_from_panics_on_underflow -- --exact"

    # Module 3: FrameAllocator trait and NullAllocator
    gate_header "S8P3_FRAME_ALLOCATOR" "FrameAllocator trait and NullAllocator"

    run_gate_test "S8P3_NULL_ALLOC_NEVER" \
        "cargo test -p kernel mm::frame::tests::null_allocator_never_allocates -- --exact"

    run_gate_test "S8P3_NULL_ALLOC_ZERO" \
        "cargo test -p kernel mm::frame::tests::null_allocator_reports_zero_frames -- --exact"

    run_gate_test "S8P3_NULL_ALLOC_DEALLOC" \
        "cargo test -p kernel mm::frame::tests::null_allocator_deallocate_is_noop -- --exact"

    # Module 4: BumpAllocator implementation
    gate_header "S8P3_BUMP_ALLOCATOR" "BumpAllocator for early boot"

    run_gate_test "S8P3_BUMP_NEW" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_new_creates_valid_allocator -- --exact"

    run_gate_test "S8P3_BUMP_PANIC_BASE" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_new_panics_on_misaligned_base -- --exact"

    run_gate_test "S8P3_BUMP_PANIC_SIZE" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_new_panics_on_non_page_multiple_size -- --exact"

    run_gate_test "S8P3_BUMP_SEQUENTIAL" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_allocates_sequential_frames -- --exact"

    run_gate_test "S8P3_BUMP_EXHAUSTION" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_exhaustion_returns_none -- --exact"

    run_gate_test "S8P3_BUMP_DEALLOC_NOOP" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_deallocate_is_noop -- --exact"

    run_gate_test "S8P3_BUMP_RESET" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_reset_clears_allocations -- --exact"

    run_gate_test "S8P3_BUMP_TOTAL" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_tracks_total_frames -- --exact"

    run_gate_test "S8P3_BUMP_LARGE" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_large_region -- --exact"

    run_gate_test "S8P3_BUMP_PANIC_LARGE" \
        "cargo test -p kernel mm::bump::tests::bump_allocator_panics_on_too_large_region -- --exact"

    # Summary
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
    echo -e "${MAGENTA}S8 Phase 3 Frame Allocator Gate Summary${NOCOLOR}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
    echo -e "Total assertions: ${BLUE}${GATE_TOTAL}${NOCOLOR}"
    echo -e "Passed:           ${GREEN}${GATE_PASSED}${NOCOLOR}"
    echo -e "Failed:           ${RED}${GATE_FAILED}${NOCOLOR}"

    if [ ${GATE_FAILED} -eq 0 ]; then
        echo -e "${GREEN}✓ All gate assertions passed!${NOCOLOR}"
        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
        exit 0
    else
        echo -e "${RED}✗ Gate failed: ${GATE_FAILED} assertion(s) failed${NOCOLOR}"
        echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NOCOLOR}"
        exit 1
    fi
}

# Run main
main "$@"
