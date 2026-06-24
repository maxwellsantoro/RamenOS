#!/usr/bin/env bash
# Foundry gate for S12.3 ACPI DMAR / VT-d inventory on the Tier-1 golden machine.
#
# Runs only when RAMEN_HIL_GOLDEN_MACHINE=1. Default CI skips (fail-closed under
# RAMEN_CI_STRICT=1). See: docs/plans/2026-06-21-s12-golden-machine-design.md §Phase 3

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S12.3 IOMMU Inventory Foundry Gate ==="

fail() {
  echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: FAIL code=$1 detail=$2" >&2
  exit 1
}

skip_gate() {
  local reason="$1"
  echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: skipped ($reason)"
  if [[ "${RAMEN_CI_STRICT:-}" == "1" ]]; then
    echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: FAIL (strict mode, skip not allowed)" >&2
    exit 1
  fi
  exit 0
}

assert_serial_log() {
  local log="$1"
  test -f "$log" || fail "SERIAL_LOG_MISSING" "serial log not found: $log"

  grep -q "RAMEN OS" "$log" \
    || fail "BOOT_BANNER_MISSING" "serial log missing RAMEN OS boot banner"
  grep -q "golden_machine: iommu_present=1" "$log" \
    || fail "IOMMU_PRESENT_MISSING" "serial log missing golden_machine: iommu_present=1"
}

wait_for_serial_log() {
  local log="$1"
  local pattern="$2"
  local timeout_s="$3"
  local max_iters=$((timeout_s * 5))
  for _ in $(seq 1 "$max_iters"); do
    if [[ -f "$log" ]] && grep -aq "$pattern" "$log"; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

capture_serial_boot() {
  local dev="$1"
  local log="$2"
  local timeout_s="$3"

  if [[ ! -e "$dev" ]]; then
    fail "SERIAL_DEV_MISSING" "serial device not found: $dev"
  fi

  rm -f "$log"
  echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO capturing serial dev=${dev} timeout_s=${timeout_s}"
  echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO operator=power_cycle_nuc_and_select_usb_boot"

  if command -v stty >/dev/null 2>&1; then
    stty -f "$dev" 115200 raw -echo 2>/dev/null \
      || stty -F "$dev" 115200 raw -echo 2>/dev/null \
      || true
  fi

  if command -v timeout >/dev/null 2>&1; then
    timeout "${timeout_s}s" cat "$dev" >"$log" || true
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "${timeout_s}s" cat "$dev" >"$log" || true
  else
    cat "$dev" >"$log" &
    local cat_pid=$!
    if ! wait_for_serial_log "$log" "golden_machine: iommu_present=1" "$timeout_s"; then
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    else
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    fi
  fi
}

find_uefi_bin() {
  local target="$1"
  local base="$ROOT_DIR/target/$target/debug"
  if [[ -f "$base/kernel_uefi.efi" ]]; then
    echo "$base/kernel_uefi.efi"
    return 0
  fi
  if [[ -f "$base/kernel_uefi" ]]; then
    echo "$base/kernel_uefi"
    return 0
  fi
  fail "UEFI_BIN_MISSING" "kernel_uefi binary not found for $target"
}

echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO step=inventory"

test -f docs/plans/2026-06-21-s12-golden-machine-design.md \
  || fail "DESIGN_DOC_MISSING" "S12 design doc not found"

test -f hardware/golden_machine_v0.toml \
  || fail "MANIFEST_MISSING" "hardware/golden_machine_v0.toml not found"

grep -q 'OP_IOMMU_INVENTORY' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_IOMMU_INVENTORY in kernel init"

grep -q 'iommu_inventory' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add iommu_inventory init profile"

test -f kernel_uefi/src/iommu_probe.rs \
  || fail "IOMMU_PROBE_MODULE_MISSING" "kernel_uefi iommu_probe module missing"

grep -q 'set_iommu_probe' kernel/src/boot.rs \
  || fail "IOMMU_PROBE_STORAGE_MISSING" "kernel boot must store IOMMU probe info"

grep -q 'iommu_ok = "golden_machine: iommu_present=1"' hardware/golden_machine_v0.toml \
  || fail "MANIFEST_MARKER_MISSING" "manifest must declare iommu_ok marker"

if [[ "${RAMEN_HIL_GOLDEN_MACHINE:-}" != "1" ]]; then
  skip_gate "RAMEN_HIL_GOLDEN_MACHINE not set"
fi

echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO step=build_boot_artifacts"

OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
INIT_DIR="$OUT_DIR/init"
LOG_DIR="$OUT_DIR/logs"
mkdir -p "$UEFI_DIR/x86_64/EFI/BOOT" "$INIT_DIR" "$LOG_DIR"

cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet

python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_iommu_inventory.img" \
  --profile iommu_inventory

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
cp "$X86_BIN" "$UEFI_DIR/x86_64/EFI/BOOT/BOOTX64.EFI"
cp "$INIT_DIR/init_iommu_inventory.img" "$UEFI_DIR/x86_64/EFI/BOOT/init.img"

echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO operator_steps="
echo "  1. Build USB boot tree with profile=iommu_inventory (or copy EFI/BOOT from gate output)."
echo "  2. Ensure firmware VT-d is enabled on the Intel NUC reference."
echo "  3. Boot from USB; serial must emit golden_machine: iommu_present=1."

if [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
  echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: INFO step=validate_serial_log path=${RAMEN_HIL_SERIAL_LOG}"
  assert_serial_log "$RAMEN_HIL_SERIAL_LOG"
elif [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
  LOG="$LOG_DIR/iommu_inventory_serial.log"
  TIMEOUT_S="${RAMEN_HIL_BOOT_TIMEOUT_S:-120}"
  capture_serial_boot "$RAMEN_HIL_SERIAL_DEV" "$LOG" "$TIMEOUT_S"
  if ! wait_for_serial_log "$LOG" "golden_machine: iommu_present=1" 5; then
    echo "--- hil serial log (tail) ---" >&2
    tail -n 60 "$LOG" >&2 || true
    fail "IOMMU_INVENTORY_TIMEOUT" \
      "serial capture timed out before golden_machine: iommu_present=1"
  fi
  assert_serial_log "$LOG"
else
  fail "SERIAL_INPUT_MISSING" \
    "set RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 or RAMEN_HIL_SERIAL_LOG=/path/to/capture.log"
fi

echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: PASS"
echo "FOUNDRY_S12_IOMMU_INVENTORY_S12_3: ok"