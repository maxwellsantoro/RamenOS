#!/usr/bin/env bash
# Foundry gate for S10.0 Native Runner.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
mkdir -p "$ROOT_DIR/out/foundry/selected-tests"
SELECTED_RUN_ROOT="$(mktemp -d "$ROOT_DIR/out/foundry/selected-tests/native-runner.XXXXXX")"

echo "=== S10.0 Native Runner Foundry Gate ==="
echo

PASSED=0
FAILED=0

pass() { echo "PASS: $1"; PASSED=$((PASSED + 1)); }
fail() { echo "FAIL: $1"; FAILED=$((FAILED + 1)); }

echo "Ensuring wasm32-unknown-unknown target is installed..."
rustup target add wasm32-unknown-unknown

# Assertion 1: native_runner binary builds
echo "Asserting native_runner builds..."
if cargo build --locked -p native_runner 2>&1 >/dev/null; then
    pass "native_runner builds"
else
    fail "native_runner builds"
fi

# Assertion 2: hello_wasm.wasm builds
echo
echo "Asserting hello_wasm builds..."
if cargo build --locked -p hello_wasm --target wasm32-unknown-unknown --target-dir "$ROOT_DIR/target" 2>&1 >/dev/null; then
    pass "hello_wasm builds"
else
    fail "hello_wasm builds"
fi

# Assertion 3-5: Run integration tests
echo
echo "Running integration tests..."
if python3 "$ROOT_DIR/tools/ci/selected_test_runner.py" \
    --package native_runner --test integration_test \
    --required-input "$ROOT_DIR/target/wasm32-unknown-unknown/debug/hello_wasm.wasm" \
    --output "$SELECTED_RUN_ROOT/integration"; then
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
