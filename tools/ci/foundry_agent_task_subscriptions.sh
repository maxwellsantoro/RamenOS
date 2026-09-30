#!/usr/bin/env bash
# A2.5 shared typed subscription lifecycle; scripted host/Linux evidence only.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
[[ "$(uname -s)" == Linux ]] || { echo 'Subscription comparison gate requires Linux' >&2; exit 1; }
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
export RAMEN_TASK_SUBSCRIPTION_FIXTURE="$PWD/out/agent-task-subscriptions/fixture"
mkdir -p "$RAMEN_TASK_SUBSCRIPTION_FIXTURE"
rm -f "$PWD/out/agent-task-subscriptions/report.json"
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$RAMEN_TASK_SUBSCRIPTION_FIXTURE"
cargo test -p agent_task_adapter --features agent_task_v1_dev --test subscriptions
cargo test -p kernel_api --test agent_task_protocol
cargo test -p store_service --features agent_task_v1_dev --test agent_task_service pull_subscriptions_are_connection_bound_and_discarded_on_disconnect
cargo build -p agent_task_adapter --features agent_task_v1_dev,linux_task_v1_dev --bins
python3 tools/agent_task/test_subscriptions.py --rt "$TARGET_DIR/debug/agent_task_rt_adapter" --lt "$TARGET_DIR/debug/agent_task_lt_adapter" --fixture "$RAMEN_TASK_SUBSCRIPTION_FIXTURE" --worker "$RAMEN_TASK_VALIDATOR_WORKER" --evidence "$PWD/out/agent-task-subscriptions"
echo 'FOUNDRY_AGENT_TASK_SUBSCRIPTIONS: PASS environment=linux claim=scripted-shared-subscription-lifecycle'
