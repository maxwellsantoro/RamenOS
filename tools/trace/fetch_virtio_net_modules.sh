#!/usr/bin/env bash
# Fetch and decompress virtio_net module chain for the compat kernel (6.6.50).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/oracle_capture/modules"
MODULES_DEB="$OUT_DIR/linux-modules.deb"
MODULES_URL="${VIRTIO_NET_MODULES_URL:-https://kernel.ubuntu.com/~kernel-ppa/mainline/v6.6.50/amd64/linux-modules-6.6.50-060650-generic_6.6.50-060650.202409080634_amd64.deb}"
MODULE_ROOT="lib/modules/6.6.50-060650-generic"

mkdir -p "$OUT_DIR"

if [[ ! -f "$MODULES_DEB" ]]; then
  curl --fail --silent --show-error --location \
    --retry 3 --retry-delay 2 \
    "$MODULES_URL" -o "$MODULES_DEB"
fi

extract_deb() {
  local deb="$1"
  local out_dir="$2"
  rm -rf "$out_dir"
  mkdir -p "$out_dir"
  if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb -x "$deb" "$out_dir"
    return 0
  fi
  local tmp
  tmp="$(mktemp -d)"
  cp "$deb" "$tmp/pkg.deb"
  (
    cd "$tmp"
    ar x pkg.deb
    local data_tar
    if [[ -f data.tar ]]; then
      data_tar="data.tar"
    else
      data_tar="$(ls data.tar.* 2>/dev/null | head -n 1)"
    fi
    tar -xf "$data_tar" -C "$out_dir"
  )
  rm -rf "$tmp"
}

extract_deb "$MODULES_DEB" "$OUT_DIR/extracted"

if ! command -v zstd >/dev/null 2>&1; then
  echo "fetch_virtio_net_modules: missing zstd" >&2
  exit 2
fi

for pair in \
  "kernel/net/core/failover.ko.zst:failover.ko" \
  "kernel/drivers/net/net_failover.ko.zst:net_failover.ko" \
  "kernel/drivers/net/virtio_net.ko.zst:virtio_net.ko"; do
  src_rel="${pair%%:*}"
  dst_name="${pair##*:}"
  src="$OUT_DIR/extracted/$MODULE_ROOT/$src_rel"
  dst="$OUT_DIR/$dst_name"
  if [[ ! -f "$src" ]]; then
    echo "fetch_virtio_net_modules: missing $src" >&2
    exit 2
  fi
  zstd -d -f "$src" -o "$dst"
done

echo "virtio_net modules: $OUT_DIR"