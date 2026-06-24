#!/usr/bin/env bash
# Foundry gate for S10.5.1 Broker / Kernel Harness Bridge (one semantic path).
#
# Phase 0: inventory + design doc (PASS today).
# Phases 1-3: broker allowlist, proxy roundtrip, supervisor E2E.
#
# See: docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.5.1 Broker / Kernel Harness Bridge Foundry Gate ==="

fail() {
  echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: FAIL code=$1 detail=$2" >&2
  exit 1
}

# ---------------------------------------------------------------------------
# Phase 0: Design + inventory
# ---------------------------------------------------------------------------
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INFO step=inventory"

test -f docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md \
  || fail "DESIGN_DOC_MISSING" "S10.5.1 design doc not found"

grep -q 'SemanticHarnessGrantOps' docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md \
  || fail "DESIGN_INCOMPLETE" "design must pin SemanticHarnessGrantOps"

grep -q 'KernelHarnessProxy' docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md \
  || fail "DESIGN_INCOMPLETE" "design must pin KernelHarnessProxy"

grep -q 'handle_count' kernel_api/src/generated/domain_manager_v1.generated.rs \
  || fail "INVENTORY" "GrantCapabilitiesReply still handle_count only"

grep -q 'SimulatedKernelOps' services/domain_manager/src/broker.rs \
  || fail "INVENTORY" "broker still expected to use SimulatedKernelOps today"

grep -q 'S10.1: Stub implementation' runtime_supervisor/src/native_wasm_runner.rs \
  || fail "INVENTORY" "native_wasm_runner grant stub marker missing"

echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY grant_reply_handle_count_only=true"
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY simulated_kernel_ops=true"
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY native_wasm_grant_stub=true"
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY s10_5_0_qemu_snapshot_sha256_prefix=9c0de4419f03f426"

# ---------------------------------------------------------------------------
# Phase 1: Broker semantic harness allowlist
# ---------------------------------------------------------------------------
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INFO step=broker_semantic_harness_grant"

if ! grep -rq 'SemanticHarnessGrantOps' services/domain_manager/src/ 2>/dev/null; then
  fail "BROKER_GRANT_OPS_MISSING" \
    "implement SemanticHarnessGrantOps in domain_manager::broker (see S10.5.1 design §1)"
fi

cargo test -p domain_manager semantic_harness_grant --quiet \
  || fail "BROKER_GRANT_TESTS" "semantic_harness_grant unit tests failed or not implemented"

# ---------------------------------------------------------------------------
# Phase 2: Kernel harness proxy roundtrip
# ---------------------------------------------------------------------------
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INFO step=kernel_harness_proxy_roundtrip"

if [[ ! -d services/kernel_harness_proxy ]]; then
  fail "PROXY_CRATE_MISSING" \
    "create services/kernel_harness_proxy (see S10.5.1 design §3)"
fi

cargo test -p kernel_harness_proxy proxy_get_snapshot_roundtrip --quiet \
  || fail "PROXY_ROUNDTRIP" "proxy_get_snapshot_roundtrip failed"

# Assert deterministic sha256 prefix contract.
PROXY_OUTPUT="$(cargo test -p kernel_harness_proxy proxy_get_snapshot_roundtrip --quiet -- --nocapture 2>&1)"
grep -q '9c0de4419f03f426' <<<"$PROXY_OUTPUT" \
  || fail "SHA256_PREFIX_MISMATCH" "proxy snapshot must match S10.5.0 prefix 9c0de4419f03f426"

echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: METRIC snapshot_sha256_prefix=9c0de4419f03f426"

# ---------------------------------------------------------------------------
# Phase 3: Supervisor bridge E2E (host)
# ---------------------------------------------------------------------------
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INFO step=supervisor_semantic_bridge_e2e"

cargo test -p runtime_supervisor semantic_harness_bridge_e2e --quiet \
  || fail "SUPERVISOR_E2E" "semantic_harness_bridge_e2e failed or not implemented"

echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: PASS"
echo "FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: ok"
