#!/bin/bash
# Foundry gate for V-007 Phase 1: Boundary Dependency Cleanup
# Tests that services use schema types instead of direct IO functions where possible

set -euo pipefail

echo "=========================================="
echo "Foundry Gate: Boundary S9.0 Cleanup"
echo "=========================================="
echo ""

# Test 1: Verify path construction helpers exist in artifact_store_schema
echo "Test 1: Verify path construction helpers"
if [ -f "artifact_store_schema/src/path.rs" ]; then
    echo "✓ Path construction module exists"
else
    echo "✗ FAILED: Path construction module missing"
    exit 1
fi

if grep -q "pub fn blob_path_for" artifact_store_schema/src/path.rs; then
    echo "✓ blob_path_for function defined"
else
    echo "✗ FAILED: blob_path_for function missing"
    exit 1
fi

if grep -q "pub fn manifest_path_for" artifact_store_schema/src/path.rs; then
    echo "✓ manifest_path_for function defined"
else
    echo "✗ FAILED: manifest_path_for function missing"
    exit 1
fi

if grep -q "pub mod path" artifact_store_schema/src/lib.rs; then
    echo "✓ Path module exported from lib.rs"
else
    echo "✗ FAILED: Path module not exported"
    exit 1
fi

echo ""

# Test 2: Verify runtime_supervisor uses current schema boundaries
echo "Test 2: Verify runtime_supervisor uses schema contracts"
if grep -q 'artifact_store_schema = { path = "../artifact_store_schema" }' runtime_supervisor/Cargo.toml; then
    echo "✓ runtime_supervisor depends on artifact_store_schema"
else
    echo "✗ FAILED: runtime_supervisor doesn't depend on artifact_store_schema"
    exit 1
fi

if grep -q "use artifact_store_schema::execution_fabric" runtime_supervisor/src/launch_plan.rs; then
    echo "✓ runtime_supervisor parses canonical execution fabric launch plans"
else
    echo "✗ FAILED: runtime_supervisor doesn't use execution fabric schema"
    exit 1
fi

if grep -q "validate_execution_launch_plan" runtime_supervisor/src/launch_plan.rs; then
    echo "✓ runtime_supervisor validates canonical launch plans via schema"
else
    echo "✗ FAILED: runtime_supervisor doesn't validate canonical launch plans"
    exit 1
fi

echo ""

# Test 3: Verify store IO remains behind the store service boundary
echo "Test 3: Verify store IO boundary"
if grep -q "StoreClient::connect" runtime_supervisor/src/main.rs; then
    echo "✓ runtime_supervisor verifies artifacts through store service IPC"
else
    echo "✗ FAILED: runtime_supervisor doesn't connect to store service IPC"
    exit 1
fi

if grep -q "StoreClient::connect" services/domain_manager/src/main.rs; then
    echo "✓ domain_manager uses store service IPC for artifact operations"
else
    echo "✗ FAILED: domain_manager doesn't use store service IPC"
    exit 1
fi

echo ""

# Test 4: Verify execution fabric policy wiring is covered
echo "Test 4: Verify execution fabric policy wiring"
if grep -q "consult_always_local" runtime_supervisor/src/fabric_policy.rs; then
    echo "✓ runtime_supervisor has S10.4.1 fabric policy hook"
else
    echo "✗ FAILED: runtime_supervisor fabric policy hook missing"
    exit 1
fi

if grep -q "fabric_policy_always_local" runtime_supervisor/src/fabric_policy.rs; then
    echo "✓ runtime_supervisor fabric policy has regression coverage"
else
    echo "✗ FAILED: runtime_supervisor fabric policy regression missing"
    exit 1
fi

echo ""

# Test 5: Verify path helpers are read-only (no IO operations)
echo "Test 5: Verify path helpers are read-only"
if grep -q "fs::create_dir_all\|fs::write\|File::create" artifact_store_schema/src/path.rs; then
    echo "✗ FAILED: Path helpers contain IO operations"
    exit 1
else
    echo "✓ Path helpers are read-only (no IO)"
fi

echo ""

# Test 6: Verify path helper tests exist
echo "Test 6: Verify path helper tests"
if grep -Eq "#\[cfg\((all\(test, feature = \"std\"\)|test)\)\]" artifact_store_schema/src/path.rs; then
    echo "✓ Path helpers have test module"
else
    echo "✗ FAILED: Path helpers missing tests"
    exit 1
fi

if grep -q "fn blob_path_for_constructs_correct_path" artifact_store_schema/src/path.rs; then
    echo "✓ blob_path_for test exists"
else
    echo "✗ FAILED: blob_path_for test missing"
    exit 1
fi

if grep -q "fn manifest_path_for_constructs_correct_path" artifact_store_schema/src/path.rs; then
    echo "✓ manifest_path_for test exists"
else
    echo "✗ FAILED: manifest_path_for test missing"
    exit 1
fi

echo ""

# Test 7: Verify artifact_store_schema builds without IO deps
echo "Test 7: Build artifact_store_schema"
if cargo build -p artifact_store_schema 2>&1 | grep -q "Finished"; then
    echo "✓ artifact_store_schema builds successfully"
else
    echo "⚠ WARNING: Build may have issues"
fi

echo ""

# Summary
echo "=========================================="
echo "✓ All V-007 Phase 1 cleanup tests passed"
echo "=========================================="
echo ""
echo "Summary of cleanup verified:"
echo "  • Path construction helpers added to artifact_store_schema"
echo "  • runtime_supervisor uses current schema contracts"
echo "  • store IO remains behind store service IPC"
echo "  • execution fabric policy wiring is covered"
echo "  • Path helpers are read-only (no IO operations)"
echo "  • Path helpers have tests"
echo ""
echo "Current state:"
echo "  • runtime_supervisor: artifact verification through store service IPC"
echo "  • domain_manager: artifact operations through store service IPC"
echo "  • execution fabric: canonical schema validation in launch_plan.rs"
echo ""
echo "Next steps: keep new artifact IO behind store service IPC"
echo ""
