#!/usr/bin/env bash
set -euo pipefail

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_HARDENING_WAVE_C: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# V-11 / SC-09: kernel/services/store boundary split
run_gate_test "WAVEC_V11_SCHEMA_TYPES_SERIALIZABLE" \
  cargo test -p artifact_store_schema tests::schema_types_are_serializable -- --exact
run_gate_test "WAVEC_V11_CONTENT_ID_VALIDATION" \
  cargo test -p artifact_store_schema tests::content_id_validation_works -- --exact
run_gate_test "WAVEC_V11_EVIDENCE_POLICY_VALIDATION" \
  cargo test -p artifact_store_schema tests::evidence_policy_validation_works -- --exact
run_gate_test "WAVEC_V11_TRACE_VALIDATION" \
  cargo test -p artifact_store_schema tests::trace_validation_works -- --exact
run_gate_test "WAVEC_V11_OBSERVED_CAPS_VALIDATION" \
  cargo test -p artifact_store_schema tests::observed_caps_validation_works -- --exact
run_gate_test "WAVEC_V11_CLAIM_VALIDATION" \
  cargo test -p artifact_store_schema tests::claim_validation_works -- --exact
echo "FOUNDRY_HARDENING_WAVE_C: PASS control=V-11"

# V-15 / SC-10: pinned nightly toolchain
run_gate_test "WAVEC_V15_RUST_TOOLCHAIN_PINNED" \
  grep -q "nightly-2026-02-08" rust-toolchain.toml
echo "FOUNDRY_HARDENING_WAVE_C: PASS control=V-15"

# SC-11: unsafe safety comments in arch modules
run_gate_test "WAVEC_SC11_X86_SAFETY_COMMENTS" \
  grep -q "SAFETY:" kernel/src/arch/x86_64.rs
run_gate_test "WAVEC_SC11_AARCH64_SAFETY_COMMENTS" \
  grep -q "SAFETY:" kernel/src/arch/aarch64.rs
echo "FOUNDRY_HARDENING_WAVE_C: PASS control=SC-11"

# SC-12: multi-encoding evidence redaction
run_gate_test "WAVEC_SC12_HEX_REDACTION" \
  cargo test -p artifact_store_schema evidence_policy::tests::redacts_hex_markers -- --exact
run_gate_test "WAVEC_SC12_HEX_CASE_INSENSITIVE" \
  cargo test -p artifact_store_schema evidence_policy::tests::redacts_hex_markers_case_insensitive -- --exact
run_gate_test "WAVEC_SC12_BASE64_REDACTION" \
  cargo test -p artifact_store_schema evidence_policy::tests::redacts_base64_markers -- --exact
run_gate_test "WAVEC_SC12_COMBINED_PATTERNS" \
  cargo test -p artifact_store_schema evidence_policy::tests::redacts_combined_patterns -- --exact
run_gate_test "WAVEC_SC12_UTF8_HANDLING" \
  cargo test -p artifact_store_schema evidence_policy::tests::handles_non_utf8_for_hex_base64_only -- --exact
echo "FOUNDRY_HARDENING_WAVE_C: PASS control=SC-12"

echo "FOUNDRY_HARDENING_WAVE_C: ok"
