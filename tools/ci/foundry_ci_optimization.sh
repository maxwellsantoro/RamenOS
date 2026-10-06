#!/usr/bin/env bash
# Tooling regressions preserve full gate coverage and fresh runtime evidence.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/test_stage_timer.py
python3 tools/ci/test_broker_gate_once.py
python3 tools/ci/test_ci_lanes.py
python3 tools/ci/test_build_cache.py
python3 tools/ci/test_dev_check.py
python3 tools/ci/test_dev_recipe.py
python3 tools/ci/test_cargo_artifact.py
cargo test --locked -p idl_codegen --test content_stable
printf '%s\n' 'FOUNDRY_CI_OPTIMIZATION: PASS scope=tooling'
