#!/usr/bin/env bash
# A2.4 scoped shell commands and durable Linux transactions; no model comparison.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
[[ "$(uname -s)" == Linux ]] || { echo 'LS transaction gate requires Linux' >&2; exit 1; }
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
export RAMEN_TASK_LS_EVIDENCE="$PWD/out/agent-task-ls-transactions"
export RAMEN_TASK_LS_FIXTURE="$RAMEN_TASK_LS_EVIDENCE/fixture"
mkdir -p "$RAMEN_TASK_LS_FIXTURE"
rm -f "$RAMEN_TASK_LS_EVIDENCE/report.json"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$RAMEN_TASK_LS_FIXTURE"
python3 tools/agent_task/test_ls_transactions.py
echo 'FOUNDRY_AGENT_TASK_LS_TRANSACTIONS: PASS environment=linux claim=scripted-scoped-shell-transactions'
