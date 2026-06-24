#!/usr/bin/env bash
# Promote a live virtio-net pci_mmio_tracer JSONL capture into the Reference Vault.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  echo "usage: $0 <tracer-events.jsonl> [out-trace.json]" >&2
  exit 2
}

SRC_JSONL="${1:-}"
DST_TRACE="${2:-drivers/reference_vaults/virtio-net/traces/oracle_init_trace.json}"

if [[ -z "$SRC_JSONL" || "${3:-}" != "" ]]; then
  usage
fi

if [[ ! -f "$SRC_JSONL" ]]; then
  echo "RAMEN_S11_PROMOTE_VIRTIO_NET_CAPTURE: fail src=$SRC_JSONL error=missing_jsonl" >&2
  exit 1
fi

TMP_TRACE="$(mktemp -t ramen_virtio_net_trace.XXXXXX.json)"
cleanup() {
  rm -f "$TMP_TRACE"
}
trap cleanup EXIT

cargo run -p driver_foundry -- import-jsonl "$SRC_JSONL" "$TMP_TRACE"
cargo run -p driver_foundry -- replay-trace "$TMP_TRACE"
cargo run -p driver_foundry -- assert-live-trace "$TMP_TRACE"

mkdir -p "$(dirname "$DST_TRACE")"
cp "$TMP_TRACE" "$DST_TRACE"

echo "RAMEN_S11_PROMOTE_VIRTIO_NET_CAPTURE: ok src=$SRC_JSONL dst=$DST_TRACE"
