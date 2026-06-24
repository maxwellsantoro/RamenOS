#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=== Running S0 gates ==="
"$ROOT_DIR/tools/ci/foundry_all_s0.sh"

echo "=== Running S1 gate ==="
"$ROOT_DIR/tools/ci/foundry_artifact_s1.sh"

echo "=== Running S2 gates ==="
"$ROOT_DIR/tools/ci/foundry_compat_s2.sh"
"$ROOT_DIR/tools/ci/foundry_init_s2_2.sh"

echo "=== Running S3 gates ==="
"$ROOT_DIR/tools/ci/foundry_trace_s3.sh"
"$ROOT_DIR/tools/ci/foundry_portal_file_ro_s3.sh"
"$ROOT_DIR/tools/ci/foundry_driver_capsule_s3x.sh"

echo "=== Running S4 gate ==="
"$ROOT_DIR/tools/ci/foundry_store_s4.sh"

echo "=== Running S5 gates ==="
"$ROOT_DIR/tools/ci/foundry_store_s5.sh"
"$ROOT_DIR/tools/ci/foundry_posix_s5.sh"

echo "FOUNDRY_ALL_S0_S1_S2_S3_S4_S5: ok"
