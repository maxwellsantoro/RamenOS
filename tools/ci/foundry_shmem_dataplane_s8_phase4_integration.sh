#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
UEFI_DIR="$OUT_DIR/uefi"
LOG_DIR="$OUT_DIR/logs"
INIT_DIR="$OUT_DIR/init"

mkdir -p "$UEFI_DIR" "$LOG_DIR"

echo "=== S8 Phase 4: Data-Plane Integration Gate (QEMU-Based) ==="
echo ""

# Helper functions
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

wait_for_log() {
    local log="$1"
    local timeout_s="$2"
    local max_iters=$((timeout_s * 5))

    for _ in $(seq 1 "$max_iters"); do
        if [[ -f "$log" ]] && grep -q "shmem_test:" "$log"; then
            return 0
        fi
        sleep 0.2
    done
    return 1
}

run_qemu() {
    local arch="$1"
    local log="$LOG_DIR/qemu_${arch}_shmem.log"
    shift
    rm -f "$log"

    "$@" -nographic -serial file:"$log" -monitor none -no-reboot -no-shutdown &
    local pid=$!
    if ! wait_for_log "$log" 20; then
        kill "$pid" >/dev/null 2>&1 || true
        wait "$pid" >/dev/null 2>&1 || true
        echo "ERROR: QEMU did not produce expected output" >&2
        cat "$log" || true
        return 1
    fi
    kill "$pid" >/dev/null 2>&1 || true
    wait "$pid" >/dev/null 2>&1 || true
}

# Check requirements
require_cmd cargo
require_cmd qemu-system-x86_64
require_cmd python3

# Build kernel with test_protocols feature
echo "[1/5] Building kernel with test_protocols..."
cargo run -p idl_codegen -- --in "$ROOT_DIR/idl/harness/ping_harness.toml" --out "$ROOT_DIR/kernel_api/src/generated/ping_harness.generated.rs"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --features kernel/test_protocols
echo "✓ Kernel build successful"
echo ""

# Build init image with shmem_test profile
echo "[2/5] Building init image with shmem_test profile..."
python3 "$ROOT_DIR/tools/init/build_init_image.py" --out "$INIT_DIR/init_shmem_test.img" --profile shmem_test
echo "✓ Init image build successful"
echo ""

# Prepare UEFI environment
echo "[3/5] Preparing UEFI environment..."
X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
X86_DIR="$UEFI_DIR/x86_64/EFI/BOOT"
mkdir -p "$X86_DIR"
cp "$X86_BIN" "$X86_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_shmem_test.img" "$X86_DIR/init.img"

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
echo "✓ UEFI environment ready"
echo ""

# Run QEMU with shmem test
echo "[4/5] Running QEMU with shared-memory test..."
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
echo ""

# Parse and assert results
echo "[5/5] Parsing test results..."
LOG="$LOG_DIR/qemu_x86_64_shmem.log"

# Check all 6 tests passed
if grep -q "shmem_test: map_region_increments_refcount PASS" "$LOG"; then
    echo "  ✓ map_region_increments_refcount"
else
    echo "  ✗ map_region_increments_refcount"
    grep "shmem_test: map_region_increments_refcount" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=MAP_REFCOUNT detail=map_region refcount test failed"
    exit 1
fi

if grep -q "shmem_test: map_region_multiple_times_increments_refcount PASS" "$LOG"; then
    echo "  ✓ map_region_multiple_times_increments_refcount"
else
    echo "  ✗ map_region_multiple_times_increments_refcount"
    grep "shmem_test: map_region_multiple_times_increments_refcount" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=MAP_MULTIPLE detail=multiple mapping test failed"
    exit 1
fi

if grep -q "shmem_test: unmap_region_decrements_refcount PASS" "$LOG"; then
    echo "  ✓ unmap_region_decrements_refcount"
else
    echo "  ✗ unmap_region_decrements_refcount"
    grep "shmem_test: unmap_region_decrements_refcount" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=UNMAP_REFCOUNT detail=unmap refcount test failed"
    exit 1
fi

if grep -q "shmem_test: close_region_fails_with_active_mappings PASS" "$LOG"; then
    echo "  ✓ close_region_fails_with_active_mappings"
else
    echo "  ✗ close_region_fails_with_active_mappings"
    grep "shmem_test: close_region_fails_with_active_mappings" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=CLOSE_ACTIVE detail=close with active mappings test failed"
    exit 1
fi

if grep -q "shmem_test: close_region_succeeds_after_all_unmaps PASS" "$LOG"; then
    echo "  ✓ close_region_succeeds_after_all_unmaps"
else
    echo "  ✗ close_region_succeeds_after_all_unmaps"
    grep "shmem_test: close_region_succeeds_after_all_unmaps" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=CLOSE_UNMAP detail=close after unmap test failed"
    exit 1
fi

if grep -q "shmem_test: map_region_checks_rights_against_flags PASS" "$LOG"; then
    echo "  ✓ map_region_checks_rights_against_flags"
else
    echo "  ✗ map_region_checks_rights_against_flags"
    grep "shmem_test: map_region_checks_rights_against_flags" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=RIGHTS_CHECK detail=rights validation test failed"
    exit 1
fi

# Check summary line
if grep -q "shmem_test: 6/6 tests passed" "$LOG"; then
    echo "  ✓ All tests passed"
else
    echo "  ✗ Test summary incorrect"
    grep "shmem_test:" "$LOG" || true
    echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: FAIL code=SUMMARY detail=expected 6/6 tests passed"
    exit 1
fi

echo ""
echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4_INTEGRATION: PASS"
echo "=== S8 Phase 4 Integration Tests Complete ==="
