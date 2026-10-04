#!/usr/bin/env bash
# A2.2 scripted JSON consumer, no model or target enforcement claim.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
export RAMEN_TASK_ADAPTER_EVIDENCE_DIR="${RAMEN_TASK_ADAPTER_EVIDENCE_DIR:-$PWD/out/agent-task-adapter}"
cargo test -p agent_task_adapter
cargo test -p agent_task_adapter --features agent_task_v1_dev
cargo build -p agent_task_adapter --features agent_task_v1_dev --bin agent_task_rt_adapter
python3 tools/agent_task/test_rt_adapter.py "$TARGET_DIR/debug/agent_task_rt_adapter"
echo "FOUNDRY_AGENT_TASK_ADAPTER: PASS environment=host claim=scripted-json-to-native-task"
