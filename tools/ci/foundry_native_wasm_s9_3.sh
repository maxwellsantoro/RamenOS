#!/usr/bin/env bash
# V-006 Phase 4: Native WASM SDK and Example Gate
#
# Goals:
# - Verify SDK compiles for wasm32-unknown-unknown target
# - Verify hello_wasm example builds and produces valid WASM
# - Run clippy on WASM targets
#
# Usage:
#   tools/ci/foundry_native_wasm_s9_3.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/foundry/native_wasm"
LOG_FILE="$OUT_DIR/build.log"

mkdir -p "$OUT_DIR"
rm -f "$LOG_FILE"

echo "FOUNDRY_NATIVE_WASM: START"
echo "FOUNDRY_NATIVE_WASM: INFO verifying native WASM SDK and example"

cd "$ROOT_DIR"

# Ensure wasm32 target is installed
echo "FOUNDRY_NATIVE_WASM: INFO checking wasm32-unknown-unknown target"
rustup target add wasm32-unknown-unknown

# Run codegen first to generate SDK bindings
echo "FOUNDRY_NATIVE_WASM: INFO running codegen"
cargo run -p idl_codegen -- \
  --in idl/harness/echo_harness_v0.toml \
  --out sdk/src/generated/harness_echo_v0.rs \
  --lang wasm-imports

# Build SDK for wasm32 target
echo "FOUNDRY_NATIVE_WASM: INFO building SDK for wasm32-unknown-unknown"
cargo build -p ramen_sdk --target wasm32-unknown-unknown 2>&1 | tee "$LOG_FILE"

# Build hello_wasm example
echo "FOUNDRY_NATIVE_WASM: INFO building hello_wasm example"
cargo build -p hello_wasm --target wasm32-unknown-unknown 2>&1 | tee -a "$LOG_FILE"

# Verify WASM binary exists and is valid
WASM_FILE="$ROOT_DIR/target/wasm32-unknown-unknown/debug/hello_wasm.wasm"
if [[ ! -f "$WASM_FILE" ]]; then
  echo "FOUNDRY_NATIVE_WASM: FAIL code=wasm_not_found detail=$WASM_FILE"
  exit 1
fi

# Check file type
FILE_TYPE="$(file -b "$WASM_FILE")"
if [[ ! "$FILE_TYPE" =~ "WebAssembly" ]]; then
  echo "FOUNDRY_NATIVE_WASM: FAIL code=invalid_wasm detail=$FILE_TYPE"
  exit 1
fi

echo "FOUNDRY_NATIVE_WASM: INFO wasm_binary=$WASM_FILE"
echo "FOUNDRY_NATIVE_WASM: INFO file_type=$FILE_TYPE"

# Run clippy on WASM targets
echo "FOUNDRY_NATIVE_WASM: INFO running clippy on WASM targets"
cargo clippy -p ramen_sdk --target wasm32-unknown-unknown 2>&1 | tee -a "$LOG_FILE"
cargo clippy -p hello_wasm --target wasm32-unknown-unknown 2>&1 | tee -a "$LOG_FILE"

# Check for warnings
warning_count="$(grep -E -c '(^warning:| warning:)' "$LOG_FILE" || true)"
echo "FOUNDRY_NATIVE_WASM: METRIC warning_count=${warning_count}"

if [[ "$warning_count" -gt 0 ]]; then
  echo "FOUNDRY_NATIVE_WASM: FAIL code=warnings_present detail=warning_count=${warning_count}"
  exit 1
fi

echo "FOUNDRY_NATIVE_WASM: INFO log_path=${LOG_FILE}"
echo "FOUNDRY_NATIVE_WASM: PASS"
