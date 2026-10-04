#!/usr/bin/env bash
# Named interrupted lifecycle / lost-reply evidence, never model or metal evidence.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/agent-task-reconciliation"
mkdir -p "$EVIDENCE/fixture"
rm -f "$EVIDENCE/report.json"
[[ "$(uname -s)" == Linux ]] || { echo 'Reconciliation gate requires Linux' >&2; exit 1; }
python3 tools/agent_task/test_lifecycle_ledger.py
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
cargo build -p agent_task_adapter --features agent_task_v1_dev,linux_task_v1_dev --bins
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$EVIDENCE/fixture"
python3 tools/agent_task/reconciliation_gate.py --rt "$TARGET_DIR/debug/agent_task_rt_adapter" --lt "$TARGET_DIR/debug/agent_task_lt_adapter" --worker "$TARGET_DIR/debug/task_validator_worker" --fixture "$EVIDENCE/fixture" --evidence "$EVIDENCE"
cargo test -p store_service --features agent_task_v1_dev --test agent_task_service abrupt_process_crash_before_or_after_publication_recovers_one_effect
echo 'FOUNDRY_AGENT_TASK_RECONCILIATION: PASS environment=linux claim=acknowledged-lifecycle-and-explicit-receipt-recovery'
