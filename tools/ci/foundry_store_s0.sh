#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mkdir -p "$ROOT_DIR/out"
# Each smoke run owns its fixtures. Security gates deliberately corrupt CAS
# metadata; operator/past-run artifacts must never be repaired or adopted here.
OUT_DIR="$(mktemp -d "$ROOT_DIR/out/store-s0.XXXXXX")"
ARTIFACT_ROOT="$OUT_DIR/artifacts"
INSTALLED_ROOT="$OUT_DIR/installed"
INSTALLED_ARTIFACTS="$INSTALLED_ROOT/artifacts"
STORE_SOCKET="$OUT_DIR/store.sock"
STORE_LOG="$OUT_DIR/store_service.log"
STORE_BUILD_JSON="$OUT_DIR/store_build.jsonl"
STORE_BUILD_LOG="$OUT_DIR/store_build.stderr.log"

mkdir -p "$OUT_DIR" "$ARTIFACT_ROOT" "$INSTALLED_ARTIFACTS"
rm -f "$STORE_SOCKET"

# Compilation has its own synchronous phase; the unchanged readiness budget
# starts only after launching the actual server, not a Cargo compiler wrapper.
if ! cargo build -p store_service --bin store_service --message-format=json \
  >"$STORE_BUILD_JSON" 2>"$STORE_BUILD_LOG"; then
  echo "store_service build failed; evidence_dir=$OUT_DIR" >&2
  cat "$STORE_BUILD_LOG" >&2
  exit 1
fi

STORE_BIN="$(python3 "$ROOT_DIR/tools/ci/cargo_artifact.py" "$STORE_BUILD_JSON" store_service)"

RAMEN_STORE_DEV_MODE=1 \
RAMEN_STORE_ACCESS_POLICY=AllowAll \
RAMEN_STORE_SOCKET="$STORE_SOCKET" \
RAMEN_STORE_ROOT="$INSTALLED_ARTIFACTS" \
"$STORE_BIN" >"$STORE_LOG" 2>&1 &
STORE_PID=$!

cleanup() {
  kill "$STORE_PID" >/dev/null 2>&1 || true
  wait "$STORE_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in $(seq 1 100); do
  if [[ -S "$STORE_SOCKET" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -S "$STORE_SOCKET" ]]; then
  echo "store_service socket not ready: $STORE_SOCKET"
  cat "$STORE_LOG"
  exit 1
fi

store_output=$(cargo run -p store_cli -- emit-plan \
  --catalog "$ROOT_DIR/store/catalog.json" \
  --program-id "ramen.demo.hello" \
  --out "$OUT_DIR/launch_plan.json" \
  --artifact-root "$ARTIFACT_ROOT" \
  --tmp-root "$OUT_DIR/tmp" \
  --store-socket "$STORE_SOCKET")

echo "$store_output" | grep -q "store: emitted execution launch plan:"

artifact_ref=$(python3 - <<PY
import json
print(json.load(open("$OUT_DIR/launch_plan.json"))['artifact_ref'])
PY
)

blob_installed="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.blob"
manifest_installed="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.manifest.json"

test -f "$blob_installed"
test -f "$manifest_installed"

super_output=$(cargo run -p runtime_supervisor -- \
  --plan "$OUT_DIR/launch_plan.json" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET")

echo "$super_output" | grep -q "supervisor: plan ok program_id="

echo "FOUNDRY_STORE_S0: ok"
echo "FOUNDRY_STORE_S0: evidence_dir=$OUT_DIR"
