#!/usr/bin/env bash
# V-012 Phase 5: User-space Trace Service Client Foundry Gate
#
# Validates the user-space trace client library implementation:
# - Trace client creation and configuration
# - Trace reading and draining operations
# - Trace info retrieval
# - Mock transport for testing
# - Domain manager trace integration
# - Capability-based access control patterns

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
EVIDENCE_DIR="$OUT_DIR/evidence/v012_phase5_trace_client"

mkdir -p "$EVIDENCE_DIR"

cd "$ROOT_DIR"

echo "=== V-012 Phase 5: User-space Trace Service Client Foundry Gate ==="

# Build Phase 5.1: Trace Client Scaffold
echo "Building trace_client library..."
cargo build -p trace_client > "$EVIDENCE_DIR/build_trace_client.log" 2>&1 || {
    echo "FAIL: trace_client build failed"
    cat "$EVIDENCE_DIR/build_trace_client.log"
    exit 1
}
echo "PASS: trace_client library built successfully"

# Build Phase 5.3: Domain Manager Integration
echo "Building domain_manager with trace integration..."
cargo build -p domain_manager > "$EVIDENCE_DIR/build_domain_manager.log" 2>&1 || {
    echo "FAIL: domain_manager build failed"
    cat "$EVIDENCE_DIR/build_domain_manager.log"
    exit 1
}
echo "PASS: domain_manager built successfully"

# Test 1: Trace client builder requires domain ID
echo "Test 1: Verifying trace client builder requires domain_id..."
cargo test -p trace_client client::tests::trace_client_builder_requires_domain_id -- --nocapture \
    > "$EVIDENCE_DIR/test1_builder_requires_domain.log" 2>&1 || {
    echo "FAIL: Builder requires domain_id test failed"
    cat "$EVIDENCE_DIR/test1_builder_requires_domain.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test1_builder_requires_domain.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test1_builder_requires_domain.log"
    exit 1
}

echo "PASS: Trace client builder requires domain_id"

# Test 2: Trace client connects and stores domain ID
echo "Test 2: Verifying trace client connects and stores domain_id..."
cargo test -p trace_client client::tests::trace_client_connect_stores_domain_id -- --nocapture \
    > "$EVIDENCE_DIR/test2_connect_stores_domain.log" 2>&1 || {
    echo "FAIL: Connect stores domain_id test failed"
    cat "$EVIDENCE_DIR/test2_connect_stores_domain.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test2_connect_stores_domain.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test2_connect_stores_domain.log"
    exit 1
}

echo "PASS: Trace client connects and stores domain_id"

# Test 3: Trace client read with mock data
echo "Test 3: Verifying trace client reads data from mock transport..."
cargo test -p trace_client client::tests::trace_client_read_trace_with_mock_data -- --nocapture \
    > "$EVIDENCE_DIR/test3_read_with_data.log" 2>&1 || {
    echo "FAIL: Read with mock data test failed"
    cat "$EVIDENCE_DIR/test3_read_with_data.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test3_read_with_data.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test3_read_with_data.log"
    exit 1
}

echo "PASS: Trace client reads data from mock transport"

# Test 4: Trace client read returns no data when buffer empty
echo "Test 4: Verifying trace client returns NoData when buffer is empty..."
cargo test -p trace_client client::tests::trace_client_read_trace_no_data -- --nocapture \
    > "$EVIDENCE_DIR/test4_read_no_data.log" 2>&1 || {
    echo "FAIL: Read no data test failed"
    cat "$EVIDENCE_DIR/test4_read_no_data.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test4_read_no_data.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test4_read_no_data.log"
    exit 1
}

echo "PASS: Trace client returns NoData when buffer is empty"

# Test 5: Trace client drain collects all data
echo "Test 5: Verifying trace client drain collects all data..."
cargo test -p trace_client client::tests::trace_client_drain_with_mock_data -- --nocapture \
    > "$EVIDENCE_DIR/test5_drain_data.log" 2>&1 || {
    echo "FAIL: Drain data test failed"
    cat "$EVIDENCE_DIR/test5_drain_data.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test5_drain_data.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test5_drain_data.log"
    exit 1
}

echo "PASS: Trace client drain collects all data"

# Test 6: Trace client get_info returns metadata
echo "Test 6: Verifying trace client get_info returns metadata..."
cargo test -p trace_client client::tests::trace_client_get_info_with_mock -- --nocapture \
    > "$EVIDENCE_DIR/test6_get_info.log" 2>&1 || {
    echo "FAIL: Get info test failed"
    cat "$EVIDENCE_DIR/test6_get_info.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test6_get_info.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test6_get_info.log"
    exit 1
}

echo "PASS: Trace client get_info returns metadata"

# Test 7: Trace client disconnect prevents operations
echo "Test 7: Verifying trace client disconnect prevents operations..."
cargo test -p trace_client client::tests::trace_client_disconnect_prevents_operations -- --nocapture \
    > "$EVIDENCE_DIR/test7_disconnect.log" 2>&1 || {
    echo "FAIL: Disconnect test failed"
    cat "$EVIDENCE_DIR/test7_disconnect.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test7_disconnect.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test7_disconnect.log"
    exit 1
}

echo "PASS: Trace client disconnect prevents operations"

