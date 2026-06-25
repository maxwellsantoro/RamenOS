#!/usr/bin/env bash
# S7 Security Hardening Phase 3: POSIX Runner Security Enforcement Foundry Gate
#
# Validates runtime enforcement via deterministic unit tests in runtime_supervisor:
# - POSIX runner refuses to execute without RAMEN_POSIX_RUNNER_ACK_RISK=1
# - POSIX runner logs security warnings when RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1 is set
# - POSIX runner executes successfully with proper acknowledgment
# - Default portable sandbox profile reports its actual controls

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
S7_POSIX_DIR="$OUT_DIR/s7_posix_test"
EVIDENCE_DIR="$OUT_DIR/evidence/s7_posix_runner"

mkdir -p "$S7_POSIX_DIR" "$EVIDENCE_DIR"

cd "$ROOT_DIR"

echo "=== S7 POSIX Runner Security Enforcement Foundry Gate ==="

echo "Building runtime_supervisor test target..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev --no-run \
    > "$EVIDENCE_DIR/build.log" 2>&1 || {
    echo "FAIL: runtime_supervisor test build failed"
    cat "$EVIDENCE_DIR/build.log"
    exit 1
}
echo "PASS: runtime_supervisor tests built successfully"

# Test 1: Kill-switch enforcement without ACK
echo "Test 1: Verifying POSIX runner blocks execution without RAMEN_POSIX_RUNNER_ACK_RISK=1..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev posix_run_v0_requires_ack_risk_env_var -- --nocapture \
    > "$EVIDENCE_DIR/test1_no_ack.log" 2>&1 || {
    echo "FAIL: Kill-switch test failed"
    cat "$EVIDENCE_DIR/test1_no_ack.log"
    exit 1
}

grep -q "RAMEN_POSIX_RUNNER_ACK_RISK=1" "$EVIDENCE_DIR/test1_no_ack.log" || {
    echo "FAIL: Expected ACK risk guidance not found"
    cat "$EVIDENCE_DIR/test1_no_ack.log"
    exit 1
}

grep -q "POSIX RUNNER EXECUTION BLOCKED" "$EVIDENCE_DIR/test1_no_ack.log" || {
    echo "FAIL: Expected blocked execution message not found"
    cat "$EVIDENCE_DIR/test1_no_ack.log"
    exit 1
}

echo "PASS: POSIX runner correctly blocks execution without acknowledgment"

# Test 2: Sandbox disabled warning
echo "Test 2: Verifying warning emission when sandbox is disabled..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev posix_run_v0_allows_execution_when_sandbox_disabled -- --nocapture \
    > "$EVIDENCE_DIR/test2_sandbox_disabled.log" 2>&1 || {
    echo "FAIL: Sandbox-disabled test failed"
    cat "$EVIDENCE_DIR/test2_sandbox_disabled.log"
    exit 1
}

grep -q "WARNING: SANDBOX DISABLED" "$EVIDENCE_DIR/test2_sandbox_disabled.log" || {
    echo "FAIL: Expected sandbox disabled warning not found"
    cat "$EVIDENCE_DIR/test2_sandbox_disabled.log"
    exit 1
}

grep -q "SECURITY RISK" "$EVIDENCE_DIR/test2_sandbox_disabled.log" || {
    echo "FAIL: Expected SECURITY RISK warning not found"
    cat "$EVIDENCE_DIR/test2_sandbox_disabled.log"
    exit 1
}

echo "PASS: Sandbox-disabled warning verified"

# Test 3: ACK allows execution
echo "Test 3: Verifying execution succeeds with ACK set..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev posix_run_v0_allows_execution_with_ack_risk -- --nocapture \
    > "$EVIDENCE_DIR/test3_with_ack.log" 2>&1 || {
    echo "FAIL: ACK execution test failed"
    cat "$EVIDENCE_DIR/test3_with_ack.log"
    exit 1
}

