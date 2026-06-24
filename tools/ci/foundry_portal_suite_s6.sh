#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
TRACE_DIR="$OUT_DIR/trace"
INSTALLED_ROOT="$OUT_DIR/installed"
S6_DIR="$OUT_DIR/portal_suite_s6"
STORE_SOCKET="$S6_DIR/store.sock"
STORE_LOG="$S6_DIR/store_service.log"

mkdir -p "$TRACE_DIR" "$INSTALLED_ROOT/artifacts" "$S6_DIR"
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
  --in "$ROOT_DIR/idl/portals/clipboard_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/portal_clipboard.generated.rs"
cargo run -p idl_codegen -- \
  --in "$ROOT_DIR/idl/portals/notifications_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/portal_notifications.generated.rs"
cargo run -p idl_codegen -- \
  --in "$ROOT_DIR/idl/portals/screen_capture_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/portal_screen_capture.generated.rs"

run_out=$(cargo run -p portals --bin portal_suite -- \
  --trace-dir "$TRACE_DIR" \
  --store-socket "$STORE_SOCKET")

echo "$run_out" | grep -q "PORTAL_SUITE: clipboard ok"
echo "$run_out" | grep -q "PORTAL_SUITE: notifications ok"
echo "$run_out" | grep -q "PORTAL_SUITE: screen_capture ok"
echo "$run_out" | grep -q "PORTAL_SUITE: ok"

read -r clip_trace clip_obs clip_scenario <<<"$(echo "$run_out" | sed -E -n 's/^PORTAL_SUITE: clipboard ok trace=(sha256:[0-9a-f]+) observed=(sha256:[0-9a-f]+) scenario=(sha256:[0-9a-f]+).*/\1 \2 \3/p' | tail -n 1)"
read -r notif_trace notif_obs notif_scenario <<<"$(echo "$run_out" | sed -E -n 's/^PORTAL_SUITE: notifications ok trace=(sha256:[0-9a-f]+) observed=(sha256:[0-9a-f]+) scenario=(sha256:[0-9a-f]+).*/\1 \2 \3/p' | tail -n 1)"
read -r screen_trace screen_obs screen_scenario <<<"$(echo "$run_out" | sed -E -n 's/^PORTAL_SUITE: screen_capture ok trace=(sha256:[0-9a-f]+) observed=(sha256:[0-9a-f]+) scenario=(sha256:[0-9a-f]+).*/\1 \2 \3/p' | tail -n 1)"

for id in "$clip_trace" "$clip_obs" "$clip_scenario" \
          "$notif_trace" "$notif_obs" "$notif_scenario" \
          "$screen_trace" "$screen_obs" "$screen_scenario"; do
  [[ -n "$id" ]]
  blob="$INSTALLED_ROOT/artifacts/${id#sha256:}.blob"
  manifest="$INSTALLED_ROOT/artifacts/${id#sha256:}.manifest.json"
  [[ -f "$blob" ]]
  [[ -f "$manifest" ]]
done

cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_clipboard.json"
cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_notifications.json"
cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_screen_capture.json"

cargo run -p store_cli -- validate-observed-caps --src "$TRACE_DIR/portal_clipboard_observed.json"
cargo run -p store_cli -- validate-observed-caps --src "$TRACE_DIR/portal_notifications_observed.json"
cargo run -p store_cli -- validate-observed-caps --src "$TRACE_DIR/portal_screen_capture_observed.json"

cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_clipboard_scenario.json"
cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_notifications_scenario.json"
cargo run -p store_cli -- validate-trace --src "$TRACE_DIR/portal_screen_capture_scenario.json"

python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$INSTALLED_ROOT/artifacts/${clip_trace#sha256:}.blob"
python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$INSTALLED_ROOT/artifacts/${notif_trace#sha256:}.blob"
python3 "$ROOT_DIR/tools/trace/replay_protocol_trace.py" --trace "$INSTALLED_ROOT/artifacts/${screen_trace#sha256:}.blob"

ROOT_DIR="$ROOT_DIR" python3 - <<'PY'
import json
import os
import pathlib
import sys

root = pathlib.Path(os.environ["ROOT_DIR"]) / "out" / "trace"

checks = [
    ("portal_clipboard_observed.json", "portal.clipboard"),
    ("portal_notifications_observed.json", "portal.notifications"),
    ("portal_screen_capture_observed.json", "portal.screen_capture"),
]
for name, cap in checks:
    data = json.loads((root / name).read_text())
    caps = data.get("capabilities", [])
    if not any(c.get("cap") == cap for c in caps):
        sys.exit(f"missing cap {cap} in {name}")
PY

echo "FOUNDRY_PORTAL_SUITE_S6: ok"
