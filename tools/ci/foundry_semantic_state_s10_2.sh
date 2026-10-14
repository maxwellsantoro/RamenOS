#!/usr/bin/env bash
# Foundry gate for S10.2 Semantic State Substrate (schema + delivery path + runner E2E).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.2 Semantic State Substrate Foundry Gate ==="
mkdir -p "$ROOT_DIR/out/foundry/selected-tests"
SELECTED_RUN_ROOT="$(mktemp -d "$ROOT_DIR/out/foundry/selected-tests/semantic-state.XXXXXX")"
RUNNER="$ROOT_DIR/tools/ci/selected_test_runner.py"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=idl_contract"
test -f idl/services/semantic_state_v1.toml
test -f kernel_api/src/generated/semantic_state_v1.generated.rs
test -f sdk/src/generated/services_semantic_state_v1.rs

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=schema_roundtrip"
python3 "$RUNNER" --package artifact_store_schema \
  --filter semantic_state --filter filter_ --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/schema"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=snapshot_unit_tests"
python3 "$RUNNER" --package semantic_state \
  --require-filter subscribe_delivery --require-filter cap_filtered --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/semantic-state"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=kernel_api_wire"
python3 "$RUNNER" --package kernel_api --filter semantic_state_v1 --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/kernel-api"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=native_runner_e2e"
# The integration harness reads this exact workspace fixture path.
rustup target add wasm32-unknown-unknown
cargo build --locked -p hello_wasm --target wasm32-unknown-unknown --target-dir "$ROOT_DIR/target" --quiet
python3 "$RUNNER" --package native_runner --test integration_test \
  --required-input "$ROOT_DIR/target/wasm32-unknown-unknown/debug/hello_wasm.wasm" \
  --output "$SELECTED_RUN_ROOT/native-runner"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=subscribe_delivery"
# Both named subsets were required in the actual full-suite outcomes above.
echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO subscribe_delivery=verified_in_full_suite"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=capability_filter"
echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO filter_=verified_in_schema_union cap_filtered=verified_in_full_suite"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: INFO step=domain_manager_reactor_publish"
python3 "$RUNNER" --package domain_manager --filter inventory_snapshot_publish --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/domain-manager"

echo "FOUNDRY_SEMANTIC_STATE_S10_2: PASS"
echo "FOUNDRY_SEMANTIC_STATE_S10_2: ok"
