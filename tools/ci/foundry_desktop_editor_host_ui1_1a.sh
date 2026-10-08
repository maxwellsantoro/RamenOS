#!/usr/bin/env bash
# Default-off volatile, in-process editor acceptance; no Store/device/target proof.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/desktop/ui1-1a"
mkdir -p "$EVIDENCE"
rm -f "$EVIDENCE/result.json"
# This gate exclusively owns this runtime evidence directory.
rm -rf "$EVIDENCE/cases"
mkdir -p "$EVIDENCE/cases"

cargo build -p desktop_service --lib --no-default-features >"$EVIDENCE/default-build.log" 2>&1
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
PROBE_DIR="$(mktemp -d)"
trap 'rm -rf "$PROBE_DIR"' EXIT
cat >"$PROBE_DIR/default_api.rs" <<'RS'
extern crate desktop_service;
use desktop_service::editor_dev::HostDesktop;
fn main() {}
RS
if rustc --edition 2021 "$PROBE_DIR/default_api.rs" \
  --extern "desktop_service=$TARGET_DIR/debug/libdesktop_service.rlib" \
  -L "dependency=$TARGET_DIR/debug/deps" -o "$PROBE_DIR/default_api" \
  >"$EVIDENCE/default-api.log" 2>&1; then
  echo 'FOUNDRY_DESKTOP_EDITOR_HOST_UI1_1A: FAIL development API exposed by default' >&2
  exit 1
fi
rg -q 'could not find.*editor_dev.*desktop_service' "$EVIDENCE/default-api.log"
cargo test -p desktop_service --features desktop_v0_dev --test editor_host --no-run --message-format=json \
  >"$EVIDENCE/build.jsonl" 2>"$EVIDENCE/build.log"
cargo clippy -p desktop_service --features desktop_v0_dev --test editor_host -- -D warnings \
  >"$EVIDENCE/clippy.log" 2>&1
cargo test -p desktop_service --features desktop_v0_dev --test editor_host -- --list \
  >"$EVIDENCE/cases.log" 2>&1
python3 tools/ci/desktop_editor_result.py --check-list "$EVIDENCE/cases.log"
RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE="$EVIDENCE/cases" \
  cargo test -p desktop_service --features desktop_v0_dev --test editor_host -- --test-threads=1 \
  >"$EVIDENCE/tests.log" 2>&1
python3 tools/ci/desktop_editor_result.py --evidence "$EVIDENCE"
echo 'FOUNDRY_DESKTOP_EDITOR_HOST_UI1_1A: PASS scope=volatile-in-process-editor target=false store=false device=false containment=false'
