#!/usr/bin/env bash
# Regression gate for project-review boundary fixes, including StoreClient
# idle recovery, response deadlines and uncertain-ingestion non-replay.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/test_review_boundaries.py
python3 tools/ci/test_boot_feature_authority.py
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
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter ipc_v0::review_authority --filter mm::bootstrap::tests:: \
  --output "$REVIEW_RUN/boot-authority-default"
python3 tools/ci/selected_test_runner.py --package kernel --features test_protocols \
  --filter ipc_v0::review_authority \
  --output "$REVIEW_RUN/boot-authority-oracle"
python3 tools/ci/selected_test_runner.py --package kernel_uefi \
  --filter bootstrap::tests:: --output "$REVIEW_RUN/firmware-conversion"
# Wire generations must remain representable and permanently retire at exhaustion.
python3 tools/ci/selected_test_runner.py --package kernel_api \
  --filter handle_last_wire_generation_roundtrips_for_all_resource_kinds \
  --output "$REVIEW_RUN/wire-generation-api"
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter cap_table::tests::wire_generation_ \
  --filter shmem::tests::wire_generation_ \
  --filter trace_cap::tests::wire_generation_ \
  --output "$REVIEW_RUN/wire-generation-tables"
# Ordinary strict Store publication must be readable after an actual restart.
python3 tools/ci/selected_test_runner.py --package store_service --test strict_publication \
  --filter review_strict_ --output "$REVIEW_RUN/strict-store-publication"
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter ipc_v0::review_caller_namespace:: \
  --filter ipc_v0::review_consent_consumer:: --filter shmem::review_target_consent:: \
  --output "$REVIEW_RUN/caller-and-target-consent"
python3 tools/ci/selected_test_runner.py --package kernel --features test_protocols \
  --filter review_namespace_oracle_payload_caps_deny_then_each_owner_transfers \
  --output "$REVIEW_RUN/provider-namespaces"
python3 tools/ci/selected_test_runner.py --package store_service \
  --filter review_publication_sink_enforces_strict_policy \
  --filter review_signed_dedup_reconciles_original_intent \
  --output "$REVIEW_RUN/strict-store-sink-recovery"
python3 tools/ci/selected_test_runner.py --package store_service --test ingest_transport \
  --filter rejected_descriptor_reply_discards_transport_before_next_explicit_read \
  --output "$REVIEW_RUN/descriptor-denial-recovery"
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter ipc_v0::review_trace_delivery:: \
  --filter trace_service::review_delivery_retirement:: \
  --output "$REVIEW_RUN/trace-shmem-delivery"

# Live Store authority must survive restart and serialize revoke with consumption.
python3 tools/ci/selected_test_runner.py --package store_service \
  --filter live_authority::tests:: --output "$REVIEW_RUN/store-live-authority"
python3 tools/ci/selected_test_runner.py --package store_service \
  --filter review_staged_before_live_revoke \
  --output "$REVIEW_RUN/store-staged-revocation"
# Updated review: explicit codecs, UTF-8, truthful retention, original outcomes
# and operation-specific authority lifetimes. Discovery must stay nonempty.
python3 tools/ci/selected_test_runner.py --package kernel_api \
  --filter wire_codec_ --output "$REVIEW_RUN/generated-wire-codecs"
python3 tools/ci/selected_test_runner.py --package kernel_api --test wire_codec \
  --filter review_wire_ --output "$REVIEW_RUN/wire-padding-and-validity"
# Unsupported bool/enum/reference/raw-Copy decodes are actual compile-fail docs.
cargo test -p kernel_api --doc --quiet
python3 tools/ci/selected_test_runner.py --package kernel \
  --filter review_domain_name_ --filter review_fixed_capacity \
  --filter review_mapping_outcomes --filter review_updated_wire_consumers \
  --output "$REVIEW_RUN/updated-native-boundaries"
python3 tools/ci/selected_test_runner.py --package store_service \
  --filter live_authority::epoch_tests:: \
  --filter live_lease_drains_same_id --filter admitted_read_rechecks_health \
  --filter admitted_publication_excludes_management --filter panicking_lease_drop \
  --filter development_active_id_capacity \
  --output "$REVIEW_RUN/store-drain-and-epoch"
echo "FOUNDRY_REVIEW_BOUNDARIES: PASS"
