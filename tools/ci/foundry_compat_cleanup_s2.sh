#!/usr/bin/env bash
# Actual host process regressions; this companion does not boot a VM.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
mkdir -p out
EVIDENCE="$(mktemp -d "$ROOT_DIR/out/compat-s2-cleanup.XXXXXX")"
echo "FOUNDRY_COMPAT_CLEANUP_S2: evidence_dir=$EVIDENCE"

# Fail before building or launching a fixture when safe ownership is unavailable.
python3 - <<'PY'
import os, signal, sys
if not sys.platform.startswith("linux") or not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
    sys.exit("FOUNDRY_COMPAT_CLEANUP_S2: INCOMPLETE (Linux pidfd required)")
try:
    fd = os.pidfd_open(os.getpid(), 0)
    try:
        signal.pidfd_send_signal(fd, 0)
    finally:
        os.close(fd)
except OSError as error:
    sys.exit(f"FOUNDRY_COMPAT_CLEANUP_S2: INCOMPLETE ({error})")
PY

python3 tools/ci/test_cargo_artifact.py -v >"$EVIDENCE/selector.log" 2>&1
cargo build --locked -p store_service --bin store_service -p store_cli --bin store_cli \
  -p runtime_supervisor --bin runtime_supervisor --message-format=json \
  >"$EVIDENCE/build.jsonl" 2>"$EVIDENCE/build.stderr.log"
STORE_BIN="$(python3 tools/ci/cargo_artifact.py "$EVIDENCE/build.jsonl" store_service)"
CLI_BIN="$(python3 tools/ci/cargo_artifact.py "$EVIDENCE/build.jsonl" store_cli)"
SUPERVISOR_BIN="$(python3 tools/ci/cargo_artifact.py "$EVIDENCE/build.jsonl" runtime_supervisor)"
python3 tools/ci/test_compat_s2_cleanup.py --store-bin "$STORE_BIN" --cli-bin "$CLI_BIN" \
  --supervisor-bin "$SUPERVISOR_BIN" --evidence "$EVIDENCE/processes" \
  >"$EVIDENCE/processes.log" 2>&1
python3 - "$EVIDENCE" "$STORE_BIN" "$CLI_BIN" "$SUPERVISOR_BIN" <<'PY'
import hashlib, json, platform, subprocess, sys
from pathlib import Path

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate field")
        value[key] = item
    return value

def require(condition, reason):
    if not condition:
        raise ValueError(reason)

evidence = Path(sys.argv[1])
report = json.loads((evidence / "processes/result.json").read_text(), object_pairs_hook=unique)
expected = {"normal": 0, "timeout": 1, "interruption": 143, "build_failure": 1,
            "unprovable_shutdown": 1, "missing_first": 1, "missing_second": 1}
require(report["passed"] is True and report["vm_executed"] is False, "failed or wrong-scope fixture")
require(report["fixture"] == "ordinary_host_process_not_vm", "unknown fixture scope")
require(len(report["cases"]) == 7 and {c["name"] for c in report["cases"]} == set(expected), "incomplete case inventory")
for case in report["cases"]:
    require(type(case["gate_exit"]) is int and case["gate_exit"] == expected[case["name"]], "incorrect exit status")
    require(case["failures"] == [] and case["unrelated_survived"] is True, "failed cleanup or unrelated-process check")
    unknown = case["name"] == "unprovable_shutdown"
    require(case["expected_cleanup_unknown"] is unknown, "incorrect UNKNOWN classification")
    require(case["child_remained_after_gate"] is unknown, "incorrect owned-child outcome")
binaries = dict(zip(("store_service", "store_cli", "runtime_supervisor"), map(Path, sys.argv[2:])))
require(report["binary_sha256"] == {name: sha(path) for name, path in binaries.items()}, "stale binary evidence")
sources = ("runtime_supervisor/Cargo.toml", "runtime_supervisor/src/main.rs", "Cargo.lock", "justfile",
           "tools/ci/foundry_ci_extended.sh", "tools/ci/foundry_compat_cleanup_s2.sh",
           "tools/ci/foundry_compat_s2.sh", "tools/ci/foundry_store_s0.sh", "tools/ci/cargo_artifact.py",
           "tools/ci/test_cargo_artifact.py", "tools/ci/test_compat_s2_cleanup.py",
           "services/native_runner/src/generated/mod.rs",
           "services/native_runner/src/generated/harness_echo_v1_host.rs",
           "services/native_runner/src/generated/harness_trace_v2_host.rs",
           "services/native_runner/src/generated/harness_shmem_control_v1_host.rs",
           "services/native_runner/src/generated/services_semantic_state_v1_host.rs")
result = dict(schema_version=1, gate="foundry-compat-cleanup-s2", outcome="PASS",
              scope="actual-host-process-cleanup", host=platform.platform(), vm_executed=False,
              source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              dirty_diff_sha256=hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD", "--binary"])).hexdigest(),
              source_sha256={p: sha(Path(p)) for p in sources}, binary_sha256=report["binary_sha256"],
              cases=report["cases"], artifact_sha256={str(p.relative_to(evidence)): sha(p)
                  for p in sorted(evidence.rglob("*")) if p.is_file()})
(evidence / "result.json").write_text(json.dumps(result, indent=2) + "\n")
PY
echo "FOUNDRY_COMPAT_CLEANUP_S2: PASS"
