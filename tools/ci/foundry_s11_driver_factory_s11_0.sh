#!/usr/bin/env bash
# Foundry gate for S11 Driver Factory MVP — S11.0/S11.1 inventory + capture scaffold.
#
# See: docs/plans/2026-02-20-s11-driver-factory-mvp.md

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S11 Driver Factory MVP Foundry Gate (S11.0/S11.1) ==="

fail() {
  echo "FOUNDRY_S11_DRIVER_FACTORY_S11_0: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S11_DRIVER_FACTORY_S11_0: INFO step=inventory"

test -f docs/plans/2026-02-20-s11-driver-factory-mvp.md \
  || fail "DESIGN_DOC_MISSING" "S11 design doc not found"

grep -q 'CHOSEN.*virtio-net' docs/plans/2026-02-20-s11-driver-factory-mvp.md \
  || fail "DEVICE_NOT_CHOSEN" "S11 Oracle device selection must pin virtio-net"

test -d drivers/reference_vaults \
  || fail "REFERENCE_VAULTS_MISSING" "drivers/reference_vaults directory missing"

if [[ ! -f tools/trace/pci_mmio_tracer.c ]]; then
  echo "FOUNDRY_S11_DRIVER_FACTORY_S11_0: RED pci_mmio_tracer=NOT_IMPLEMENTED"
  fail "CAPTURE_TOOLING_MISSING" "implement tools/trace/pci_mmio_tracer.c (S11.1)"
fi

grep -q 'module_init' tools/trace/pci_mmio_tracer.c \
  || fail "CAPTURE_TOOLING_INCOMPLETE" "pci_mmio_tracer must be a Linux kernel module"

grep -q 'ramen_pci_mmio_trace_record' tools/trace/pci_mmio_tracer.c \
  || fail "CAPTURE_TOOLING_INCOMPLETE" "pci_mmio_tracer must expose an auditable record helper"

cargo test -p artifact_store_schema driver_protocol_trace --quiet \
  || fail "TRACE_SCHEMA_TESTS" "DriverProtocolTraceV0 schema tests failed"

cargo run -p capsule_relay -- --list-trace-kinds 2>/dev/null \
  | grep -q 'driver_protocol_trace_v0' \
  || fail "CAPSULE_RELAY_TRACE_KIND" "capsule_relay must advertise driver_protocol_trace_v0"

if [[ ! -f tools/ci/foundry_s11_replay.sh ]]; then
  fail "REPLAY_GATE_MISSING" "add S11.2 replay gate foundry_s11_replay.sh"
fi

if [[ ! -x tools/trace/capture_virtio_net_oracle.sh ]]; then
  fail "CAPTURE_SCRIPT_MISSING" "add executable tools/trace/capture_virtio_net_oracle.sh"
fi

echo "FOUNDRY_S11_DRIVER_FACTORY_S11_0: PASS"
echo "FOUNDRY_S11_DRIVER_FACTORY_S11_0: ok"
