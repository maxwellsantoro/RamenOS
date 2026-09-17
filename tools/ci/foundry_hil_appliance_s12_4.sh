#!/usr/bin/env bash
# Foundry gate for S12.4 / S13.9 HIL Appliance Controller scaffold.
#
# Default CI validates docs/manifests/schema only. Physical controller checks run
# only with RAMEN_HIL_APPLIANCE=1.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

GATE_ID="FOUNDRY_HIL_APPLIANCE_S12_4"

fail() {
  echo "$GATE_ID: FAIL code=$1 detail=$2" >&2
  exit 1
}

skip_appliance() {
  local reason="$1"
  echo "$GATE_ID: INFO appliance=skipped reason=$reason"
}

require_file() {
  local path="$1"
  local code="$2"
  test -f "$path" || fail "$code" "missing required file: $path"
}

echo "=== HIL Appliance Controller Gate (S12.4 / S13.9) ==="
echo "$GATE_ID: INFO step=docs_manifest_inventory"

PLAN="docs/plans/2026-06-22-hil-appliance-controller.md"
MANIFEST="hardware/hil_appliance_v0.toml"
GOLDEN_MANIFEST="hardware/golden_machine_v0.toml"
EVIDENCE_LEVELS="EVIDENCE_LEVELS.md"
EVIDENCE_SCHEMA="docs/HIL_APPLIANCE_EVIDENCE_V0.md"
NEXT_TASKS="NEXT_TASKS.md"
HARDWARE_STRATEGY="docs/HARDWARE_STRATEGY.md"
SERIAL_CAPTURE_SCRIPT="tools/hil/appliance_capture_serial.sh"

require_file "$PLAN" "PLAN_MISSING"
require_file "$MANIFEST" "MANIFEST_MISSING"
require_file "$GOLDEN_MANIFEST" "GOLDEN_MANIFEST_MISSING"
require_file "$EVIDENCE_LEVELS" "EVIDENCE_LEVELS_MISSING"
require_file "$EVIDENCE_SCHEMA" "EVIDENCE_SCHEMA_MISSING"
require_file "$NEXT_TASKS" "NEXT_TASKS_MISSING"
require_file "$HARDWARE_STRATEGY" "HARDWARE_STRATEGY_MISSING"
require_file "$SERIAL_CAPTURE_SCRIPT" "SERIAL_CAPTURE_SCRIPT_MISSING"
test -x "$SERIAL_CAPTURE_SCRIPT" \
  || fail "SERIAL_CAPTURE_SCRIPT_NOT_EXECUTABLE" "$SERIAL_CAPTURE_SCRIPT must be executable"

grep -q 'HIL Appliance Controller' "$PLAN" \
  || fail "PLAN_NAME_MISSING" "plan must use canonical HIL Appliance Controller name"
grep -q 'hardware/hil_appliance_v0.toml' "$PLAN" \
  || fail "PLAN_MANIFEST_REF_MISSING" "plan must reference appliance manifest"
grep -q 'foundry_hil_appliance_s12_4.sh' "$PLAN" \
  || fail "PLAN_GATE_REF_MISSING" "plan must reference this gate"
grep -q 'docs/HIL_APPLIANCE_EVIDENCE_V0.md' "$PLAN" \
  || fail "PLAN_SCHEMA_REF_MISSING" "plan must reference wrapper evidence schema"
grep -q 'tools/hil/appliance_capture_serial.sh' "$PLAN" \
  || fail "PLAN_SERIAL_CAPTURE_REF_MISSING" "plan must reference the serial observer script"

grep -q 'PASS/HIL-APPLIANCE' "$EVIDENCE_LEVELS" \
  || fail "EVIDENCE_LEVEL_MISSING" "EVIDENCE_LEVELS must define PASS/HIL-APPLIANCE"
grep -qi 'appliance observations are target truth' "$EVIDENCE_LEVELS" \
  || fail "CLAIM_SAFETY_MISSING" "EVIDENCE_LEVELS must reject appliance-as-target-truth claim"

grep -q 'evidence_kind.*hil_appliance_run_v0' "$EVIDENCE_SCHEMA" \
  || fail "SCHEMA_KIND_MISSING" "evidence schema must define hil_appliance_run_v0"
grep -q 'gate_evidence' "$EVIDENCE_SCHEMA" \
  || fail "SCHEMA_GATE_EVIDENCE_MISSING" "appliance schema must reference per-gate evidence"
grep -q 'The appliance is not an oracle' "$EVIDENCE_SCHEMA" \
  || fail "SCHEMA_ORACLE_GUARD_MISSING" "schema doc must reject appliance oracle wording"
