#!/usr/bin/env bash
# V-012 Phase 4: Kernel-side Trace Service Foundry Gate
#
# Validates the kernel-side trace service implementation:
# - Trace buffer creation and destruction
# - Trace reading with capability validation
# - Trace info retrieval
# - Domain-scoped buffer isolation
# - Capability-based access control
# - Error handling for invalid operations

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
EVIDENCE_DIR="$OUT_DIR/evidence/v012_phase4_trace_service"

mkdir -p "$EVIDENCE_DIR"

cd "$ROOT_DIR"

echo "=== V-012 Phase 4: Kernel-side Trace Service Foundry Gate ==="

echo "Building kernel library..."
cargo build -p kernel --lib --no-run > "$EVIDENCE_DIR/build.log" 2>&1 || {
    echo "FAIL: kernel library build failed"
    cat "$EVIDENCE_DIR/build.log"
    exit 1
}
echo "PASS: kernel library built successfully"

# Test 1: Create trace buffer succeeds with valid params
echo "Test 1: Verifying trace buffer creation with valid parameters..."
cargo test -p kernel trace_service::create_trace_buffer_succeeds_with_valid_params -- --nocapture \
    > "$EVIDENCE_DIR/test1_create_success.log" 2>&1 || {
    echo "FAIL: Trace buffer creation test failed"
    cat "$EVIDENCE_DIR/test1_create_success.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test1_create_success.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test1_create_success.log"
    exit 1
}

echo "PASS: Trace buffer creation succeeds with valid parameters"

# Test 2: Create trace buffer fails with invalid domain
echo "Test 2: Verifying trace buffer creation rejects invalid domain..."
cargo test -p kernel trace_service::create_trace_buffer_fails_with_invalid_domain -- --nocapture \
    > "$EVIDENCE_DIR/test2_invalid_domain.log" 2>&1 || {
    echo "FAIL: Invalid domain test failed"
    cat "$EVIDENCE_DIR/test2_invalid_domain.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test2_invalid_domain.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test2_invalid_domain.log"
    exit 1
}

echo "PASS: Trace buffer creation rejects invalid domain"

# Test 3: Create trace buffer fails with invalid size
echo "Test 3: Verifying trace buffer creation rejects invalid size..."
cargo test -p kernel trace_service::create_trace_buffer_fails_with_invalid_size -- --nocapture \
    > "$EVIDENCE_DIR/test3_invalid_size.log" 2>&1 || {
    echo "FAIL: Invalid size test failed"
    cat "$EVIDENCE_DIR/test3_invalid_size.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test3_invalid_size.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test3_invalid_size.log"
    exit 1
}

echo "PASS: Trace buffer creation rejects invalid size"

# Test 4: Create trace buffer fails if buffer exists
echo "Test 4: Verifying trace buffer creation rejects duplicate buffer..."
cargo test -p kernel trace_service::create_trace_buffer_fails_if_buffer_exists -- --nocapture \
    > "$EVIDENCE_DIR/test4_duplicate_buffer.log" 2>&1 || {
    echo "FAIL: Duplicate buffer test failed"
    cat "$EVIDENCE_DIR/test4_duplicate_buffer.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test4_duplicate_buffer.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test4_duplicate_buffer.log"
    exit 1
}

echo "PASS: Trace buffer creation rejects duplicate buffer"

# Test 5: Destroy trace buffer succeeds with valid cap
echo "Test 5: Verifying trace buffer destruction with valid capability..."
cargo test -p kernel trace_service::destroy_trace_buffer_succeeds_with_valid_cap -- --nocapture \
    > "$EVIDENCE_DIR/test5_destroy_success.log" 2>&1 || {
    echo "FAIL: Trace buffer destruction test failed"
    cat "$EVIDENCE_DIR/test5_destroy_success.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test5_destroy_success.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test5_destroy_success.log"
    exit 1
}

echo "PASS: Trace buffer destruction succeeds with valid capability"

# Test 6: Destroy trace buffer fails with invalid cap
echo "Test 6: Verifying trace buffer destruction rejects invalid capability..."
cargo test -p kernel trace_service::destroy_trace_buffer_fails_with_invalid_cap -- --nocapture \
    > "$EVIDENCE_DIR/test6_destroy_invalid_cap.log" 2>&1 || {
    echo "FAIL: Invalid capability destruction test failed"
    cat "$EVIDENCE_DIR/test6_destroy_invalid_cap.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test6_destroy_invalid_cap.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test6_destroy_invalid_cap.log"
    exit 1
}

echo "PASS: Trace buffer destruction rejects invalid capability"

# Test 7: Destroy trace buffer fails with stale generation
echo "Test 7: Verifying trace buffer destruction rejects stale generation..."
cargo test -p kernel trace_service::destroy_trace_buffer_fails_with_stale_generation -- --nocapture \
    > "$EVIDENCE_DIR/test7_stale_generation.log" 2>&1 || {
    echo "FAIL: Stale generation test failed"
    cat "$EVIDENCE_DIR/test7_stale_generation.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test7_stale_generation.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test7_stale_generation.log"
    exit 1
}

echo "PASS: Trace buffer destruction rejects stale generation"

# Test 8: Destroy trace buffer fails without admin rights
echo "Test 8: Verifying trace buffer destruction rejects non-admin..."
cargo test -p kernel trace_service::destroy_trace_buffer_fails_without_admin_rights -- --nocapture \
    > "$EVIDENCE_DIR/test8_no_admin_rights.log" 2>&1 || {
    echo "FAIL: Non-admin rights test failed"
    cat "$EVIDENCE_DIR/test8_no_admin_rights.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test8_no_admin_rights.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test8_no_admin_rights.log"
    exit 1
}

