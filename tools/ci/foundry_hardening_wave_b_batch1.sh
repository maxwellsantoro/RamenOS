#!/usr/bin/env bash
set -euo pipefail

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_HARDENING_WAVE_B_BATCH1: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

run_gate_test() {
  local code="$1"
  shift
  if ! (cd "$ROOT_DIR" && "$@"); then
    fail "$code" "command failed: $*"
  fi
}

# V-07 / SC-06: trace ring atomic ordering and index progression semantics
run_gate_test "WBB1_V07_TRACE_RING_ATOMIC_ORDERING" \
  cargo test -p kernel trace_ring::tests::writer_release_semantics_visible_to_reader -- --exact
run_gate_test "WBB1_V07_TRACE_RING_MONOTONIC_INDEX" \
  cargo test -p kernel trace_ring::tests::monotonic_index_progression_with_wrap -- --exact
run_gate_test "WBB1_V07_TRACE_RING_WRITER_EXCLUSIVE" \
  cargo test -p kernel trace_ring::tests::writer_claim_is_exclusive -- --exact
run_gate_test "WBB1_V07_TRACE_RING_EMPTY_RING" \
  cargo test -p kernel trace_ring::tests::reader_returns_zero_on_empty_ring -- --exact
run_gate_test "WBB1_V07_TRACE_RING_BUFFER_LIMIT" \
  cargo test -p kernel trace_ring::tests::reader_respects_output_buffer_limit -- --exact
run_gate_test "WBB1_V07_TRACE_RING_DETERMINISTIC_FAST_FORWARD" \
  cargo test -p kernel trace_ring::tests::overwrite_fast_forward_is_deterministic -- --exact
echo "FOUNDRY_HARDENING_WAVE_B_BATCH1: PASS control=V-07"

# V-08 / SC-07: init image parser checked arithmetic and bounds validation
run_gate_test "WBB1_V08_INIT_BOUNDS_CHECK" \
  cargo test -p kernel init::tests::init_rejects_malformed_header_magic -- --exact
run_gate_test "WBB1_V08_INIT_OVERFLOW_REJECT" \
  cargo test -p kernel init::tests::init_rejects_overflow_offset_calculation -- --exact
run_gate_test "WBB1_V08_INIT_BOUNDS_VIOLATION" \
  cargo test -p kernel init::tests::init_rejects_bounds_violation -- --exact
run_gate_test "WBB1_V08_INIT_ZERO_CONTENT_LEN" \
  cargo test -p kernel init::tests::init_rejects_zero_content_len -- --exact
run_gate_test "WBB1_V08_INIT_VALID_ACCEPT" \
  cargo test -p kernel init::tests::init_accepts_valid_image -- --exact
echo "FOUNDRY_HARDENING_WAVE_B_BATCH1: PASS control=V-08"

echo "FOUNDRY_HARDENING_WAVE_B_BATCH1: ok"
