# S7 Security Hardening Phase 3: Foundry Gates

**Last Updated:** 2026-02-18
**Status:** Complete

This document describes the Foundry gates created to verify the security hardening fixes implemented in S7 Phase 2.

## Overview

The S7 security hardening effort added the following security fixes to the RamenOS codebase:

1. **Fail-closed store signature policy** - Service aborts startup without `RAMEN_STORE_TRUSTED_KEYS`
2. **Fail-closed access control** - Defaults to `RequireCredentials` instead of `AllowAll`
3. **Exact path matching** - Replaced substring matching with `std::fs::canonicalize()`
4. **DomainArtifactRegistry integration** - Made "global" explicit via directory structure
5. **POSIX runner runtime enforcement** - Requires `RAMEN_POSIX_RUNNER_ACK_RISK=1`

This Phase 3 creates Foundry gates to verify these security fixes are working correctly.

## Foundry Gates

### Gate 1: POSIX Runner Security Enforcement

**Script:** [`tools/ci/foundry_s7_posix_runner_security.sh`](../tools/ci/foundry_s7_posix_runner_security.sh)

**Purpose:** Verifies that the POSIX runner enforces security restrictions at runtime.

**Tests:**
1. POSIX runner refuses to execute without `RAMEN_POSIX_RUNNER_ACK_RISK=1`
2. POSIX runner logs security warnings when `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` is set
3. POSIX runner executes successfully with proper acknowledgment
4. Sandbox is enabled by default

**Evidence Artifacts:**
- `out/evidence/s7_posix_runner/test1_no_ack.log` - Log showing execution blocked without acknowledgment
- `out/evidence/s7_posix_runner/test2_sandbox_disabled.log` - Log showing sandbox disabled warning
- `out/evidence/s7_posix_runner/test3_sandbox_enabled.log` - Log showing sandbox enabled by default
- `out/evidence/s7_posix_runner/summary.md` - Evidence summary

**Environment Variables Tested:**
- `RAMEN_POSIX_RUNNER_ACK_RISK=1` - Required for execution (kill-switch)
- `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` - Disables sandbox with warning (DANGEROUS)

**How to Run:**
```bash
# Run individual gate
just foundry-s7-posix-runner-security

# Or run directly
./tools/ci/foundry_s7_posix_runner_security.sh
```

### Gate 2: Store Signature Validation Fail-Closed

**Script:** [`tools/ci/foundry_s7_store_signature_security.sh`](../tools/ci/foundry_s7_store_signature_security.sh)

**Purpose:** Verifies that the store service enforces fail-closed signature validation.

**Tests:**
1. Store service aborts startup without `RAMEN_STORE_TRUSTED_KEYS` in production mode
2. Store service allows unsigned artifacts with `RAMEN_STORE_DEV_MODE=1` (with warnings)
3. Store service accepts signed artifacts with valid signatures
4. Signature validation infrastructure exists and is properly implemented

**Evidence Artifacts:**
- `out/evidence/s7_store_signature/test1_no_keys.log` - Log showing abort without trusted keys
- `out/evidence/s7_store_signature/test2_dev_mode.log` - Log showing dev mode warnings
- `out/evidence/s7_store_signature/test3_with_keys.log` - Log showing successful key loading
- `out/evidence/s7_store_signature/test3_keygen.log` - Log from key generation
- `out/evidence/s7_store_signature/summary.md` - Evidence summary

**Environment Variables Tested:**
- `RAMEN_STORE_TRUSTED_KEYS` - Path to trusted Ed25519 public keys (REQUIRED in production)
- `RAMEN_STORE_DEV_MODE=1` - Allows unsigned artifacts (DEVELOPMENT ONLY)

**Signature Policies Verified:**
- `RequireSignature` - Default when `RAMEN_STORE_TRUSTED_KEYS` is set
- `AllowUnsigned` - Used when `RAMEN_STORE_DEV_MODE=1` is set

**How to Run:**
```bash
# Run individual gate
just foundry-s7-store-signature-security

# Or run directly
./tools/ci/foundry_s7_store_signature_security.sh
```

### Gate 3: Access Control Fail-Closed