# Test 8: Mock transport create trace buffer
echo "Test 8: Verifying mock transport create_trace_buffer..."
cargo test -p trace_client ipc::tests::mock_transport_create_trace_buffer -- --nocapture \
    > "$EVIDENCE_DIR/test8_mock_create.log" 2>&1 || {
    echo "FAIL: Mock create test failed"
    cat "$EVIDENCE_DIR/test8_mock_create.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test8_mock_create.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test8_mock_create.log"
    exit 1
}

echo "PASS: Mock transport create_trace_buffer works"

# Test 9: Mock transport read trace
echo "Test 9: Verifying mock transport read_trace..."
cargo test -p trace_client ipc::tests::mock_transport_read_trace_with_data -- --nocapture \
    > "$EVIDENCE_DIR/test9_mock_read.log" 2>&1 || {
    echo "FAIL: Mock read test failed"
    cat "$EVIDENCE_DIR/test9_mock_read.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test9_mock_read.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test9_mock_read.log"
    exit 1
}

echo "PASS: Mock transport read_trace works"

# Test 10: Status to error mapping
echo "Test 10: Verifying status to error mapping..."
cargo test -p trace_client ipc::tests::status_to_error_permission_denied -- --nocapture \
    > "$EVIDENCE_DIR/test10_status_error.log" 2>&1 || {
    echo "FAIL: Status error mapping test failed"
    cat "$EVIDENCE_DIR/test10_status_error.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test10_status_error.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test10_status_error.log"
    exit 1
}

echo "PASS: Status to error mapping works"

# Test 11: Domain trace manager can be created
echo "Test 11: Verifying domain trace manager can be created..."
cargo test -p domain_manager trace_integration::tests::domain_trace_manager_can_be_created -- --nocapture \
    > "$EVIDENCE_DIR/test11_dm_create.log" 2>&1 || {
    echo "FAIL: Domain trace manager creation test failed"
    cat "$EVIDENCE_DIR/test11_dm_create.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test11_dm_create.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test11_dm_create.log"
    exit 1
}

echo "PASS: Domain trace manager can be created"

# Test 12: Domain trace manager collect on shutdown
echo "Test 12: Verifying domain trace manager collect on shutdown..."
cargo test -p domain_manager trace_integration::tests::domain_trace_manager_collect_on_shutdown_empty -- --nocapture \
    > "$EVIDENCE_DIR/test12_dm_shutdown.log" 2>&1 || {
    echo "FAIL: Domain shutdown collection test failed"
    cat "$EVIDENCE_DIR/test12_dm_shutdown.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test12_dm_shutdown.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test12_dm_shutdown.log"
    exit 1
}

echo "PASS: Domain trace manager collects on shutdown"

# Test 13: Trace collection result
echo "Test 13: Verifying trace collection result..."
cargo test -p domain_manager trace_integration::tests::trace_collection_result_new -- --nocapture \
    > "$EVIDENCE_DIR/test13_result.log" 2>&1 || {
    echo "FAIL: Trace collection result test failed"
    cat "$EVIDENCE_DIR/test13_result.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test13_result.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test13_result.log"
    exit 1
}

echo "PASS: Trace collection result works"

# Test 14: Capability rights checks
echo "Test 14: Verifying capability rights checks..."
cargo test -p trace_client capability::tests::trace_capability_all_rights -- --nocapture \
    > "$EVIDENCE_DIR/test14_caps.log" 2>&1 || {
    echo "FAIL: Capability rights test failed"
    cat "$EVIDENCE_DIR/test14_caps.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test14_caps.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test14_caps.log"
    exit 1
}

echo "PASS: Capability rights checks work"

# Test 15: Run all trace_client tests
echo "Test 15: Running all trace_client tests..."
cargo test -p trace_client -- --nocapture \
    > "$EVIDENCE_DIR/test15_all_trace_client.log" 2>&1 || {
    echo "FAIL: All trace_client tests failed"
    cat "$EVIDENCE_DIR/test15_all_trace_client.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test15_all_trace_client.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test15_all_trace_client.log"
    exit 1
}

echo "PASS: All trace_client tests passed"

# Test 16: Run all domain_manager trace_integration tests
echo "Test 16: Running all domain_manager trace_integration tests..."
cargo test -p domain_manager trace_integration -- --nocapture \
    > "$EVIDENCE_DIR/test16_dm_integration.log" 2>&1 || {
    echo "FAIL: All domain_manager trace_integration tests failed"
    cat "$EVIDENCE_DIR/test16_dm_integration.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test16_dm_integration.log" || {
    echo "FAIL: Expected test result not found"
    cat "$EVIDENCE_DIR/test16_dm_integration.log"
    exit 1
}

echo "PASS: All domain_manager trace_integration tests passed"

# Summary
echo ""
echo "=== V-012 Phase 5 Trace Client Summary ==="
echo "✓ Trace client scaffold (builder pattern, error types)"
echo "✓ Trace operations (read, drain, get_info)"
echo "✓ Mock transport for testing"
echo "✓ Domain manager trace integration"
echo "✓ Capability-based access control patterns"
echo "✓ All unit tests passed"
echo ""
echo "PASS: V-012 Phase 5 trace client implementation complete"
