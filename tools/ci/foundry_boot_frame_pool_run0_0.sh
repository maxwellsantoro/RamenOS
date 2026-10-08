#!/usr/bin/env bash
# Pure admission fixtures only; no firmware transition or physical access proof.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/boot-frame-pool/run0-0"
mkdir -p "$EVIDENCE"
rm -f "$EVIDENCE/result.json"
cargo test -p kernel --lib mm::boot_pool_contract_tests:: --no-run --message-format=json \
  > "$EVIDENCE/build.jsonl" 2> "$EVIDENCE/build.log"
cargo test -p kernel --lib mm::boot_pool_contract_tests:: -- --list \
  > "$EVIDENCE/cases.log" 2>&1
python3 tools/ci/boot_pool_result.py --check-list "$EVIDENCE/cases.log"
cargo test -p kernel --lib mm::boot_pool_contract_tests:: -- --test-threads=1 \
  > "$EVIDENCE/tests.log" 2>&1
python3 tools/ci/boot_pool_result.py --evidence "$EVIDENCE"
echo 'FOUNDRY_BOOT_FRAME_POOL_RUN0_0: PASS scope=pure-map-retention-admission firmware=false physical_access=false'
