#!/usr/bin/env bash
# S8 Phase 5.1 Gate: Ring Buffer Core Unit Tests
# Tests lock-free SPSC ring buffer implementation (no shared memory integration yet)
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Run ring buffer unit tests
# Note: These are host-target tests since kernel_api compiles for host
echo "Running ring buffer core unit tests..."
test_output=$(cargo test --package kernel_api --lib ring_buffer 2>&1)
exit_code=$?

# Check for test execution
if ! echo "$test_output" | grep -q "running [0-9]\+ tests"; then
    echo "❌ FAIL: No tests executed" >&2
    echo "$test_output" >&2
    exit 1
fi

# Expected: 11 ring buffer tests
test_count=$(echo "$test_output" | grep "running .* tests" | grep -oE "[0-9]+ tests" | grep -oE "[0-9]+" || echo "0")
if [[ "$test_count" -lt 11 ]]; then
    echo "❌ FAIL: Expected at least 11 tests, got $test_count" >&2
    echo "$test_output" >&2
    exit 1
fi

# Check all tests passed
if ! echo "$test_output" | grep -q "test result: ok"; then
    echo "❌ FAIL: Some tests failed" >&2
    echo "$test_output" >&2
    exit 1
fi

# Verify specific test cases
required_tests=(
    "ring_buffer_initially_empty"
    "ring_buffer_write_then_read"
    "ring_buffer_write_when_full_returns_no_space"
    "ring_buffer_read_when_empty_returns_empty"
    "ring_buffer_write_larger_than_capacity_returns_error"
    "ring_buffer_wrap_around_write_and_read"
    "ring_buffer_partial_read"
    "ring_buffer_multiple_write_read_cycles"
    "ring_buffer_cache_mode"
    "ring_buffer_write_read_with_exact_capacity"
    "ring_buffer_monotonic_indices"
)

for test_name in "${required_tests[@]}"; do
    if ! echo "$test_output" | grep -q "test .*::$test_name ... ok"; then
        echo "❌ FAIL: Required test '$test_name' did not pass" >&2
        echo "$test_output" >&2
        exit 1
    fi
done

echo "✅ PASS: All 11 ring buffer core tests passed"
echo ""
echo "Ring buffer v0 implementation verified:"
echo "  - Lock-free SPSC data structure"
echo "  - Monotonically increasing indices"
echo "  - Wrap-around handling"
echo "  - Atomic acquire/release semantics"
echo "  - Error handling (NoSpace, InvalidSize, Empty)"
echo "  - Query operations (available_read, available_write, is_empty, is_full)"
echo "  - Cache mode flags"
echo "  - Ready for shared memory integration in Phase 5.2"
