#!/usr/bin/env bash
set -euxo pipefail

# S5 Foundry Gate: Port It Now Wizard
# Tests crash context, graduation tracking, minimal policy proposal
# Includes: runner identity, evidence bundle, exit metrics, deterministic output

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${ROOT_DIR}/out"
S5_DIR="${OUT_DIR}/s5_test"
INSTALLED_ROOT="${OUT_DIR}/installed"
STORE_SOCKET="${S5_DIR}/store.sock"
STORE_LOG="${S5_DIR}/store_service.log"

mkdir -p "${S5_DIR}" "${INSTALLED_ROOT}/artifacts"
rm -f "${STORE_SOCKET}"

RAMEN_STORE_DEV_MODE=1 \
RAMEN_STORE_ACCESS_POLICY=AllowAll \
RAMEN_STORE_SOCKET="${STORE_SOCKET}" \
RAMEN_STORE_ROOT="${INSTALLED_ROOT}/artifacts" \
cargo run -p store_service >"${STORE_LOG}" 2>&1 &
STORE_PID=$!

cleanup() {
  kill "${STORE_PID}" >/dev/null 2>&1 || true
  wait "${STORE_PID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in $(seq 1 100); do
  if [[ -S "${STORE_SOCKET}" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -S "${STORE_SOCKET}" ]]; then
  echo "store_service socket not ready: ${STORE_SOCKET}"
  cat "${STORE_LOG}"
  exit 1
fi

# --- Test 1: Crash Context validation (with runner + evidence bundle + exit metrics) ---
echo "Testing crash context..."

CRASH_CTX="${S5_DIR}/crash_context.json"
cat > "${CRASH_CTX}" <<EOF
{
  "schema_version": 1,
  "component_id": "org.test.app",
  "run_id": "run-001",
  "crash_timestamp": "2026-02-05T12:00:00Z",
  "exit_reason": {
    "signal": {
      "signal": 11,
      "name": "SIGSEGV"
    }
  },
  "target_level": "posix",
  "evidence": ["sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abcd"],
  "evidence_bundle": {
    "stdout_tail_ref": "sha256:stdout_aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444",
    "stderr_tail_ref": "sha256:stderr_aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444",
    "runner_log_ref": "sha256:runlog_aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444",
    "scenario_trace_ref": "sha256:trace__aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444",
    "extras": [
      {"key": "custom_diag", "ref_id": "sha256:custom_aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444"}
    ]
  },
  "runner": {
    "kind": "compat_vm",
    "version": "0.0.12",
    "build_hash": "sha256:build_hash_placeholder_abc123def456abc123def456abc123def456abc123"
  },
  "exit_metrics": {
    "wall_budget_ms": 30000,
    "wall_elapsed_ms": 12500,
    "memory_budget_bytes": 536870912,
    "memory_peak_bytes": 536870912,
    "oom_events_ref": "sha256:oom_ev_aaa111bbb222ccc333ddd444eee555fff666aaa111bbb222ccc333ddd444"
  },
  "summary": "Crashed during POSIX personality attempt"
}
EOF

cargo run -p store_cli -- validate-crash-context --src "${CRASH_CTX}"
echo "FOUNDRY_STORE_S5: crash context validation ok"

# --- Test 1b: Backward compat — old crash context without new fields ---
echo "Testing crash context backward compat..."

CRASH_CTX_OLD="${S5_DIR}/crash_context_old.json"
cat > "${CRASH_CTX_OLD}" <<EOF
{
  "schema_version": 1,
  "component_id": "org.test.legacy",
  "run_id": "run-legacy",
  "crash_timestamp": "2026-02-05T12:00:00Z",
  "exit_reason": "oom"
}
EOF

cargo run -p store_cli -- validate-crash-context --src "${CRASH_CTX_OLD}"
echo "FOUNDRY_STORE_S5: crash context backward compat ok"

# --- Test 2: Graduation tracking (with runner on attempt + JSON-load fix) ---
echo "Testing graduation tracking..."

GRADUATION="${S5_DIR}/graduation.json"
cat > "${GRADUATION}" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.app",
  "current_level": "compat",
  "target_level": "native",
  "attempts": [
    {
      "target_level": "compat",
      "attempt_number": 1,
      "timestamp": "2026-02-05T10:00:00Z",
      "result": "success",
      "scenario_trace_ref": "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abcd",
      "runner": {
        "kind": "compat_vm",
        "version": "0.0.11"
      }
    },
    {
      "target_level": "posix",
      "attempt_number": 1,
      "timestamp": "2026-02-05T11:00:00Z",
      "result": "crashed",
      "crash_context_ref": "sha256:def456abc123def456abc123def456abc123def456abc123def456abc123defg",
      "runner": {
        "kind": "posix_runner",
        "version": "0.0.11",
        "build_hash": "sha256:build_posix_abc123def456abc123def456abc123def456abc123def456ab"
      }
    },
    {
      "target_level": "posix",
      "attempt_number": 2,
      "timestamp": "2026-02-05T12:00:00Z",
      "result": "success",
      "scenario_trace_ref": "sha256:fed987cba654fed987cba654fed987cba654fed987cba654fed987cba654fedg",
      "runner": {
        "kind": "posix_runner",
        "version": "0.0.12"
      }
    }
  ],
  "level_status": []
}
EOF

