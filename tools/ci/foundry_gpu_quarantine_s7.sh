#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/s7_gpu"
TRACE_DIR="$OUT_DIR/trace"
INSTALLED_ROOT="$OUT_DIR/installed"
STORE_OUT="$OUT_DIR/store"
ARTIFACT_ROOT="$OUT_DIR/artifacts"
TMP_ROOT="$OUT_DIR/tmp"
STORE_SOCKET="$OUT_DIR/store.sock"
STORE_LOG="$OUT_DIR/store_service.log"

S7_MIN_PROTOCOL_EVENTS="${S7_MIN_PROTOCOL_EVENTS:-6}"
S7_MIN_PROTOCOL_PAIRS="${S7_MIN_PROTOCOL_PAIRS:-3}"
S7_MIN_SCENARIO_EVENTS="${S7_MIN_SCENARIO_EVENTS:-4}"
S7_MIN_OBSERVED_CAPS="${S7_MIN_OBSERVED_CAPS:-1}"
S7_MIN_CAP_GRANTED="${S7_MIN_CAP_GRANTED:-1}"
S7_MIN_CAP_USED="${S7_MIN_CAP_USED:-1}"
S7_MIN_EXPORT_WIDTH="${S7_MIN_EXPORT_WIDTH:-1280}"
S7_MIN_EXPORT_HEIGHT="${S7_MIN_EXPORT_HEIGHT:-720}"
S7_EVIDENCE_POLICY_PATH="${S7_EVIDENCE_POLICY_PATH:-$ROOT_DIR/evidence_policy.toml}"

