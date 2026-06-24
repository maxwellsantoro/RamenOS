#!/bin/bash
# V-012 Phase 3: Trace Capability-Based Access Control Foundry Gate
#
# This gate verifies the implementation of:
# - Trace capability table with domain-scoped rights
# - READ/WRITE/ADMIN permission checking
# - Capability grant and revoke operations
# - Generation counter for capability validation
#
# Usage: just foundry-v012-phase3-trace-capabilities

set -e

echo "=== V-012 Phase 3: Trace Capability-Based Access Control ==="
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
# Part 1: Trace Capability Table Tests
# ============================================================================

echo "Part 1: Trace Capability Table"
echo "-------------------------------"

run_test \
    "Allocate trace cap succeeds" \
    "cargo test --package kernel --lib allocate_trace_cap_succeeds --quiet"

run_test \
    "Allocate rejects invalid domain" \
    "cargo test --package kernel --lib allocate_rejects_invalid_domain --quiet"

run_test \
    "Deallocate frees slot" \
    "cargo test --package kernel --lib deallocate_frees_slot --quiet"

run_test \
    "Deallocate increments generation" \
    "cargo test --package kernel --lib deallocate_increments_generation --quiet"

run_test \
    "Allocate multiple caps" \
    "cargo test --package kernel --lib allocate_multiple_caps --quiet"

echo ""

# ============================================================================
# Part 2: Rights Management Tests
# ============================================================================

echo "Part 2: Rights Management"
echo "-------------------------"

run_test \
    "Grant rights adds permissions" \
    "cargo test --package kernel --lib grant_rights_adds_permissions --quiet"

run_test \
    "Revoke rights removes permissions" \
    "cargo test --package kernel --lib revoke_rights_removes_permissions --quiet"

run_test \
    "Check right validates permissions" \
    "cargo test --package kernel --lib check_right_validates_permissions --quiet"

run_test \
    "TraceCap convenience methods" \
    "cargo test --package kernel --lib trace_cap_convenience_methods --quiet"

echo ""

# ============================================================================
# Part 3: Domain Integration Tests
# ============================================================================

echo "Part 3: Domain Integration"
echo "-------------------------"

run_test \
    "Get domain ID returns correct domain" \
    "cargo test --package kernel --lib get_domain_id_returns_correct_domain --quiet"

echo ""

# ============================================================================
# Part 4: Security Assertions
# ============================================================================

echo "Part 4: Security Assertions"
echo "---------------------------"

# Verify trace rights constants exist
echo -n "[$((TOTAL + 1))] Checking TRACE_RIGHT_READ constant ... "
TOTAL=$((TOTAL + 1))
if grep -q "TRACE_RIGHT_READ.*0x01" kernel_api/src/lib.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

echo -n "[$((TOTAL + 1))] Checking TRACE_RIGHT_WRITE constant ... "
TOTAL=$((TOTAL + 1))
if grep -q "TRACE_RIGHT_WRITE.*0x02" kernel_api/src/lib.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

echo -n "[$((TOTAL + 1))] Checking TRACE_RIGHT_ADMIN constant ... "
TOTAL=$((TOTAL + 1))
if grep -q "TRACE_RIGHT_ADMIN.*0x04" kernel_api/src/lib.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

echo -n "[$((TOTAL + 1))] Checking TRACE_RIGHT_ALL constant ... "
TOTAL=$((TOTAL + 1))
if grep -q "TRACE_RIGHT_ALL.*0x07" kernel_api/src/lib.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify TraceCapTable exists
echo -n "[$((TOTAL + 1))] Checking TraceCapTable type ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub struct TraceCapTable" kernel/src/trace_cap.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify TraceCap exists
echo -n "[$((TOTAL + 1))] Checking TraceCap type ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub struct TraceCap" kernel/src/trace_cap.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify TraceCap has domain_id field
echo -n "[$((TOTAL + 1))] Checking TraceCap domain_id field ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub domain_id: DomainId" kernel/src/trace_cap.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify TraceCap has rights_mask field
echo -n "[$((TOTAL + 1))] Checking TraceCap rights_mask field ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub rights_mask: u8" kernel/src/trace_cap.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

# Verify generation counter exists
echo -n "[$((TOTAL + 1))] Checking TraceCap generation field ... "
TOTAL=$((TOTAL + 1))
if grep -q "pub generation: u64" kernel/src/trace_cap.rs; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    FAILURES=$((FAILURES + 1))
fi

echo ""

# ============================================================================
# Part 5: Integration Tests
# ============================================================================

echo "Part 5: Integration Tests"
echo "-------------------------"

run_test \
    "Trace capability module compiles" \
    "cargo check --package kernel --lib --quiet"

run_test \
    "Kernel API with trace rights compiles" \
    "cargo check --package kernel_api --lib --quiet"

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
    echo -e "${GREEN}✓ All V-012 Phase 3 tests passed!${NC}"
    echo ""
    echo "Trace capability-based access control is properly implemented with:"
    echo "  - Domain-scoped trace capabilities"
    echo "  - Fine-grained rights (READ/WRITE/ADMIN)"
    echo "  - Capability grant and revoke operations"
    echo "  - Generation counter for validation"
    exit 0
else
    echo -e "${RED}✗ V-012 Phase 3 gate failed with $FAILURES failures${NC}"
    exit 1
fi
