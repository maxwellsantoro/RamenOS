#!/usr/bin/env bash
# Foundry gate for S10.5 Host-to-Target Integration.
#
# Phase 0: inventory assertions (PASS today — documents host vs QEMU split).
# Phase 1: QEMU semantic snapshot.
# S10.5.1 host broker/proxy bridge is covered by foundry_broker_kernel_bridge_s10_5_1.sh.
#
# See: docs/plans/2026-06-17-s10-5-host-to-target-integration.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="$ROOT_DIR/target"

echo "=== S10.5 Host-to-Target Integration Foundry Gate ==="

fail() {
  echo "FOUNDRY_HOST_TARGET_S10_5: FAIL code=$1 detail=$2" >&2
  exit 1
}

# ---------------------------------------------------------------------------
# Phase 0: Inventory — what runs where (must pass before implementation)
# ---------------------------------------------------------------------------
echo "FOUNDRY_HOST_TARGET_S10_5: INFO step=inventory"

test -f docs/plans/2026-06-17-s10-5-host-to-target-integration.md \
  || fail "DESIGN_DOC_MISSING" "S10.5 design doc not found"

grep -q "CHOSEN" docs/plans/2026-06-17-s10-5-host-to-target-integration.md \
  || fail "OPTION_UNPINNED" "design doc must pin chosen option"

grep -q 'wasmtime' services/native_runner/Cargo.toml \
  || fail "INVENTORY" "native_runner must depend on wasmtime (host executor)"

grep -q 'cdylib' services/semantic_state/Cargo.toml \
  || fail "INVENTORY" "semantic_state must be a WASM cdylib"

if grep -q 'qemu' tools/ci/foundry_native_runner_s10_0.sh \
  || grep -q 'qemu' tools/ci/foundry_native_runner_s10_1.sh; then
  fail "INVENTORY" "native_runner gates must not invoke QEMU (host-only today)"
fi

grep -q 'SimulatedKernelOps' services/domain_manager/src/broker.rs \
  || fail "INVENTORY" "domain_manager broker still uses SimulatedKernelOps"

grep -q 'shmem_test' tools/init/build_init_image.py \
  || fail "INVENTORY" "expected QEMU init profile shmem_test in build_init_image.py"

echo "FOUNDRY_HOST_TARGET_S10_5: INVENTORY native_runner_host_wasmtime=true"
echo "FOUNDRY_HOST_TARGET_S10_5: INVENTORY semantic_state_wasm32_cdylib=true"
echo "FOUNDRY_HOST_TARGET_S10_5: INVENTORY qemu_runs_kernel_init_only=true"
echo "FOUNDRY_HOST_TARGET_S10_5: INVENTORY broker_simulated_kernel_ops=true"
echo "FOUNDRY_HOST_TARGET_S10_5: INVENTORY option_a_semantic_state_on_qemu=chosen"

# ---------------------------------------------------------------------------
# Phase 1: QEMU semantic get_snapshot
# ---------------------------------------------------------------------------
echo "FOUNDRY_HOST_TARGET_S10_5: INFO step=qemu_semantic_get_snapshot"

if ! grep -q 'semantic_snapshot' tools/init/build_init_image.py; then
  fail "INIT_PROFILE_MISSING" \
    "add semantic_snapshot profile + OP_SEMANTIC_SNAPSHOT (see S10.5 design doc §3)"
fi

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
    if [[ -f "$log" ]] && grep -q "$pattern" "$log"; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd python3

echo "FOUNDRY_HOST_TARGET_S10_5: INFO building kernel_uefi (test_protocols)"
cargo run -p idl_codegen -- --in "$ROOT_DIR/idl/harness/ping_harness.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/ping_harness.generated.rs" >/dev/null
cargo build -p kernel_uefi --target x86_64-unknown-uefi --features kernel/test_protocols --quiet

echo "FOUNDRY_HOST_TARGET_S10_5: INFO building init image profile=semantic_snapshot"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_semantic_snapshot.img" \
  --profile semantic_snapshot

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_semantic_snapshot.img" "$X86_DIR/init.img"

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

LOG="$LOG_DIR/qemu_x86_64_semantic_snapshot.log"
rm -f "$LOG"

echo "FOUNDRY_HOST_TARGET_S10_5: INFO qemu boot x86_64"
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

if ! wait_for_log "$LOG" "semantic_state: get_snapshot ok" 30; then
  kill "$QEMU_PID" >/dev/null 2>&1 || true
  wait "$QEMU_PID" >/dev/null 2>&1 || true
  echo "--- qemu serial log (tail) ---" >&2
  tail -n 40 "$LOG" >&2 || true
  fail "SEMANTIC_SNAPSHOT_MISSING" \
    "serial log missing 'semantic_state: get_snapshot ok' — implement OP_SEMANTIC_SNAPSHOT"
fi

kill "$QEMU_PID" >/dev/null 2>&1 || true
wait "$QEMU_PID" >/dev/null 2>&1 || true

grep -q "semantic_state: get_snapshot ok" "$LOG" \
  || fail "SEMANTIC_SNAPSHOT_MISSING" "get_snapshot marker not in log"

grep -qE 'semantic_state: snapshot_sha256=[0-9a-f]{16}' "$LOG" \
  || fail "SNAPSHOT_SHA256_MISSING" "verifiable snapshot_sha256 prefix not in serial log"

SHA_PREFIX="$(grep -E 'semantic_state: snapshot_sha256=[0-9a-f]{16}' "$LOG" | head -n1 \
  | sed -E 's/.*snapshot_sha256=([0-9a-f]{16}).*/\1/')"
echo "FOUNDRY_HOST_TARGET_S10_5: METRIC snapshot_sha256_prefix=${SHA_PREFIX}"

echo "FOUNDRY_HOST_TARGET_S10_5: PASS"
echo "FOUNDRY_HOST_TARGET_S10_5: ok"
