#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"

mkdir -p "$UEFI_DIR" "$LOG_DIR"

echo "FOUNDRY_S0: boot gate (UEFI + QEMU)"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 2
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
  echo "UEFI binary not found for target $target" >&2
  return 1
}

assert_log() {
  local log="$1"
  grep -q "RAMEN OS S0 boot" "$log"
  # S8 Phase 3: Assert memory manager initialization
  grep -q "mm: allocator ready" "$log"
  grep -q "init: hello" "$log"
  grep -q "init: ping/pong ok" "$log"
  grep -q "init: ipc badlen small ok" "$log"
  grep -q "init: ipc badlen large ok" "$log"
  grep -q "init: ipc unknown proto ok" "$log"
  grep -q "init: trace ok" "$log"
}

wait_for_log() {
  local log="$1"
  local timeout_s="$2"
  local max_iters=$((timeout_s * 5))

  for _ in $(seq 1 "$max_iters"); do
    if [[ -f "$log" ]] && assert_log "$log"; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd qemu-system-aarch64

cargo run -p idl_codegen -- --in "$ROOT_DIR/idl/harness/ping_harness.toml" --out "$ROOT_DIR/kernel_api/src/generated/ping_harness.generated.rs"

cargo build -p kernel_uefi --target x86_64-unknown-uefi
cargo build -p kernel_aarch64 --target aarch64-unknown-none --release

INIT_IMAGE="$INIT_DIR/init_default.img"
python3 "$ROOT_DIR/tools/init/build_init_image.py" --out "$INIT_IMAGE" --profile default

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
AARCH_BIN="$ROOT_DIR/target/aarch64-unknown-none/release/kernel_aarch64"

X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"

cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_IMAGE" "$X86_DIR/init.img"

OVMF_CODE="$(find_firmware OVMF_CODE \
  /usr/share/OVMF/OVMF_CODE_4M.fd \
  /usr/share/OVMF/OVMF_CODE.fd \
  /usr/share/edk2/ovmf/OVMF_CODE.fd \
  /opt/homebrew/share/edk2-ovmf/x64/OVMF_CODE.fd \
  /opt/homebrew/share/qemu/edk2-x86_64-code.fd \
)"
OVMF_VARS_TEMPLATE="$(find_firmware OVMF_VARS \
  /usr/share/OVMF/OVMF_VARS_4M.fd \
  /usr/share/OVMF/OVMF_VARS.fd \
  /usr/share/edk2/ovmf/OVMF_VARS.fd \
  /opt/homebrew/share/edk2-ovmf/x64/OVMF_VARS.fd \
  /opt/homebrew/share/qemu/edk2-x86_64-vars.fd \
)" || true
OVMF_VARS="$(prepare_vars "$OVMF_VARS_TEMPLATE" "$UEFI_DIR/x86_64_vars.fd")"

run_qemu() {
  local arch="$1"
  local log="$LOG_DIR/qemu_${arch}.log"
  shift
  rm -f "$log"

  "$@" -nographic -serial file:"$log" -monitor none -no-reboot -no-shutdown &
  local pid=$!
  if ! wait_for_log "$log" 20; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
    assert_log "$log"
  fi
  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
}

echo "FOUNDRY_S0: qemu x86_64"
if [[ -n "${OVMF_VARS:-}" ]]; then
  run_qemu x86_64 \
    qemu-system-x86_64 \
    -machine q35 \
    -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$OVMF_VARS" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64"
else
  echo "WARN: OVMF_VARS not found; proceeding with CODE only"
  run_qemu x86_64 \
    qemu-system-x86_64 \
    -machine q35 \
    -m 512M \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64"
fi

echo "FOUNDRY_S0: qemu aarch64 (direct kernel)"
run_qemu aarch64 \
  qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a57 \
  -m 512M \
  -kernel "$AARCH_BIN" \
  -device loader,file="$INIT_IMAGE",addr=0x44000000,force-raw=on

echo "FOUNDRY_S0: ok"