cargo run -p store_cli -- validate-graduation --src "${GRADUATION}"

# Verify graduation status command shows progression (JSON-load fix)
graduation_out=$(cargo run -p store_cli -- graduation-status --src "${GRADUATION}" 2>&1)
echo "${graduation_out}"

# After update_status(), current_level should be posix (best achieved)
echo "${graduation_out}" | grep -q "Current level: posix"

# Progression summary should be non-empty and contain level markers
echo "${graduation_out}" | grep -q "Progression:"
echo "${graduation_out}" | grep -q "compat"
echo "${graduation_out}" | grep -q "posix"

echo "FOUNDRY_STORE_S5: graduation tracking ok"

# --- Test 3: Minimal policy validation (with proposer_version) ---
echo "Testing minimal policy..."

MINIMAL_POLICY="${S5_DIR}/minimal_policy.json"
cat > "${MINIMAL_POLICY}" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.app",
  "target_level": "posix",
  "capabilities": [
    {
      "cap": "portal.file_picker.ro",
      "scope": "prompt",
      "reason": "observed 5 uses",
      "observed_use_count": 5,
      "required": true
    },
    {
      "cap": "portal.clipboard",
      "scope": "once_prompt",
      "reason": "observed 2 uses",
      "observed_use_count": 2,
      "required": false
    }
  ],
  "excluded": [
    {
      "cap": "unused_cap",
      "reason": "granted but never used"
    }
  ],
  "strictness_score": 62,
  "proposer_version": "0.1.0"
}
EOF

cargo run -p store_cli -- validate-minimal-policy --src "${MINIMAL_POLICY}"
echo "FOUNDRY_STORE_S5: minimal policy validation ok"

# --- Test 4: Policy proposal from observed caps (deterministic output) ---
echo "Testing policy proposal..."

OBSERVED_CAPS="${S5_DIR}/observed_caps_for_policy.json"
cat > "${OBSERVED_CAPS}" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.app",
  "run_id": "run-002",
  "capabilities": [
    {
      "cap": "portal.file_picker.ro",
      "scope": { "artifact_ids": [] },
      "counts": { "granted": 5, "used": 5 },
      "evidence": []
    },
    {
      "cap": "portal.clipboard",
      "scope": { "artifact_ids": [] },
      "counts": { "granted": 2, "used": 2 },
      "evidence": []
    },
    {
      "cap": "network.tcp",
      "scope": { "artifact_ids": [] },
      "counts": { "granted": 10, "used": 10 },
      "evidence": []
    },
    {
      "cap": "unused_cap",
      "scope": { "artifact_ids": [] },
      "counts": { "granted": 1, "used": 0 },
      "evidence": []
    }
  ],
  "evidence": []
}
EOF

PROPOSED_POLICY="${S5_DIR}/proposed_policy.json"
policy_out=$(cargo run -p store_cli -- propose-policy \
  --program-id org.test.app \
  --target-level posix \
  --observed-caps "${OBSERVED_CAPS}" \
  --out "${PROPOSED_POLICY}" \
  --store-socket "${STORE_SOCKET}" 2>&1)
