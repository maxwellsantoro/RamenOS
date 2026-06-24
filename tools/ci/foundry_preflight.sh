#!/usr/bin/env bash
# Local preflight gate that mirrors CI execution order.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "FOUNDRY_PREFLIGHT: START"
echo "FOUNDRY_PREFLIGHT: INFO step=fmt-check"
cargo fmt --all --check

echo "FOUNDRY_PREFLIGHT: INFO step=codegen"
bash tools/ci/run_codegen.sh

echo "FOUNDRY_PREFLIGHT: INFO step=idl-lint"
bash tools/ci/foundry_idl_lint.sh

echo "FOUNDRY_PREFLIGHT: INFO step=build-targets"
just build-targets

echo "FOUNDRY_PREFLIGHT: INFO step=lint-baseline-strict"
bash tools/ci/foundry_lint_baseline.sh

echo "FOUNDRY_PREFLIGHT: INFO step=lint-strict-tranches"
bash tools/ci/foundry_lint_strict_tranche1.sh
bash tools/ci/foundry_lint_strict_tranche2.sh
bash tools/ci/foundry_lint_strict_tranche3.sh
bash tools/ci/foundry_lint_strict_tranche4.sh
bash tools/ci/foundry_lint_strict_tranche5.sh
bash tools/ci/foundry_lint_strict_tranche6.sh

echo "FOUNDRY_PREFLIGHT: INFO step=workspace-tests-host"
cargo test --workspace --exclude kernel_uefi --exclude kernel_aarch64

echo "FOUNDRY_PREFLIGHT: INFO step=foundry-umbrella"
tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh

echo "FOUNDRY_PREFLIGHT: INFO step=foundry-ci-extended"
bash tools/ci/foundry_ci_extended.sh

echo "FOUNDRY_PREFLIGHT: PASS"
