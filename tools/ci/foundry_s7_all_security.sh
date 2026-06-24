#!/usr/bin/env bash
# S7 Security Hardening Phase 3: All Security Gates Combined
#
# Runs all S7 security hardening Foundry gates:
# - POSIX Runner Security Enforcement
# - Store Signature Validation Fail-Closed
# - Access Control Fail-Closed

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=== S7 Security Hardening Phase 3: All Security Gates ==="
echo ""
echo "This script runs all S7 security hardening Foundry gates:"
echo "  1. POSIX Runner Security Enforcement"
echo "  2. Store Signature Validation Fail-Closed"
echo "  3. Access Control Fail-Closed"
echo ""

# Track results
POSIX_PASSED=false
STORE_PASSED=false
ACCESS_PASSED=false

# Run POSIX Runner Security Gate
echo "========================================"
echo "Running POSIX Runner Security Gate..."
echo "========================================"
if "$ROOT_DIR/tools/ci/foundry_s7_posix_runner_security.sh"; then
    POSIX_PASSED=true
    echo "✓ POSIX Runner Security Gate: PASS"
else
    echo "✗ POSIX Runner Security Gate: FAIL"
fi
echo ""

# Run Store Signature Security Gate
echo "========================================"
echo "Running Store Signature Security Gate..."
echo "========================================"
if "$ROOT_DIR/tools/ci/foundry_s7_store_signature_security.sh"; then
    STORE_PASSED=true
    echo "✓ Store Signature Security Gate: PASS"
else
    echo "✗ Store Signature Security Gate: FAIL"
fi
echo ""

# Run Access Control Security Gate
echo "========================================"
echo "Running Access Control Security Gate..."
echo "========================================"
if "$ROOT_DIR/tools/ci/foundry_s7_access_control_security.sh"; then
    ACCESS_PASSED=true
    echo "✓ Access Control Security Gate: PASS"
else
    echo "✗ Access Control Security Gate: FAIL"
fi
echo ""

# Summary
echo "========================================"
echo "S7 Security Hardening Gate Summary"
echo "========================================"
echo ""

if [ "$POSIX_PASSED" = true ]; then
    echo "✓ POSIX Runner Security Enforcement: PASS"
else
    echo "✗ POSIX Runner Security Enforcement: FAIL"
fi

if [ "$STORE_PASSED" = true ]; then
    echo "✓ Store Signature Validation Fail-Closed: PASS"
else
    echo "✗ Store Signature Validation Fail-Closed: FAIL"
fi

if [ "$ACCESS_PASSED" = true ]; then
    echo "✓ Access Control Fail-Closed: PASS"
else
    echo "✗ Access Control Fail-Closed: FAIL"
fi

echo ""

# Overall result
if [ "$POSIX_PASSED" = true ] && [ "$STORE_PASSED" = true ] && [ "$ACCESS_PASSED" = true ]; then
    echo "=== FOUNDRY_S7_ALL_SECURITY: PASS ==="
    exit 0
else
    echo "=== FOUNDRY_S7_ALL_SECURITY: FAIL ==="
    exit 1
fi
