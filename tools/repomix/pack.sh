#!/usr/bin/env bash
# Pack RamenOS into review-sized Repomix artifacts.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PROFILE="${1:-map}"
case "$PROFILE" in
  map|default)
    CONFIG="repomix.config.json"
    ;;
  contracts|kernel|store|desktop)
    CONFIG="tools/repomix/repomix.config.${PROFILE}.json"
    ;;
  all)
    failed=0
    for p in map contracts kernel store desktop; do
      if ! "$0" "$p"; then
        failed=1
      fi
    done
    echo "All review packs attempted (repomix-output.*.xml)."
    exit "$failed"
    ;;
  *)
    echo "usage: $0 [map|contracts|kernel|store|desktop|all]" >&2
    exit 2
    ;;
esac

if ! command -v repomix >/dev/null 2>&1; then
  echo "repomix not found on PATH" >&2
  exit 127
fi

echo "Packing profile=${PROFILE} config=${CONFIG}"
set +e
repomix --config "$CONFIG"
status=$?
set -e

out="$(python3 -c 'import json,sys; from pathlib import Path; print(json.loads(Path(sys.argv[1]).read_text())["output"]["filePath"])' "$CONFIG")"
if [[ -f "$out" ]]; then
  python3 -c 'import os,sys; n=os.path.getsize(sys.argv[1]); print(f"Wrote {sys.argv[1]}: {n/1024:.0f} KB (~{n/4:,.0f} tokens chars/4)")' "$out"
fi

# Token-budget overflow still writes the pack; treat it as a hard failure for CI/agents.
exit "$status"
