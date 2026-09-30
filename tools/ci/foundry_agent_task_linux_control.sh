#!/usr/bin/env bash
# A2.1 requires a real Linux container engine; unavailable means failure, never PASS.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [[ "$(uname -s)" != Linux ]]; then
    echo "FOUNDRY_AGENT_TASK_LINUX_CONTROL: Linux required" >&2
    exit 1
fi
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "$FIXTURE_DIR"' EXIT
cargo run -p store_service --features agent_task_v1_dev --example export_agent_task_fixture -- "$FIXTURE_DIR"
chmod 755 "$FIXTURE_DIR"
python3 tools/agent_task/linux_control_gate.py --fixture "$FIXTURE_DIR" \
    --worker "$TARGET_DIR/debug/task_validator_worker" \
    --evidence "${RAMEN_TASK_LINUX_EVIDENCE_DIR:-out/agent-task-linux-control}"
echo "FOUNDRY_AGENT_TASK_LINUX_CONTROL: PASS environment=host claim=linux-scoped-shell-development-fixture"