echo "PASS: Trace buffer destruction rejects non-admin capability"

# Test 9: Read trace succeeds with valid cap
echo "Test 9: Verifying trace reading with valid capability..."
cargo test -p kernel trace_service::read_trace_succeeds_with_valid_cap -- --nocapture \
    > "$EVIDENCE_DIR/test9_read_success.log" 2>&1 || {
    echo "FAIL: Trace reading test failed"
    cat "$EVIDENCE_DIR/test9_read_success.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test9_read_success.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test9_read_success.log"
    exit 1
}

echo "PASS: Trace reading succeeds with valid capability"

# Test 10: Read trace fails with invalid cap
echo "Test 10: Verifying trace reading rejects invalid capability..."
cargo test -p kernel trace_service::read_trace_fails_with_invalid_cap -- --nocapture \
    > "$EVIDENCE_DIR/test10_read_invalid_cap.log" 2>&1 || {
    echo "FAIL: Invalid capability read test failed"
    cat "$EVIDENCE_DIR/test10_read_invalid_cap.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test10_read_invalid_cap.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test10_read_invalid_cap.log"
    exit 1
}

echo "PASS: Trace reading rejects invalid capability"

# Test 11: Read trace fails with invalid offset
echo "Test 11: Verifying trace reading rejects invalid offset..."
cargo test -p kernel trace_service::read_trace_fails_with_invalid_offset -- --nocapture \
    > "$EVIDENCE_DIR/test11_invalid_offset.log" 2>&1 || {
    echo "FAIL: Invalid offset read test failed"
    cat "$EVIDENCE_DIR/test11_invalid_offset.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test11_invalid_offset.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test11_invalid_offset.log"
    exit 1
}

echo "PASS: Trace reading rejects invalid offset"

# Test 12: Read trace fails without read rights
echo "Test 12: Verifying trace reading rejects non-read capability..."
cargo test -p kernel trace_service::read_trace_fails_without_read_rights -- --nocapture \
    > "$EVIDENCE_DIR/test12_no_read_rights.log" 2>&1 || {
    echo "FAIL: Non-read rights test failed"
    cat "$EVIDENCE_DIR/test12_no_read_rights.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test12_no_read_rights.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test12_no_read_rights.log"
    exit 1
}

echo "PASS: Trace reading rejects non-read capability"

# Test 13: Get trace info returns correct metadata
echo "Test 13: Verifying trace info retrieval returns correct metadata..."
cargo test -p kernel trace_service::get_trace_info_returns_correct_metadata -- --nocapture \
    > "$EVIDENCE_DIR/test13_info_success.log" 2>&1 || {
    echo "FAIL: Trace info retrieval test failed"
    cat "$EVIDENCE_DIR/test13_info_success.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test13_info_success.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test13_info_success.log"
    exit 1
}

echo "PASS: Trace info retrieval returns correct metadata"

# Test 14: Get trace info fails with invalid cap
echo "Test 14: Verifying trace info retrieval rejects invalid capability..."
cargo test -p kernel trace_service::get_trace_info_fails_with_invalid_cap -- --nocapture \
    > "$EVIDENCE_DIR/test14_info_invalid_cap.log" 2>&1 || {
    echo "FAIL: Invalid capability info test failed"
    cat "$EVIDENCE_DIR/test14_info_invalid_cap.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test14_info_invalid_cap.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test14_info_invalid_cap.log"
    exit 1
}

echo "PASS: Trace info retrieval rejects invalid capability"

# Test 15: Trace service isolation prevents cross-domain access
echo "Test 15: Verifying trace service isolation prevents cross-domain access..."
cargo test -p kernel trace_service::trace_service_isolation_prevents_cross_domain_access -- --nocapture \
    > "$EVIDENCE_DIR/test15_isolation.log" 2>&1 || {
    echo "FAIL: Cross-domain isolation test failed"
    cat "$EVIDENCE_DIR/test15_isolation.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test15_isolation.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test15_isolation.log"
    exit 1
}

echo "PASS: Trace service isolation prevents cross-domain access"

# Test 16: Trace service full lifecycle
echo "Test 16: Verifying trace service full lifecycle..."
cargo test -p kernel trace_service::trace_service_full_lifecycle -- --nocapture \
    > "$EVIDENCE_DIR/test16_lifecycle.log" 2>&1 || {
    echo "FAIL: Full lifecycle test failed"
    cat "$EVIDENCE_DIR/test16_lifecycle.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test16_lifecycle.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test16_lifecycle.log"
    exit 1
}

echo "PASS: Trace service full lifecycle works correctly"

# Test 17: Run all trace service tests
echo "Test 17: Running all trace service tests..."
cargo test -p kernel trace_service -- --nocapture \
    > "$EVIDENCE_DIR/test17_all_tests.log" 2>&1 || {
    echo "FAIL: All trace service tests failed"
    cat "$EVIDENCE_DIR/test17_all_tests.log"
    exit 1
}

# Count passed tests
TEST_COUNT=$(grep -c "test result: ok" "$EVIDENCE_DIR/test17_all_tests.log" || echo "0")
echo "PASS: All trace service tests passed ($TEST_COUNT tests)"

# Summary
echo ""
echo "=== V-012 Phase 4 Trace Service Summary ==="
echo "✓ Trace buffer creation and destruction"
echo "✓ Trace reading with capability validation"
echo "✓ Trace info retrieval"
echo "✓ Domain-scoped buffer isolation"
echo "✓ Capability-based access control"
echo "✓ Error handling for invalid operations"
echo "✓ Full lifecycle test"
echo "✓ All $TEST_COUNT unit tests passed"
echo ""
echo "PASS: V-012 Phase 4 trace service implementation complete"
