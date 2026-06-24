#!/usr/bin/env bash
# Foundry gate for S13.0 Persistent Storage contract scaffold.
#
# Inventory + negative assertions only. Does not require physical NVMe.
# See: docs/plans/2026-06-21-s13-persistent-storage-design.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S13 Persistent Storage Smoke Gate (S13.0) ==="

fail() {
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=inventory"

DESIGN_DOC="$ROOT_DIR/docs/plans/2026-06-21-s13-persistent-storage-design.md"
STORAGE_MANIFEST="$ROOT_DIR/hardware/storage_contract_v0.toml"
BLOCK_IDL="$ROOT_DIR/idl/harness/block_v1.toml"
VAULT_DIR="$ROOT_DIR/drivers/reference_vaults/virtio-blk"

test -f "$DESIGN_DOC" \
  || fail "DESIGN_DOC_MISSING" "S13 design doc not found"

test -f "$STORAGE_MANIFEST" \
  || fail "STORAGE_MANIFEST_MISSING" "hardware/storage_contract_v0.toml not found"

grep -q 'CHOSEN.*virtio-blk' "$DESIGN_DOC" \
  || fail "ORACLE_DEVICE_UNPINNED" "design doc must pin virtio-blk Oracle device"

grep -q 'foundry_s13_persistent_storage_s13_0.sh' "$DESIGN_DOC" \
  || fail "GATE_NOT_DOCUMENTED" "design doc must reference this gate"

grep -q 'device = "virtio-blk-pci"' "$STORAGE_MANIFEST" \
  || fail "ORACLE_DEVICE_MANIFEST" "manifest must declare virtio-blk-pci Oracle"

grep -q 'harness = "harness.block"' "$STORAGE_MANIFEST" \
  || fail "HARNESS_CONTRACT_MISSING" "manifest must require harness.block"

grep -q 'device = "nvme_pcie"' "$STORAGE_MANIFEST" \
  || fail "METAL_DEVICE_MANIFEST" "manifest must declare nvme_pcie metal target"

grep -q 'default_ci = "skip"' "$STORAGE_MANIFEST" \
  || fail "HIL_POLICY_MISSING" "manifest must default metal HIL to skip in CI"

test -f "$BLOCK_IDL" \
  || fail "BLOCK_IDL_MISSING" "idl/harness/block_v1.toml missing"

grep -q 'namespace = "harness.block"' "$BLOCK_IDL" \
  || fail "BLOCK_IDL_NAMESPACE" "block_v1 must define harness.block"

test -d "$VAULT_DIR" \
  || fail "VAULT_MISSING" "virtio-blk Reference Vault missing"

test -f "$VAULT_DIR/README.md" \
  || fail "VAULT_README_MISSING" "virtio-blk vault README missing"

test -f "$VAULT_DIR/notes.md" \
  || fail "VAULT_NOTES_MISSING" "virtio-blk vault notes missing"

test -f "$VAULT_DIR/harness.toml" \
  || fail "VAULT_HARNESS_MISSING" "virtio-blk vault harness context missing"

test -f "$VAULT_DIR/datasheets/virtio-blk-v1.3.md" \
  || fail "VAULT_DATASHEET_MISSING" "virtio-blk vault datasheet missing"

echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=idl_codegen"

bash "$ROOT_DIR/tools/ci/run_codegen.sh" >/dev/null

grep -q 'include!("generated/block_v1.generated.rs")' kernel_api/src/lib.rs \
  || fail "BLOCK_BINDING_NOT_INCLUDED" "kernel_api must include generated block_v1 binding"

cargo test -p kernel_api generated::block_v1 --quiet 2>/dev/null \
  || cargo test -p kernel_api --quiet --lib 2>/dev/null \
  || fail "KERNEL_API_TESTS" "kernel_api tests failed after block_v1 codegen"

echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=negative_assertions"

if grep -rq 'ioctl' idl/harness/block_v1.toml 2>/dev/null; then
  fail "IOCTL_ESCAPE_IN_BLOCK_IDL" "block harness IDL must not use ioctl patterns"
fi

if grep -q 'foundry_s13_nvme_boot' tools/ci/foundry_ci_extended.sh 2>/dev/null; then
  if ! grep -q 'RAMEN_HIL_GOLDEN_MACHINE' tools/ci/foundry_ci_extended.sh; then
    fail "HIL_CI_UNGUARDED" "metal NVMe gate in CI must be guarded by RAMEN_HIL_GOLDEN_MACHINE"
  fi
fi

echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=implementation_inventory"

if [[ -x tools/trace/capture_virtio_blk_oracle.sh ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC virtio_blk_capture=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED virtio_blk_capture=NOT_IMPLEMENTED"
fi

if [[ -f drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC oracle_init_trace=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED oracle_init_trace=NOT_IMPLEMENTED"
fi

if [[ -f drivers/reference_vaults/virtio-blk/traces/oracle_block_trace.json ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC oracle_block_trace=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED oracle_block_trace=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s13_runtime_block_s13_6.sh ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC runtime_block_gate=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED runtime_block_gate=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s13_nvme_boot_s13_7.sh ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC nvme_boot_gate=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED nvme_boot_gate=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s13_atomic_update_s13_8.sh ]]; then
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: METRIC atomic_update_gate=present"
else
  echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: RED atomic_update_gate=NOT_IMPLEMENTED"
fi

if grep -q 'foundry_s13_atomic_update' tools/ci/foundry_ci_extended.sh 2>/dev/null; then
  if ! grep -q 'RAMEN_HIL_GOLDEN_MACHINE' tools/ci/foundry_ci_extended.sh; then
    fail "HIL_CI_UNGUARDED" "metal atomic update gate in CI must be guarded by RAMEN_HIL_GOLDEN_MACHINE"
  fi
fi

if [[ "${RAMEN_HIL_GOLDEN_MACHINE:-}" == "1" ]]; then
  if [[ -x tools/ci/foundry_s13_nvme_boot_s13_7.sh ]]; then
    echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=nvme_boot_delegated"
    bash tools/ci/foundry_s13_nvme_boot_s13_7.sh
  fi
  if [[ -x tools/ci/foundry_s13_atomic_update_s13_8.sh ]]; then
    echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: INFO step=atomic_update_delegated"
    bash tools/ci/foundry_s13_atomic_update_s13_8.sh
  fi
fi

echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: PASS"
echo "FOUNDRY_S13_PERSISTENT_STORAGE_S13_0: ok"