echo "PASS: Execution succeeds with acknowledgment"

# Test 4: Default portable sandbox profile is honest about enabled controls
echo "Test 4: Verifying default sandbox profile reports actual controls..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev posix_run_v0_uses_sandbox_by_default -- --nocapture \
    > "$EVIDENCE_DIR/test4_sandbox_default.log" 2>&1 || {
    echo "FAIL: Sandbox-default test failed"
    cat "$EVIDENCE_DIR/test4_sandbox_default.log"
    exit 1
}

grep -q "Sandbox profile: host-portable-rlimits-only" "$EVIDENCE_DIR/test4_sandbox_default.log" || {
    echo "FAIL: Expected host-portable sandbox profile marker not found"
    cat "$EVIDENCE_DIR/test4_sandbox_default.log"
    exit 1
}

grep -q "Sandbox controls configured: seccomp=false namespaces=false chroot=false rlimits=true" "$EVIDENCE_DIR/test4_sandbox_default.log" || {
    echo "FAIL: Expected actual default sandbox controls not found"
    cat "$EVIDENCE_DIR/test4_sandbox_default.log"
    exit 1
}

if grep -q "WARNING: SANDBOX DISABLED" "$EVIDENCE_DIR/test4_sandbox_default.log"; then
    echo "FAIL: Unexpected sandbox-disabled warning in default path"
    cat "$EVIDENCE_DIR/test4_sandbox_default.log"
    exit 1
fi

echo "PASS: Default sandbox profile is honest"

echo "Test 4b: Verifying default sandbox config unit contract..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev posix_run_v0_default_profile_is_rlimits_only -- --nocapture \
    > "$EVIDENCE_DIR/test4b_sandbox_profile.log" 2>&1 || {
    echo "FAIL: Sandbox profile unit contract failed"
    cat "$EVIDENCE_DIR/test4b_sandbox_profile.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test4b_sandbox_profile.log" || {
    echo "FAIL: Sandbox profile unit contract did not pass"
    cat "$EVIDENCE_DIR/test4b_sandbox_profile.log"
    exit 1
}

echo "PASS: Default sandbox profile unit contract verified"

# Test 5: Seccomp filter enforcement
echo "Test 5: Verifying seccomp filter blocks dangerous syscalls..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev seccomp_filter_blocks_execve_syscall -- --nocapture \
    > "$EVIDENCE_DIR/test5_seccomp_execve.log" 2>&1 || {
    echo "FAIL: Seccomp execve test failed"
    cat "$EVIDENCE_DIR/test5_seccomp_execve.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test5_seccomp_execve.log" || {
    echo "FAIL: Seccomp execve test did not pass"
    cat "$EVIDENCE_DIR/test5_seccomp_execve.log"
    exit 1
}

echo "PASS: Seccomp filter blocks execve syscall"

# Test 6: Resource limit enforcement
echo "Test 6: Verifying resource limits are enforced..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev rlimit_enforces_process_limit -- --nocapture \
    > "$EVIDENCE_DIR/test6_rlimit_nproc.log" 2>&1 || {
    echo "FAIL: Resource limit test failed"
    cat "$EVIDENCE_DIR/test6_rlimit_nproc.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test6_rlimit_nproc.log" || {
    echo "FAIL: Resource limit test did not pass"
    cat "$EVIDENCE_DIR/test6_rlimit_nproc.log"
    exit 1
}

echo "PASS: Resource limits are enforced"

# Test 7: Chroot confinement
echo "Test 7: Verifying chroot confines filesystem access..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev chroot_confines_filesystem_access -- --nocapture \
    > "$EVIDENCE_DIR/test7_chroot_confinement.log" 2>&1 || {
    echo "FAIL: Chroot confinement test failed"
    cat "$EVIDENCE_DIR/test7_chroot_confinement.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test7_chroot_confinement.log" || {
    echo "FAIL: Chroot confinement test did not pass"
    cat "$EVIDENCE_DIR/test7_chroot_confinement.log"
    exit 1
}