**Script:** [`tools/ci/foundry_s7_access_control_security.sh`](../tools/ci/foundry_s7_access_control_security.sh)

**Purpose:** Verifies that the store service enforces fail-closed access control.

**Tests:**
1. Access control defaults to `RequireCredentials` (not `AllowAll`)
2. `RAMEN_STORE_ACCESS_POLICY` environment variable works correctly
3. Access control infrastructure exists and is properly implemented
4. Audit logging integration exists for access control

**Evidence Artifacts:**
- `out/evidence/s7_access_control/test1_default_policy.log` - Log showing default policy
- `out/evidence/s7_access_control/test2a_allowall.log` - Log showing AllowAll with warnings
- `out/evidence/s7_access_control/test2b_requirecred.log` - Log showing RequireCredentials
- `out/evidence/s7_access_control/test2c_knownservice.log` - Log showing RequireKnownService
- `out/evidence/s7_access_control/test2d_whitelist.log` - Log showing Whitelist
- `out/evidence/s7_access_control/summary.md` - Evidence summary

**Environment Variables Tested:**
- `RAMEN_STORE_ACCESS_POLICY` - Sets the access control policy (default: `RequireCredentials`)

**Access Control Policies Verified:**
- `RequireCredentials` (default) - Requires valid capabilities for all operations
- `AllowAll` (with warning) - No access control - SECURITY RISK
- `RequireKnownService` - Requires client to be a known service
- `Whitelist` - Requires client to be in whitelist

**Access Decision Types Verified:**
- `Allowed` - Access granted
- `Denied` - Access denied - insufficient rights
- `InvalidCapability` - Access denied - invalid capability
- `Expired` - Access denied - capability expired
- `UnknownClient` - Access denied - unknown client

**How to Run:**
```bash
# Run individual gate
just foundry-s7-access-control-security

# Or run directly
./tools/ci/foundry_s7_access_control_security.sh
```

### Combined Gate: All S7 Security Gates

**Script:** [`tools/ci/foundry_s7_all_security.sh`](../tools/ci/foundry_s7_all_security.sh)

**Purpose:** Runs all three S7 security hardening gates in sequence.

**How to Run:**
```bash
# Run all S7 security gates
just foundry-s7-all-security

# Or run directly
./tools/ci/foundry_s7_all_security.sh
```

## Security Assertions Verified

### POSIX Runner Security Enforcement
1. ✅ POSIX runner refuses to execute without `RAMEN_POSIX_RUNNER_ACK_RISK=1`
2. ✅ POSIX runner logs security warnings when `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` is set
3. ✅ POSIX runner executes successfully with proper acknowledgment
4. ✅ Sandbox is enabled by default

### Store Signature Validation Fail-Closed
1. ✅ Store service aborts startup without `RAMEN_STORE_TRUSTED_KEYS` in production mode
2. ✅ Store service allows unsigned artifacts with `RAMEN_STORE_DEV_MODE=1` (with warnings)
3. ✅ Store service accepts signed artifacts with valid signatures
4. ✅ Signature validation infrastructure is properly implemented

### Access Control Fail-Closed
1. ✅ Access control defaults to `RequireCredentials` (not `AllowAll`)
2. ✅ Access control infrastructure is properly implemented
3. ✅ `RAMEN_STORE_ACCESS_POLICY` environment variable works correctly
4. ✅ Audit logging integration exists for access control

## Evidence Artifacts

All gates produce evidence artifacts in the `out/evidence/` directory:

```
out/evidence/
├── s7_posix_runner/
│   ├── test1_no_ack.log
│   ├── test2_sandbox_disabled.log
│   ├── test3_sandbox_enabled.log
│   └── summary.md
├── s7_store_signature/
│   ├── test1_no_keys.log
│   ├── test2_dev_mode.log
│   ├── test3_with_keys.log
│   ├── test3_keygen.log
│   └── summary.md
└── s7_access_control/
    ├── test1_default_policy.log
    ├── test2a_allowall.log
    ├── test2b_requirecred.log
    ├── test2c_knownservice.log
    ├── test2d_whitelist.log
    └── summary.md
```

