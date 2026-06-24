#!/usr/bin/env bash
# Foundry gate for S10.4 Execution Fabric (IDL + schemas + simulation).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.4 Execution Fabric Foundry Gate ==="

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=idl_contract"
test -f idl/services/execution_fabric_v1.toml
test -f kernel_api/src/generated/execution_fabric_v1.generated.rs

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=schema_tests"
cargo test -p artifact_store_schema execution_fabric --quiet
cargo test -p execution_fabric --quiet

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=kernel_api_wire"
cargo test -p kernel_api execution_fabric_v1 --quiet

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=simulation_snapshot"
TMP_SIM="$(mktemp)"
cargo run -p execution_fabric -- --simulate >"$TMP_SIM"
grep -q '"nodes"' "$TMP_SIM"
rm -f "$TMP_SIM"

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=store_cli_validation"
TMP_PLAN="$(mktemp)"
cat >"$TMP_PLAN" <<'EOF'
{
  "schema_version": 1,
  "program_id": "demo.app",
  "artifact_ref": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "runner": { "runner": "native_wasm_v0" },
  "output_contract": {},
  "runner_config": {}
}
EOF
cargo run -p store_cli -- validate-execution-launch-plan --src "$TMP_PLAN" >/dev/null
rm -f "$TMP_PLAN"

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=simulation_policy"
cargo test -p execution_fabric denied_lease --quiet
cargo test -p execution_fabric execution_trace_is_monotonic --quiet
cargo test -p artifact_store_schema trace_requires_monotonic --quiet

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: INFO step=emit_plan_canonical_roundtrip"
cargo test -p store_cli canonical_execution_launch_plan --quiet
cargo test -p runtime_supervisor parses_canonical_with_runner_config --quiet
cargo test -p runtime_supervisor fabric_policy_always_local --quiet

echo "FOUNDRY_EXECUTION_FABRIC_S10_4: PASS"
echo "FOUNDRY_EXECUTION_FABRIC_S10_4: ok"
