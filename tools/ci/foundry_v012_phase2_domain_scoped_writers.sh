#!/bin/bash
# V-012 Phase 2: Domain-Scoped Trace Writers Foundry Gate
#
# This gate verifies the implementation of:
# - Domain-scoped TraceWriter with automatic claim release
# - Per-domain trace emit and read operations
# - Cross-domain trace isolation
# - Writer claim semantics and drop behavior
#
# Usage: just foundry-v012-phase2-domain-scoped-writers

set -e

echo "=== V-012 Phase 2: Domain-Scoped Trace Writers ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILURES=0
TOTAL=0

# Test runner function
run_test() {
    local name="$1"
    local command="$2"

    TOTAL=$((TOTAL + 1))
    echo -n "[$TOTAL] Testing: $name ... "

    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        FAILURES=$((FAILURES + 1))
        return 1
    fi
}

# ============================================================================
# Part 1: Per-Domain Trace Buffer Tests (V-012 Phase 1)
# ============================================================================

echo "Part 1: Per-Domain Trace Buffers (V-012 Phase 1)"
echo "-----------------------------------------------"

run_test \
    "Per-domain buffers are isolated" \
    "cargo test --package kernel --lib per_domain_buffers_are_isolated --quiet"

run_test \
    "Domain events don't leak to other domains" \
    "cargo test --package kernel --lib domain_events_dont_leak_to_other_domains --quiet"

run_test \
    "Per-domain ring overflow handling" \
    "cargo test --package kernel --lib per_domain_ring_overflow_handling --quiet"

run_test \
    "Domain ring respects output buffer limit" \
    "cargo test --package kernel --lib domain_ring_respects_output_buffer_limit --quiet"

run_test \
    "Writer release semantics visible to reader" \
    "cargo test --package kernel --lib writer_release_semantics_visible_to_reader --quiet"

echo ""

# ============================================================================
# Part 2: Domain-Scoped Writer Tests (V-012 Phase 2)
# ============================================================================

echo "Part 2: Domain-Scoped Writer Implementation (V-012 Phase 2)"
echo "-----------------------------------------------------------"

run_test \
    "Claim writer returns domain-scoped writer" \
    "cargo test --package kernel --lib claim_writer_returns_domain_scoped_writer --quiet"

run_test \
    "Claim writer fails if already claimed" \
    "cargo test --package kernel --lib claim_writer_fails_if_already_claimed --quiet"

run_test \
    "Claim writer is per-domain" \
    "cargo test --package kernel --lib claim_writer_is_per_domain --quiet"

run_test \
    "Writer drop releases claim" \
    "cargo test --package kernel --lib writer_drop_releases_claim --quiet"

run_test \
    "Writer emit_domain uses correct ring" \
    "cargo test --package kernel --lib writer_emit_domain_uses_correct_ring --quiet"

run_test \
    "Writer emit_domain panics on legacy writer" \
    "cargo test --package kernel --lib writer_emit_domain_panics_on_legacy_writer --quiet"

run_test \
    "Writer domain_id accessor" \
    "cargo test --package kernel --lib writer_domain_id_accessor --quiet"

run_test \
    "Multiple writers per domain isolation" \
    "cargo test --package kernel --lib multiple_writers_per_domain_isolation --quiet"

echo ""

# ============================================================================
# Part 3: Helper Function Tests (V-012 Phase 2)
# ============================================================================

echo "Part 3: Helper Functions (V-012 Phase 2)"
echo "----------------------------------------"

run_test \
    "read_domain_trace helper function" \
    "cargo test --package kernel --lib read_domain_trace_helper --quiet"

run_test \
    "emit_domain_trace helper function" \
    "cargo test --package kernel --lib emit_domain_trace_helper --quiet"

echo ""

# ============================================================================
# Part 4: Cross-Domain Isolation Tests (V-012 Phase 2)
# ============================================================================

echo "Part 4: Cross-Domain Isolation (V-012 Phase 2)"
echo "----------------------------------------------"

run_test \
    "Cross-domain isolation with writers" \
    "cargo test --package kernel --lib cross_domain_isolation_with_writers --quiet"

echo ""

# ============================================================================
# Part 5: Legacy API Compatibility
# ============================================================================

echo "Part 5: Legacy API Compatibility"
echo "--------------------------------"

run_test \
    "Legacy API still works" \
    "cargo test --package kernel --lib legacy_api_still_works --quiet"

run_test \
    "Legacy read skips overwritten events" \
    "cargo test --package kernel --lib legacy_read_skips_overwritten_events --quiet"

echo ""

# ============================================================================
# Part 6: Domain Registry Integration
# ============================================================================

echo "Part 6: Domain Registry Integration"
echo "-----------------------------------"

run_test \
    "Kernel domain pre-registered" \
    "cargo test --package kernel --lib kernel_domain_pre_registered --quiet"

run_test \
    "Register multiple domains" \
    "cargo test --package kernel --lib register_multiple_domains --quiet"

run_test \
    "Register rejects duplicate ID" \
    "cargo test --package kernel --lib register_rejects_duplicate_id --quiet"

echo ""

# ============================================================================
# Part 7: Security Assertions
# ============================================================================

echo "Part 7: Security Assertions"
echo "---------------------------"

# Verify TraceWriter has Drop impl
echo -n "[$((TOTAL + 1))] Checking TraceWriter Drop impl ... "
TOTAL=$((TOTAL + 1))
if grep -q "impl Drop for TraceWriter" kernel/src/trace_ring.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify TraceWriter has domain_id field
echo -n "[$((TOTAL + 1))] Checking TraceWriter domain_id field ... "
TOTAL=$((TOTAL + 1))
if grep -q "domain_id: DomainId" kernel/src/trace_ring.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify emit_domain function exists
echo -n "[$((TOTAL + 1))] Checking emit_domain function ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub fn emit_domain" kernel/src/trace_ring.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify read_domain_trace function exists
echo -n "[$((TOTAL + 1))] Checking read_domain_trace function ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub fn read_domain_trace" kernel/src/trace_ring.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify per-domain writer_claimed flag
echo -n "[$((TOTAL + 1))] Checking per-domain writer_claimed flags ... "
TOTAL=$((TOTAL + 1))
if grep -q "writer_claimed: AtomicBool" kernel/src/trace_ring.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify MAX_DOMAINS constant exists
echo -n "[$((TOTAL + 1))] Checking MAX_DOMAINS constant ... "
TOTAL=$((TOTAL + 1))
if grep -q "MAX_DOMAINS" kernel/src/domain_registry.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

echo ""

# ============================================================================
# Summary
# ============================================================================

echo "=== Summary ==="
echo "Total tests: $TOTAL"
echo "Passed: $((TOTAL - FAILURES))"
echo "Failed: $FAILURES"
echo ""

if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}✓ All V-012 Phase 2 tests passed!${NC}"
    echo ""
    echo "Domain-scoped trace writers are properly implemented with:"
    echo "  - Automatic claim release via Drop trait"
    echo "  - Per-domain emit and read operations"
    echo "  - Cross-domain isolation guarantees"
    echo "  - Helper functions for convenience"
    exit 0
else
    echo -e "${RED}✗ V-012 Phase 2 gate failed with $FAILURES failures${NC}"
    exit 1
fi
