#!/usr/bin/env bash
# Foundry gate for S13.4 virtio-blk harness.block sector Oracle capture.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S13 Block Sector Oracle Capture Gate (S13.4) ==="

fail() {
  echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: FAIL code=$1 detail=$2" >&2
  exit 1
}

VAULT_DIR="drivers/reference_vaults/virtio-blk"
TRACE_FIXTURE="$VAULT_DIR/traces/oracle_block_trace.json"
CAPTURE_SCRIPT="tools/trace/capture_virtio_blk_sector_oracle.sh"
PROMOTE_CAPTURE="tools/trace/promote_virtio_blk_sector_capture.sh"
CAPTURE_SOURCE="tools/trace/virtio_blk_sector_oracle_capture.c"

echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: INFO step=inventory"

test -f "$CAPTURE_SOURCE" \
  || fail "CAPTURE_SOURCE_MISSING" "virtio_blk_sector_oracle_capture.c missing"

test -x "$CAPTURE_SCRIPT" \
  || fail "CAPTURE_SCRIPT_MISSING" "capture_virtio_blk_sector_oracle.sh missing or not executable"

test -x "$PROMOTE_CAPTURE" \
  || fail "CAPTURE_PROMOTION_MISSING" "promote_virtio_blk_sector_capture.sh missing or not executable"

test -f "$TRACE_FIXTURE" \
  || fail "ORACLE_TRACE_MISSING" "virtio-blk Oracle block trace fixture missing"

grep -q 'pub mod virtio_blk_sector' driver_foundry/src/lib.rs \
  || fail "BLK_SECTOR_DRIVER_MISSING" "driver_foundry must expose virtio_blk_sector"

grep -q 'MockBlockHarness' kernel_api/src/mock.rs \
  || fail "MOCK_BLOCK_HARNESS_MISSING" "kernel_api must expose MockBlockHarness"

echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: INFO step=capture_promotion_dry_run"
TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
python3 - "$TMP_DIR/live-sector-events.jsonl" <<'PY'
import json, sys

path = sys.argv[1]
payload = "00" * 512
meta = {
    "metadata": {
        "oracle": "linux-virtio-blk",
        "device_model": "virtio-blk-pci",
        "harness": "harness.block",
        "harness_version": "1",
        "capture_tool": "virtio_blk_sector_oracle_capture",
    }
}
events = [
    {
        "seq": 1,
        "timestamp_ns": 10,
        "kind": "read_blocks",
        "request_id": 1,
        "lba": 0,
        "block_count": 1,
        "block_size": 512,
        "shm_cap": 4096,
        "offset": 0,
        "len": 512,
        "payload_hex": payload,
        "status": 0,
        "bytes": 512,
        "result": "ok",
    },
    {
        "seq": 2,
        "timestamp_ns": 11,
        "kind": "write_blocks",
        "request_id": 2,
        "lba": 1,
        "block_count": 1,
        "block_size": 512,
        "shm_cap": 4096,
        "offset": 512,
        "len": 512,
        "payload_hex": payload,
        "status": 0,
        "bytes": 512,
        "result": "ok",
    },
]
with open(path, "w") as out:
    out.write(json.dumps(meta) + "\n")
    for event in events:
        out.write(json.dumps(event) + "\n")
PY
"$PROMOTE_CAPTURE" "$TMP_DIR/live-sector-events.jsonl" "$TMP_DIR/oracle_block_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "virtio-blk sector capture promotion script failed dry-run"
grep -Eq '"trace_id": "sha256:[0-9a-f]{64}"' "$TMP_DIR/oracle_block_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "promoted dry-run trace missing live trace_id"

echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: INFO step=trace_fixture_schema"
cargo test -p artifact_store_schema virtio_blk_reference_vault_sector_fixture_validates --quiet \
  || fail "TRACE_FIXTURE_SCHEMA" "virtio-blk Oracle block trace fixture failed schema validation"

echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: INFO step=trace_fixture_replay"
cargo run -p driver_foundry -- replay-sector-trace "$TRACE_FIXTURE" \
  || fail "SECTOR_TRACE_FIXTURE_REPLAY" "virtio-blk Oracle block trace fixture failed replay"

cargo test -p driver_foundry virtio_blk_sector --quiet \
  || fail "BLK_SECTOR_DRIVER_TESTS" "virtio-blk sector driver unit tests failed"

cargo test -p kernel_api block_harness --quiet \
  || fail "MOCK_BLOCK_HARNESS_TESTS" "MockBlockHarness tests failed"

if [[ "${REQUIRE_LIVE_ORACLE_TRACE:-0}" == "1" ]]; then
  echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: INFO step=live_trace_provenance"
  cargo run -p driver_foundry -- assert-live-sector-trace "$TRACE_FIXTURE" \
    || fail "LIVE_TRACE_PROVENANCE" "virtio-blk block trace fixture is not a live Oracle capture"
fi

echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: PASS"
echo "FOUNDRY_S13_BLOCK_SECTOR_ORACLE_S13_4: ok"