grep -q 'RAMEN_HIL_SERIAL_LOG' "$SERIAL_CAPTURE_SCRIPT" \
  || fail "SERIAL_CAPTURE_STALE_LOG_POLICY_MISSING" "serial capture script must handle development log policy"
grep -q 'STALE_LOG_IN_GRADUATION' "$SERIAL_CAPTURE_SCRIPT" \
  || fail "SERIAL_CAPTURE_GRADUATION_GUARD_MISSING" "serial capture script must reject stale logs in graduation mode"
grep -q 'serial_markers_observed' "$SERIAL_CAPTURE_SCRIPT" \
  || fail "SERIAL_CAPTURE_MARKER_SCAN_MISSING" "serial capture script must scan serial markers"
grep -q 'target_hil_evidence_markers' "$SERIAL_CAPTURE_SCRIPT" \
  || fail "SERIAL_CAPTURE_HIL_MARKER_SCAN_MISSING" "serial capture script must parse hil_evidence markers"
grep -q 'claim_path' "$EVIDENCE_LEVELS" \
  || fail "CLAIM_PATH_DOCTRINE_MISSING" "EVIDENCE_LEVELS must require per-gate claim_path disambiguation"

grep -q 'manifest = "hardware/hil_appliance_v0.toml"' "$GOLDEN_MANIFEST" \
  || fail "GOLDEN_APPLIANCE_LINK_MISSING" "golden machine manifest must link appliance manifest"

grep -q 'Pi GPIO UART is 3.3V TTL only' "$HARDWARE_STRATEGY" \
  || fail "TTL_WARNING_MISSING" "hardware strategy must document TTL-only Pi GPIO rule"
grep -q 'USB RS-232 adapter' "$HARDWARE_STRATEGY" \
  || fail "RS232_PATH_MISSING" "hardware strategy must document USB RS-232 default path"

python3 - "$MANIFEST" <<'PY'
import re
import sys

manifest = sys.argv[1]
text = open(manifest, encoding="utf-8").read()

def section(name: str) -> str:
    match = re.search(rf"^\[{re.escape(name)}\]\n(.*?)(?=^\[|\Z)", text, re.M | re.S)
    assert match, f"missing section [{name}]"
    return match.group(1)

def scalar(blob: str, key: str) -> str:
    match = re.search(rf"^{re.escape(key)}\s*=\s*(.+)$", blob, re.M)
    assert match, f"missing key {key}"
    raw = match.group(1).strip()
    if raw.startswith('"') and raw.endswith('"'):
        return raw[1:-1]
    return raw

assert scalar(text, "schema_version") == "1"
assert scalar(text, "role") == "hil_appliance_controller"

controller = section("controller")
requirements = section("requirements")
serial = section("serial")
evidence = section("evidence")

assert scalar(controller, "trust_boundary") == "lab_infrastructure_not_target_tcb"
assert scalar(requirements, "serial_capture") == "usb_rs232_default"
assert scalar(requirements, "power_control") == "intel_amt_primary"
assert scalar(requirements, "reset_control") == "intel_amt_primary"
assert scalar(serial, "raw_gpio_uart") == "ttl_3v3_only"
assert scalar(serial, "forbidden") == "direct_pi_gpio_to_rs232_db9"
assert scalar(evidence, "schema") == "docs/HIL_APPLIANCE_EVIDENCE_V0.md"

required_match = re.search(r"required_fields\s*=\s*\[(.*?)\]", evidence, re.S)
assert required_match, "missing evidence.required_fields"
required_fields = set(re.findall(r'"([^"]+)"', required_match.group(1)))
for field in [
    "schema_version",
    "evidence_level",
    "appliance_id",
    "target_id",
    "git_sha",
    "gate",
    "serial_log_sha256",
    "controller_log_sha256",
    "power_events",
    "gate_evidence",
    "result",
]:
    assert field in required_fields, field
PY

appliance_line="$(grep -n '| H0 | S12.4.1 HIL appliance serial observer' "$NEXT_TASKS" | head -n1 | cut -d: -f1 || true)"
s13_line="$(grep -n '| H3 | Add M.2 2280 PCIe NVMe and run S13 metal graduation' "$NEXT_TASKS" | head -n1 | cut -d: -f1 || true)"
[[ -n "$appliance_line" ]] || fail "NEXT_TASKS_H0_MISSING" "NEXT_TASKS must put the serial observer first in the physical lane as H0"
[[ -n "$s13_line" ]] || fail "NEXT_TASKS_S13_MISSING" "NEXT_TASKS must keep S13 graduation after appliance work"
if (( appliance_line >= s13_line )); then
  fail "NEXT_TASKS_ORDER" "appliance must precede S13 metal graduation"
