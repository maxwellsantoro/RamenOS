#!/usr/bin/env bash
# Foundry gate for S11.2 Driver Factory replay scoreboard.
#
# S11.1 lands capture tooling and DriverProtocolTraceV0 validation only.
# S11.2 must implement the mock hardware replay scoreboard.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S11 Driver Factory Replay Gate (S11.2) ==="

fail() {
  echo "FOUNDRY_S11_REPLAY: FAIL code=$1 detail=$2" >&2
  exit 1
}

echo "FOUNDRY_S11_REPLAY: INFO step=inventory"

grep -q 'pub mod mock' kernel_api/src/lib.rs \
  || fail "MOCK_MODULE_MISSING" "kernel_api must expose mock hardware helpers"

test -f kernel_api/src/mock.rs \
  || fail "MOCK_PCI_DEVICE_MISSING" "kernel_api/src/mock.rs missing"

grep -q 'ReplayScoreboard' kernel_api/src/mock.rs \
  || fail "REPLAY_SCOREBOARD_MISSING" "implement ReplayScoreboard"

grep -q 'MockPciDevice' kernel_api/src/mock.rs \
  || fail "MOCK_PCI_DEVICE_MISSING" "implement MockPciDevice"

grep -q 'MockPacketHarness' kernel_api/src/mock.rs \
  || fail "MOCK_PACKET_HARNESS_MISSING" "implement MockPacketHarness"

test -f driver_foundry/src/lib.rs \
  || fail "TRACE_TRANSLATOR_MISSING" "driver_foundry trace translator missing"

grep -q 'trace_to_replay_events' driver_foundry/src/lib.rs \
  || fail "TRACE_TRANSLATOR_MISSING" "implement DriverProtocolTraceV0 to PciReplayEvent translation"

echo "FOUNDRY_S11_REPLAY: INFO step=kernel_api_replay_tests"
cargo test -p kernel_api replay_scoreboard --quiet \
  || fail "REPLAY_SCOREBOARD_TESTS" "ReplayScoreboard tests failed"

cargo test -p kernel_api mock_pci_device --quiet \
  || fail "MOCK_PCI_DEVICE_TESTS" "MockPciDevice tests failed"

echo "FOUNDRY_S11_REPLAY: INFO step=trace_fixture_replay"
cargo test -p driver_foundry trace_to_replay_events --quiet \
  || fail "TRACE_TRANSLATOR_TESTS" "DriverProtocolTraceV0 replay translation tests failed"

cargo run -p driver_foundry -- replay-trace drivers/reference_vaults/virtio-net/traces/oracle_init_trace.json \
  || fail "TRACE_FIXTURE_REPLAY" "virtio-net trace fixture did not replay through MockPciDevice"

echo "FOUNDRY_S11_REPLAY: INFO step=virtio_net_init_driver"
cargo test -p driver_foundry virtio_net_init --quiet \
  || fail "VIRTIO_NET_INIT_DRIVER" "virtio-net init driver replay tests failed"

cargo test -p driver_foundry replay_vault_init_trace --quiet \
  || fail "VIRTIO_NET_VAULT_REPLAY" "virtio-net init driver failed vault fixture replay"

echo "FOUNDRY_S11_REPLAY: INFO step=virtio_net_packet_driver"
cargo test -p kernel_api mock_packet_harness --quiet \
  || fail "MOCK_PACKET_HARNESS_TESTS" "MockPacketHarness tests failed"

cargo test -p driver_foundry virtio_net_packet --quiet \
  || fail "VIRTIO_NET_PACKET_DRIVER" "virtio-net packet driver replay tests failed"

cargo test -p driver_foundry replay_vault_packet_trace --quiet \
  || fail "VIRTIO_NET_PACKET_VAULT_REPLAY" "virtio-net packet driver failed vault fixture replay"

cargo run -p driver_foundry -- replay-packet-trace drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json \
  || fail "PACKET_TRACE_FIXTURE_REPLAY" "virtio-net packet trace fixture did not replay through MockPacketHarness"

echo "FOUNDRY_S11_REPLAY: PASS"
echo "FOUNDRY_S11_REPLAY: ok"
