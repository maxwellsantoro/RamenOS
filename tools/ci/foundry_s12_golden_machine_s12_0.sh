#!/usr/bin/env bash
# Foundry gate for S12.0 Golden Machine contract scaffold.
#
# Inventory + negative assertions only. Does not require physical hardware.
# See: docs/plans/2026-06-21-s12-golden-machine-design.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S12 Golden Machine Smoke Gate (S12.0) ==="

fail() {
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: FAIL code=$1 detail=$2" >&2
  exit 1
}

skip_hil() {
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO hil=skipped reason=$1"
}

echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO step=inventory"

DESIGN_DOC="$ROOT_DIR/docs/plans/2026-06-21-s12-golden-machine-design.md"
MANIFEST="$ROOT_DIR/hardware/golden_machine_v0.toml"

test -f "$DESIGN_DOC" \
  || fail "DESIGN_DOC_MISSING" "S12 design doc not found"

test -f "$MANIFEST" \
  || fail "MANIFEST_MISSING" "hardware/golden_machine_v0.toml not found"

grep -q 'CHOSEN.*Intel NUC' "$DESIGN_DOC" \
  || fail "REFERENCE_MACHINE_UNPINNED" "design doc must pin Tier-1 reference machine"

grep -q 'foundry_s12_golden_machine_s12_0.sh' "$DESIGN_DOC" \
  || fail "GATE_NOT_DOCUMENTED" "design doc must reference this gate"

grep -q 'tier = 1' "$MANIFEST" \
  || fail "MANIFEST_TIER" "manifest must declare tier = 1"

grep -q 'iommu = "vtd"' "$MANIFEST" \
  || fail "IOMMU_CONTRACT_MISSING" "Tier-1 manifest must require VT-d"

grep -q 'framebuffer = "uefi_gop"' "$MANIFEST" \
  || fail "GOP_CONTRACT_MISSING" "manifest must require UEFI GOP"

grep -q 'default_ci = "skip"' "$MANIFEST" \
  || fail "HIL_POLICY_MISSING" "manifest must default HIL to skip in CI"

test -f docs/HARDWARE_STRATEGY.md \
  || fail "HARDWARE_STRATEGY_MISSING" "docs/HARDWARE_STRATEGY.md required"

grep -q 'IOMMU' docs/HARDWARE_STRATEGY.md \
  || fail "HARDWARE_STRATEGY_IOMMU" "HARDWARE_STRATEGY must document IOMMU requirement"

echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC tier=1 reference_machine=intel-nuc-12-reference"

echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO step=negative_assertions"

# S12.0 must not pull physical HIL into default CI extended flow yet.
if grep -q 'foundry_s12_hil_boot' tools/ci/foundry_ci_extended.sh 2>/dev/null; then
  if ! grep -q 'RAMEN_HIL_GOLDEN_MACHINE' tools/ci/foundry_ci_extended.sh; then
    fail "HIL_CI_UNGUARDED" "HIL gate in CI must be guarded by RAMEN_HIL_GOLDEN_MACHINE"
  fi
fi

# Constitutional guard: no framebuffer ioctl escape hatches in native IDL.
if grep -rq 'ioctl' idl/harness/ 2>/dev/null; then
  fail "IOCTL_ESCAPE_IN_HARNESS" "harness IDL must not use ioctl patterns"
fi

echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO step=implementation_inventory"

if grep -q 'OP_GOP_PROBE' tools/init/build_init_image.py; then
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC gop_probe_init=implemented"
else
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: RED gop_probe_init=NOT_IMPLEMENTED"
fi

if grep -rq 'GraphicsOutput\|GOP\|gop_probe' kernel_uefi/src/ 2>/dev/null; then
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC gop_uefi=implemented"
else
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: RED gop_uefi=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s12_gop_probe_s12_1.sh ]]; then
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC gop_probe_gate=present"
else
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: RED gop_probe_gate=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s12_hil_boot_s12_2.sh ]]; then
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC hil_boot_gate=present"
else
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: RED hil_boot_gate=NOT_IMPLEMENTED"
fi

if [[ -x tools/ci/foundry_s12_iommu_inventory_s12_3.sh ]]; then
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: METRIC iommu_inventory_gate=present"
else
  echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: RED iommu_inventory_gate=NOT_IMPLEMENTED"
fi

if [[ "${RAMEN_HIL_GOLDEN_MACHINE:-}" == "1" ]]; then
  if [[ -x tools/ci/foundry_s12_hil_boot_s12_2.sh ]]; then
    echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO step=hil_boot_delegated"
    bash tools/ci/foundry_s12_hil_boot_s12_2.sh
  fi
  if [[ -x tools/ci/foundry_s12_iommu_inventory_s12_3.sh ]]; then
    echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: INFO step=iommu_inventory_delegated"
    bash tools/ci/foundry_s12_iommu_inventory_s12_3.sh
  fi
else
  skip_hil "RAMEN_HIL_GOLDEN_MACHINE not set"
fi

echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: PASS"
echo "FOUNDRY_S12_GOLDEN_MACHINE_S12_0: ok"