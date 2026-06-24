#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
TRACE_DIR="$OUT_DIR/trace"
INSTALLED_ROOT="$OUT_DIR/installed"
CAPSULE_OUT="$OUT_DIR/capsule_relay"
STORE_SOCKET="$CAPSULE_OUT/store.sock"
STORE_LOG="$CAPSULE_OUT/store_service.log"

mkdir -p "$TRACE_DIR" "$INSTALLED_ROOT" "$CAPSULE_OUT"
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

cargo run -p idl_codegen -- \
  --in idl/harness/capsule_control_v0.toml \
  --out kernel_api/src/generated/capsule_control_v0.generated.rs
cargo run -p idl_codegen -- \
  --in idl/harness/capsule_control_v0.toml \
  --out tools/capsule/generated/capsule_control_v0.h
cargo run -p idl_codegen -- \
  --in idl/harness/echo_harness_v0.toml \
  --out kernel_api/src/generated/echo_harness_v0.generated.rs

grep -q "typedef struct" "$ROOT_DIR/tools/capsule/generated/capsule_control_v0.h"
grep -q "HelloReply" "$ROOT_DIR/tools/capsule/generated/capsule_control_v0.h"

TRACE_OUT="$TRACE_DIR/capsule_relay.json"
PAYLOAD="$CAPSULE_OUT/payload.bin"

run_out=$(CAPSULE_PAYLOAD_DIR="$CAPSULE_OUT" cargo run -p capsule_relay -- \
  --payload "$PAYLOAD" \
  --trace-out "$TRACE_OUT" \
  --store-socket "$STORE_SOCKET")

echo "$run_out" | grep -q "CAPSULE_RELAY: hello ok"
echo "$run_out" | grep -q "CAPSULE_RELAY: health ok"
echo "$run_out" | grep -q "CAPSULE_RELAY: echo ok"
echo "$run_out" | grep -q "CAPSULE_RELAY: bad payload rejected ok"
echo "$run_out" | grep -q "CAPSULE_RELAY: shutdown ok"
echo "$run_out" | grep -q "CAPSULE_RELAY: ok"

