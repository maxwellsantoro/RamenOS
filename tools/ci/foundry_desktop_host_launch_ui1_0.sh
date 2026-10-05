#!/usr/bin/env bash
# Typed host launch/lifetime evidence, never target or process-containment proof.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/desktop/ui1-0"
mkdir -p "$EVIDENCE"
rm -f "$EVIDENCE/result.json"
# Only this gate owns this fixed runtime-evidence directory.
rm -rf "$EVIDENCE/children"
mkdir -p "$EVIDENCE/children"

cargo build -p desktop_service --lib --no-default-features > "$EVIDENCE/default-build.log" 2>&1
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
PROBE_DIR="$(mktemp -d)"
trap 'rm -rf "$PROBE_DIR"' EXIT
cat > "$PROBE_DIR/default_api.rs" <<'RS'
extern crate desktop_service;
use desktop_service::dev::DevDesktop;
fn main() {}
RS
if rustc --edition 2021 "$PROBE_DIR/default_api.rs" \
  --extern "desktop_service=$TARGET_DIR/debug/libdesktop_service.rlib" \
  -L "dependency=$TARGET_DIR/debug/deps" -o "$PROBE_DIR/default_api" \
  > "$EVIDENCE/default-api.log" 2>&1; then
  echo 'FOUNDRY_DESKTOP_HOST_LAUNCH_UI1_0: FAIL development API exposed by default' >&2
  exit 1
fi
rg -q 'could not find.*dev.*desktop_service' "$EVIDENCE/default-api.log"
if cargo build -p desktop_service --bin desktop-launch-witness --no-default-features \
  > "$EVIDENCE/default-witness.log" 2>&1; then
  echo 'FOUNDRY_DESKTOP_HOST_LAUNCH_UI1_0: FAIL witness selectable by default' >&2
  exit 1
fi
rg -q 'requires the features.*desktop_v0_dev' "$EVIDENCE/default-witness.log"

cargo test -p desktop_service --features desktop_v0_dev --test launch_lifetime --no-run --message-format=json \
  > "$EVIDENCE/build.jsonl" 2> "$EVIDENCE/build.log"
cargo clippy -p desktop_service --features desktop_v0_dev --all-targets -- -D warnings \
  > "$EVIDENCE/clippy.log" 2>&1

# Names are a frozen acceptance inventory, not discovered from implementation.
cargo test -p desktop_service --features desktop_v0_dev --test launch_lifetime -- --list \
  > "$EVIDENCE/cases.log" 2>&1
python3 tools/ci/desktop_launch_result.py --check-list "$EVIDENCE/cases.log"
RAMEN_DESKTOP_GATE_EVIDENCE="$EVIDENCE/children" \
cargo test -p desktop_service --features desktop_v0_dev --test launch_lifetime -- --test-threads=1 \
  > "$EVIDENCE/tests.log" 2>&1
python3 tools/ci/desktop_launch_result.py --evidence "$EVIDENCE"
echo 'FOUNDRY_DESKTOP_HOST_LAUNCH_UI1_0: PASS scope=typed-host-launch-lifetime target=false containment=false'
