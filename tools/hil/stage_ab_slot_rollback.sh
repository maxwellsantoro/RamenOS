#!/usr/bin/env bash
# Host-side S1 rollback rehearsal for S13.8 A/B slot graduation.
#
# Runs the artifact install/run/rollback gate to prove Store rollback discipline
# before flipping the active boot slot on metal.
#
# See: docs/plans/2026-06-21-s13-persistent-storage-design.md §Phase 7

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "STAGE_AB_SLOT_ROLLBACK: INFO running S1 artifact rollback gate"
bash "$ROOT_DIR/tools/ci/foundry_artifact_s1.sh"
echo "STAGE_AB_SLOT_ROLLBACK: METRIC s1_rollback_gate=pass"
echo "STAGE_AB_SLOT_ROLLBACK: ok"