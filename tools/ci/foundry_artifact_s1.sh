#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ROOT="$ROOT_DIR/out/artifacts"
INSTALLED_ROOT="$ROOT_DIR/out/installed"
INSTALLED_ARTIFACTS="$INSTALLED_ROOT/artifacts"
STORE_OUT="$ROOT_DIR/out/store/launch_plan.json"
STORE_SOCKET="$ROOT_DIR/out/store/store.sock"
STORE_LOG="$ROOT_DIR/out/store/store_service.log"

rm -rf "$ARTIFACT_ROOT" "$INSTALLED_ROOT"
mkdir -p "$ARTIFACT_ROOT" "$INSTALLED_ARTIFACTS"
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
  --program-id "ramen.demo.hello" \
  --out "$STORE_OUT" \
  --artifact-root "$ARTIFACT_ROOT" \
  --tmp-root "$ROOT_DIR/out/tmp" \
  --store-socket "$STORE_SOCKET")

echo "$store_output" | grep -q "store: emitted execution launch plan:"

artifact_ref=$(python3 - <<PY
import json
print(json.load(open("$STORE_OUT"))['artifact_ref'])
PY
)

blob="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.blob"
manifest="$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.manifest.json"
manifest_backup="$ROOT_DIR/out/store/${artifact_ref#sha256:}.manifest.backup.json"

if [[ ! -f "$blob" || ! -f "$manifest" ]]; then
  echo "artifact files missing" >&2
  exit 2
fi

cp "$manifest" "$manifest_backup"

run_out=$(cargo run -p runtime_supervisor -- \
  --plan "$STORE_OUT" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET")

echo "$run_out" | grep -q "supervisor: plan ok program_id="

python3 - <<PY
import json
path = "$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.manifest.json"
data = json.load(open(path))
data["schema_version"] = 2
with open(path, "w") as f:
    json.dump(data, f)
PY

set +e
schema_out=$(cargo run -p runtime_supervisor -- \
  --plan "$STORE_OUT" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET" 2>&1)
schema_status=$?
set -e

if [[ $schema_status -eq 0 ]]; then
  echo "expected failure on schema_version mismatch" >&2
  exit 3
fi

echo "$schema_out" | grep -Eq "schema_version unsupported|supervisor: artifact invalid"

cp "$manifest_backup" "$manifest"

rm -f "$INSTALLED_ARTIFACTS/${artifact_ref#sha256:}.blob"

set +e
fail_out=$(cargo run -p runtime_supervisor -- \
  --plan "$STORE_OUT" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET" 2>&1)
status=$?
set -e

if [[ $status -eq 0 ]]; then
  echo "expected failure after rollback" >&2
  exit 3
fi

echo "$fail_out" | grep -q "supervisor: artifact"

echo "FOUNDRY_ARTIFACT_S1: ok"
