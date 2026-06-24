#!/usr/bin/env bash
# Foundry gate for S13.3 virtio-blk replay scoreboard.
#
# Host-side MockPciDevice replay of the live Oracle init trace plus distilled
# virtio_blk_init driver parity checks.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S13 Block Replay Scoreboard Gate (S13.3) ==="

fail() {
  echo "FOUNDRY_S13_REPLAY: FAIL code=$1 detail=$2" >&2
  exit 1
}

TRACE_FIXTURE="drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json"

echo "FOUNDRY_S13_REPLAY: INFO step=inventory"

test -f "$TRACE_FIXTURE" \
  || fail "ORACLE_TRACE_MISSING" "virtio-blk Oracle init trace fixture missing"

grep -q 'pub mod virtio_blk_init' driver_foundry/src/lib.rs \
  || fail "BLK_INIT_DRIVER_MISSING" "driver_foundry must expose virtio_blk_init"

grep -q 'trace_to_replay_events' driver_foundry/src/lib.rs \
  || fail "TRACE_TRANSLATOR_MISSING" "driver_foundry trace translator missing"

echo "FOUNDRY_S13_REPLAY: INFO step=trace_fixture_replay"
cargo run -p driver_foundry -- replay-trace "$TRACE_FIXTURE" \
  || fail "TRACE_FIXTURE_REPLAY" "virtio-blk trace fixture did not replay through MockPciDevice"

cargo run -p driver_foundry -- replay-blk-init-trace "$TRACE_FIXTURE" \
  || fail "BLK_INIT_DRIVER_REPLAY" "virtio-blk init driver failed vault fixture replay"

echo "FOUNDRY_S13_REPLAY: INFO step=virtio_blk_init_driver"
cargo test -p driver_foundry virtio_blk_init --quiet \
  || fail "VIRTIO_BLK_INIT_DRIVER" "virtio-blk init driver replay tests failed"

cargo test -p driver_foundry replay_vault_init_trace --quiet \
  || fail "VIRTIO_BLK_VAULT_REPLAY" "virtio-blk init driver failed vault fixture replay"

echo "FOUNDRY_S13_REPLAY: INFO step=virtio_blk_sector_driver"
SECTOR_TRACE_FIXTURE="drivers/reference_vaults/virtio-blk/traces/oracle_block_trace.json"

test -f "$SECTOR_TRACE_FIXTURE" \
  || fail "ORACLE_SECTOR_TRACE_MISSING" "virtio-blk Oracle block trace fixture missing"

cargo test -p kernel_api block_harness --quiet \
  || fail "MOCK_BLOCK_HARNESS_TESTS" "MockBlockHarness tests failed"

cargo test -p driver_foundry virtio_blk_sector --quiet \
  || fail "VIRTIO_BLK_SECTOR_DRIVER" "virtio-blk sector driver replay tests failed"

cargo test -p driver_foundry replay_vault_sector_trace --quiet \
  || fail "VIRTIO_BLK_SECTOR_VAULT_REPLAY" "virtio-blk sector driver failed vault fixture replay"

cargo run -p driver_foundry -- replay-sector-trace "$SECTOR_TRACE_FIXTURE" \
  || fail "SECTOR_TRACE_FIXTURE_REPLAY" "virtio-blk block trace fixture did not replay through MockBlockHarness"

echo "FOUNDRY_S13_REPLAY: PASS"
echo "FOUNDRY_S13_REPLAY: ok"