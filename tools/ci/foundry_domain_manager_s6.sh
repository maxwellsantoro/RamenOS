#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out"
S6_DIR="$OUT_DIR/domain_manager_s6"
INSTALLED_ROOT="$OUT_DIR/installed"
STORE_SOCKET="$S6_DIR/store.sock"
STORE_LOG="$S6_DIR/store_service.log"

mkdir -p "$S6_DIR" "$OUT_DIR/trace" "$INSTALLED_ROOT/artifacts"
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
  --in "$ROOT_DIR/idl/harness/domain_manager_v1.toml" \
  --out "$ROOT_DIR/kernel_api/src/generated/domain_manager_v1.generated.rs"

run_out=$(cargo run -p domain_manager -- \
  --trace-dir "$OUT_DIR/trace" \
  --installed-root "$INSTALLED_ROOT" \
  --store-socket "$STORE_SOCKET")

echo "$run_out" | grep -q "DOMAIN_MANAGER: start ok domain=100 generation=1"
echo "$run_out" | grep -q "DOMAIN_MANAGER: restart policy triggered domain=100 action=2 generation=2 restarts=1"
echo "$run_out" | grep -q "DOMAIN_MANAGER: list total=2 running=1 restarting=0 stopped=1"
echo "$run_out" | grep -q "DOMAIN_MANAGER: stop ok domain=100 generation=2"
echo "$run_out" | grep -q "DOMAIN_MANAGER: lifecycle api ok"
echo "$run_out" | grep -q "DOMAIN_MANAGER: restart policy ok"
echo "$run_out" | grep -q "DOMAIN_MANAGER: multi-domain ok"
echo "$run_out" | grep -q "DOMAIN_MANAGER: ok"

echo "FOUNDRY_DOMAIN_MANAGER_S6: ok"
