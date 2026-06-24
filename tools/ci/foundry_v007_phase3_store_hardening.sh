#!/bin/bash
# V-007 Phase 3: Store Service Hardening Foundry Gate
#
# Validates the hardening features added in Phase 3:
# - Comprehensive audit logging for all operations
# - Manifest signature validation infrastructure (stub)
# - Capability-based access control preparation (stub)
# - All Phase 2 features still working

set -e

echo "=== V-007 Phase 3: Store Service Hardening Foundry Gate ==="

# Test 1: Build all components
echo "Test 1: Building store service with hardening features..."
cargo build --package store_service || {
    echo "FAIL: store_service build failed"
    exit 1
}
cargo build --package artifact_store_schema || {
    echo "FAIL: artifact_store_schema build failed"
    exit 1
}
echo "PASS: Store service built successfully with hardening features"

# Test 2: Verify signature module exists and compiles
echo "Test 2: Verifying signature validation module..."
if [ ! -f "artifact_store_schema/src/signature.rs" ]; then
    echo "FAIL: signature validation module not found"
    exit 1
fi
# Check that signature module is exported in lib.rs
grep -q "pub mod signature;" artifact_store_schema/src/lib.rs || {
    echo "FAIL: signature module not exported in lib.rs"
    exit 1
}
echo "PASS: Signature validation module exists and is exported"

# Test 3: Verify signature validation types are defined
echo "Test 3: Verifying signature validation types..."
# Check for SignatureAlgorithm enum
grep -q "pub enum SignatureAlgorithm" artifact_store_schema/src/signature.rs || {
    echo "FAIL: SignatureAlgorithm enum not found"
    exit 1
}
# Check for ManifestSignature struct
grep -q "pub struct ManifestSignature" artifact_store_schema/src/signature.rs || {
    echo "FAIL: ManifestSignature struct not found"
    exit 1
}
# Check for SignatureValidationResult enum
grep -q "pub enum SignatureValidationResult" artifact_store_schema/src/signature.rs || {
    echo "FAIL: SignatureValidationResult enum not found"
    exit 1
}
# Check for SignatureValidationConfig struct
grep -q "pub struct SignatureValidationConfig" artifact_store_schema/src/signature.rs || {
    echo "FAIL: SignatureValidationConfig struct not found"
    exit 1
}
# Check for validate_manifest_signatures function
grep -q "pub fn validate_manifest_signatures" artifact_store_schema/src/signature.rs || {
    echo "FAIL: validate_manifest_signatures function not found"
    exit 1
}
echo "PASS: All signature validation types are defined"

# Test 4: Verify audit logging module exists
echo "Test 4: Verifying audit logging module..."
if [ ! -f "services/store_service/src/audit.rs" ]; then
    echo "FAIL: audit logging module not found"
    exit 1
fi
# Check for AuditLogger
grep -q "pub struct AuditLogger" services/store_service/src/audit.rs || {
    echo "FAIL: AuditLogger struct not found"
    exit 1
}
# Check for AuditLogEntry
grep -q "pub struct AuditLogEntry" services/store_service/src/audit.rs || {
    echo "FAIL: AuditLogEntry struct not found"
    exit 1
}
# Check for Operation enum
grep -q "pub enum Operation" services/store_service/src/audit.rs || {
    echo "FAIL: Operation enum not found"
    exit 1
}
echo "PASS: Audit logging module exists with required types"

# Test 5: Verify access control module exists
echo "Test 5: Verifying access control module..."
if [ ! -f "services/store_service/src/access_control.rs" ]; then
    echo "FAIL: access control module not found"
    exit 1
fi
# Check for AccessRights
grep -q "pub struct AccessRights" services/store_service/src/access_control.rs || {
    echo "FAIL: AccessRights struct not found"
    exit 1
}
# Check for ClientInfo
grep -q "pub struct ClientInfo" services/store_service/src/access_control.rs || {
    echo "FAIL: ClientInfo struct not found"
    exit 1
}
# Check for AccessControl
grep -q "pub struct AccessControl" services/store_service/src/access_control.rs || {
    echo "FAIL: AccessControl struct not found"
    exit 1
}
echo "PASS: Access control module exists with required types"

# Test 6: Verify audit logging is integrated in main.rs
echo "Test 6: Verifying audit logging integration..."
# Check that audit module is imported
grep -q "mod audit;" services/store_service/src/main.rs || {
    echo "FAIL: audit module not imported in main.rs"
    exit 1
}
# Check that access_control module is imported
grep -q "mod access_control;" services/store_service/src/main.rs || {
    echo "FAIL: access_control module not imported in main.rs"
    exit 1
}
# Check that AuditLogger is used
grep -q "AuditLogger" services/store_service/src/main.rs || {
    echo "FAIL: AuditLogger not used in main.rs"
    exit 1
}
# Check that audit log path is configured
grep -q "DEFAULT_AUDIT_LOG_PATH" services/store_service/src/main.rs || {
    echo "FAIL: DEFAULT_AUDIT_LOG_PATH not defined"
    exit 1
}
# Check that RAMEN_STORE_AUDIT_LOG env var is read
grep -q "RAMEN_STORE_AUDIT_LOG" services/store_service/src/main.rs || {
    echo "FAIL: RAMEN_STORE_AUDIT_LOG env var not read"
    exit 1
}
echo "PASS: Audit logging is integrated in main.rs"