echo "${policy_out}"
echo "${policy_out}" | grep -q "minimal policy written"

# Verify proposed policy has expected structure
PROPOSED_CONTENT=$(cat "${PROPOSED_POLICY}")
echo "${PROPOSED_CONTENT}" | grep -q "portal.file_picker.ro"
echo "${PROPOSED_CONTENT}" | grep -q "prompt"
echo "${PROPOSED_CONTENT}" | grep -q "excluded"
echo "${PROPOSED_CONTENT}" | grep -q "proposer_version"

# --- Test 4b: Deterministic output — same inputs produce identical artifact ---
echo "Testing deterministic policy output..."

PROPOSED_POLICY_2="${S5_DIR}/proposed_policy_2.json"
cargo run -p store_cli -- propose-policy \
  --program-id org.test.app \
  --target-level posix \
  --observed-caps "${OBSERVED_CAPS}" \
  --out "${PROPOSED_POLICY_2}" \
  --store-socket "${STORE_SOCKET}" 2>&1

# Compare the two policy artifacts (strip observed_caps_ref since content IDs
# are stable when the file doesn't change, but compare everything else)
diff "${PROPOSED_POLICY}" "${PROPOSED_POLICY_2}"
echo "FOUNDRY_STORE_S5: deterministic policy output ok"

echo "FOUNDRY_STORE_S5: policy proposal ok"

# --- Test 5: Negative assertions ---
echo "Testing negative assertions..."

# Bad crash context (missing component_id)
BAD_CRASH="${S5_DIR}/bad_crash.json"
cat > "${BAD_CRASH}" <<EOF
{
  "schema_version": 1,
  "component_id": "",
  "run_id": "run-001",
  "crash_timestamp": "2026-02-05T12:00:00Z",
  "exit_reason": { "unknown": null }
}
EOF

if cargo run -p store_cli -- validate-crash-context --src "${BAD_CRASH}" 2>&1; then
  echo "ERROR: should have rejected bad crash context"
  exit 1
fi

# Bad crash context (bad evidence bundle ref)
BAD_CRASH_BUNDLE="${S5_DIR}/bad_crash_bundle.json"
cat > "${BAD_CRASH_BUNDLE}" <<EOF
{
  "schema_version": 1,
  "component_id": "app",
  "run_id": "run-001",
  "crash_timestamp": "2026-02-05T12:00:00Z",
  "exit_reason": "oom",
  "evidence_bundle": {
    "stdout_tail_ref": "bad-ref-not-sha256"
  }
}
EOF

if cargo run -p store_cli -- validate-crash-context --src "${BAD_CRASH_BUNDLE}" 2>&1; then
  echo "ERROR: should have rejected bad evidence bundle ref"
  exit 1
fi

# Bad crash context (empty runner kind)
BAD_CRASH_RUNNER="${S5_DIR}/bad_crash_runner.json"
cat > "${BAD_CRASH_RUNNER}" <<EOF
{
  "schema_version": 1,
  "component_id": "app",
  "run_id": "run-001",
  "crash_timestamp": "2026-02-05T12:00:00Z",
  "exit_reason": "timeout",
  "runner": {
    "kind": ""
  }
}
EOF

if cargo run -p store_cli -- validate-crash-context --src "${BAD_CRASH_RUNNER}" 2>&1; then
  echo "ERROR: should have rejected empty runner kind"
  exit 1
fi

# Bad graduation (bad content ref)
BAD_GRAD="${S5_DIR}/bad_graduation.json"
cat > "${BAD_GRAD}" <<EOF
{
  "schema_version": 1,
  "program_id": "app",
  "current_level": "compat",
  "target_level": "native",
  "attempts": [],
  "queue_item_ref": "bad-ref"
}
EOF

if cargo run -p store_cli -- validate-graduation --src "${BAD_GRAD}" 2>&1; then
  echo "ERROR: should have rejected bad graduation"
  exit 1
fi

echo "FOUNDRY_STORE_S5: negative assertions ok"

echo "FOUNDRY_STORE_S5: ok"
