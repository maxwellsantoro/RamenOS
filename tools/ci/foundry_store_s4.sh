#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
S4_DIR="$OUT_DIR/s4_test"
INSTALLED_ROOT="$OUT_DIR/installed"
STORE_SOCKET="$S4_DIR/store.sock"
STORE_LOG="$S4_DIR/store_service.log"

mkdir -p "$S4_DIR" "$INSTALLED_ROOT/artifacts"
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

# Create test evidence artifacts (minimal valid traces)
TRACE_JSON="$S4_DIR/test_trace.json"
cat > "$TRACE_JSON" <<'EOF'
{
  "schema_version": 1,
  "trace_type": "scenario_trace",
  "scenario_trace": {
    "metadata": { "scenario_id": "test.scenario.v0" },
    "events": [
      { "seq": 1, "name": "start", "payload": {} }
    ]
  }
}
EOF

# Ingest the trace to get a content ID
trace_id=$(cargo run -p store_cli -- ingest --src "$TRACE_JSON" --installed-root "$INSTALLED_ROOT" --kind "scenario_trace" --store-socket "$STORE_SOCKET")
echo "S4 test trace_id=$trace_id"

# Create queue item 1: app.one with clipboard prereq
ITEM1="$S4_DIR/queue_item_1.json"
cat > "$ITEM1" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.app_one",
  "target_level": "posix",
  "evidence": {
    "scenario_traces": ["$trace_id"],
    "observed_caps": null,
    "protocol_traces": []
  },
  "prereqs": [
    { "kind": "portal", "name": "clipboard" }
  ],
  "scoring": {
    "vote_weight": 3,
    "leverage": 4,
    "reuse": 2,
    "effort": 4,
    "risk": 2
  },
  "priority": 3.0,
  "explanation": []
}
EOF

# Create queue item 2: app.two with clipboard + audio prereqs
ITEM2="$S4_DIR/queue_item_2.json"
cat > "$ITEM2" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.app_two",
  "target_level": "native",
  "evidence": {
    "scenario_traces": ["$trace_id"],
    "observed_caps": null,
    "protocol_traces": []
  },
  "prereqs": [
    { "kind": "portal", "name": "clipboard" },
    { "kind": "harness", "name": "audio.capture" }
  ],
  "scoring": {
    "vote_weight": 2,
    "leverage": 5,
    "reuse": 3,
    "effort": 5,
    "risk": 3
  },
  "priority": 2.0,
  "explanation": []
}
EOF

# Validate queue items
cargo run -p store_cli -- validate-queue-item --src "$ITEM1"
cargo run -p store_cli -- validate-queue-item --src "$ITEM2"
echo "FOUNDRY_STORE_S4: queue item validation ok"

# Explain priority
explain_out=$(cargo run -p store_cli -- explain-priority --src "$ITEM1" 2>&1)
echo "$explain_out" | grep -q "Priority:"
echo "$explain_out" | grep -q "Vote weight"
echo "FOUNDRY_STORE_S4: explain priority ok"

# Generate prerequisites graph
GRAPH_JSON="$S4_DIR/prereq_graph.json"
GRAPH_DOT="$S4_DIR/prereq_graph.dot"
graph_out=$(cargo run -p store_cli -- prereq-graph \
  --src "$ITEM1" --src "$ITEM2" \
  --out "$GRAPH_JSON" \
  --dot "$GRAPH_DOT" 2>&1)

# Verify graph was created
[[ -f "$GRAPH_JSON" ]] || { echo "prereq graph JSON missing"; exit 2; }
[[ -f "$GRAPH_DOT" ]] || { echo "prereq graph DOT missing"; exit 2; }

# Verify graph contains expected nodes and edges
GRAPH_JSON_CONTENT=$(cat "$GRAPH_JSON")
echo "$GRAPH_JSON_CONTENT" | grep -q "org.test.app_one"
echo "$GRAPH_JSON_CONTENT" | grep -q "org.test.app_two"
echo "$GRAPH_JSON_CONTENT" | grep -q "prereq:portal:clipboard"
echo "$GRAPH_JSON_CONTENT" | grep -q "prereq:harness:audio.capture"

# Verify high-leverage detection (clipboard blocks both apps)
echo "$graph_out" | grep -q "clipboard"
echo "$graph_out" | grep -q "blocks 2 items"
echo "FOUNDRY_STORE_S4: prereq graph ok"

# Ingest queue item to get content ID for claim
item1_id=$(cargo run -p store_cli -- ingest --src "$ITEM1" --installed-root "$INSTALLED_ROOT" --kind "queue_item" --store-socket "$STORE_SOCKET")
echo "S4 item1_id=$item1_id"

# Create and validate claim
CLAIM="$S4_DIR/claim.json"
cargo run -p store_cli -- claim \
  --item "$item1_id" \
  --claimant "test@example.com" \
  --lease-secs 604800 \
  --out "$CLAIM"

cargo run -p store_cli -- validate-claim --src "$CLAIM"

# Verify claim content
CLAIM_CONTENT=$(cat "$CLAIM")
echo "$CLAIM_CONTENT" | grep -q "$item1_id"
echo "$CLAIM_CONTENT" | grep -q "test@example.com"
echo "$CLAIM_CONTENT" | grep -q "604800"
echo "FOUNDRY_STORE_S4: claim workflow ok"

# Resolve claim chain: latest valid claim wins; expired claims ignored.
CLAIM_OLD="$S4_DIR/claim_old.json"
cat > "$CLAIM_OLD" <<EOF
{
  "schema_version": 1,
  "queue_item_id": "$item1_id",
  "claimant_id": "old@example.com",
  "timestamp": "2026-02-05T10:00:00Z",
  "lease_duration_secs": 3600
}
EOF

CLAIM_NEW="$S4_DIR/claim_new.json"
cat > "$CLAIM_NEW" <<EOF
{
  "schema_version": 1,
  "queue_item_id": "$item1_id",
  "claimant_id": "new@example.com",
  "timestamp": "2026-02-05T12:00:00Z",
  "lease_duration_secs": 86400
}
EOF

resolve_out=$(cargo run -p store_cli -- resolve-claim \
  --src "$CLAIM_OLD" \
  --src "$CLAIM_NEW" \
  --now "2026-02-05T13:00:00Z" 2>&1)
echo "$resolve_out" | grep -q "claim winner"
echo "$resolve_out" | grep -q "new@example.com"

resolve_expired_out=$(cargo run -p store_cli -- resolve-claim \
  --src "$CLAIM_OLD" \
  --now "2026-02-06T13:00:00Z" 2>&1)
echo "$resolve_expired_out" | grep -q "no active claim"
echo "FOUNDRY_STORE_S4: claim resolution ok"

# Test validation failures
echo "Testing negative assertions..."

# Bad priority should fail validation
BAD_ITEM="$S4_DIR/bad_queue_item.json"
cat > "$BAD_ITEM" <<EOF
{
  "schema_version": 1,
  "program_id": "org.test.bad",
  "target_level": "posix",
  "evidence": {
    "scenario_traces": ["$trace_id"]
  },
  "prereqs": [],
  "scoring": {
    "vote_weight": 3,
    "leverage": 4,
    "reuse": 2,
    "effort": 4,
    "risk": 2
  },
  "priority": 999.0,
  "explanation": []
}
EOF

if cargo run -p store_cli -- validate-queue-item --src "$BAD_ITEM" 2>/dev/null; then
  echo "bad priority should have failed validation" >&2
  exit 2
fi
echo "FOUNDRY_STORE_S4: negative assertions ok"

echo "FOUNDRY_STORE_S4: ok"
