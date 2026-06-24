#!/usr/bin/env bash
# Build a UEFI FAT boot tree for the Tier-1 golden machine USB stick.
#
# Output layout (ready to copy onto a FAT32 USB partition):
#   EFI/BOOT/BOOTX64.EFI
#   EFI/BOOT/init.img
#
# See: docs/plans/2026-06-21-s12-golden-machine-design.md §Phase 2

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

OUT_DIR="${RAMEN_HIL_USB_OUT:-$ROOT_DIR/out/hil/usb_boot}"
INIT_DIR="$ROOT_DIR/out/init"
UEFI_BOOT_DIR="$OUT_DIR/EFI/BOOT"

fail() {
  echo "BUILD_USB_BOOT_IMAGE: FAIL code=$1 detail=$2" >&2
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

echo "BUILD_USB_BOOT_IMAGE: INFO building kernel_uefi"
cargo build -p kernel_uefi --target x86_64-unknown-uefi --quiet

echo "BUILD_USB_BOOT_IMAGE: INFO building init image profile=hil_boot"
mkdir -p "$INIT_DIR" "$UEFI_BOOT_DIR"
python3 "$ROOT_DIR/tools/init/build_init_image.py" \
  --out "$INIT_DIR/init_hil_boot.img" \
  --profile hil_boot

X86_BIN="$(find_uefi_bin x86_64-unknown-uefi)"
cp "$X86_BIN" "$UEFI_BOOT_DIR/BOOTX64.EFI"
cp "$INIT_DIR/init_hil_boot.img" "$UEFI_BOOT_DIR/init.img"

echo "BUILD_USB_BOOT_IMAGE: METRIC out_dir=${OUT_DIR}"
echo "BUILD_USB_BOOT_IMAGE: METRIC efi_boot=${UEFI_BOOT_DIR}/BOOTX64.EFI"
echo "BUILD_USB_BOOT_IMAGE: METRIC init_img=${UEFI_BOOT_DIR}/init.img"
echo "BUILD_USB_BOOT_IMAGE: ok"