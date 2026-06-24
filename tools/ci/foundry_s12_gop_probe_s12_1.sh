#!/usr/bin/env bash
# Foundry gate for S12.1 UEFI GOP probe (QEMU OVMF stepping stone).
#
# See: docs/plans/2026-06-21-s12-golden-machine-design.md §Phase 1

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S12.1 UEFI GOP Probe Foundry Gate ==="

fail() {
  echo "FOUNDRY_S12_GOP_PROBE_S12_1: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S12_GOP_PROBE_S12_1: INFO step=inventory"

test -f docs/plans/2026-06-21-s12-golden-machine-design.md \
  || fail "DESIGN_DOC_MISSING" "S12 design doc not found"

grep -q 'OP_GOP_PROBE' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_GOP_PROBE in kernel init"

grep -q 'gop_probe' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add gop_probe init profile"

test -f kernel_uefi/src/gop_probe.rs \
  || fail "GOP_PROBE_MODULE_MISSING" "kernel_uefi gop_probe module missing"

grep -q 'set_gop_probe' kernel/src/boot.rs \
  || fail "GOP_PROBE_STORAGE_MISSING" "kernel boot must store GOP probe info"

OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"
mkdir -p "$UEFI_DIR" "$LOG_DIR" "$INIT_DIR"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "MISSING_CMD" "required command not found: $1"
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

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd python3

echo "FOUNDRY_S12_GOP_PROBE_S12_1: INFO building kernel_uefi"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet

echo "FOUNDRY_S12_GOP_PROBE_S12_1: INFO building init image profile=gop_probe"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_gop_probe.img" \
  --profile gop_probe

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_gop_probe.img" "$X86_DIR/init.img"

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

LOG="$LOG_DIR/qemu_x86_64_gop_probe.log"
rm -f "$LOG"

echo "FOUNDRY_S12_GOP_PROBE_S12_1: INFO qemu boot x86_64"
if [[ -n "${OVMF_VARS:-}" ]]; then
  qemu-system-x86_64 \
    -machine q35 -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$OVMF_VARS" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
    -nographic -serial file:"$LOG" -monitor none -no-reboot -no-shutdown &
else
  qemu-system-x86_64 \
    -machine q35 -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
    -nographic -serial file:"$LOG" -monitor none -no-reboot -no-shutdown &
fi
QEMU_PID=$!

if ! wait_for_log "$LOG" "golden_machine: gop_probe ok" 30; then
  kill "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  echo "--- qemu serial log (tail) ---" >&2
  tail -n 40 "$LOG" >&2 || true
  fail "GOP_PROBE_MISSING" \
    "serial log missing 'golden_machine: gop_probe ok'"
fi

kill "$QEMU_PID" >/dev/null 2>&1 || true
wait "$QEMU_PID" >/dev/null 2>&1 || true

grep -q "golden_machine: gop_probe ok" "$LOG" \
  || fail "GOP_PROBE_MISSING" "gop_probe marker not in log"
grep -q "golden_machine: gop_fill ok" "$LOG" \
  || fail "GOP_FILL_MISSING" "gop_fill marker not in log"

GOP_WIDTH="$(grep -E 'golden_machine: gop_width=[0-9]+' "$LOG" | head -n1 \
  | sed -E 's/.*gop_width=([0-9]+).*/\1/')"
GOP_HEIGHT="$(grep -E 'golden_machine: gop_height=[0-9]+' "$LOG" | head -n1 \
  | sed -E 's/.*gop_height=([0-9]+).*/\1/')"
GOP_PIXEL_FORMAT="$(grep -E 'golden_machine: gop_pixel_format=[0-9]+' "$LOG" | head -n1 \
  | sed -E 's/.*gop_pixel_format=([0-9]+).*/\1/')"

if [[ -z "$GOP_WIDTH" || "$GOP_WIDTH" -eq 0 ]]; then
  fail "GOP_WIDTH_INVALID" "gop_width must be nonzero"
fi
if [[ -z "$GOP_HEIGHT" || "$GOP_HEIGHT" -eq 0 ]]; then
  fail "GOP_HEIGHT_INVALID" "gop_height must be nonzero"
fi
if [[ -z "$GOP_PIXEL_FORMAT" || "$GOP_PIXEL_FORMAT" -gt 3 ]]; then
  fail "GOP_PIXEL_FORMAT_INVALID" "gop_pixel_format must be 0..3"
fi

echo "FOUNDRY_S12_GOP_PROBE_S12_1: METRIC gop_width=${GOP_WIDTH}"
echo "FOUNDRY_S12_GOP_PROBE_S12_1: METRIC gop_height=${GOP_HEIGHT}"
echo "FOUNDRY_S12_GOP_PROBE_S12_1: METRIC gop_pixel_format=${GOP_PIXEL_FORMAT}"

echo "FOUNDRY_S12_GOP_PROBE_S12_1: PASS"
echo "FOUNDRY_S12_GOP_PROBE_S12_1: ok"