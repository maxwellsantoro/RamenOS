#!/usr/bin/env bash
# Capture a HIL appliance serial transcript and emit wrapper evidence.
#
# This script observes only. It does not press power/reset, select boot media, or
# upgrade a stale serial log into graduation evidence.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# shellcheck source=hil_gate_common.sh
source "$ROOT_DIR/tools/hil/hil_gate_common.sh"

GATE_ID="RAMEN_HIL_APPLIANCE_SERIAL"

fail() {
  echo "$GATE_ID: FAIL code=$1 detail=$2" >&2
  exit 1
}

now_ms() {
  python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
}

utc_stamp() {
  python3 - <<'PY'
from datetime import datetime, timezone
print(datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"))
PY
}

configure_serial() {
  local dev="$1"
  if command -v stty >/dev/null 2>&1; then
    stty -F "$dev" 115200 raw -echo 2>/dev/null \
      || stty -f "$dev" 115200 raw -echo 2>/dev/null \
      || fail "SERIAL_STTY_FAILED" "failed to configure serial device: $dev"
  fi
}

capture_serial_device() {
  local dev="$1"
  local out="$2"
  local timeout_s="$3"

  [[ -e "$dev" ]] || fail "SERIAL_DEV_MISSING" "serial device not found: $dev"
  configure_serial "$dev"
  : >"$out"

  if command -v timeout >/dev/null 2>&1; then
    timeout "${timeout_s}s" cat "$dev" >"$out" || true
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "${timeout_s}s" cat "$dev" >"$out" || true
  else
    cat "$dev" >"$out" &
    local cat_pid=$!
    sleep "$timeout_s"
    kill "$cat_pid" >/dev/null 2>&1 || true
    wait "$cat_pid" >/dev/null 2>&1 || true
  fi
}

mkdir -p "${RAMEN_HIL_EVIDENCE_DIR:-out/evidence}"
EVIDENCE_DIR="${RAMEN_HIL_EVIDENCE_DIR:-out/evidence}"
APPLIANCE_ID="${RAMEN_HIL_APPLIANCE_ID:-pi-hil-01}"
TARGET_ID="${RAMEN_HIL_TARGET_ID:-${RAMEN_HIL_MACHINE_ID:-intel-nuc-12-reference}}"
RUN_STAMP="$(utc_stamp)"
RUN_ID="${RAMEN_HIL_RUN_ID:-hil_appliance_${RUN_STAMP}_${APPLIANCE_ID}_serial_observer}"
if [[ ! "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
  fail "RUN_ID_INVALID" "RAMEN_HIL_RUN_ID may contain only letters, digits, dot, underscore, and dash"
fi
SERIAL_LOG="$EVIDENCE_DIR/${RUN_ID}.serial.log"
CONTROLLER_LOG="$EVIDENCE_DIR/${RUN_ID}.controller.log"
EVIDENCE_JSON="$EVIDENCE_DIR/${RUN_ID}.json"
TIMEOUT_S="${RAMEN_HIL_CAPTURE_TIMEOUT_S:-30}"
SERIAL_DEV="${RAMEN_HIL_SERIAL_DEV:-}"
SERIAL_SOURCE_LOG="${RAMEN_HIL_SERIAL_LOG:-}"
SERIAL_INPUT_KIND=""
EVIDENCE_LEVEL=""
STARTED_MS="$(now_ms)"

if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -n "$SERIAL_SOURCE_LOG" ]]; then
  fail "STALE_LOG_IN_GRADUATION" "RAMEN_HIL_GRADUATION=1 forbids RAMEN_HIL_SERIAL_LOG"
fi

if [[ -n "$SERIAL_SOURCE_LOG" && -n "$SERIAL_DEV" ]]; then
  fail "SERIAL_INPUT_AMBIGUOUS" "set only one of RAMEN_HIL_SERIAL_LOG or RAMEN_HIL_SERIAL_DEV"
fi

{
  echo "$GATE_ID: run_id=$RUN_ID"
  echo "$GATE_ID: appliance_id=$APPLIANCE_ID"
  echo "$GATE_ID: target_id=$TARGET_ID"
  echo "$GATE_ID: timeout_s=$TIMEOUT_S"
} >"$CONTROLLER_LOG"

if [[ -n "$SERIAL_SOURCE_LOG" ]]; then
  [[ -f "$SERIAL_SOURCE_LOG" ]] || fail "SERIAL_LOG_MISSING" "serial log not found: $SERIAL_SOURCE_LOG"
  cp "$SERIAL_SOURCE_LOG" "$SERIAL_LOG"
  SERIAL_INPUT_KIND="development_log"
  EVIDENCE_LEVEL="PASS/HIL-LOG"
  echo "$GATE_ID: serial_input=development_log path=$SERIAL_SOURCE_LOG" >>"$CONTROLLER_LOG"
elif [[ -n "$SERIAL_DEV" ]]; then
  SERIAL_INPUT_KIND="live_device"
  EVIDENCE_LEVEL="PASS/HIL-APPLIANCE"
  echo "$GATE_ID: serial_input=live_device dev=$SERIAL_DEV" >>"$CONTROLLER_LOG"
  capture_serial_device "$SERIAL_DEV" "$SERIAL_LOG" "$TIMEOUT_S"
else
  fail "SERIAL_INPUT_MISSING" "set RAMEN_HIL_SERIAL_DEV for live capture or RAMEN_HIL_SERIAL_LOG for development replay"
fi

if [[ ! -s "$SERIAL_LOG" ]]; then
  fail "NO_SERIAL_BYTES" "serial capture produced no bytes"
fi

if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" ]]; then
  ramen_hil_assert_provenance_markers "$SERIAL_LOG" \
    || fail "PROVENANCE_MISSING" "graduation serial capture missing hil_evidence provenance markers"
fi

ENDED_MS="$(now_ms)"

python3 - "$EVIDENCE_JSON" "$SERIAL_LOG" "$CONTROLLER_LOG" "$RUN_ID" "$APPLIANCE_ID" \
  "$TARGET_ID" "$STARTED_MS" "$ENDED_MS" "${SERIAL_DEV:-}" "$SERIAL_INPUT_KIND" "$EVIDENCE_LEVEL" <<'PY'
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

(
    out_path,
    serial_log,
    controller_log,
    run_id,
    appliance_id,
    target_id,
    started_ms,
    ended_ms,
    serial_dev,
    serial_input_kind,
    evidence_level,
) = sys.argv[1:12]

def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def git_sha() -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    except Exception:
        return "unknown"

serial_text = Path(serial_log).read_text(encoding="utf-8", errors="replace")
observed = []
hil_markers = {}
for raw_line in serial_text.splitlines():
    line = raw_line.strip()
    if not line:
        continue
    if "RAMEN OS" in line:
        observed.append("RAMEN OS")
    if "golden_machine:" in line:
        observed.append(line[line.index("golden_machine:") :])
    if "persistent_storage:" in line:
        observed.append(line[line.index("persistent_storage:") :])
    if "hil_evidence:" in line:
        marker = line[line.index("hil_evidence:") :]
        observed.append(marker)
        payload = marker.split("hil_evidence:", 1)[1].strip()
        if "=" in payload:
            key, value = payload.split("=", 1)
            hil_markers[key.strip()] = value.strip()

deduped = list(dict.fromkeys(observed))
payload = {
    "schema_version": 1,
    "evidence_kind": "hil_appliance_run_v0",
    "evidence_level": evidence_level,
    "run_id": run_id,
    "appliance_id": appliance_id,
    "target_id": target_id,
    "git_sha": git_sha(),
    "gate": os.environ.get("RAMEN_HIL_CAPTURE_GATE", "s12_4_1_serial_observer"),
    "started_at_unix_ms": int(started_ms),
    "ended_at_unix_ms": int(ended_ms),
    "serial_device": serial_dev or os.environ.get("RAMEN_HIL_SERIAL_LOG", ""),
    "serial_input_kind": serial_input_kind,
    "serial_log": serial_log,
    "serial_log_sha256": sha256_file(serial_log),
    "controller_log": controller_log,
    "controller_log_sha256": sha256_file(controller_log),
    "power_events": [],
    "artifact_hashes": {},
    "serial_markers_observed": deduped,
    "target_hil_evidence_markers": hil_markers,
    "gate_evidence": [],
    "result": "pass",
    "graduation_mode": os.environ.get("RAMEN_HIL_GRADUATION", "") == "1",
}
Path(out_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

echo "$GATE_ID: METRIC evidence_json=$EVIDENCE_JSON"
echo "$GATE_ID: METRIC serial_log=$SERIAL_LOG"
echo "$GATE_ID: PASS level=$EVIDENCE_LEVEL"
echo "$GATE_ID: ok"
