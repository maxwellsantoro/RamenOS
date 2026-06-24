#!/usr/bin/env bash
# Foundry gate for S13.2 virtio-blk Oracle capture + Reference Vault promotion.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S13 virtio-blk Oracle Capture Gate (S13.2) ==="

fail() {
  echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: FAIL code=$1 detail=$2" >&2
  exit 1
}

VAULT_DIR="drivers/reference_vaults/virtio-blk"
TRACE_FIXTURE="$VAULT_DIR/traces/oracle_init_trace.json"
DATASHEET="$VAULT_DIR/datasheets/virtio-blk-v1.3.md"
CAPTURE_SCRIPT="tools/trace/capture_virtio_blk_oracle.sh"
PROMOTE_CAPTURE="tools/trace/promote_virtio_blk_capture.sh"
CAPTURE_SOURCE="tools/trace/virtio_blk_oracle_capture.c"

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=inventory"

test -d "$VAULT_DIR" \
  || fail "VAULT_MISSING" "virtio-blk Reference Vault missing"

test -f "$VAULT_DIR/README.md" \
  || fail "VAULT_README_MISSING" "virtio-blk vault README missing"

test -f "$VAULT_DIR/notes.md" \
  || fail "VAULT_NOTES_MISSING" "virtio-blk vault notes missing"

test -f "$VAULT_DIR/harness.toml" \
  || fail "VAULT_HARNESS_MISSING" "virtio-blk vault harness context missing"

test -f "$DATASHEET" \
  || fail "VAULT_DATASHEET_MISSING" "virtio-blk vault datasheet notes missing"

test -f "$CAPTURE_SOURCE" \
  || fail "CAPTURE_SOURCE_MISSING" "virtio_blk_oracle_capture.c missing"

test -x "$CAPTURE_SCRIPT" \
  || fail "CAPTURE_SCRIPT_MISSING" "capture_virtio_blk_oracle.sh missing or not executable"

test -x "$PROMOTE_CAPTURE" \
  || fail "CAPTURE_PROMOTION_MISSING" "promote_virtio_blk_capture.sh missing or not executable"

test -f "$TRACE_FIXTURE" \
  || fail "ORACLE_TRACE_MISSING" "virtio-blk Oracle trace fixture missing"

grep -q 'namespace = "harness.block"' idl/harness/block_v1.toml \
  || fail "BLOCK_IDL_MISSING" "block_v1 IDL must define harness.block"

grep -q 'include!("generated/block_v1.generated.rs")' kernel_api/src/lib.rs \
  || fail "BLOCK_BINDING_NOT_INCLUDED" "kernel_api must include generated block_v1 binding"

grep -q 'pub mod virtio_blk_init' driver_foundry/src/lib.rs \
  || fail "BLK_INIT_DRIVER_MISSING" "driver_foundry must expose virtio_blk_init"

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=datasheet_inventory"
grep -q 'OASIS Virtual I/O Device (VIRTIO) Version 1.3' "$DATASHEET" \
  || fail "DATASHEET_SOURCE_MISSING" "virtio-blk datasheet must pin the OASIS VIRTIO source"
grep -q 'VIRTIO_BLK_F_SIZE_MAX' "$DATASHEET" \
  || fail "DATASHEET_BLK_CAPACITY_MISSING" "virtio-blk datasheet must include capacity feature anchor"
grep -q 'DRIVER_OK' "$DATASHEET" \
  || fail "DATASHEET_DRIVER_OK_MISSING" "virtio-blk datasheet must include initialization status anchor"

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=capture_promotion_dry_run"
TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
cat >"$TMP_DIR/live-tracer-events.jsonl" <<'JSONL'
{"metadata":{"oracle":"linux-virtio-blk","device_model":"virtio-blk-pci","pci_vendor_id":6900,"pci_device_id":4097,"pci_bdf":"0000:00:04.0","capture_tool":"virtio_blk_oracle_capture"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
{"seq":2,"timestamp_ns":11,"kind":"mmio_write","bar":0,"offset":18,"width":2,"value":1,"result":"ok"}
JSONL
"$PROMOTE_CAPTURE" "$TMP_DIR/live-tracer-events.jsonl" "$TMP_DIR/oracle_init_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "virtio-blk live capture promotion script failed dry-run"
grep -Eq '"trace_id": "sha256:[0-9a-f]{64}"' "$TMP_DIR/oracle_init_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "promoted dry-run trace missing live trace_id"

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=trace_fixture_schema"
cargo test -p artifact_store_schema virtio_blk_reference_vault_trace_fixture_validates --quiet \
  || fail "TRACE_FIXTURE_SCHEMA" "virtio-blk Oracle trace fixture failed schema validation"

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=trace_fixture_replay"
cargo run -p driver_foundry -- replay-trace "$TRACE_FIXTURE" \
  || fail "TRACE_FIXTURE_REPLAY" "virtio-blk Oracle trace fixture failed mock replay"

cargo run -p driver_foundry -- replay-blk-init-trace "$TRACE_FIXTURE" \
  || fail "BLK_INIT_DRIVER_REPLAY" "virtio-blk init driver failed vault fixture replay"

cargo test -p driver_foundry virtio_blk_init --quiet \
  || fail "BLK_INIT_DRIVER_TESTS" "virtio-blk init driver unit tests failed"

if [[ "${REQUIRE_LIVE_ORACLE_TRACE:-0}" == "1" ]]; then
  echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: INFO step=live_trace_provenance"
  cargo run -p driver_foundry -- assert-live-trace "$TRACE_FIXTURE" \
    || fail "LIVE_TRACE_PROVENANCE" "virtio-blk trace fixture is not a live Oracle capture"
fi

echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: PASS"
echo "FOUNDRY_S13_VIRTIO_BLK_ORACLE_S13_2: ok"