fi

echo "$GATE_ID: METRIC docs_manifest=pass"

# Contract fixtures must never inherit live hardware or graduation settings.
(
  for name in $(compgen -v RAMEN_HIL_); do unset "$name"; done
  echo "$GATE_ID: INFO step=per_gate_evidence_contract"
  python3 tools/hil/test_provenance.py ProvenanceTests
  echo "$GATE_ID: METRIC per_gate_evidence_contract=pass"

echo "$GATE_ID: INFO step=serial_observer_contract"
SERIAL_OBSERVER_TMP="$(mktemp -d "${TMPDIR:-/tmp}/ramen-hil-serial.XXXXXX")"
SERIAL_OBSERVER_FIXTURE="$SERIAL_OBSERVER_TMP/fixture.serial.log"
SERIAL_OBSERVER_EVIDENCE="$SERIAL_OBSERVER_TMP/evidence"
SERIAL_OBSERVER_STDOUT="$SERIAL_OBSERVER_TMP/observer.out"
SERIAL_OBSERVER_NEGATIVE_STDOUT="$SERIAL_OBSERVER_TMP/observer-negative.out"
SERIAL_OBSERVER_EMPTY_LOG="$SERIAL_OBSERVER_TMP/empty.serial.log"
cat >"$SERIAL_OBSERVER_FIXTURE" <<'EOF'
RAMEN OS
golden_machine: hil_boot ok
persistent_storage: nvme_boot ok
hil_evidence: git_sha=fixture
hil_evidence: init_profile=hil_boot
hil_evidence: machine_id=lenovo-thinkcentre-m900-i7-6700-lab-01
hil_evidence: storage_manifest_sha256=fixture
hil_evidence: kernel_build_id=fixture
hil_evidence: init_img_sha256=fixture
hil_evidence: boot_epoch_nonce=fixture
EOF

RAMEN_HIL_RUN_ID=hil_appliance_gate_serial_observer \
RAMEN_HIL_EVIDENCE_DIR="$SERIAL_OBSERVER_EVIDENCE" \
RAMEN_HIL_SERIAL_LOG="$SERIAL_OBSERVER_FIXTURE" \
  bash "$SERIAL_CAPTURE_SCRIPT" >"$SERIAL_OBSERVER_STDOUT"

SERIAL_OBSERVER_JSON="$SERIAL_OBSERVER_EVIDENCE/hil_appliance_gate_serial_observer.json"
SERIAL_OBSERVER_LOG="$SERIAL_OBSERVER_EVIDENCE/hil_appliance_gate_serial_observer.serial.log"
SERIAL_OBSERVER_CONTROLLER="$SERIAL_OBSERVER_EVIDENCE/hil_appliance_gate_serial_observer.controller.log"
test -s "$SERIAL_OBSERVER_JSON" \
  || fail "SERIAL_OBSERVER_JSON_MISSING" "serial observer evidence JSON was not written"
test -s "$SERIAL_OBSERVER_LOG" \
  || fail "SERIAL_OBSERVER_LOG_MISSING" "serial observer transcript was not archived"
test -s "$SERIAL_OBSERVER_CONTROLLER" \
  || fail "SERIAL_OBSERVER_CONTROLLER_LOG_MISSING" "serial observer controller log was not archived"

python3 - "$SERIAL_OBSERVER_JSON" "$SERIAL_OBSERVER_LOG" "$SERIAL_OBSERVER_CONTROLLER" <<'PY'
import hashlib
import json
import sys

json_path, serial_log, controller_log = sys.argv[1:4]
with open(json_path, encoding="utf-8") as f:
    payload = json.load(f)

def digest(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        h.update(f.read())
    return h.hexdigest()

assert payload["schema_version"] == 1
assert payload["evidence_kind"] == "hil_appliance_run_v0"
assert payload["evidence_level"] == "PASS/HIL-LOG"
assert payload["run_id"] == "hil_appliance_gate_serial_observer"
assert payload["serial_input_kind"] == "development_log"
assert payload["serial_log_sha256"] == digest(serial_log)
assert payload["controller_log_sha256"] == digest(controller_log)
markers = payload["serial_markers_observed"]
assert "RAMEN OS" in markers
assert "golden_machine: hil_boot ok" in markers
assert "persistent_storage: nvme_boot ok" in markers
assert payload["target_hil_evidence_markers"]["git_sha"] == "fixture"
assert payload["target_hil_evidence_markers"]["boot_epoch_nonce"] == "fixture"
PY

if RAMEN_HIL_GRADUATION=1 \
  RAMEN_HIL_RUN_ID=hil_appliance_gate_stale_negative \
  RAMEN_HIL_EVIDENCE_DIR="$SERIAL_OBSERVER_EVIDENCE" \
  RAMEN_HIL_SERIAL_LOG="$SERIAL_OBSERVER_FIXTURE" \
  bash "$SERIAL_CAPTURE_SCRIPT" >"$SERIAL_OBSERVER_NEGATIVE_STDOUT" 2>&1; then
  fail "SERIAL_OBSERVER_STALE_LOG_NEGATIVE_MISSED" "graduation mode must reject RAMEN_HIL_SERIAL_LOG"
fi

if RAMEN_HIL_RUN_ID='../bad' \
  RAMEN_HIL_EVIDENCE_DIR="$SERIAL_OBSERVER_EVIDENCE" \
  RAMEN_HIL_SERIAL_LOG="$SERIAL_OBSERVER_FIXTURE" \
  bash "$SERIAL_CAPTURE_SCRIPT" >"$SERIAL_OBSERVER_NEGATIVE_STDOUT" 2>&1; then
  fail "SERIAL_OBSERVER_BAD_RUN_ID_NEGATIVE_MISSED" "serial observer must reject unsafe run ids"
fi

: >"$SERIAL_OBSERVER_EMPTY_LOG"
if RAMEN_HIL_RUN_ID=hil_appliance_gate_empty_negative \
  RAMEN_HIL_EVIDENCE_DIR="$SERIAL_OBSERVER_EVIDENCE" \
  RAMEN_HIL_SERIAL_LOG="$SERIAL_OBSERVER_EMPTY_LOG" \
  bash "$SERIAL_CAPTURE_SCRIPT" >"$SERIAL_OBSERVER_NEGATIVE_STDOUT" 2>&1; then
  fail "SERIAL_OBSERVER_EMPTY_LOG_NEGATIVE_MISSED" "serial observer must reject empty transcripts"
fi

rm -rf "$SERIAL_OBSERVER_TMP"
echo "$GATE_ID: METRIC serial_observer_contract=pass"
)


echo "$GATE_ID: INFO step=default_ci_policy"
if [[ "${RAMEN_HIL_APPLIANCE:-}" != "1" ]]; then
  if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" || "${RAMEN_HIL_GOLDEN_MACHINE:-}" == "1" ]]; then
    fail "APPLIANCE_REQUIRED" "live/graduation HIL requires RAMEN_HIL_APPLIANCE=1"
  fi
  skip_appliance "RAMEN_HIL_APPLIANCE not set"
  echo "$GATE_ID: PASS/QEMU docs_and_manifest_only"
  echo "$GATE_ID: ok"
  exit 0
fi

echo "$GATE_ID: INFO step=appliance_inventory"

if [[ "${RAMEN_HIL_GRADUATION:-}" == "1" && -n "${RAMEN_HIL_SERIAL_LOG:-}" ]]; then
  fail "STALE_LOG_IN_GRADUATION" "RAMEN_HIL_GRADUATION=1 forbids RAMEN_HIL_SERIAL_LOG"
fi

SERIAL_DEV="${RAMEN_HIL_SERIAL_DEV:-}"
[[ -n "$SERIAL_DEV" ]] || fail "SERIAL_DEV_UNSET" "set RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 for appliance inventory"
[[ -e "$SERIAL_DEV" ]] || fail "SERIAL_DEV_MISSING" "serial device not found: $SERIAL_DEV"

if command -v stty >/dev/null 2>&1; then
  stty -F "$SERIAL_DEV" 115200 raw -echo 2>/dev/null \
    || stty -f "$SERIAL_DEV" 115200 raw -echo 2>/dev/null \
    || fail "SERIAL_STTY_FAILED" "failed to configure serial device: $SERIAL_DEV"
fi

# Inventory is not the completion signal: acquire an actual transcript.
export RAMEN_HIL_APPLIANCE_ID="${RAMEN_HIL_APPLIANCE_ID:-pi-hil-01}"
export RAMEN_HIL_TARGET_ID="${RAMEN_HIL_TARGET_ID:-${RAMEN_HIL_MACHINE_ID:-lenovo-thinkcentre-m900-i7-6700-lab-01}}"
bash "$SERIAL_CAPTURE_SCRIPT"
echo "$GATE_ID: PASS/HIL-APPLIANCE serial_observer"
echo "$GATE_ID: ok"
