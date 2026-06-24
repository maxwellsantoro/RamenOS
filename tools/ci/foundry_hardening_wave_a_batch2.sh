#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_HARDENING_WAVE_A_BATCH2: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# V-03 / SC-03 host-shell execution path default-disabled for posix_runner_v0.
run_gate_test "WAB2_V03_POSIX_DISABLED_DEFAULT" \
  cargo test -p runtime_supervisor tests::posix_runner_v0_disabled_without_feature -- --exact
run_gate_test "WAB2_V03_POSIX_ENABLED_FEATURE" \
  cargo test -p runtime_supervisor --features posix_runner_v0_dev tests::posix_runner_v0_enabled_with_feature -- --exact
echo "FOUNDRY_HARDENING_WAVE_A_BATCH2: PASS control=V-03"

# V-02 / V-14 / SC-02 strict wire payload length and fail-closed unknown-type codegen.
run_gate_test "WAB2_V02_WIRE_LEN_MISMATCH" \
  cargo test -p kernel_api tests::wire_read_payload_rejects_len_mismatch_larger_than_type -- --exact
run_gate_test "WAB2_V02_WIRE_LEN_TOO_SMALL" \
  cargo test -p kernel_api tests::wire_read_payload_rejects_len_too_small -- --exact
run_gate_test "WAB2_V14_WIRE_WRITE_STRICT" \
  cargo test -p kernel_api tests::wire_write_payload_sets_strict_len_and_zeroes_tail -- --exact
run_gate_test "WAB2_V14_CODEGEN_UNKNOWN_TYPE_RUST" \
  cargo test -p idl_codegen tests::render_rust_fails_closed_on_unknown_type -- --exact
run_gate_test "WAB2_V14_CODEGEN_UNKNOWN_TYPE_C" \
  cargo test -p idl_codegen tests::render_c_fails_closed_on_unknown_type -- --exact
echo "FOUNDRY_HARDENING_WAVE_A_BATCH2: PASS control=V-02"
echo "FOUNDRY_HARDENING_WAVE_A_BATCH2: PASS control=V-14"

echo "FOUNDRY_HARDENING_WAVE_A_BATCH2: ok"
