#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"

mkdir -p "$UEFI_DIR" "$LOG_DIR" "$INIT_DIR"

echo "FOUNDRY_INIT_S2_2: init boundary gate"

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

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd qemu-system-aarch64
require_cmd python3

cargo run -p idl_codegen -- --in "$ROOT_DIR/idl/harness/ping_harness.toml" --out "$ROOT_DIR/kernel_api/src/generated/ping_harness.generated.rs"
cargo build -p kernel_uefi --target x86_64-unknown-uefi
cargo build -p kernel_aarch64 --target aarch64-unknown-none --release

INIT_DEFAULT="$INIT_DIR/init_default.img"
INIT_ALT="$INIT_DIR/init_alt.img"
INIT_BAD="$INIT_DIR/init_bad.img"

python3 "$ROOT_DIR/tools/init/build_init_image.py" --out "$INIT_DEFAULT" --profile default
python3 "$ROOT_DIR/tools/init/build_init_image.py" --out "$INIT_ALT" --profile alt
python3 "$ROOT_DIR/tools/init/build_init_image.py" --out "$INIT_BAD" --profile bad

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
AARCH_BIN="$ROOT_DIR/target/aarch64-unknown-none/release/kernel_aarch64"

X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"

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

assert_default_log() {
  local log="$1"
  grep -q "RAMEN OS S0 boot" "$log"
  grep -q "init: hello" "$log"
  grep -q "init: ping/pong ok" "$log"
  grep -q "init: ipc badlen small ok" "$log"
  grep -q "init: ipc badlen large ok" "$log"
  grep -q "init: ipc unknown proto ok" "$log"
  grep -q "init: trace ok" "$log"
}

assert_alt_log() {
  local log="$1"
  grep -q "RAMEN OS S0 boot" "$log"
  grep -q "init: alt hello" "$log"
  grep -q "init: ping/pong ok" "$log"
  grep -q "init: ipc badlen small ok" "$log"
  grep -q "init: ipc badlen large ok" "$log"
  grep -q "init: ipc unknown proto ok" "$log"
  grep -q "init: trace ok" "$log"
}

assert_bad_log() {
  local log="$1"
  grep -q "RAMEN OS S0 boot" "$log"
  grep -q "init: missing content id" "$log"
}

wait_for_assert() {
  local log="$1"
  local timeout_s="$2"
  local assert_fn="$3"
  local max_iters=$((timeout_s * 5))

  for _ in $(seq 1 "$max_iters"); do
    if [[ -f "$log" ]] && "$assert_fn" "$log"; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

run_qemu_x86() {
  local log="$1"
  local assert_fn="$2"
  rm -f "$log"
  if [[ -n "${OVMF_VARS:-}" ]]; then
    qemu-system-x86_64 \
      -machine q35 \
      -m 512M \
      -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
      -drive if=pflash,format=raw,file="$OVMF_VARS" \
      -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
      -nographic -serial file:"$log" -monitor none -no-reboot -no-shutdown &
  else
    qemu-system-x86_64 \
      -machine q35 \
      -m 512M \
      -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
      -drive format=raw,file=fat:rw:"$UEFI_DIR/x86_64" \
      -nographic -serial file:"$log" -monitor none -no-reboot -no-shutdown &
  fi
  local pid=$!
  if ! wait_for_assert "$log" 20 "$assert_fn"; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
    "$assert_fn" "$log"
  fi
  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
}

run_qemu_aarch64() {
  local log="$1"
  local image="$2"
  local assert_fn="$3"
  rm -f "$log"
  qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -m 512M \
    -kernel "$AARCH_BIN" \
    -device loader,file="$image",addr=0x44000000,force-raw=on \
    -nographic -serial file:"$log" -monitor none -no-reboot -no-shutdown &
  local pid=$!
  if ! wait_for_assert "$log" 20 "$assert_fn"; then
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
    "$assert_fn" "$log"
  fi
  kill "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
}

echo "FOUNDRY_INIT_S2_2: x86_64 default"
cp "$INIT_DEFAULT" "$X86_DIR/init.img"
log="$LOG_DIR/qemu_init_x86_default.log"
run_qemu_x86 "$log" assert_default_log
assert_default_log "$log"

echo "FOUNDRY_INIT_S2_2: x86_64 alt"
cp "$INIT_ALT" "$X86_DIR/init.img"
log="$LOG_DIR/qemu_init_x86_alt.log"
run_qemu_x86 "$log" assert_alt_log
assert_alt_log "$log"

echo "FOUNDRY_INIT_S2_2: x86_64 bad"
cp "$INIT_BAD" "$X86_DIR/init.img"
log="$LOG_DIR/qemu_init_x86_bad.log"
run_qemu_x86 "$log" assert_bad_log
assert_bad_log "$log"

echo "FOUNDRY_INIT_S2_2: aarch64 default"
log="$LOG_DIR/qemu_init_aarch_default.log"
run_qemu_aarch64 "$log" "$INIT_DEFAULT" assert_default_log
assert_default_log "$log"

echo "FOUNDRY_INIT_S2_2: aarch64 alt"
log="$LOG_DIR/qemu_init_aarch_alt.log"
run_qemu_aarch64 "$log" "$INIT_ALT" assert_alt_log
assert_alt_log "$log"

echo "FOUNDRY_INIT_S2_2: aarch64 bad"
log="$LOG_DIR/qemu_init_aarch_bad.log"
run_qemu_aarch64 "$log" "$INIT_BAD" assert_bad_log
assert_bad_log "$log"

echo "FOUNDRY_INIT_S2_2: ok"