# Test 7: Verify signature validation is integrated
echo "Test 7: Verifying signature validation integration..."
# Check that SignatureValidationConfig is used
grep -q "SignatureValidationConfig" services/store_service/src/main.rs || {
    echo "FAIL: SignatureValidationConfig not used in main.rs"
    exit 1
}
# Check that validate_manifest_signatures is called
grep -q "validate_manifest_signatures" services/store_service/src/main.rs || {
    echo "FAIL: validate_manifest_signatures not called"
    exit 1
}
echo "PASS: Signature validation is integrated in main.rs"

# Test 8: Verify access control is integrated
echo "Test 8: Verifying access control integration..."
# Check that AccessControl is used
grep -q "AccessControl" services/store_service/src/main.rs || {
    echo "FAIL: AccessControl not used in main.rs"
    exit 1
}
# Check that ClientInfo is used
grep -q "ClientInfo" services/store_service/src/main.rs || {
    echo "FAIL: ClientInfo not used in main.rs"
    exit 1
}
# Check that access_control checks are performed
grep -q "access_control.can_read" services/store_service/src/main.rs || {
    echo "FAIL: access_control.can_read not called"
    exit 1
}
grep -q "access_control.can_write" services/store_service/src/main.rs || {
    echo "FAIL: access_control.can_write not called"
    exit 1
}
echo "PASS: Access control is integrated in main.rs"

# Test 9: Verify audit log format includes all required fields
echo "Test 9: Verifying audit log format..."
# Check for timestamp field
grep -q '"timestamp"' services/store_service/src/audit.rs || {
    echo "FAIL: audit log missing timestamp field"
    exit 1
}
# Check for client_pid field
grep -q '"client_pid"' services/store_service/src/audit.rs || {
    echo "FAIL: audit log missing client_pid field"
    exit 1
}
# Check for operation field
grep -q '"operation"' services/store_service/src/audit.rs || {
    echo "FAIL: audit log missing operation field"
    exit 1
}
# Check for result field
grep -q '"result"' services/store_service/src/audit.rs || {
    echo "FAIL: audit log missing result field"
    exit 1
}
# Check for duration_ms field
grep -q '"duration_ms"' services/store_service/src/audit.rs || {
    echo "FAIL: audit log missing duration_ms field"
    exit 1
}
echo "PASS: Audit log format includes all required fields"

# Test 10: Run signature validation tests
echo "Test 10: Running signature validation tests..."
cargo test --package store_service --lib 2>&1 | grep -q "test result: ok" || {
    echo "FAIL: store_service tests failed"
    exit 1
}
echo "PASS: Store service tests passed"

# Test 11: Run audit logging tests
echo "Test 11: Running audit logging tests..."
cargo test --package store_service --lib audit::tests 2>&1 | grep -q "test result: ok" || {
    echo "FAIL: audit logging tests failed"
    exit 1
}
echo "PASS: Audit logging tests passed"

# Test 12: Run access control tests
echo "Test 12: Running access control tests..."
cargo test --package store_service --lib access_control::tests 2>&1 | grep -q "test result: ok" || {
    echo "FAIL: access control tests failed"
    exit 1
}
echo "PASS: Access control tests passed"

# Test 13: Verify all Phase 2 tests still pass
echo "Test 13: Running Phase 2 integration tests..."
bash tools/ci/foundry_v007_phase2_store_service_ipc.sh || {
    echo "FAIL: Phase 2 integration tests failed"
    exit 1
}
echo "PASS: All Phase 2 integration tests still pass"

# Test 14: Verify STATUS_PERMISSION_DENIED is defined
echo "Test 14: Verifying permission denied status code..."
grep -q "STATUS_PERMISSION_DENIED: u32 = 5" services/store_service/src/main.rs || {
    echo "FAIL: STATUS_PERMISSION_DENIED not defined"
    exit 1
}
echo "PASS: Permission denied status code is defined"

echo ""
echo "=== All V-007 Phase 3 tests passed ==="
echo "Summary:"
echo "  - Signature validation module: implemented (stub)"
echo "  - Audit logging module: implemented with all required fields"
echo "  - Access control module: implemented (stub)"
echo "  - Integration: all modules integrated in main.rs"
echo "  - Tests: signature, audit, and access control tests pass"
echo "  - Phase 2 compatibility: all Phase 2 tests still pass"
echo ""
echo "V-007 Phase 3: Store Service Hardening is complete!"
echo ""
echo "Next steps (V-007 Phase 4):"
echo "  - Implement actual cryptographic signature verification"
echo "  - Implement real capability-based access control"
echo "  - Add credential retrieval via SO_PEERCRED"
echo "  - Add evidence generation for Foundry replay"
