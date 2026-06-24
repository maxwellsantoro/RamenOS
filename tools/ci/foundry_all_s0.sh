#!/usr/bin/env bash
set -euxo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "--- Running boot gate (S0) ---"
"$ROOT_DIR/tools/ci/foundry_s0.sh"
echo "--- Boot gate passed ---"

echo "--- Running store gate (S0) ---"
"$ROOT_DIR/tools/ci/foundry_store_s0.sh"
echo "--- Store gate passed ---"

echo "FOUNDRY_ALL_S0: ok"