trace_id=$(echo "$run_out" | sed -E -n 's/^CAPSULE_RELAY: trace_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
control_trace_id=$(echo "$run_out" | sed -E -n 's/^CAPSULE_RELAY: control_trace_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
observed_id=$(echo "$run_out" | sed -E -n 's/^CAPSULE_RELAY: observed_caps_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
scenario_id=$(echo "$run_out" | sed -E -n 's/^CAPSULE_RELAY: scenario_trace_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)
payload_id=$(echo "$run_out" | sed -E -n 's/^CAPSULE_RELAY: payload_content_id=(sha256:[0-9a-f]+).*/\1/p' | tail -n 1)

if [[ -z "$trace_id" || -z "$control_trace_id" || -z "$observed_id" || -z "$scenario_id" || -z "$payload_id" ]]; then
  echo "missing content ids in capsule relay output" >&2
  exit 2
fi

TRACE_STEM="${TRACE_OUT%.json}"
CONTROL_TRACE_JSON="${TRACE_STEM}_control.json"
OBSERVED_JSON="${TRACE_STEM}_observed.json"
SCENARIO_JSON="${TRACE_STEM}_scenario.json"

cargo run -p store_cli -- validate-trace --src "$TRACE_OUT"
cargo run -p store_cli -- validate-trace --src "$CONTROL_TRACE_JSON"
cargo run -p store_cli -- validate-observed-caps --src "$OBSERVED_JSON"
cargo run -p store_cli -- validate-trace --src "$SCENARIO_JSON"

trace_blob="$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.blob"
trace_manifest="$INSTALLED_ROOT/artifacts/${trace_id#sha256:}.manifest.json"
control_blob="$INSTALLED_ROOT/artifacts/${control_trace_id#sha256:}.blob"
control_manifest="$INSTALLED_ROOT/artifacts/${control_trace_id#sha256:}.manifest.json"
obs_blob="$INSTALLED_ROOT/artifacts/${observed_id#sha256:}.blob"
obs_manifest="$INSTALLED_ROOT/artifacts/${observed_id#sha256:}.manifest.json"
scenario_blob="$INSTALLED_ROOT/artifacts/${scenario_id#sha256:}.blob"
scenario_manifest="$INSTALLED_ROOT/artifacts/${scenario_id#sha256:}.manifest.json"

for path in "$trace_blob" "$trace_manifest" "$control_blob" "$control_manifest" \
            "$obs_blob" "$obs_manifest" "$scenario_blob" "$scenario_manifest"; do
  if [[ ! -f "$path" ]]; then
    echo "missing artifact: $path" >&2
    exit 2
  fi
done

python3 tools/trace/replay_protocol_trace.py --trace "$trace_blob"
python3 tools/trace/replay_protocol_trace.py --trace "$control_blob"

TRACE_ID="$trace_id" PAYLOAD_ID="$payload_id" OBS_BLOB="$obs_blob" python3 - <<'PY'
import json
import os
import sys

trace_id = os.environ["TRACE_ID"]
payload_id = os.environ["PAYLOAD_ID"]
obs_path = os.environ["OBS_BLOB"]

with open(obs_path, "r", encoding="utf-8") as f:
    obs = json.load(f)

caps = obs.get("capabilities", [])
cap = next((c for c in caps if c.get("cap") == "harness.echo"), None)
if not cap:
    sys.exit("observed_caps missing harness.echo")
if trace_id not in cap.get("evidence", []):
    sys.exit("observed_caps missing trace evidence")
if payload_id not in cap.get("scope", {}).get("artifact_ids", []):
    sys.exit("observed_caps missing payload scope")
if cap.get("counts", {}).get("used") != 1:
    sys.exit("observed_caps used count mismatch")
PY

TRACE_ID="$trace_id" CONTROL_TRACE_ID="$control_trace_id" OBS_ID="$observed_id" SCENARIO_BLOB="$scenario_blob" python3 - <<'PY'
import json
import os
import sys

trace_id = os.environ["TRACE_ID"]
control_trace_id = os.environ["CONTROL_TRACE_ID"]
observed_id = os.environ["OBS_ID"]
scenario_path = os.environ["SCENARIO_BLOB"]

with open(scenario_path, "r", encoding="utf-8") as f:
    scenario = json.load(f)

if scenario.get("trace_type") != "scenario_trace":
    sys.exit("scenario trace_type mismatch")

events = scenario.get("scenario_trace", {}).get("events", [])
refs = [e for e in events if e.get("name") == "protocol_trace_ref"]
if trace_id not in [r.get("payload", {}).get("content_id") for r in refs]:
    sys.exit("scenario trace missing echo trace ref")
if control_trace_id not in [r.get("payload", {}).get("content_id") for r in refs]:
    sys.exit("scenario trace missing control trace ref")

obs_refs = [e for e in events if e.get("name") == "observed_caps_ref"]
if observed_id not in [r.get("payload", {}).get("content_id") for r in obs_refs]:
    sys.exit("scenario trace missing observed caps ref")

payload_refs = [e for e in events if e.get("name") == "payload_ref"]
if not payload_refs:
    sys.exit("scenario trace missing payload_ref event")
PY

echo "FOUNDRY_DRIVER_CAPSULE_S3X: host-only ok"

# --- VM mode test (optional, requires kernel + QEMU) ---
if [[ -z "${S2_COMPAT_KERNEL:-}" ]]; then
  echo "FOUNDRY_DRIVER_CAPSULE_S3X: skipping VM mode (S2_COMPAT_KERNEL not set)"
elif ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "FOUNDRY_DRIVER_CAPSULE_S3X: skipping VM mode (qemu-system-x86_64 not found)"
else
  # Build capsule agent initrd
  if ! "$ROOT_DIR/tools/capsule/build_capsule_initrd.sh" 2>/dev/null; then
    echo "FOUNDRY_DRIVER_CAPSULE_S3X: skipping VM mode (initrd build failed, likely missing static compiler)"
  else
    CAPSULE_INITRD="$CAPSULE_OUT/capsule_initrd.cpio.gz"
    SOCKET_PATH="$CAPSULE_OUT/agent.sock"
    VM_TRACE_OUT="$TRACE_DIR/capsule_relay_vm.json"

    # Clean up stale socket
    rm -f "$SOCKET_PATH"

    vm_run_out=$(CAPSULE_PAYLOAD_DIR="$CAPSULE_OUT" cargo run -p capsule_relay -- \
      --mode vm \
      --kernel "$S2_COMPAT_KERNEL" \
      --initrd "$CAPSULE_INITRD" \
      --socket-path "$SOCKET_PATH" \
      --vm-timeout-secs 30 \
      --payload "$PAYLOAD" \
      --trace-out "$VM_TRACE_OUT" \
      --store-socket "$STORE_SOCKET")

    # Verify VM protocol output (no bad payload test in VM mode)
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: mode = vm"
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: hello ok"
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: health ok"
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: echo ok"
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: shutdown ok"
    echo "$vm_run_out" | grep -q "CAPSULE_RELAY: ok"

    echo "FOUNDRY_DRIVER_CAPSULE_S3X: VM mode ok"
  fi
fi

echo "FOUNDRY_DRIVER_CAPSULE_S3X: ok"
