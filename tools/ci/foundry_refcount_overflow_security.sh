#!/usr/bin/env bash
# Foundry Gate: NEW-001 Refcount Overflow Security Regression Test
#
# This gate verifies that the critical refcount overflow vulnerability (NEW-001)
# has been fixed and cannot be reintroduced.
#
# Original vulnerability:
# - refcount was u32, allowing attacker to map region 2^32 times
# - Overflow would wrap refcount to 0
# - close_region would free frames while mappings remained active
# - Result: use-after-free, memory corruption, privilege escalation
#
# Fix:
# - Changed refcount to u64 (much harder to overflow in practice)
# - Added checked arithmetic that returns STATUS_OVERFLOW on overflow
# - Added checked arithmetic that returns STATUS_UNDERFLOW on underflow
# - This prevents wraparound and forces graceful failure

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_REFCOUNT_OVERFLOW_SECURITY: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  local description="$2"
  shift 2
  echo "Running: ${description}"
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
  echo "✓ PASS: ${description}"
}

echo "=== NEW-001 Refcount Overflow Security Regression Gate ==="
echo ""
echo "This gate verifies that the refcount overflow vulnerability is fixed:"
echo "  1. Overflow protection: map_region fails with STATUS_OVERFLOW"
echo "  2. Underflow protection: unmap_region handles zero refcount correctly"
echo ""

# Test 1: Refcount overflow protection
# Verifies that attempting to increment refcount beyond u64::MAX fails gracefully
run_gate_test \
  "NEW001_OVERFLOW_PROTECTION" \
  "Refcount overflow protection - map_region fails with STATUS_OVERFLOW when refcount would overflow" \
  cargo test -p kernel shmem::tests::refcount_overflow_protection -- --exact

echo ""

# Test 2: Refcount underflow protection
# Verifies that attempting to decrement refcount below 0 fails gracefully
run_gate_test \
  "NEW001_UNDERFLOW_PROTECTION" \
  "Refcount underflow protection - unmap_region handles zero refcount correctly" \
  cargo test -p kernel shmem::tests::refcount_underflow_protection -- --exact

echo ""
echo "=== NEW-001 Refcount Overflow Security Gate: PASS ==="
