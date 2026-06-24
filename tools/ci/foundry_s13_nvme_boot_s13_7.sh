#!/usr/bin/env bash
# Foundry gate for S13.7 metal NVMe boot on the Tier-1 golden machine.
#
# QEMU negative smoke always runs (FAT boot must report not_nvme). Metal serial
# validation runs only when RAMEN_HIL_GOLDEN_MACHINE=1. Default CI skips metal
# legs (fail-closed under RAMEN_CI_STRICT=1).
#
# See: docs/plans/2026-06-21-s13-persistent-storage-design.md §Phase 6

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
export ROOT_DIR
# shellcheck source=../hil/hil_gate_common.sh
source "$ROOT_DIR/tools/hil/hil_gate_common.sh"

echo "=== S13.7 Metal NVMe Boot Foundry Gate ==="

fail() {
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: FAIL code=$1 detail=$2" >&2
  exit 1
}

skip_metal() {
  local reason="$1"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: skipped metal ($reason)"
  if [[ "${RAMEN_CI_STRICT:-}" == "1" ]]; then
    echo "FOUNDRY_S13_NVME_BOOT_S13_7: FAIL (strict mode, skip not allowed)" >&2
    exit 1
  fi
}

assert_metal_serial_log() {
  local log="$1"
  test -f "$log" || fail "SERIAL_LOG_MISSING" "serial log not found: $log"

  grep -q "RAMEN OS" "$log" \
    || fail "BOOT_BANNER_MISSING" "serial log missing RAMEN OS boot banner"
  grep -q "persistent_storage: nvme_boot ok" "$log" \
    || fail "NVME_BOOT_MISSING" "serial log missing persistent_storage: nvme_boot ok"

  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" || -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
    ramen_hil_assert_provenance_markers "$log" \
      || fail "PROVENANCE_MISSING" "serial log missing hil_evidence provenance markers"
  fi

  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" ]]; then
    grep -q "hil_evidence: boot_epoch_nonce=" "$log" \
      || fail "BOOT_NONCE_MISSING" "graduation log missing hil_evidence: boot_epoch_nonce"
    if grep -q "hil_evidence: boot_epoch_nonce=0" "$log"; then
      fail "BOOT_NONCE_ZERO" "graduation requires non-zero boot_epoch_nonce (set_ramenos_boot_nonce.sh)"
    fi
  fi
}

