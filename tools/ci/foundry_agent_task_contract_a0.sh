#!/usr/bin/env bash
# SW0 A0: deterministic contract fixtures; no service/kernel boundary claim.
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo test -p artifact_store_schema --test agent_task_contract
echo "FOUNDRY_AGENT_TASK_CONTRACT_A0: PASS environment=host claim=contract-model"
