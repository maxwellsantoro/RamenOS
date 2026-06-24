#!/usr/bin/env bash
# Foundry gate for S10.0 Native Runner.

set -euo pipefail

echo "=== S10.0 Native Runner Foundry Gate ==="
echo

PASSED=0
FAILED=0

pass() { echo "PASS: $1"; PASSED=$((PASSED + 1)); }
fail() { echo "FAIL: $1"; FAILED=$((FAILED + 1)); }

# Assertion 1: native_runner binary builds
echo "Asserting native_runner builds..."
if cargo build -p native_runner 2>&1 >/dev/null; then
    pass "native_runner builds"
else
    fail "native_runner builds"
fi

# Assertion 2: hello_wasm.wasm builds
echo
echo "Asserting hello_wasm builds..."
if cargo build -p hello_wasm --target wasm32-unknown-unknown 2>&1 >/dev/null; then
    pass "hello_wasm builds"
else
    fail "hello_wasm builds"
fi

# Assertion 3-5: Run integration tests
echo
echo "Running integration tests..."
if cargo test -p native_runner --test integration_test 2>&1 >/dev/null; then
    pass "integration tests pass"
else
    fail "integration tests"
fi

# Summary
echo
echo "=== S10.0 Summary ==="
echo "PASSED: ${PASSED}"
echo "FAILED: ${FAILED}"

if [[ ${FAILED} -eq 0 ]]; then
    echo
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: METRIC passed=${PASSED} failed=0"
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: ok"
    exit 0
else
    echo
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: METRIC passed=${PASSED} failed=${FAILED}"
    echo "FOUNDRY_NATIVE_RUNNER_S10_0: FAIL"
    exit 1
fi