Each gate produces a `summary.md` file documenting:
- Test results
- Evidence artifacts
- Security assertions verified
- Environment variables tested

## Running the Gates

### Individual Gates
```bash
# POSIX Runner Security
just foundry-s7-posix-runner-security

# Store Signature Security
just foundry-s7-store-signature-security

# Access Control Security
just foundry-s7-access-control-security
```

### All Gates
```bash
# Run all S7 security gates
just foundry-s7-all-security
```

### Direct Execution
```bash
# Run individual gate directly
./tools/ci/foundry_s7_posix_runner_security.sh
./tools/ci/foundry_s7_store_signature_security.sh
./tools/ci/foundry_s7_access_control_security.sh

# Run all gates directly
./tools/ci/foundry_s7_all_security.sh
```

## Expected Output

### Successful Gate Execution
```
=== S7 POSIX Runner Security Enforcement Foundry Gate ===
Test 1: Verifying POSIX runner blocks execution without RAMEN_POSIX_RUNNER_ACK_RISK=1...
PASS: POSIX runner correctly blocks execution without RAMEN_POSIX_RUNNER_ACK_RISK=1
...
=== FOUNDRY_S7_POSIX_RUNNER_SECURITY: PASS ===
Evidence artifacts saved to: out/evidence/s7_posix_runner
```

### Combined Gate Execution
```
=== S7 Security Hardening Phase 3: All Security Gates ===

Running POSIX Runner Security Gate...
✓ POSIX Runner Security Gate: PASS

Running Store Signature Security Gate...
✓ Store Signature Security Gate: PASS

Running Access Control Security Gate...
✓ Access Control Security Gate: PASS

========================================
S7 Security Hardening Gate Summary
========================================

✓ POSIX Runner Security Enforcement: PASS
✓ Store Signature Validation Fail-Closed: PASS
✓ Access Control Fail-Closed: PASS

=== FOUNDRY_S7_ALL_SECURITY: PASS ===
```

## Integration with CI/CD

These gates can be integrated into CI/CD pipelines to ensure security hardening fixes remain in place:

```yaml
# Example GitHub Actions workflow
- name: Run S7 Security Hardening Gates
  run: |
    just foundry-s7-all-security
```

## Related Documentation

- [POSIX Runner Security Guide](../runtime_supervisor/POSIX_RUNNER_SECURITY.md)
- [S7 Security Hardening Phase 2](S7_SECURITY_HARDENING_PHASE2.md)
- [Security Status](../SECURITY_STATUS.md)
- [Constitution](../CONSTITUTION.md)

## Security Considerations

### Development Mode
The `RAMEN_STORE_DEV_MODE=1` environment variable allows unsigned artifacts for development purposes. This should **NEVER** be used in production environments.

### AllowAll Access Policy
The `RAMEN_STORE_ACCESS_POLICY=AllowAll` setting disables all access control. This is a **SECURITY RISK** and should only be used for testing purposes.

### POSIX Runner Sandbox Disabling
The `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1` setting disables sandbox protection. This is a **SECURITY RISK** and should only be used for debugging purposes.

## Troubleshooting

### Gate Fails with "Permission Denied"
Ensure the gate scripts are executable:
```bash
chmod +x tools/ci/foundry_s7_*.sh
```

### Store Service Fails to Start
Check that the socket path doesn't already exist:
```bash
rm -f out/s7_*/store*.sock
```

### Evidence Directory Issues
Ensure the evidence directory exists and is writable:
```bash
mkdir -p out/evidence/s7_*
```

## Future Enhancements

Potential future enhancements to the S7 security gates:

1. Add tests for exact path matching with `std::fs::canonicalize()`
2. Add tests for DomainArtifactRegistry integration
3. Add performance benchmarks for security checks
4. Add fuzzing tests for input validation
5. Add integration tests with real signed artifacts

## References

- [Foundry Gate Pattern](../tools/ci/foundry_s0.sh)
- [Hardening Wave Gates](../tools/ci/foundry_hardening_wave_a_batch1.sh)
- [V-007 Phase 3 Store Hardening](../tools/ci/foundry_v007_phase3_store_hardening.sh)
