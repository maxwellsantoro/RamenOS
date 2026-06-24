#!/usr/bin/env bash
# Foundry gate for S11.8 runtime harness.net packet I/O in QEMU.
#
# Closes S11 Definition of Done item (4): distilled virtio-net send/receive under
# native harness.net control in QEMU, not only driver_foundry host replay.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S11.8 Runtime harness.net Packet I/O Foundry Gate ==="

fail() {
  echo "FOUNDRY_S11_RUNTIME_NET_S11_8: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: INFO step=inventory"

grep -q 'net_packet_io' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add net_packet_io init profile"

grep -q 'OP_NET_PACKET_IO' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_NET_PACKET_IO in kernel init"

test -f kernel/src/net_harness.rs \
  || fail "NET_HARNESS_MISSING" "kernel net_harness provider missing"

grep -q 'net_packet_oracle_vector' kernel_api/src/lib.rs \
  || fail "ORACLE_VECTOR_MISSING" "kernel_api net_packet_oracle_vector missing"

grep -q 'NET_V1_PROTOCOL_ID' kernel/src/ipc_v0.rs \
  || fail "IPC_DISPATCH_MISSING" "ipc_v0 must dispatch harness.net protocol"

cargo test -p kernel net_harness --features test_protocols --quiet \
  || fail "NET_HARNESS_TESTS" "kernel net_harness unit tests failed"

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: INFO step=host_unit_tests ok"

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

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: INFO building kernel_uefi (test_protocols)"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --features kernel/test_protocols --quiet

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: INFO building init image profile=net_packet_io"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_net_packet_io.img" \
  --profile net_packet_io

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_net_packet_io.img" "$X86_DIR/init.img"

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

LOG="$LOG_DIR/qemu_x86_64_net_packet_io.log"
rm -f "$LOG"

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: INFO qemu boot x86_64"
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

if ! wait_for_log "$LOG" "harness.net: packet_io ok" 30; then
  kill "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  echo "--- qemu serial log (tail) ---" >&2
  tail -n 40 "$LOG" >&2 || true
  fail "PACKET_IO_MISSING" \
    "serial log missing 'harness.net: packet_io ok' — implement OP_NET_PACKET_IO"
fi

kill "$QEMU_PID" >/dev/null 2>&1 || true
wait "$QEMU_PID" >/dev/null 2>&1 || true

grep -q "harness.net: send_packet ok" "$LOG" \
  || fail "SEND_PACKET_MISSING" "send_packet marker not in serial log"
grep -q "harness.net: receive_packet ok" "$LOG" \
  || fail "RECEIVE_PACKET_MISSING" "receive_packet marker not in serial log"
grep -qE 'harness.net: trace_sha256_prefix=482af3005a3520aa' "$LOG" \
  || fail "TRACE_PREFIX_MISSING" "live Oracle trace prefix not in serial log"

echo "FOUNDRY_S11_RUNTIME_NET_S11_8: METRIC trace_sha256_prefix=482af3005a3520aa"
echo "FOUNDRY_S11_RUNTIME_NET_S11_8: PASS"
echo "FOUNDRY_S11_RUNTIME_NET_S11_8: ok"