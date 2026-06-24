#!/usr/bin/env bash
# Foundry gate: IDL protocol IDs and direct IPC wire types must be canonical.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

python3 tools/ci/idl_lint.py