fail() {
  local code="$1"
  local detail="$2"
  echo "FOUNDRY_GPU_QUARANTINE_S7: FAIL code=${code} detail=${detail}" >&2
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

require_file() {
  local path="$1"
  local code="$2"
  [[ -f "$path" ]] || fail "$code" "missing file: $path"
}

extract_metric_field() {
  local line="$1"
  local key="$2"
  python3 - "$line" "$key" <<'PY'
import re
import sys

line = sys.argv[1]
key = sys.argv[2]
pattern = re.compile(rf"(?:^|\s){re.escape(key)}=([^\s]+)")
m = pattern.search(line)
if not m:
    raise SystemExit(1)
print(m.group(1))
PY
}

mkdir -p "$TRACE_DIR" "$INSTALLED_ROOT/artifacts" "$STORE_OUT" "$ARTIFACT_ROOT" "$TMP_ROOT"
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
  fail "S7_STORE_SOCKET_NOT_READY" "socket not ready: $STORE_SOCKET"
fi

cargo run -p idl_codegen -- \
  --in "$ROOT_DIR/idl/harness/gpu_quarantine_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/gpu_quarantine_v1.generated.rs"

run_out=$(cargo run -p domain_manager -- \
  --trace-dir "$TRACE_DIR" \
  --installed-root "$INSTALLED_ROOT" \
  --program-id "org.ramen.domain_manager" \
  --run-id "domain_manager_gpu_s7_gate" \
  --store-socket "$STORE_SOCKET")

require_contains "$run_out" "DOMAIN_MANAGER: gpu export ok domain=700 surface=" "S7_SENTINEL_EXPORT_MISSING"
require_contains "$run_out" "DOMAIN_MANAGER: gpu scanout ok domain=700 surface=" "S7_SENTINEL_SCANOUT_MISSING"
require_contains "$run_out" "DOMAIN_MANAGER: gpu invalid capability rejected" "S7_SENTINEL_INVALID_CAP_REJECT_MISSING"
require_contains "$run_out" "DOMAIN_MANAGER: gpu stale frame rejected" "S7_SENTINEL_STALE_FRAME_REJECT_MISSING"
require_contains "$run_out" "DOMAIN_MANAGER: gpu stop ok domain=700 generation=1" "S7_SENTINEL_STOP_MISSING"

read -r trace_id observed_id scenario_id <<<"$(echo "$run_out" | sed -E -n 's/^DOMAIN_MANAGER: gpu quarantine ok trace=(sha256:[0-9a-f]+) observed=(sha256:[0-9a-f]+) scenario=(sha256:[0-9a-f]+).*/\1 \2 \3/p' | tail -n 1)"

[[ -n "${trace_id:-}" ]] || fail "S7_CONTENT_ID_TRACE_MISSING" "missing trace content id sentinel"
[[ -n "${observed_id:-}" ]] || fail "S7_CONTENT_ID_OBSERVED_MISSING" "missing observed content id sentinel"
[[ -n "${scenario_id:-}" ]] || fail "S7_CONTENT_ID_SCENARIO_MISSING" "missing scenario content id sentinel"

run_out_replay=$(cargo run -p domain_manager -- \
  --trace-dir "$TRACE_DIR" \
  --installed-root "$INSTALLED_ROOT" \
  --program-id "org.ramen.domain_manager" \
  --run-id "domain_manager_gpu_s7_gate" \
  --store-socket "$STORE_SOCKET")

read -r trace_id_replay observed_id_replay scenario_id_replay <<<"$(echo "$run_out_replay" | sed -E -n 's/^DOMAIN_MANAGER: gpu quarantine ok trace=(sha256:[0-9a-f]+) observed=(sha256:[0-9a-f]+) scenario=(sha256:[0-9a-f]+).*/\1 \2 \3/p' | tail -n 1)"

[[ -n "${trace_id_replay:-}" ]] || fail "S7_REPLAY_CONTENT_ID_TRACE_MISSING" "missing replay trace content id sentinel"
[[ -n "${observed_id_replay:-}" ]] || fail "S7_REPLAY_CONTENT_ID_OBSERVED_MISSING" "missing replay observed content id sentinel"
[[ -n "${scenario_id_replay:-}" ]] || fail "S7_REPLAY_CONTENT_ID_SCENARIO_MISSING" "missing replay scenario content id sentinel"

for id in "$trace_id" "$observed_id" "$scenario_id"; do
  blob="$INSTALLED_ROOT/artifacts/${id#sha256:}.blob"
  manifest="$INSTALLED_ROOT/artifacts/${id#sha256:}.manifest.json"
  require_file "$blob" "S7_ARTIFACT_BLOB_MISSING"
  require_file "$manifest" "S7_ARTIFACT_MANIFEST_MISSING"
done

for id in "$trace_id_replay" "$observed_id_replay" "$scenario_id_replay"; do
  blob="$INSTALLED_ROOT/artifacts/${id#sha256:}.blob"
  manifest="$INSTALLED_ROOT/artifacts/${id#sha256:}.manifest.json"
  require_file "$blob" "S7_REPLAY_ARTIFACT_BLOB_MISSING"
  require_file "$manifest" "S7_REPLAY_ARTIFACT_MANIFEST_MISSING"
done

cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/gpu_quarantine_protocol.json"
cargo run -p store_cli -- validate-observed-caps --src "$TRACE_DIR/gpu_quarantine_observed.json"
cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/gpu_quarantine_scenario.json"

replay_primary_out=$(python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.blob")
require_contains "$replay_primary_out" "REPLAY_PROTOCOL_TRACE: METRIC source=trace" "S7_REPLAY_METRIC_MISSING"
require_contains "$replay_primary_out" "REPLAY_PROTOCOL_TRACE: ok" "S7_REPLAY_OK_MISSING"

replay_dup_trace="$TRACE_DIR/gpu_quarantine_protocol_duplicate.json"
cp "$INSTALLED_ROOT/artifacts/${trace_id_replay#sha256:}.blob" "$replay_dup_trace"

replay_compare_out=$(python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" \
  --trace "$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.blob" \
  --compare "$replay_dup_trace")
require_contains "$replay_compare_out" "REPLAY_PROTOCOL_TRACE: MATCH" "S7_REPLAY_MATCH_MISSING"

trace_metric_line=$(echo "$replay_compare_out" | grep -F "REPLAY_PROTOCOL_TRACE: METRIC source=trace" | tail -n 1)
compare_metric_line=$(echo "$replay_compare_out" | grep -F "REPLAY_PROTOCOL_TRACE: METRIC source=compare" | tail -n 1)
[[ -n "${trace_metric_line:-}" ]] || fail "S7_REPLAY_TRACE_METRIC_LINE_MISSING" "missing replay trace metric line"
[[ -n "${compare_metric_line:-}" ]] || fail "S7_REPLAY_COMPARE_METRIC_LINE_MISSING" "missing replay compare metric line"

trace_digest=$(extract_metric_field "$trace_metric_line" "digest") || fail "S7_REPLAY_TRACE_DIGEST_MISSING" "missing trace digest"
compare_digest=$(extract_metric_field "$compare_metric_line" "digest") || fail "S7_REPLAY_COMPARE_DIGEST_MISSING" "missing compare digest"
[[ "$trace_digest" == "$compare_digest" ]] || fail "S7_REPLAY_DIGEST_UNSTABLE" "trace_digest=$trace_digest compare_digest=$compare_digest"

[[ "$trace_id" == "$trace_id_replay" ]] || fail "S7_REPLAY_TRACE_CONTENT_ID_UNSTABLE" "trace_id=$trace_id replay_trace_id=$trace_id_replay"
[[ "$observed_id" == "$observed_id_replay" ]] || fail "S7_REPLAY_OBSERVED_CONTENT_ID_UNSTABLE" "observed_id=$observed_id replay_observed_id=$observed_id_replay"
[[ "$scenario_id" == "$scenario_id_replay" ]] || fail "S7_REPLAY_SCENARIO_CONTENT_ID_UNSTABLE" "scenario_id=$scenario_id replay_scenario_id=$scenario_id_replay"

echo "FOUNDRY_GPU_QUARANTINE_S7: METRIC replay_digest=$trace_digest"
echo "FOUNDRY_GPU_QUARANTINE_S7: METRIC replay_trace_id=$trace_id replay_observed_id=$observed_id replay_scenario_id=$scenario_id"
echo "FOUNDRY_GPU_QUARANTINE_S7: REPLAY_DETERMINISM ok"

TRACE_DIR="$TRACE_DIR" \
S7_MIN_PROTOCOL_EVENTS="$S7_MIN_PROTOCOL_EVENTS" \
S7_MIN_PROTOCOL_PAIRS="$S7_MIN_PROTOCOL_PAIRS" \
S7_MIN_SCENARIO_EVENTS="$S7_MIN_SCENARIO_EVENTS" \
S7_MIN_OBSERVED_CAPS="$S7_MIN_OBSERVED_CAPS" \
S7_MIN_CAP_GRANTED="$S7_MIN_CAP_GRANTED" \
S7_MIN_CAP_USED="$S7_MIN_CAP_USED" \
S7_MIN_EXPORT_WIDTH="$S7_MIN_EXPORT_WIDTH" \
S7_MIN_EXPORT_HEIGHT="$S7_MIN_EXPORT_HEIGHT" \
python3 - <<'PY'
import json
import os
import pathlib
import sys


def fail(code: str, detail: str) -> None:
    print(f"FOUNDRY_GPU_QUARANTINE_S7: FAIL code={code} detail={detail}", file=sys.stderr)
    raise SystemExit(1)


def parse_int(name: str) -> int:
    try:
        return int(os.environ[name])
    except Exception as exc:  # pragma: no cover - defensive in gate script
        fail("S7_THRESHOLD_ENV_INVALID", f"invalid {name}: {exc}")


trace_dir = pathlib.Path(os.environ["TRACE_DIR"])
protocol = json.loads((trace_dir / "gpu_quarantine_protocol.json").read_text())
scenario = json.loads((trace_dir / "gpu_quarantine_scenario.json").read_text())
obs = json.loads((trace_dir / "gpu_quarantine_observed.json").read_text())

min_protocol_events = parse_int("S7_MIN_PROTOCOL_EVENTS")
min_protocol_pairs = parse_int("S7_MIN_PROTOCOL_PAIRS")
min_scenario_events = parse_int("S7_MIN_SCENARIO_EVENTS")
min_observed_caps = parse_int("S7_MIN_OBSERVED_CAPS")
min_cap_granted = parse_int("S7_MIN_CAP_GRANTED")
min_cap_used = parse_int("S7_MIN_CAP_USED")
min_export_width = parse_int("S7_MIN_EXPORT_WIDTH")
min_export_height = parse_int("S7_MIN_EXPORT_HEIGHT")

events = protocol.get("protocol_trace", {}).get("events", [])
protocol_events = len(events)
if protocol_events < min_protocol_events:
    fail("S7_THRESHOLD_PROTOCOL_EVENTS", f"protocol_events={protocol_events} threshold>={min_protocol_events}")
if protocol_events % 2 != 0:
    fail("S7_THRESHOLD_PROTOCOL_PAIRING", f"protocol_events must be even, got {protocol_events}")

protocol_pairs = protocol_events // 2
if protocol_pairs < min_protocol_pairs:
    fail("S7_THRESHOLD_PROTOCOL_PAIRS", f"protocol_pairs={protocol_pairs} threshold>={min_protocol_pairs}")

for i, event in enumerate(events):
    seq = event.get("seq")
    if seq != i + 1:
        fail("S7_THRESHOLD_PROTOCOL_SEQ", f"event_index={i} seq={seq} expected={i + 1}")

scenario_events = scenario.get("scenario_trace", {}).get("events", [])
scenario_event_count = len(scenario_events)
if scenario_event_count < min_scenario_events:
    fail(
        "S7_THRESHOLD_SCENARIO_EVENTS",
        f"scenario_events={scenario_event_count} threshold>={min_scenario_events}",
    )

scenario_names = {e.get("name") for e in scenario_events}
required_names = {"protocol_trace_ref", "observed_caps_ref", "display_export", "scanout_frame"}
if not required_names.issubset(scenario_names):
    missing = sorted(required_names - scenario_names)
    fail("S7_THRESHOLD_SCENARIO_REQUIRED_EVENTS", f"missing={','.join(missing)}")

caps = obs.get("capabilities", [])
cap_count = len(caps)
if cap_count < min_observed_caps:
    fail("S7_THRESHOLD_OBSERVED_CAPS", f"capabilities={cap_count} threshold>={min_observed_caps}")

export_caps = [c for c in caps if c.get("cap") == "domain.gpu_quarantine.export_display"]
if not export_caps:
    fail("S7_THRESHOLD_EXPORT_CAP_MISSING", "missing cap domain.gpu_quarantine.export_display")

export_cap = export_caps[0]
counts = export_cap.get("counts", {})
granted = int(counts.get("granted", 0))
used = int(counts.get("used", 0))
if granted < min_cap_granted:
    fail("S7_THRESHOLD_CAP_GRANTED", f"granted={granted} threshold>={min_cap_granted}")
if used < min_cap_used:
    fail("S7_THRESHOLD_CAP_USED", f"used={used} threshold>={min_cap_used}")

summary = obs.get("summary", {})
width = int(summary.get("width", 0))
height = int(summary.get("height", 0))
if width < min_export_width:
    fail("S7_THRESHOLD_EXPORT_WIDTH", f"width={width} threshold>={min_export_width}")
if height < min_export_height:
    fail("S7_THRESHOLD_EXPORT_HEIGHT", f"height={height} threshold>={min_export_height}")

print(
    f"FOUNDRY_GPU_QUARANTINE_S7: METRIC protocol_events={protocol_events} protocol_pairs={protocol_pairs} "
    f"scenario_events={scenario_event_count} observed_caps={cap_count}"
)
print(
    f"FOUNDRY_GPU_QUARANTINE_S7: METRIC export_cap_granted={granted} export_cap_used={used} "
    f"width={width} height={height}"
)
print("FOUNDRY_GPU_QUARANTINE_S7: THRESHOLDS ok")
PY

store_output=$(cargo run -p store_cli -- emit-plan \
  --catalog "$ROOT_DIR/store/catalog.json" \
  --program-id "ramen.gpu.scanout" \
  --out "$STORE_OUT/launch_plan_gpu.json" \
  --artifact-root "$ARTIFACT_ROOT" \
  --installed-root "$INSTALLED_ROOT" \
  --tmp-root "$TMP_ROOT" \
  --store-socket "$STORE_SOCKET")

require_contains "$store_output" "store: emitted execution launch plan:" "S7_SENTINEL_STORE_EMIT_MISSING"

artifact_ref=$(python3 - <<PY
import json
print(json.load(open("$STORE_OUT/launch_plan_gpu.json"))["artifact_ref"])
PY
)

[[ -f "$INSTALLED_ROOT/artifacts/${artifact_ref#sha256:}.blob" ]] \
  || fail "S7_PLAN_ARTIFACT_BLOB_MISSING" "missing blob for emitted launch plan artifact"
[[ -f "$INSTALLED_ROOT/artifacts/${artifact_ref#sha256:}.manifest.json" ]] \
  || fail "S7_PLAN_ARTIFACT_MANIFEST_MISSING" "missing manifest for emitted launch plan artifact"

super_output=$(cargo run -p runtime_supervisor -- \
  --plan "$STORE_OUT/launch_plan_gpu.json" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET")

require_contains "$super_output" "supervisor: plan ok program_id=ramen.gpu.scanout runner=gpu_quarantine_v1" "S7_SENTINEL_SUPERVISOR_PLAN_MISSING"
require_contains "$super_output" "gpu_runner_v1: start ok domain=700 profile=1" "S7_SENTINEL_RUNNER_START_MISSING"
require_contains "$super_output" "gpu_runner_v1: export ok surface=" "S7_SENTINEL_RUNNER_EXPORT_MISSING"
require_contains "$super_output" "gpu_runner_v1: scanout ok frame_seq=1" "S7_SENTINEL_RUNNER_SCANOUT_MISSING"
require_contains "$super_output" "gpu_runner_v1: ok" "S7_SENTINEL_RUNNER_OK_MISSING"

# Negative assertion: malformed payload should fail replay.
cp "$TRACE_DIR/gpu_quarantine_protocol.json" "$TRACE_DIR/gpu_quarantine_protocol_malformed.json"
TRACE_DIR="$TRACE_DIR" python3 - <<'PY'
import json
import os
import pathlib

trace_dir = pathlib.Path(os.environ["TRACE_DIR"])
path = trace_dir / "gpu_quarantine_protocol_malformed.json"
doc = json.loads(path.read_text())
doc["protocol_trace"]["events"][0]["bytes_hex"] += "0"
path.write_text(json.dumps(doc, indent=2))
PY

if replay_fail_out=$(python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$TRACE_DIR/gpu_quarantine_protocol_malformed.json" 2>&1); then
  fail "S7_NEGATIVE_REPLAY_EXPECTED_FAIL" "expected malformed protocol replay to fail"
fi
require_contains "$replay_fail_out" "hex length must be even" "S7_NEGATIVE_REPLAY_REASON_MISMATCH"

# Negative assertion: policy violation (forged display capability token) must fail runner.
STORE_OUT="$STORE_OUT" python3 - <<'PY'
import json
import os
import pathlib

store_out = pathlib.Path(os.environ["STORE_OUT"])
plan = json.loads((store_out / "launch_plan_gpu.json").read_text())
# runtime_supervisor validates display_cap_token_* against expected token fields.
# Forge a non-zero token to ensure the runner rejects the plan.
if plan.get("schema_version") == 1:
    payload = json.loads(plan["runner_config"]["config_json"])
    payload["gpu_quarantine"]["display_cap_token_low"] = 1
    plan["runner_config"]["config_json"] = json.dumps(payload)
else:
    plan["gpu_quarantine"]["display_cap_token_low"] = 1
(store_out / "launch_plan_gpu_bad_cap.json").write_text(json.dumps(plan, indent=2))
PY

if bad_cap_out=$(cargo run -p runtime_supervisor -- --plan "$STORE_OUT/launch_plan_gpu_bad_cap.json" --installed-root "$INSTALLED_ROOT" --store-socket "$STORE_SOCKET" 2>&1); then
  fail "S7_NEGATIVE_INVALID_CAP_EXPECTED_FAIL" "expected supervisor to reject invalid gpu display capability"
fi
require_contains "$bad_cap_out" "invalid display_cap_token" "S7_NEGATIVE_INVALID_CAP_REASON_MISMATCH"

# Negative assertion: malformed launch config (zero width) must fail emit-plan.
OUT_DIR="$OUT_DIR" ROOT_DIR="$ROOT_DIR" python3 - <<'PY'
import json
import os
import pathlib

root = pathlib.Path(os.environ["ROOT_DIR"])
out_dir = pathlib.Path(os.environ["OUT_DIR"])
catalog = json.loads((root / "store" / "catalog.json").read_text())
entry = next(e for e in catalog["entries"] if e.get("program_id") == "ramen.gpu.scanout")
entry = dict(entry)
entry["program_id"] = "ramen.gpu.scanout.bad"
entry["gpu_quarantine"] = dict(entry["gpu_quarantine"])
entry["gpu_quarantine"]["width"] = 0
bad = {"entries": [entry]}
(out_dir / "catalog_gpu_bad.json").write_text(json.dumps(bad, indent=2))
PY

if bad_plan_out=$(cargo run -p store_cli -- emit-plan \
  --catalog "$OUT_DIR/catalog_gpu_bad.json" \
  --program-id "ramen.gpu.scanout.bad" \
  --out "$STORE_OUT/launch_plan_gpu_bad.json" \
  --artifact-root "$ARTIFACT_ROOT" \
  --installed-root "$INSTALLED_ROOT" \
  --tmp-root "$TMP_ROOT" \
  --store-socket "$STORE_SOCKET" 2>&1); then
  fail "S7_NEGATIVE_BAD_PLAN_EXPECTED_FAIL" "expected store emit-plan to reject malformed gpu config"
fi
require_contains "$bad_plan_out" "gpu_quarantine width/height must be non-zero" "S7_NEGATIVE_BAD_PLAN_REASON_MISMATCH"

POLICY_INSTALLED_ROOT="$OUT_DIR/policy_installed"
mkdir -p "$POLICY_INSTALLED_ROOT/artifacts"

cargo run -p store_cli -- ingest \
  --src "$TRACE_DIR/gpu_quarantine_protocol.json" \
  --installed-root "$POLICY_INSTALLED_ROOT" \
  --kind "trace_artifact_v0" \
  --channel "Experimental" \
  --evidence-policy "$S7_EVIDENCE_POLICY_PATH" \
  --store-socket "$STORE_SOCKET" >/dev/null

cargo run -p store_cli -- ingest \
  --src "$TRACE_DIR/gpu_quarantine_observed.json" \
  --installed-root "$POLICY_INSTALLED_ROOT" \
  --kind "observed_caps_v0" \
  --channel "Experimental" \
  --evidence-policy "$S7_EVIDENCE_POLICY_PATH" \
  --store-socket "$STORE_SOCKET" >/dev/null

cargo run -p store_cli -- ingest \
  --src "$TRACE_DIR/gpu_quarantine_scenario.json" \
  --installed-root "$POLICY_INSTALLED_ROOT" \
  --kind "scenario_trace" \
  --channel "Experimental" \
  --evidence-policy "$S7_EVIDENCE_POLICY_PATH" \
  --store-socket "$STORE_SOCKET" >/dev/null

cat >"$TMP_ROOT/evidence_policy_tiny.toml" <<'TOML'
schema_version = 1
max_bytes = 64
kinds = ["trace_artifact_v0"]
redact_literals = []
replacement = "[REDACTED]"
TOML

if policy_fail_out=$(cargo run -p store_cli -- ingest \
  --src "$TRACE_DIR/gpu_quarantine_protocol.json" \
  --installed-root "$POLICY_INSTALLED_ROOT" \
  --kind "trace_artifact_v0" \
  --channel "Experimental" \
  --evidence-policy "$TMP_ROOT/evidence_policy_tiny.toml" \
  --store-socket "$STORE_SOCKET" 2>&1); then
  fail "S7_NEGATIVE_POLICY_EXPECTED_FAIL" "expected evidence policy size violation to fail ingest"
fi
require_contains "$policy_fail_out" "evidence exceeds policy size limit" "S7_NEGATIVE_POLICY_REASON_MISMATCH"
require_contains "$policy_fail_out" "EVIDENCE_POLICY_SIZE_LIMIT_EXCEEDED" "S7_NEGATIVE_POLICY_CODE_MISMATCH"

echo "FOUNDRY_GPU_QUARANTINE_S7: NEGATIVE_ASSERTIONS ok"
echo "FOUNDRY_GPU_QUARANTINE_S7: POLICY ok"

echo "FOUNDRY_GPU_QUARANTINE_S7: ok"
