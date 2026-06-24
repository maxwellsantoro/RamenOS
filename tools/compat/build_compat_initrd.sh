#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/compat_s2"
ROOTFS="$OUT_DIR/root"
INITRD="$OUT_DIR/initrd.cpio.gz"

CC="${CC:-cc}"
CFLAGS="${CFLAGS:--O2 -static}"
if [[ "$(uname -s)" == "Darwin" ]]; then
  CFLAGS="${CFLAGS/ -static/}"
  CFLAGS="$CFLAGS -Wno-deprecated-declarations"
fi

rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"

if ! command -v "$CC" >/dev/null 2>&1; then
  echo "Missing compiler: $CC" >&2
  exit 2
fi

$CC $CFLAGS -o "$ROOTFS/init" "$ROOT_DIR/tools/compat/compat_init.c"
chmod 0755 "$ROOTFS/init"

(
  cd "$ROOTFS"
  find . -print0 | cpio --null -H newc -o
) | gzip -9 > "$INITRD"

echo "compat initrd: $INITRD"
