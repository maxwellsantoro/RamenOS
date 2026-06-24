#!/usr/bin/env bash
# Build a UEFI FAT boot tree for S13.7 metal NVMe ESP installation.
#
# Output layout (copy onto the NVMe EFI System Partition):
#   EFI/BOOT/BOOTX64.EFI
#   EFI/BOOT/init.img
#
# See: docs/plans/2026-06-21-s13-persistent-storage-design.md §Phase 6

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

OUT_DIR="${RAMEN_HIL_NVME_OUT:-$ROOT_DIR/out/hil/nvme_boot}"
INIT_DIR="$ROOT_DIR/out/init"
UEFI_BOOT_DIR="$OUT_DIR/EFI/BOOT"

fail() {
  echo "BUILD_NVME_BOOT_IMAGE: FAIL code=$1 detail=$2" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "MISSING_CMD" "required command not found: $1"
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

require_cmd cargo
require_cmd python3

echo "BUILD_NVME_BOOT_IMAGE: INFO building kernel_uefi"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet

echo "BUILD_NVME_BOOT_IMAGE: INFO building init image profile=nvme_boot"
mkdir -p "$INIT_DIR" "$UEFI_BOOT_DIR"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_nvme_boot.img" \
  --profile nvme_boot

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
cp "$X86_BIN" "$UEFI_BOOT_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_nvme_boot.img" "$UEFI_BOOT_DIR/init.img"

echo "BUILD_NVME_BOOT_IMAGE: METRIC out_dir=${OUT_DIR}"
echo "BUILD_NVME_BOOT_IMAGE: METRIC efi_boot=${UEFI_BOOT_DIR}/BOOTX64.EFI"
echo "BUILD_NVME_BOOT_IMAGE: METRIC init_img=${UEFI_BOOT_DIR}/init.img"
echo "BUILD_NVME_BOOT_IMAGE: INFO operator_steps="
echo "  1. Identify the NVMe EFI System Partition (GPT type EF00) on the golden machine."
echo "  2. Mount the ESP (read-only inspection first) and back up existing boot entries if needed."
echo "  3. Copy contents of ${OUT_DIR}/ onto the ESP root (EFI/BOOT/...)."
echo "  4. Configure firmware to boot RamenOS from the NVMe ESP (disable Secure Boot if needed)."
echo "  5. Attach serial (115200 8N1); power-cycle and capture until persistent_storage: nvme_boot ok."
echo "BUILD_NVME_BOOT_IMAGE: ok"