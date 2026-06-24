#!/usr/bin/env bash
# Extended Foundry gates for CI: security remediation + native runner + semantic
# state + active Driver Factory inventory.

set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=== Running S7 security umbrella gate ==="
"$ROOT_DIR/tools/ci/foundry_s7_all_security.sh"

echo "=== Running S9.0 boundary cleanup gate ==="
"$ROOT_DIR/tools/ci/foundry_boundary_s9_0_cleanup.sh"

echo "=== Running S9.0 trace isolation gate ==="
"$ROOT_DIR/tools/ci/foundry_trace_isolation_s9_0_per_domain.sh"

echo "=== Running V-012 Phase 5 trace client gate ==="
"$ROOT_DIR/tools/ci/foundry_v012_phase5_trace_client.sh"

echo "=== Running S10.0 native runner gate ==="
"$ROOT_DIR/tools/ci/foundry_native_runner_s10_0.sh"

echo "=== Running S10.1 native runner gate (CI-safe) ==="
SKIP_E2E_ASSERTIONS=1 "$ROOT_DIR/tools/ci/foundry_native_runner_s10_1.sh"

echo "=== Running S10.2 semantic state gate ==="
bash "$ROOT_DIR/tools/ci/foundry_semantic_state_s10_2.sh"

echo "=== Running S10.3 projection storage gate ==="
bash "$ROOT_DIR/tools/ci/foundry_projection_storage_s10_3.sh"

echo "=== Running S10.4 execution fabric gate ==="
bash "$ROOT_DIR/tools/ci/foundry_execution_fabric_s10_4.sh"

echo "=== Running S10.5.0 host-target semantic snapshot gate ==="
bash "$ROOT_DIR/tools/ci/foundry_host_target_s10_5.sh"

echo "=== Running S10.5.1 broker/kernel bridge gate ==="
bash "$ROOT_DIR/tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh"

echo "=== Running S10.5.2 QEMU IPC bridge gate ==="
bash "$ROOT_DIR/tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh"

echo "=== Running S11.0/S11.1 Driver Factory inventory gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s11_driver_factory_s11_0.sh"

echo "=== Running S11.2 replay gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s11_replay.sh"

echo "=== Running S11.3 virtio-net Reference Vault gate ==="
REQUIRE_LIVE_ORACLE_TRACE=1 bash "$ROOT_DIR/tools/ci/foundry_s11_reference_vault_s11_3.sh"

echo "=== Running S11.8 runtime harness.net packet I/O gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s11_runtime_net_s11_8.sh"

echo "=== Running S12.0 golden machine smoke gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s12_golden_machine_s12_0.sh"

echo "=== Running S12.1 UEFI GOP probe gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s12_gop_probe_s12_1.sh"

echo "=== Running S12.4 HIL appliance controller scaffold gate ==="
bash "$ROOT_DIR/tools/ci/foundry_hil_appliance_s12_4.sh"

echo "=== Running G0 RamenOrg governance scaffold gate ==="
bash "$ROOT_DIR/tools/ci/foundry_org_governance_g0.sh"

echo "=== Running S13.0 persistent storage smoke gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s13_persistent_storage_s13_0.sh"

echo "=== Running S13.2 virtio-blk Oracle capture gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s13_virtio_blk_oracle_s13_2.sh"

echo "=== Running S13.3 block replay scoreboard gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s13_replay.sh"

echo "=== Running S13.4 block sector Oracle capture gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s13_block_sector_oracle_s13_4.sh"

echo "=== Running S13.6 runtime harness.block gate ==="
bash "$ROOT_DIR/tools/ci/foundry_s13_runtime_block_s13_6.sh"

echo "FOUNDRY_CI_EXTENDED: ok"
