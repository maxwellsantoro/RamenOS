#!/usr/bin/env bash
# Regression gate for project-review boundary fixes, including StoreClient
# idle recovery, response deadlines and uncertain-ingestion non-replay.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/test_review_boundaries.py
python3 tools/agent_task/test_lt_staging.py
python3 tools/hil/test_provenance.py
cargo test -p kernel create_region_rejects_unsupported_power_of_two_page_sizes --quiet
cargo test -p store_service --quiet
cargo test -p kernel review_ --quiet
cargo test -p native_runner review_ --quiet
cargo test -p native_runner --test execution_deadline --quiet
cargo test -p runtime_supervisor review_native_ --quiet
# Every new family must discover actual tests; a renamed/removed assertion
# cannot turn into a zero-test green. These are additive to the existing scope.
mkdir -p out/foundry/selected-tests
REVIEW_RUN="$(mktemp -d "$PWD/out/foundry/selected-tests/review-foundations.XXXXXX")"
python3 tools/ci/selected_test_runner.py --package kernel_api \
  --filter review_ring_empty_operations --output "$REVIEW_RUN/kernel-api"
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter review_block_fragmented_transfer --filter review_net_fragmented_transfer \
  --filter review_trace_short_and_zero --filter review_trace_metadata_and_offsets \
  --filter review_trace_overflow --filter review_trace_ipc \
  --filter review_shmem_linear_range --output "$REVIEW_RUN/kernel"
python3 tools/ci/selected_test_runner.py --package domain_manager \
  --filter review_grant_repeat_denies --filter review_grant_failed_revoke \
  --filter review_grant_failed_rollback --filter review_grant_ipc --output "$REVIEW_RUN/broker"
python3 tools/ci/selected_test_runner.py --package runtime_supervisor \
  --filter review_launch_invalid_explicit --filter review_launch_node_selection \
  --output "$REVIEW_RUN/supervisor"
echo "FOUNDRY_REVIEW_BOUNDARIES: PASS"
