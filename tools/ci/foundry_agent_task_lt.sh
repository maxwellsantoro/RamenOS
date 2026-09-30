#!/usr/bin/env bash
# A2.3 independent Linux transaction broker; no model-comparison claim.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
[[ "$(uname -s)" == Linux ]] || { echo 'LT gate requires Linux' >&2; exit 1; }
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
export RAMEN_TASK_LT_FIXTURE="$PWD/out/agent-task-lt/fixture"
mkdir -p "$RAMEN_TASK_LT_FIXTURE"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$RAMEN_TASK_LT_FIXTURE"
export RAMEN_TASK_ADAPTER_EVIDENCE_DIR="$PWD/out/agent-task-lt"
rm -f "$RAMEN_TASK_ADAPTER_EVIDENCE_DIR/report.json"
python3 tools/agent_task/test_lt_backend.py
cargo test -p agent_task_adapter --features linux_task_v1_dev
cargo build -p agent_task_adapter --features linux_task_v1_dev --bin agent_task_lt_adapter
export RAMEN_TASK_ADAPTER_EVIDENCE_DIR="$PWD/out/agent-task-lt"
python3 tools/agent_task/test_rt_adapter.py "$TARGET_DIR/debug/agent_task_lt_adapter"
cargo build -p agent_task_adapter --features agent_task_v1_dev --bin agent_task_rt_adapter
python3 tools/agent_task/compare_typed_adapters.py --rt "$TARGET_DIR/debug/agent_task_rt_adapter" --lt "$TARGET_DIR/debug/agent_task_lt_adapter" --fixture "$RAMEN_TASK_LT_FIXTURE" --worker "$RAMEN_TASK_VALIDATOR_WORKER" --evidence "$RAMEN_TASK_ADAPTER_EVIDENCE_DIR"
echo 'FOUNDRY_AGENT_TASK_LT: PASS environment=linux claim=scripted-typed-transactions'
