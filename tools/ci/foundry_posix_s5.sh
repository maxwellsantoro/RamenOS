#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/posix_s5"
ARTIFACT_ROOT="$ROOT_DIR/out/artifacts"
INSTALLED_ROOT="$ROOT_DIR/out/installed"
INSTALLED_ARTIFACTS="$INSTALLED_ROOT/artifacts"
STORE_SOCKET="$OUT_DIR/store.sock"
STORE_LOG="$OUT_DIR/store_service.log"

mkdir -p "$OUT_DIR" "$ARTIFACT_ROOT" "$INSTALLED_ARTIFACTS"
rm -f "$STORE_SOCKET"

RAMEN_STORE_DEV_MODE=1 \
RAMEN_STORE_ACCESS_POLICY=AllowAll \
RAMEN_STORE_SOCKET="$STORE_SOCKET" \
RAMEN_STORE_ROOT="$INSTALLED_ARTIFACTS" \
cargo run -p store_service >"$STORE_LOG" 2>&1 &
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
  --program-id "ramen.posix.hello" \
  --out "$OUT_DIR/launch_plan.json" \
  --artifact-root "$ARTIFACT_ROOT" \
  --tmp-root "$ROOT_DIR/out/tmp" \
  --store-socket "$STORE_SOCKET")

echo "$store_output" | grep -q "store: emitted execution launch plan:"

artifact_ref=$(python3 - <<PY
import json
print(json.load(open("$OUT_DIR/launch_plan.json"))['artifact_ref'])
PY
)

blob="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.blob"
manifest="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.manifest.json"
test -f "$blob"
test -f "$manifest"

set +e
run_output=$(cargo run -p runtime_supervisor -- \
  --plan "$OUT_DIR/launch_plan.json" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET" 2>&1)
run_status=$?
set -e

echo "$run_output" | grep -q "supervisor: plan ok program_id=ramen.posix.hello runner=posix_runner_v0"

if echo "$run_output" | grep -q "runner=posix_runner_v0 disabled"; then
  echo "FOUNDRY_POSIX_S5: posix runner disabled mitigation path ok"
else
  if [[ $run_status -ne 0 ]]; then
    echo "runtime_supervisor failed unexpectedly for posix gate" >&2
    echo "$run_output" >&2
    exit 2
  fi
  echo "$run_output" | grep -q "POSIX_RUNNER_V0: hello"
  echo "$run_output" | grep -q "supervisor: posix_run_v0 exited status=exit status: 0"
fi

echo "FOUNDRY_POSIX_S5: ok"
