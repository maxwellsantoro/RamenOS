#!/usr/bin/env bash
# Tooling regressions preserve full gate coverage and fresh runtime evidence.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/test_stage_timer.py
python3 tools/ci/test_broker_gate_once.py
python3 tools/ci/test_ci_lanes.py
python3 tools/ci/test_editor_source_inventory.py
python3 tools/ci/test_build_cache.py
python3 tools/ci/test_dev_check.py
python3 tools/ci/test_dev_recipe.py
python3 tools/ci/test_cargo_artifact.py
python3 tools/ci/test_store_test_dependencies.py
python3 tools/ci/test_acceptance_build.py
python3 tools/ci/test_test_timing.py
python3 tools/ci/test_compiler_cache_control.py
python3 tools/ci/test_build_streamlining.py
python3 tools/ci/test_selected_test_runner.py
python3 tools/ci/test_editor_process_preparation.py
python3 tools/ci/test_qemu_boot_observation.py
cargo test --locked -p idl_codegen --test content_stable
printf '%s\n' 'FOUNDRY_CI_OPTIMIZATION: PASS scope=tooling'