wait_for_log() {
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

wait_for_serial_log() {
  local log="$1"
  local pattern="$2"
  local timeout_s="$3"
  wait_for_log "$log" "$pattern" "$timeout_s"
}

capture_serial_boot() {
  local dev="$1"
  local log="$2"
  local timeout_s="$3"

  if [[ ! -e "$dev" ]]; then
    fail "SERIAL_DEV_MISSING" "serial device not found: $dev"
  fi

  rm -f "$log"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO capturing serial dev=${dev} timeout_s=${timeout_s}"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO operator=power_cycle_nuc_and_boot_from_nvme_esp"

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
    if ! wait_for_serial_log "$log" "persistent_storage: nvme_boot ok" "$timeout_s"; then
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    else
      kill "$cat_pid" >/dev/null 2>&1 || true
      wait "$cat_pid" >/dev/null 2>&1 || true
    fi
  fi
}

find_firmware() {
  local env_var="$1"
  shift
  local override="${!env_var:-}"
  if [[ -n "$override" && -f "$override" ]]; then
    echo "$override"
    return 0
  fi
  for candidate in "$@"; do
    if [[ -f "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

prepare_vars() {
  local template="$1"
  local out_file="$2"
  if [[ -z "$template" || ! -f "$template" ]]; then
    echo ""
    return 0
  fi
  if [[ ! -f "$out_file" ]]; then
    cp "$template" "$out_file"
  fi
  echo "$out_file"
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

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "MISSING_CMD" "required command not found: $1"
}

echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO step=inventory"

DESIGN_DOC="$ROOT_DIR/docs/plans/2026-06-21-s13-persistent-storage-design.md"
STORAGE_MANIFEST="$ROOT_DIR/hardware/storage_contract_v0.toml"

test -f "$DESIGN_DOC" \
  || fail "DESIGN_DOC_MISSING" "S13 design doc not found"

test -f "$STORAGE_MANIFEST" \
  || fail "STORAGE_MANIFEST_MISSING" "hardware/storage_contract_v0.toml not found"

grep -q 'foundry_s13_nvme_boot_s13_7.sh' "$DESIGN_DOC" \
  || fail "GATE_NOT_DOCUMENTED" "design doc must reference this gate"

grep -q 'OP_NVME_BOOT' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_NVME_BOOT in kernel init"

grep -q 'nvme_boot' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add nvme_boot init profile"

test -f kernel_uefi/src/nvme_boot_probe.rs \
  || fail "NVME_PROBE_MODULE_MISSING" "kernel_uefi nvme_boot_probe module missing"

grep -q 'set_nvme_boot_probe' kernel/src/boot.rs \
  || fail "NVME_PROBE_STORAGE_MISSING" "kernel boot must store NVMe boot probe info"

grep -q 'nvme_boot_ok = "persistent_storage: nvme_boot ok"' "$STORAGE_MANIFEST" \
  || fail "MANIFEST_MARKER_MISSING" "manifest must declare nvme_boot_ok marker"

test -x tools/hil/build_nvme_boot_image.sh \
  || fail "NVME_BUILD_SCRIPT_MISSING" "tools/hil/build_nvme_boot_image.sh must be executable"

OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"
mkdir -p "$UEFI_DIR" "$LOG_DIR" "$INIT_DIR"

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd python3

echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO step=build_boot_artifacts"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_nvme_boot.img" \
  --profile nvme_boot

X86_BIN="$(ramen_hil_build_kernel_uefi "$ROOT_DIR" "$INIT_DIR/init_nvme_boot.img")"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_nvme_boot.img" "$X86_DIR/init.img"

echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO step=qemu_negative_smoke"

OVMF_CODE="$(find_firmware OVMF_CODE \
  /usr/share/OVMF/OVMF_CODE_4M.fd \
  /usr/share/OVMF/OVMF_CODE.fd \
  /usr/share/edk2/ovmf/OVMF_CODE.fd \
  /opt/homebrew/share/edk2-ovmf/x64/OVMF_CODE.fd \
  /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
)" || fail "OVMF_MISSING" "OVMF_CODE firmware not found"

OVMF_VARS_TEMPLATE="$(find_firmware OVMF_VARS \
  /usr/share/OVMF/OVMF_VARS_4M.fd \
  /usr/share/OVMF/OVMF_VARS.fd \
  /usr/share/edk2/ovmf/OVMF_VARS.fd \
  /opt/homebrew/share/edk2-ovmf/x64/OVMF_VARS.fd \
  /opt/homebrew/share/qemu/edk2-x86_64-vars.fd \
)" || true
OVMF_VARS="$(prepare_vars "$OVMF_VARS_TEMPLATE" "$UEFI_DIR/x86_64_vars.fd")"

NEG_LOG="$LOG_DIR/qemu_x86_64_nvme_boot_negative.log"
rm -f "$NEG_LOG"

if [[ -n "${OVMF_VARS:-}" ]]; then
  qemu-system-x86_64 \
    -machine q35 -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$OVMF_VARS" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
    -nographic -serial file:"$NEG_LOG" -monitor none -no-reboot -no-shutdown &
else
  qemu-system-x86_64 \
    -machine q35 -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
    -nographic -serial file:"$NEG_LOG" -monitor none -no-reboot -no-shutdown &
fi
QEMU_PID=$!

if ! wait_for_log "$NEG_LOG" "persistent_storage: nvme_boot failed reason=not_nvme" 30; then
  kill "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  echo "--- qemu negative serial log (tail) ---" >&2
  tail -n 40 "$NEG_LOG" >&2 || true
  fail "NVME_NEGATIVE_SMOKE_MISSING" \
    "serial log missing 'persistent_storage: nvme_boot failed reason=not_nvme'"
fi

kill "$QEMU_PID" >/dev/null 2>&1 || true
wait "$QEMU_PID" >/dev/null 2>&1 || true

echo "FOUNDRY_S13_NVME_BOOT_S13_7: METRIC qemu_negative_smoke=pass"

if [[ "${RAMEN_HIL_GOLDEN_MACHINE:-}" != "1" ]]; then
  skip_metal "RAMEN_HIL_GOLDEN_MACHINE not set"
  EVIDENCE_LEVEL="PASS/QEMU"
  ramen_hil_emit_evidence_json \
    "$ROOT_DIR/out/evidence/s13_7_nvme_boot_evidence.json" \
    "foundry_s13_nvme_boot_s13_7" \
    "$EVIDENCE_LEVEL" \
    "$NEG_LOG" \
    "persistent_storage: nvme_boot failed reason=not_nvme" \
    "$X86_BIN" \
    "$INIT_DIR/init_nvme_boot.img"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: METRIC evidence_level=${EVIDENCE_LEVEL}"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: PASS level=${EVIDENCE_LEVEL}"
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: ok"
  exit 0
fi

echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO step=build_nvme_boot_image"
bash "$ROOT_DIR/tools/hil/build_nvme_boot_image.sh"

NVME_OUT="${RAMEN_HIL_NVME_OUT:-$ROOT_DIR/out/hil/nvme_boot}"
test -f "$NVME_OUT/EFI/BOOT/BOOTX64.EFI" \
  || fail "NVME_EFI_MISSING" "NVMe boot tree missing BOOTX64.EFI"
test -f "$NVME_OUT/EFI/BOOT/init.img" \
  || fail "NVME_INIT_MISSING" "NVMe boot tree missing init.img"

echo "FOUNDRY_S13_NVME_BOOT_S13_7: METRIC nvme_boot_tree=${NVME_OUT}"
echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO operator_steps="
echo "  1. Copy ${NVME_OUT}/ onto the NVMe EFI System Partition."
echo "  2. Configure UEFI to boot from the NVMe ESP."
echo "  3. Run tools/hil/set_ramenos_boot_nonce.sh before reboot (graduation mode)."
echo "  4. Attach serial (115200 8N1); capture until persistent_storage: nvme_boot ok."

ramen_hil_resolve_serial_input \
  || fail "GRADUATION_SERIAL_POLICY" "invalid serial input for graduation mode"

if [[ -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
  echo "FOUNDRY_S13_NVME_BOOT_S13_7: INFO step=validate_serial_log path=${RAMEN_HIL_SERIAL_LOG}"
  assert_metal_serial_log "$RAMEN_HIL_SERIAL_LOG"
  METAL_LOG="$RAMEN_HIL_SERIAL_LOG"
elif [[ -n "${RAMEN_HIL_SERIAL_DEV:-}" ]]; then
  LOG="$LOG_DIR/nvme_boot_serial.log"
  TIMEOUT_S="${RAMEN_HIL_BOOT_TIMEOUT_S:-120}"
  capture_serial_boot "$RAMEN_HIL_SERIAL_DEV" "$LOG" "$TIMEOUT_S"
  if ! wait_for_serial_log "$LOG" "persistent_storage: nvme_boot ok" 5; then
    echo "--- hil serial log (tail) ---" >&2
    tail -n 60 "$LOG" >&2 || true
    fail "NVME_BOOT_TIMEOUT" \
      "serial capture timed out before persistent_storage: nvme_boot ok"
  fi
  assert_metal_serial_log "$LOG"
  METAL_LOG="$LOG"
else
  fail "SERIAL_INPUT_MISSING" \
    "set RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 or RAMEN_HIL_SERIAL_LOG=/path/to/capture.log"
fi

EVIDENCE_LEVEL="$(ramen_hil_evidence_level)"
NVME_EFI="${NVME_OUT}/EFI/BOOT/BOOTX64.EFI"
NVME_INIT="${NVME_OUT}/EFI/BOOT/init.img"
ramen_hil_emit_evidence_json \
  "$ROOT_DIR/out/evidence/s13_7_nvme_boot_evidence.json" \
  "foundry_s13_nvme_boot_s13_7" \
  "$EVIDENCE_LEVEL" \
  "${METAL_LOG:-$RAMEN_HIL_SERIAL_LOG}" \
  "persistent_storage: nvme_boot ok" \
  "$NVME_EFI" \
  "$NVME_INIT"

echo "FOUNDRY_S13_NVME_BOOT_S13_7: METRIC evidence_level=${EVIDENCE_LEVEL}"
echo "FOUNDRY_S13_NVME_BOOT_S13_7: PASS level=${EVIDENCE_LEVEL}"
echo "FOUNDRY_S13_NVME_BOOT_S13_7: ok"