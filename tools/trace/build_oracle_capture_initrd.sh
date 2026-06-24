#!/usr/bin/env bash
# Build a minimal initrd that captures live virtio-net Oracle PCI/MMIO events.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/oracle_capture"
ROOTFS="$OUT_DIR/root"
INITRD="$OUT_DIR/initrd.cpio.gz"
CAPTURE_SOURCE="${CAPTURE_SOURCE:-$ROOT_DIR/tools/trace/virtio_net_oracle_capture.c}"

CC="${CC:-cc}"
CFLAGS="${CFLAGS:--O2 -static}"
if [[ "$(uname -s)" == "Darwin" ]]; then
  CFLAGS="${CFLAGS/ -static/}"
  CFLAGS="$CFLAGS -Wno-deprecated-declarations"
fi

if ! command -v "$CC" >/dev/null 2>&1; then
  echo "Missing compiler: $CC" >&2
  exit 2
fi
if ! command -v cpio >/dev/null 2>&1; then
  echo "Missing cpio" >&2
  exit 2
fi
if ! command -v gzip >/dev/null 2>&1; then
  echo "Missing gzip" >&2
  exit 2
fi

rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"

build_init_binary() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    if command -v zig >/dev/null 2>&1; then
      zig cc -target x86_64-linux-musl -static -O2 \
        -o "$ROOTFS/init" "$CAPTURE_SOURCE"
      return
    fi
    if command -v docker >/dev/null 2>&1; then
      capture_rel="${CAPTURE_SOURCE#"$ROOT_DIR"/}"
      docker run --rm \
        -v "$ROOT_DIR:/src" \
        -w /src \
        alpine:3.20 \
        sh -c "apk add --no-cache build-base >/dev/null && \
          gcc -O2 -static -o /src/out/oracle_capture/root/init /src/${capture_rel}"
      return
    fi
    echo "On macOS, install zig or docker to cross-compile the capture initrd" >&2
    exit 2
  fi

  $CC $CFLAGS -o "$ROOTFS/init" "$CAPTURE_SOURCE"
}

build_init_binary
chmod 0755 "$ROOTFS/init"

if [[ "$CAPTURE_SOURCE" == *virtio_net_packet_oracle_capture.c ]]; then
  bash "$ROOT_DIR/tools/trace/fetch_virtio_net_modules.sh" >/dev/null
  mkdir -p "$ROOTFS/lib/modules"
  cp "$OUT_DIR/modules/failover.ko" \
    "$OUT_DIR/modules/net_failover.ko" \
    "$OUT_DIR/modules/virtio_net.ko" \
    "$ROOTFS/lib/modules/"
fi

(
  cd "$ROOTFS"
  find . -print0 | cpio --null -H newc -o
) | gzip -9 > "$INITRD"

echo "oracle capture initrd: $INITRD"