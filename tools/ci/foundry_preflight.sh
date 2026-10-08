#!/usr/bin/env bash
# Canonical serialized inventory is shared with isolated CI lane runners.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
echo "FOUNDRY_PREFLIGHT: START"
python3 tools/ci/ci_lanes.py all
echo "FOUNDRY_PREFLIGHT: PASS"
