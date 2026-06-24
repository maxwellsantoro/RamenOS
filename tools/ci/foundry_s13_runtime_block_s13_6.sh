#!/usr/bin/env bash
# Foundry gate for S13.6 runtime harness.block sector I/O in QEMU.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S13.6 Runtime harness.block Sector I/O Foundry Gate ==="

fail() {
  echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: INFO step=inventory"

grep -q 'block_io' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add block_io init profile"

grep -q 'OP_BLOCK_IO' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_BLOCK_IO in kernel init"

test -f kernel/src/block_harness.rs \
  || fail "BLOCK_HARNESS_MISSING" "kernel block_harness provider missing"

grep -q 'block_oracle_vector' kernel_api/src/lib.rs \
  || fail "ORACLE_VECTOR_MISSING" "kernel_api block_oracle_vector missing"

grep -q 'BLOCK_V1_PROTOCOL_ID' kernel/src/ipc_v0.rs \
  || fail "IPC_DISPATCH_MISSING" "ipc_v0 must dispatch harness.block protocol"

cargo test -p kernel block_harness --features test_protocols --quiet \
  || fail "BLOCK_HARNESS_TESTS" "kernel block_harness unit tests failed"

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: INFO step=host_unit_tests ok"

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

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: INFO building kernel_uefi (test_protocols)"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --features kernel/test_protocols --quiet

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: INFO building init image profile=block_io"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_block_io.img" \
  --profile block_io

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_block_io.img" "$X86_DIR/init.img"

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

LOG="$LOG_DIR/qemu_x86_64_block_io.log"
rm -f "$LOG"

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: INFO qemu boot x86_64"
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

if ! wait_for_log "$LOG" "persistent_storage: harness.block ok" 30; then
  kill "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  echo "--- qemu serial log (tail) ---" >&2
  tail -n 40 "$LOG" >&2 || true
  fail "BLOCK_IO_MISSING" \
    "serial log missing 'persistent_storage: harness.block ok' — implement OP_BLOCK_IO"
fi

kill "$QEMU_PID" >/dev/null 2>&1 || true
wait "$QEMU_PID" >/dev/null 2>&1 || true

grep -q "persistent_storage: block_read ok" "$LOG" \
  || fail "BLOCK_READ_MISSING" "block_read marker not in serial log"
grep -q "persistent_storage: block_write ok" "$LOG" \
  || fail "BLOCK_WRITE_MISSING" "block_write marker not in serial log"
grep -qE 'persistent_storage: trace_sha256_prefix=eb816f3657bb5807' "$LOG" \
  || fail "TRACE_PREFIX_MISSING" "live Oracle init trace prefix not in serial log"

echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: METRIC trace_sha256_prefix=eb816f3657bb5807"
echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: PASS"
echo "FOUNDRY_S13_RUNTIME_BLOCK_S13_6: ok"