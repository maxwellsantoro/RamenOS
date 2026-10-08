#!/usr/bin/env bash
# Root-owned exact source snapshot and independently reviewed admission profile.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
if [[ "$#" -eq 0 ]]; then
  mkdir -p "$REPO/out/desktop"
  TASK_PACKET="$(mktemp -d "$REPO/out/desktop/native-task-source.XXXXXXXX")"
  TASK_SOURCE_SHA="$(python3 "$REPO/tools/ci/native_save_task_sources.py" --repo "$REPO" \
    --registry-sha256 c2db944d5a78469e4310f2ce7e002bb51abff5d4e3d68962172535184eda438a \
    --output "$TASK_PACKET/source-manifest.json")"
  set -- "$TASK_PACKET/source-manifest.json" "$TASK_SOURCE_SHA" "$TASK_PACKET/run"
elif [[ "$#" -ne 3 ]]; then
  echo 'usage: gate SOURCE_MANIFEST SOURCE_MANIFEST_SHA256 NEW_ABSOLUTE_RUN_ROOT' >&2
  exit 2
fi
exec python3 "$REPO/tools/ci/desktop_editor_task_result.py" --repo "$REPO" \
  --source-manifest "$1" --source-manifest-sha256 "$2" --run-root "$3"
