#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_SHMEM_CONTRACT_S8_PHASE1: FAIL code=${code} detail=${detail}" >&2
  exit 1
}

require_contains() {
  local haystack="$1"
  local needle="$2"
  local code="$3"
  if ! grep -Fq "$needle" <<<"$haystack"; then
    fail "$code" "missing sentinel: $needle"
  fi
}

cargo run -p idl_codegen -- \
  --in "$ROOT_DIR/idl/harness/shmem_control_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/shmem_control_v1.generated.rs"

test_out=$(cargo test -p kernel_api shmem_control_contract -- --nocapture 2>&1)
require_contains "$test_out" "test tests::shmem_control_contract_roundtrip" "S8_PHASE1_ROUNDTRIP_TEST_MISSING"
require_contains "$test_out" "test tests::shmem_control_contract_payload_sizes_fit_ipc_envelope" "S8_PHASE1_PAYLOAD_SIZE_TEST_MISSING"
require_contains "$test_out" "test result: ok" "S8_PHASE1_KERNEL_API_TEST_FAIL"

echo "FOUNDRY_SHMEM_CONTRACT_S8_PHASE1: METRIC codegen=ok test_filter=shmem_control_contract"
echo "FOUNDRY_SHMEM_CONTRACT_S8_PHASE1: ok"
