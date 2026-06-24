#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
TRACE_DIR="$OUT_DIR/trace"
INSTALLED_ROOT="$OUT_DIR/installed"
STORE_SOCKET="$TRACE_DIR/store.sock"
STORE_LOG="$TRACE_DIR/store_service.log"

mkdir -p "$TRACE_DIR" "$INSTALLED_ROOT"
rm -f "$STORE_SOCKET"

RAMEN_STORE_DEV_MODE=1 \
RAMEN_STORE_ACCESS_POLICY=AllowAll \
RAMEN_STORE_SOCKET="$STORE_SOCKET" \
RAMEN_STORE_ROOT="$INSTALLED_ROOT/artifacts" \
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

TRACE_JSON="$TRACE_DIR/protocol_trace.json"

python3 "$ROOT_DIR/tools/trace/build_trace_artifact.py" --out "$TRACE_JSON"

python3 - <<PY
import json
path = "$TRACE_JSON"
data = json.load(open(path))
evt = data["protocol_trace"]["events"][0]
evt["notes"] = "SECRET_TOKEN in trace payload"
json.dump(data, open(path, "w"), indent=2)
PY

cargo run -p store_cli -- validate-trace --src "$TRACE_JSON"

content_id=$(cargo run -p store_cli -- ingest \
  --src "$TRACE_JSON" \
  --installed-root "$INSTALLED_ROOT" \
  --kind trace_artifact_v0 \
  --channel Experimental \
  --evidence-policy "$ROOT_DIR/evidence_policy.toml" \
  --store-socket "$STORE_SOCKET")

blob="$INSTALLED_ROOT/artifacts/${content_id#sha256:}.blob"
manifest="$INSTALLED_ROOT/artifacts/${content_id#sha256:}.manifest.json"

python3 - <<PY
import json
import sys

manifest = json.load(open("$manifest"))
if manifest.get("kind") != "trace_artifact_v0":
    print("manifest kind mismatch", file=sys.stderr)
    sys.exit(2)
if manifest.get("content_id") != "$content_id":
    print("manifest content_id mismatch", file=sys.stderr)
    sys.exit(2)
PY

python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$blob"

python3 - <<PY
raw = open("$blob", "r", encoding="utf-8").read()
if "SECRET_TOKEN" in raw:
    raise SystemExit("redaction failed: SECRET_TOKEN still present")
if "[REDACTED]" not in raw:
    raise SystemExit("redaction failed: replacement marker missing")
PY

echo "FOUNDRY_TRACE_S3: ok"
