#!/usr/bin/env bash
# Foundry gate for S10.5.2 QEMU IPC bridge (chardev serial framing).
#
# See: docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.5.2 QEMU IPC Bridge Foundry Gate ==="

fail() {
  echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO step=inventory"

test -f docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md \
  || fail "DESIGN_DOC_MISSING" "S10.5.2 design doc not found"

grep -q 'semantic_ipc_bridge' tools/init/build_init_image.py \
  || fail "INIT_PROFILE_MISSING" "add semantic_ipc_bridge init profile"

grep -q 'OP_SEMANTIC_IPC_RELAY' kernel/src/init.rs \
  || fail "KERNEL_OP_MISSING" "implement OP_SEMANTIC_IPC_RELAY in kernel init"

cargo test -p kernel_api ipc_frame --quiet \
  || fail "IPC_FRAME_TESTS" "kernel_api ipc_frame tests failed"

cargo test -p kernel_harness_proxy validate_rejects --quiet \
  || fail "CHARDEV_UNIT_TESTS" "chardev_serial unit tests failed"

grep -q 'ChardevKernelBridge' services/native_runner/src/kernel_bridge.rs \
  || fail "SUPERVISOR_CHARDEV_MISSING" "ChardevKernelBridge not implemented"

grep -q 'resolve_kernel_ipc_transport' runtime_supervisor/src/native_wasm_runner.rs \
  || fail "SUPERVISOR_TRANSPORT_MISSING" "runtime_supervisor must wire kernel_ipc_transport"

cargo test -p native_runner chardev_kernel_bridge_framed_roundtrip --quiet \
  || fail "NATIVE_RUNNER_CHARDEV_TESTS" "ChardevKernelBridge unit test failed"

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO step=host_unit_tests ok"

OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"
IPC_DIR="$OUT_DIR/ipc_bridge"
mkdir -p "$UEFI_DIR" "$LOG_DIR" "$INIT_DIR" "$IPC_DIR"

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
  local dep_efi
  dep_efi="$(find "$base/deps" -maxdepth 1 -name 'kernel_uefi-*.efi' 2>/dev/null | head -n1 || true)"
  if [[ -n "$dep_efi" && -f "$dep_efi" ]]; then
    echo "$dep_efi"
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

wait_for_ipc_socket() {
  local sock="$1"
  local timeout_s="$2"
  local max_iters=$((timeout_s * 5))
  for _ in $(seq 1 "$max_iters"); do
    if [[ -S "$sock" ]]; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

run_qemu_ipc_bridge() {
  local profile="$1"
  local log="$2"
  local ipc_sock="$3"

  rm -f "$log" "$ipc_sock"

  python3 "$ROOT_DIR/tools/init/build_init_image.py" \
    --out "$INIT_DIR/init_${profile}.img" \
    --profile "$profile"

  local x86_bin
  x86_bin="$(find_uefi_bin x86_64-unknown-uefi)"
  local x86_dir="$UEFI_DIR/x86_64/EFI/BOOT"
  mkdir -p "$x86_dir"
  cp "$x86_bin" "$x86_dir/BOOTX64.EFI"
  cp "$INIT_DIR/init_${profile}.img" "$x86_dir/init.img"

  local ovmf_code ovmf_vars_template ovmf_vars=""
  ovmf_code="$(find_firmware OVMF_CODE \
    /usr/share/OVMF/OVMF_CODE_4M.fd \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/edk2/ovmf/OVMF_CODE.fd \
    /opt/homebrew/share/edk2-ovmf/x64/OVMF_CODE.fd \
    /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
  )" || fail "OVMF_MISSING" "OVMF_CODE firmware not found"

  ovmf_vars_template="$(find_firmware OVMF_VARS \
    /usr/share/OVMF/OVMF_VARS_4M.fd \
    /usr/share/OVMF/OVMF_VARS.fd \
    /usr/share/edk2/ovmf/OVMF_VARS.fd \
    /opt/homebrew/share/edk2-ovmf/x64/OVMF_VARS.fd \
    /opt/homebrew/share/qemu/edk2-x86_64-vars.fd \
  )" || true
  ovmf_vars="$(prepare_vars "$ovmf_vars_template" "$UEFI_DIR/x86_64_vars_${profile}.fd")"

  if [[ -n "${ovmf_vars:-}" ]]; then
    qemu-system-x86_64 \
      -machine q35 -m 512M \
      -drive "if=pflash,format=raw,readonly=on,file=$ovmf_code" \
      -drive "if=pflash,format=raw,file=$ovmf_vars" \
      -drive "format=raw,file=fat:rw:$UEFI_DIR/x86_64" \
      -nographic -serial "file:$log" \
      -serial "unix:$ipc_sock,server=on,wait=off" \
      -monitor none -no-reboot -no-shutdown &
  else
    qemu-system-x86_64 \
      -machine q35 -m 512M \
      -drive "if=pflash,format=raw,readonly=on,file=$ovmf_code" \
      -drive "format=raw,file=fat:rw:$UEFI_DIR/x86_64" \
      -nographic -serial "file:$log" \
      -serial "unix:$ipc_sock,server=on,wait=off" \
      -monitor none -no-reboot -no-shutdown &
  fi
}

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd python3

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO building kernel_uefi (test_protocols)"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
pkill -9 -f 'qemu-system.*ipc_bridge' >/dev/null 2>&1 || true
sleep 1
cargo run -p idl_codegen -- --in "$ROOT_DIR/idl/harness/ping_harness.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/ping_harness.generated.rs" >/dev/null
touch "$ROOT_DIR/kernel/src/init.rs"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --features kernel/test_protocols --quiet
echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO kernel_uefi build ok"

POS_LOG="$LOG_DIR/qemu_x86_64_semantic_ipc_bridge.log"
POS_SOCK="$IPC_DIR/semantic_ipc_bridge.sock"
POSITIVE_OK=0
SHA_PREFIX=""
for POS_ATTEMPT in 1 2; do
  run_qemu_ipc_bridge semantic_ipc_bridge "$POS_LOG" "$POS_SOCK"
  QEMU_PID=$!
  echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO qemu pid=${QEMU_PID} attempt=${POS_ATTEMPT}"

  if ! wait_for_ipc_socket "$POS_SOCK" 30; then
    kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
    wait "$QEMU_PID" >/dev/null 2>&1 || true
    tail -n 40 "$POS_LOG" >&2 || true
    sleep 2
    continue
  fi

  if ! wait_for_log "$POS_LOG" "semantic_ipc: ready" 60; then
    kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
    wait "$QEMU_PID" >/dev/null 2>&1 || true
    tail -n 40 "$POS_LOG" >&2 || true
    sleep 2
    continue
  fi

  echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO kernel ready; sending get_snapshot frame"
  sleep 2
  TRANSACT_OK=0
  for _ in 1 2 3; do
    if python3 "$ROOT_DIR/tools/ci/ipc_bridge_client.py" get_snapshot "$POS_SOCK"; then
      TRANSACT_OK=1
      break
    fi
    sleep 2
  done
  if [[ "$TRANSACT_OK" != "1" ]]; then
    kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
    wait "$QEMU_PID" >/dev/null 2>&1 || true
    tail -n 40 "$POS_LOG" >&2 || true
    sleep 2
    continue
  fi

  if ! wait_for_log "$POS_LOG" "semantic_ipc: get_snapshot ok" 10; then
    kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
    wait "$QEMU_PID" >/dev/null 2>&1 || true
    tail -n 40 "$POS_LOG" >&2 || true
    sleep 2
    continue
  fi

  if ! grep -qE 'semantic_ipc: snapshot_sha256=[0-9a-f]{16}' "$POS_LOG"; then
    kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
    wait "$QEMU_PID" >/dev/null 2>&1 || true
    tail -n 40 "$POS_LOG" >&2 || true
    sleep 2
    continue
  fi

  SHA_PREFIX="$(grep -E 'semantic_ipc: snapshot_sha256=[0-9a-f]{16}' "$POS_LOG" | head -n1 \
    | sed -E 's/.*snapshot_sha256=([0-9a-f]{16}).*/\1/')"
  POSITIVE_OK=1
  kill -9 "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  sleep 2
  break
done

if [[ "$POSITIVE_OK" != "1" ]]; then
  fail "HOST_TRANSACT_FAILED" "host get_snapshot frame transact failed"
fi

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: METRIC snapshot_sha256_prefix=${SHA_PREFIX}"

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: INFO step=negative_oversize_frame"

NEG_LOG="$LOG_DIR/qemu_x86_64_semantic_ipc_bridge_neg.log"
NEG_SOCK="$IPC_DIR/semantic_ipc_bridge_neg.sock"
run_qemu_ipc_bridge semantic_ipc_bridge "$NEG_LOG" "$NEG_SOCK"
NEG_PID=$!

if ! wait_for_ipc_socket "$NEG_SOCK" 30; then
  kill -9 "$NEG_PID" >/dev/null 2>&1 || true
  wait "$NEG_PID" >/dev/null 2>&1 || true
  fail "NEG_SOCKET_MISSING" "negative QEMU IPC unix socket not created"
fi

if ! wait_for_log "$NEG_LOG" "semantic_ipc: ready" 60; then
  kill -9 "$NEG_PID" >/dev/null 2>&1 || true
  wait "$NEG_PID" >/dev/null 2>&1 || true
  fail "NEG_READY_MISSING" "negative boot missing semantic_ipc: ready"
fi

sleep 2
NEG_REJECT_OK=0
NEG_SEND_OK=0
for _ in 1 2 3; do
  if python3 "$ROOT_DIR/tools/ci/ipc_bridge_client.py" oversize "$NEG_SOCK"; then
    NEG_SEND_OK=1
    if wait_for_log "$NEG_LOG" "semantic_ipc: frame_rejected reason=invalid_length" 10; then
      NEG_REJECT_OK=1
      break
    fi
  fi
  sleep 1
done
if [[ "$NEG_SEND_OK" != "1" ]]; then
  kill -9 "$NEG_PID" >/dev/null 2>&1 || true
  wait "$NEG_PID" >/dev/null 2>&1 || true
  fail "NEG_SEND_FAILED" "oversize frame client could not connect/send"
fi
if [[ "$NEG_REJECT_OK" != "1" ]]; then
  kill -9 "$NEG_PID" >/dev/null 2>&1 || true
  wait "$NEG_PID" >/dev/null 2>&1 || true
  fail "NEG_REJECT_MISSING" "oversize frame not rejected"
fi

kill -9 "$NEG_PID" >/dev/null 2>&1 || true
wait "$NEG_PID" >/dev/null 2>&1 || true

echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: PASS"
echo "FOUNDRY_QEMU_IPC_BRIDGE_S10_5_2: ok"
