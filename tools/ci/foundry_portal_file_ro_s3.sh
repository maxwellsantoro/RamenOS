#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
LOG_DIR="$OUT_DIR/logs"
PORTAL_DIR="$OUT_DIR/portal"
TRACE_OUT="$OUT_DIR/trace/portal_file_ro.json"
INSTALLED_ROOT="$OUT_DIR/installed"
STORE_SOCKET="$PORTAL_DIR/store.sock"
STORE_LOG="$PORTAL_DIR/store_service.log"

mkdir -p "$LOG_DIR" "$PORTAL_DIR"
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

INPUT_FILE="$PORTAL_DIR/demo.txt"
echo "portal_file_picker_demo_v0" >"$INPUT_FILE"

cargo run -p idl_codegen -- \
  --in "$ROOT_DIR/idl/portals/file_picker_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/portal_file_picker.generated.rs"

run_out=$(cargo run -p portals -- \
  --input "$INPUT_FILE" \
  --installed-root "$INSTALLED_ROOT" \
  --trace-out "$TRACE_OUT" \
  --store-socket "$STORE_SOCKET")

echo "$run_out" | grep -q "PORTAL_FILE_PICKER: open ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: resolve ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: read ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: bad payload rejected ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: cancel rejects reuse ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: token scope ok"
echo "$run_out" | grep -q "PORTAL_FILE_PICKER: ok"

trace_id=$(echo "$run_out" | sed -E -n 's/^PORTAL_FILE_PICKER: trace_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
if [[ -z "$trace_id" ]]; then
  echo "trace_content_id missing" >&2
  exit 2
fi

observed_id=$(echo "$run_out" | sed -E -n 's/^PORTAL_FILE_PICKER: observed_caps_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
if [[ -z "$observed_id" ]]; then
  echo "observed_caps_content_id missing" >&2
  exit 2
fi

scenario_id=$(echo "$run_out" | sed -E -n 's/^PORTAL_FILE_PICKER: scenario_trace_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
if [[ -z "$scenario_id" ]]; then
  echo "scenario_trace_content_id missing" >&2
  exit 2
fi

content_id=$(echo "$run_out" | sed -E -n 's/^PORTAL_FILE_PICKER: content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
if [[ -z "$content_id" ]]; then
  echo "content_id missing" >&2
  exit 2
fi

blob="$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.blob"
manifest="$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.manifest.json"

if [[ ! -f "$blob" || ! -f "$manifest" ]]; then
  echo "trace artifact missing from installed store" >&2
  exit 2
fi

obs_blob="$INSTALLED_ROOT/artifacts/${observed_id#sha256:}.blob"
obs_manifest="$INSTALLED_ROOT/artifacts/${observed_id#sha256:}.manifest.json"
if [[ ! -f "$obs_blob" || ! -f "$obs_manifest" ]]; then
  echo "observed_caps artifact missing from installed store" >&2
  exit 2
fi

scenario_blob="$INSTALLED_ROOT/artifacts/${scenario_id#sha256:}.blob"
scenario_manifest="$INSTALLED_ROOT/artifacts/${scenario_id#sha256:}.manifest.json"
if [[ ! -f "$scenario_blob" || ! -f "$scenario_manifest" ]]; then
  echo "scenario_trace artifact missing from installed store" >&2
  exit 2
fi

cargo run -p store_cli -- validate-trace --src "$TRACE_OUT"
cargo run -p store_cli -- validate-observed-caps --src "$obs_blob"
cargo run -p store_cli -- validate-trace --src "$scenario_blob"
python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$blob"

TRACE_ID="$trace_id" CONTENT_ID="$content_id" OBS_BLOB="$obs_blob" python3 - <<'PY'
import json
import os
import sys

trace_id = os.environ["TRACE_ID"]
content_id = os.environ["CONTENT_ID"]
obs_path = os.environ["OBS_BLOB"]

with open(obs_path, "r", encoding="utf-8") as f:
    obs = json.load(f)

caps = obs.get("capabilities", [])
cap = next((c for c in caps if c.get("cap") == "portal.file_picker.ro"), None)
if not cap:
    sys.exit("observed_caps missing portal.file_picker.ro")
if trace_id not in cap.get("evidence", []):
    sys.exit("observed_caps missing trace evidence")
scope_ids = cap.get("scope", {}).get("artifact_ids", [])
if content_id not in scope_ids:
    sys.exit("observed_caps missing selected artifact id")
counts = cap.get("counts", {})
if counts.get("used") != 1 or counts.get("granted") != 1:
    sys.exit("observed_caps counts mismatch")
PY

TRACE_ID="$trace_id" OBSERVED_ID="$observed_id" SCENARIO_BLOB="$scenario_blob" python3 - <<'PY'
import json
import os
import sys

trace_id = os.environ["TRACE_ID"]
observed_id = os.environ["OBSERVED_ID"]
path = os.environ["SCENARIO_BLOB"]

with open(path, "r", encoding="utf-8") as f:
    doc = json.load(f)

if doc.get("trace_type") != "scenario_trace":
    sys.exit("scenario trace_type mismatch")
events = doc.get("scenario_trace", {}).get("events", [])

def has_ref(name, key, value):
    for event in events:
        if event.get("name") != name:
            continue
        payload = event.get("payload") or {}
        if payload.get(key) == value:
            return True
    return False

if not has_ref("protocol_trace_ref", "content_id", trace_id):
    sys.exit("scenario trace missing protocol_trace_ref")
if not has_ref("observed_caps_ref", "content_id", observed_id):
    sys.exit("scenario trace missing observed_caps_ref")
PY

echo "FOUNDRY_PORTAL_FILE_RO_S3: ok"
