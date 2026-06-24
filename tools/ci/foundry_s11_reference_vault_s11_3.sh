#!/usr/bin/env bash
# Foundry gate for S11.3 virtio-net Reference Vault assembly.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S11 Driver Factory Reference Vault Gate (S11.3) ==="

fail() {
  echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: FAIL code=$1 detail=$2" >&2
  exit 1
}

VAULT_DIR="drivers/reference_vaults/virtio-net"
TRACE_FIXTURE="$VAULT_DIR/traces/oracle_init_trace.json"
PACKET_TRACE_FIXTURE="$VAULT_DIR/traces/oracle_packet_trace.json"
DATASHEET="$VAULT_DIR/datasheets/virtio-net-v1.3.md"
PROMOTE_CAPTURE="tools/trace/promote_virtio_net_capture.sh"
PROMOTE_PACKET_CAPTURE="tools/trace/promote_virtio_net_packet_capture.sh"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=inventory"

test -d "$VAULT_DIR" \
  || fail "VAULT_MISSING" "virtio-net Reference Vault missing"

test -f "$VAULT_DIR/README.md" \
  || fail "VAULT_README_MISSING" "virtio-net vault README missing"

test -f "$VAULT_DIR/notes.md" \
  || fail "VAULT_NOTES_MISSING" "virtio-net vault notes missing"

test -f "$VAULT_DIR/harness.toml" \
  || fail "VAULT_HARNESS_MISSING" "virtio-net vault harness context missing"

test -f "$DATASHEET" \
  || fail "VAULT_DATASHEET_MISSING" "virtio-net vault datasheet notes missing"

test -f "$TRACE_FIXTURE" \
  || fail "ORACLE_TRACE_MISSING" "virtio-net Oracle trace fixture missing"

test -f "$PACKET_TRACE_FIXTURE" \
  || fail "ORACLE_PACKET_TRACE_MISSING" "virtio-net Oracle packet trace fixture missing"

test -x "$PROMOTE_CAPTURE" \
  || fail "CAPTURE_PROMOTION_MISSING" "virtio-net live capture promotion script missing or not executable"

test -x "$PROMOTE_PACKET_CAPTURE" \
  || fail "PACKET_CAPTURE_PROMOTION_MISSING" "virtio-net packet capture promotion script missing or not executable"

grep -q 'namespace = "harness.net"' idl/harness/net_v1.toml \
  || fail "NET_IDL_MISSING" "net_v1 IDL must define harness.net"

grep -q 'include!("generated/net_v1.generated.rs")' kernel_api/src/lib.rs \
  || fail "NET_BINDING_NOT_INCLUDED" "kernel_api must include generated net_v1 binding"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=datasheet_inventory"
grep -q 'OASIS Virtual I/O Device (VIRTIO) Version 1.3' "$DATASHEET" \
  || fail "DATASHEET_SOURCE_MISSING" "virtio-net datasheet must pin the OASIS VIRTIO source"
grep -q 'VIRTIO_PCI_CAP_COMMON_CFG' "$DATASHEET" \
  || fail "DATASHEET_PCI_COMMON_CFG_MISSING" "virtio-net datasheet must include PCI common config anchor"
grep -q 'VIRTIO_NET_F_MAC' "$DATASHEET" \
  || fail "DATASHEET_NET_MAC_MISSING" "virtio-net datasheet must include MAC feature anchor"
grep -q 'VIRTIO_NET_F_CTRL_VQ' "$DATASHEET" \
  || fail "DATASHEET_CTRL_VQ_MISSING" "virtio-net datasheet must include control virtqueue feature anchor"
grep -q 'DRIVER_OK' "$DATASHEET" \
  || fail "DATASHEET_DRIVER_OK_MISSING" "virtio-net datasheet must include initialization status anchor"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=capture_promotion_dry_run"
TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
cat >"$TMP_DIR/live-tracer-events.jsonl" <<'JSONL'
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
{"seq":2,"timestamp_ns":11,"kind":"mmio_write","bar":0,"offset":18,"width":2,"value":1,"result":"ok"}
JSONL
"$PROMOTE_CAPTURE" "$TMP_DIR/live-tracer-events.jsonl" "$TMP_DIR/oracle_init_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "virtio-net live capture promotion script failed dry-run"
grep -Eq '"trace_id": "sha256:[0-9a-f]{64}"' "$TMP_DIR/oracle_init_trace.json" \
  || fail "CAPTURE_PROMOTION_DRY_RUN" "promoted dry-run trace missing live trace_id"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=packet_capture_promotion_dry_run"
cat >"$TMP_DIR/live-packet-events.jsonl" <<'JSONL'
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","harness":"harness.net","harness_version":"1","capture_tool":"virtio_net_packet_oracle_capture"}}
{"seq":1,"kind":"send_packet","timestamp_ns":10,"request_id":1,"shm_cap":4096,"offset":0,"len":2,"payload_hex":"aabb","status":0,"bytes":2,"result":"ok"}
{"seq":2,"kind":"receive_packet","timestamp_ns":11,"request_id":2,"shm_cap":4096,"offset":2048,"len":2,"payload_hex":"ccdd","status":0,"bytes":2,"result":"ok"}
JSONL
"$PROMOTE_PACKET_CAPTURE" "$TMP_DIR/live-packet-events.jsonl" "$TMP_DIR/oracle_packet_trace.json" \
  || fail "PACKET_CAPTURE_PROMOTION_DRY_RUN" "virtio-net packet capture promotion script failed dry-run"
grep -Eq '"trace_id": "sha256:[0-9a-f]{64}"' "$TMP_DIR/oracle_packet_trace.json" \
  || fail "PACKET_CAPTURE_PROMOTION_DRY_RUN" "promoted packet dry-run trace missing live trace_id"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=idl_lint"
bash tools/ci/foundry_idl_lint.sh \
  || fail "IDL_LINT_FAILED" "net_v1 IDL failed wire-contract lint"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=trace_fixture_schema"
cargo test -p artifact_store_schema virtio_net_reference_vault_trace_fixture_validates --quiet \
  || fail "TRACE_FIXTURE_SCHEMA" "virtio-net Oracle trace fixture failed schema validation"

cargo test -p artifact_store_schema virtio_net_reference_vault_packet_fixture_validates --quiet \
  || fail "PACKET_TRACE_FIXTURE_SCHEMA" "virtio-net Oracle packet trace fixture failed schema validation"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=trace_fixture_replay"
cargo run -p driver_foundry -- replay-trace "$TRACE_FIXTURE" \
  || fail "TRACE_FIXTURE_REPLAY" "virtio-net Oracle trace fixture failed replay translation"

cargo run -p driver_foundry -- replay-packet-trace "$PACKET_TRACE_FIXTURE" \
  || fail "PACKET_TRACE_FIXTURE_REPLAY" "virtio-net Oracle packet trace fixture failed replay translation"

if [[ "${REQUIRE_LIVE_ORACLE_TRACE:-0}" == "1" ]]; then
  echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=live_trace_provenance"
  cargo run -p driver_foundry -- assert-live-trace "$TRACE_FIXTURE" \
    || fail "LIVE_TRACE_PROVENANCE" "virtio-net trace fixture is not a live Oracle capture"
  cargo run -p driver_foundry -- assert-live-packet-trace "$PACKET_TRACE_FIXTURE" \
    || fail "LIVE_PACKET_TRACE_PROVENANCE" "virtio-net packet trace fixture is not a live Oracle capture"
  echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=hardware_packet_rx_provenance"
  cargo run -p driver_foundry -- assert-hardware-packet-trace "$PACKET_TRACE_FIXTURE" \
    || fail "HARDWARE_PACKET_RX_PROVENANCE" "virtio-net packet trace receive path is not hardware-captured (slirp fallback)"
fi

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: INFO step=net_binding_tests"
cargo test -p kernel_api --quiet \
  || fail "NET_BINDING_TESTS" "kernel_api tests failed with generated net_v1 binding"

echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: PASS"
echo "FOUNDRY_S11_REFERENCE_VAULT_S11_3: ok"
