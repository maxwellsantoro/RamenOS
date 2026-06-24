#!/usr/bin/env bash
# Foundry gate for S12.2 physical HIL boot on the Tier-1 golden machine.
#
# Runs only when RAMEN_HIL_GOLDEN_MACHINE=1. Default CI skips (fail-closed under
# RAMEN_CI_STRICT=1). See: docs/plans/2026-06-21-s12-golden-machine-design.md §Phase 2

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S12.2 Physical HIL Boot Foundry Gate ==="

fail() {
  echo "FOUNDRY_S12_HIL_BOOT_S12_2: FAIL code=$1 detail=$2" >&2
  exit 1
}

skip_gate() {
  local reason="$1"
  echo "FOUNDRY_S12_HIL_BOOT_S12_2: skipped ($reason)"
  if [[ "${RAMEN_CI_STRICT:-}" == "1" ]]; then
    echo "FOUNDRY_S12_HIL_BOOT_S12_2: FAIL (strict mode, skip not allowed)" >&2
    exit 1
  fi
  exit 0
}

assert_serial_log() {
  local log="$1"
  test -f "$log" || fail "SERIAL_LOG_MISSING" "serial log not found: $log"

  grep -q "RAMEN OS" "$log" \
    || fail "BOOT_BANNER_MISSING" "serial log missing RAMEN OS boot banner"
  grep -q "golden_machine: gop_probe ok" "$log" \
    || fail "GOP_PROBE_MISSING" "serial log missing golden_machine: gop_probe ok"
  grep -q "golden_machine: gop_fill ok" "$log" \
    || fail "GOP_FILL_MISSING" "serial log missing golden_machine: gop_fill ok"
  grep -q "golden_machine: hil_boot ok" "$log" \
    || fail "HIL_BOOT_MISSING" "serial log missing golden_machine: hil_boot ok"
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
  echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO capturing serial dev=${dev} timeout_s=${timeout_s}"
  echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO operator=power_cycle_nuc_and_select_usb_boot"

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
    if ! wait_for_serial_log "$log" "golden_machine: hil_boot ok" "$timeout_s"; then
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    else
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    fi
  fi
}

echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO step=inventory"

test -f docs/plans/2026-06-21-s12-golden-machine-design.md \
  || fail "DESIGN_DOC_MISSING" "S12 design doc not found"

test -f hardware/golden_machine_v0.toml \
  || fail "MANIFEST_MISSING" "hardware/golden_machine_v0.toml not found"

grep -q 'OP_HIL_BOOT' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_HIL_BOOT in kernel init"

grep -q 'hil_boot' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add hil_boot init profile"

test -x tools/hil/build_usb_boot_image.sh \
  || fail "USB_BUILD_SCRIPT_MISSING" "tools/hil/build_usb_boot_image.sh must be executable"

grep -q 'hil_boot_ok = "golden_machine: hil_boot ok"' hardware/golden_machine_v0.toml \
  || fail "MANIFEST_MARKER_MISSING" "manifest must declare hil_boot_ok marker"

if [[ "${RAMEN_HIL_GOLDEN_MACHINE:-}" != "1" ]]; then
  skip_gate "RAMEN_HIL_GOLDEN_MACHINE not set"
fi

echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO step=build_usb_boot_image"
bash "$ROOT_DIR/tools/hil/build_usb_boot_image.sh"

USB_OUT="${RAMEN_HIL_USB_OUT:-$ROOT_DIR/out/hil/usb_boot}"
test -f "$USB_OUT/EFI/BOOT/BOOTX64.EFI" \
  || fail "USB_EFI_MISSING" "USB boot tree missing BOOTX64.EFI"
test -f "$USB_OUT/EFI/BOOT/init.img" \
  || fail "USB_INIT_MISSING" "USB boot tree missing init.img"

echo "FOUNDRY_S12_HIL_BOOT_S12_2: METRIC usb_boot_tree=${USB_OUT}"
echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO operator_steps="
echo "  1. Format a USB stick as FAT32 (GPT partition table recommended)."
echo "  2. Copy contents of ${USB_OUT}/ onto the USB root (EFI/BOOT/...)."
echo "  3. On Intel NUC reference: enable UEFI USB boot, disable Secure Boot if needed."
echo "  4. Attach serial (115200 8N1) to the NUC debug header or USB-serial adapter."
echo "  5. Power-cycle and boot from USB; gate captures serial until hil_boot ok."

LOG_DIR="$ROOT_DIR/out/logs"
mkdir -p "$LOG_DIR"

if [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
  echo "FOUNDRY_S12_HIL_BOOT_S12_2: INFO step=validate_serial_log path=${RAMEN_HIL_SERIAL_LOG}"
  assert_serial_log "$RAMEN_HIL_SERIAL_LOG"
elif [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
  LOG="$LOG_DIR/hil_boot_serial.log"
  TIMEOUT_S="${RAMEN_HIL_BOOT_TIMEOUT_S:-120}"
  capture_serial_boot "$RAMEN_HIL_SERIAL_DEV" "$LOG" "$TIMEOUT_S"
  if ! wait_for_serial_log "$LOG" "golden_machine: hil_boot ok" 5; then
    echo "--- hil serial log (tail) ---" >&2
    tail -n 60 "$LOG" >&2 || true
    fail "HIL_BOOT_TIMEOUT" \
      "serial capture timed out before golden_machine: hil_boot ok"
  fi
  assert_serial_log "$LOG"
else
  fail "SERIAL_INPUT_MISSING" \
    "set RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 or RAMEN_HIL_SERIAL_LOG=/path/to/capture.log"
fi

echo "FOUNDRY_S12_HIL_BOOT_S12_2: PASS"
echo "FOUNDRY_S12_HIL_BOOT_S12_2: ok"