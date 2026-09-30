#!/usr/bin/env bash
# A2.6 finite canonical authority inventory and all-arm negative probes.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/agent-task-authority"
mkdir -p "$EVIDENCE/fixture"
rm -f "$EVIDENCE/report.json"
[[ "$(uname -s)" == Linux ]] || { echo 'Authority gate requires Linux' >&2; exit 1; }
python3 tools/agent_task/test_authority_manifest.py
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
cargo build -p agent_task_adapter --features agent_task_v1_dev,linux_task_v1_dev --bins
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$EVIDENCE/fixture"
python3 tools/agent_task/authority_conformance.py --rt "$TARGET_DIR/debug/agent_task_rt_adapter" --lt "$TARGET_DIR/debug/agent_task_lt_adapter" --fixture "$EVIDENCE/fixture" --worker "$TARGET_DIR/debug/task_validator_worker" --evidence "$EVIDENCE"
echo 'FOUNDRY_AGENT_TASK_AUTHORITY: PASS environment=linux claim=fixed-universe-authority-and-negative-cases'
