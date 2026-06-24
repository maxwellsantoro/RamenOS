#!/usr/bin/env bash
# Foundry gate for S10.2 Semantic State Substrate (schema + delivery path + runner E2E).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.2 Semantic State Substrate Foundry Gate ==="

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=idl_contract"
test -f idl/services/semantic_state_v1.toml
test -f kernel_api/src/generated/semantic_state_v1.generated.rs
test -f sdk/src/generated/services_semantic_state_v1.rs

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=schema_roundtrip"
cargo test -p artifact_store_schema semantic_state --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=snapshot_unit_tests"
cargo test -p semantic_state --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=kernel_api_wire"
cargo test -p kernel_api semantic_state_v1 --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=native_runner_e2e"
cargo test -p native_runner --test integration_test --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=subscribe_delivery"
cargo test -p semantic_state subscribe_delivery --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=capability_filter"
cargo test -p artifact_store_schema filter_ --quiet
cargo test -p semantic_state cap_filtered --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=domain_manager_reactor_publish"
cargo test -p domain_manager inventory_snapshot_publish --quiet

echo "FOUNDRY_SEMANTIC_STATE_S10_2: PASS"
echo "FOUNDRY_SEMANTIC_STATE_S10_2: ok"
