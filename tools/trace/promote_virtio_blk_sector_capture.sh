#!/usr/bin/env bash
# Promote a live virtio-blk harness sector JSONL capture into the Reference Vault.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  echo "usage: $0 <sector-events.jsonl> [out-trace.json]" >&2
  exit 2
}

SRC_JSONL="${1:-}"
DST_TRACE="${2:-drivers/reference_vaults/virtio-blk/traces/oracle_block_trace.json}"

if [[ -z "$SRC_JSONL" || "${3:-}" != "" ]]; then
  usage
fi

if [[ ! -f "$SRC_JSONL" ]]; then
  echo "RAMEN_S13_PROMOTE_VIRTIO_BLK_SECTOR_CAPTURE: fail src=$SRC_JSONL error=missing_jsonl" >&2
  exit 1
fi

TMP_TRACE="$(mktemp -t ramen_virtio_blk_sector_trace.XXXXXX.json)"
cleanup() {
  rm -f "$TMP_TRACE"
}
trap cleanup EXIT

cargo run -p driver_foundry -- import-sector-jsonl "$SRC_JSONL" "$TMP_TRACE"
cargo run -p driver_foundry -- replay-sector-trace "$TMP_TRACE"
cargo run -p driver_foundry -- assert-live-sector-trace "$TMP_TRACE"

mkdir -p "$(dirname "$DST_TRACE")"
cp "$TMP_TRACE" "$DST_TRACE"

echo "RAMEN_S13_PROMOTE_VIRTIO_BLK_SECTOR_CAPTURE: ok src=$SRC_JSONL dst=$DST_TRACE"