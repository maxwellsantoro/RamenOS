#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_HARDENING_WAVE_A_BATCH1: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# V-01 / SC-01 strict content-id validation and traversal rejection.
run_gate_test "WAB1_V01_CONTENT_ID_REJECT_CORE" \
  cargo test -p artifact_store_core tests::content_id_rejects_traversal_and_malformed_values -- --exact
run_gate_test "WAB1_V01_CONTENT_ID_ACCEPT_CORE" \
  cargo test -p artifact_store_core tests::content_id_accepts_valid_sha256_lower_hex -- --exact
run_gate_test "WAB1_V01_CONTENT_ID_RUNTIME_REJECT" \
  cargo test -p runtime_supervisor tests::verify_content_id_rejects_traversal_payloads -- --exact
run_gate_test "WAB1_V01_CONTENT_ID_RUNTIME_NONHEX" \
  cargo test -p runtime_supervisor tests::verify_content_id_rejects_non_hex_and_wrong_prefix -- --exact
echo "FOUNDRY_HARDENING_WAVE_A_BATCH1: PASS control=V-01"

# V-09 / SC-08 log-path confinement for compat runner serial sink.
run_gate_test "WAB1_V09_LOG_PATH_REJECT" \
  cargo test -p runtime_supervisor compat_runner::tests::confine_serial_log_path_rejects_absolute_and_traversal -- --exact
run_gate_test "WAB1_V09_LOG_PATH_ACCEPT" \
  cargo test -p runtime_supervisor compat_runner::tests::confine_serial_log_path_accepts_in_root_relative_path -- --exact
echo "FOUNDRY_HARDENING_WAVE_A_BATCH1: PASS control=V-09"

echo "FOUNDRY_HARDENING_WAVE_A_BATCH1: ok"
