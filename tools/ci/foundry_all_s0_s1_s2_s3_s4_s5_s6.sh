#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

export RAMEN_CI_STRICT="${RAMEN_CI_STRICT:-}"

echo "=== Running S0+S1+S2+S3+S4+S5 gates ==="
"$ROOT_DIR/tools/ci/foundry_all_s0_s1_s2_s3_s4_s5.sh"

echo "=== Running S6 domain manager gate ==="
"$ROOT_DIR/tools/ci/foundry_domain_manager_s6.sh"

echo "=== Running S6 expanded portal suite gate ==="
"$ROOT_DIR/tools/ci/foundry_portal_suite_s6.sh"

echo "=== Running S7 GPU quarantine gate ==="
"$ROOT_DIR/tools/ci/foundry_gpu_quarantine_s7.sh"

echo "=== Running S8 Phase 1 shared-memory typed-contract gate ==="
bash "$ROOT_DIR/tools/ci/foundry_shmem_contract_s8_phase1.sh"

echo "FOUNDRY_ALL_S0_S1_S2_S3_S4_S5_S6_S7: ok"
echo "FOUNDRY_ALL_S0_S1_S2_S3_S4_S5_S6_S7_S8_PHASE1: ok"
