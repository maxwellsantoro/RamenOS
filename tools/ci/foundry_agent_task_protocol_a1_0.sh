#!/usr/bin/env bash
# A1.0 contract preflight only; no service or target enforcement claim.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/idl_lint.py
cargo test -p kernel_api --test agent_task_protocol
cargo test -p artifact_store_schema --test agent_task_contract
echo "FOUNDRY_AGENT_TASK_PROTOCOL_A1_0: PASS environment=host claim=wire-contract"