echo "PASS: Chroot confines filesystem access"

# Test 8: Full helper sandbox enforcement
echo "Test 8: Verifying all helper sandbox controls work together when explicitly configured..."
cargo test -p runtime_supervisor --features posix_runner_v0_dev sandbox_full_enforcement -- --nocapture \
    > "$EVIDENCE_DIR/test8_full_sandbox.log" 2>&1 || {
    echo "FAIL: Full helper sandbox test failed"
    cat "$EVIDENCE_DIR/test8_full_sandbox.log"
    exit 1
}

grep -q "test result: ok" "$EVIDENCE_DIR/test8_full_sandbox.log" || {
    echo "FAIL: Full helper sandbox test did not pass"
    cat "$EVIDENCE_DIR/test8_full_sandbox.log"
    exit 1
}

echo "PASS: Full helper sandbox enforcement verified"

# Generate evidence summary
cat > "$EVIDENCE_DIR/summary.md" <<EOF
# S7 POSIX Runner Security Enforcement - Evidence Summary

## Test Results

### Test 1: Kill-switch blocks without RAMEN_POSIX_RUNNER_ACK_RISK=1
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test1_no_ack.log
- **Expected Output**: "RAMEN_POSIX_RUNNER_ACK_RISK=1" guidance and "POSIX RUNNER EXECUTION BLOCKED"

### Test 2: Warning emitted when sandbox disabled
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test2_sandbox_disabled.log
- **Expected Output**: "WARNING: SANDBOX DISABLED" and "SECURITY RISK"

### Test 3: Execution succeeds with ACK
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test3_with_ack.log

### Test 4: Default portable sandbox profile reports actual controls
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test4_sandbox_default.log
- **Expected Output**: "Sandbox profile: host-portable-rlimits-only", "Sandbox controls configured: seccomp=false namespaces=false chroot=false rlimits=true", and no "WARNING: SANDBOX DISABLED"

### Test 4b: Default portable sandbox unit contract
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test4b_sandbox_profile.log
- **Expected Output**: unit test verifies the default POSIX runner profile is rlimits-only

### Test 5: Seccomp filter blocks execve syscall
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test5_seccomp_execve.log
- **Expected Output**: Integration test verifies execve is blocked by seccomp BPF filter

### Test 6: Resource limits enforce process limit
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test6_rlimit_nproc.log
- **Expected Output**: Integration test verifies RLIMIT_NPROC prevents fork

### Test 7: Chroot confines filesystem access
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test7_chroot_confinement.log
- **Expected Output**: Integration test verifies files outside chroot are inaccessible

### Test 8: Full helper sandbox enforcement
- **Status**: PASS
- **Evidence**: $EVIDENCE_DIR/test8_full_sandbox.log
- **Expected Output**: helper integration test verifies all sandbox controls work together when explicitly configured

## Security Assertions Verified

1. POSIX runner refuses execution without ACK kill-switch
2. Disabling sandbox emits explicit security warnings
3. ACK permits execution path
4. Default runtime profile reports rlimits-only controls honestly
5. Seccomp helper blocks dangerous syscalls when explicitly configured
6. Resource-limit helper is exercised when explicitly configured
7. Chroot helper confines filesystem access when explicitly configured
8. Full helper sandbox profile works when explicitly configured

## Evidence Discipline

- Assertions are backed by executable integration tests in runtime_supervisor/src/sandbox.rs
- Helper integration tests verify helper behavior; default-runner tests verify the runtime profile contract
- Gate does not rely on external store service availability
- PASS/FAIL is deterministic and auditable via captured logs
- Security helpers are tested in isolation and together
EOF

echo "=== FOUNDRY_S7_POSIX_RUNNER_SECURITY: PASS ==="
echo "Evidence artifacts saved to: $EVIDENCE_DIR"
echo "Summary: $EVIDENCE_DIR/summary.md"
