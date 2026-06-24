#!/usr/bin/env bash
# Workspace lint baseline gate (strict by default).
#
# Goals:
# - Run clippy across the host workspace in a deterministic way.
# - Emit machine-readable warning metrics.
# - Fail closed on any warning unless explicitly overridden.
#
# Usage:
#   tools/ci/foundry_lint_baseline.sh
#   LINT_ALLOW_WARNINGS=1 tools/ci/foundry_lint_baseline.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/out/foundry/lint"
LOG_FILE="$OUT_DIR/clippy.log"

mkdir -p "$OUT_DIR"
rm -f "$LOG_FILE"

echo "FOUNDRY_LINT_BASELINE: START"
echo "FOUNDRY_LINT_BASELINE: INFO running cargo clippy workspace host targets"

cd "$ROOT_DIR"

# Keep scope aligned with existing host-only clippy workflow.
cargo clippy \
  --workspace \
  --all-targets \
  --exclude kernel_uefi \
  --exclude kernel_aarch64 \
  -- -W clippy::all \
  2>&1 | tee "$LOG_FILE"

warning_count="$(
  { grep '^warning: ' "$LOG_FILE" 2>/dev/null || true; } \
    | { grep -v ' generated .* warnings$' || true; } \
    | { grep -v '^warning: profiles for' || true; } \
    | wc -l \
    | tr -d ' '
)"
warning_count="${warning_count:-0}"

echo "FOUNDRY_LINT_BASELINE: METRIC warning_count=${warning_count}"
echo "FOUNDRY_LINT_BASELINE: INFO log_path=${LOG_FILE}"

allow_warnings="${LINT_ALLOW_WARNINGS:-0}"
if [[ "$allow_warnings" != "1" ]] && [[ "$warning_count" -gt 0 ]]; then
  echo "FOUNDRY_LINT_BASELINE: FAIL code=warnings_present detail=warning_count=${warning_count}"
  exit 1
fi

echo "FOUNDRY_LINT_BASELINE: PASS"
