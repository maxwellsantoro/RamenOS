#!/usr/bin/env bash
# Finite declared-interface projection; no whole-authority/model/metal claim.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/agent-task-requestable-authority"
mkdir -p "$EVIDENCE/fixture"
rm -f "$EVIDENCE/report.json"
[[ "$(uname -s)" == Linux ]] || { echo 'Requestable authority gate requires Linux' >&2; exit 1; }
python3 tools/agent_task/test_requestable_authority.py
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
cargo build -p agent_task_adapter --features agent_task_v1_dev,linux_task_v1_dev --bins
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$EVIDENCE/fixture"
python3 tools/agent_task/requestable_authority_gate.py --rt "$TARGET_DIR/debug/agent_task_rt_adapter" --lt "$TARGET_DIR/debug/agent_task_lt_adapter" --worker "$TARGET_DIR/debug/task_validator_worker" --fixture "$EVIDENCE/fixture" --evidence "$EVIDENCE"
echo 'FOUNDRY_AGENT_TASK_REQUESTABLE_AUTHORITY: PASS environment=linux claim=finite-issued-rights-and-lifecycle-points'
