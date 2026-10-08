#!/usr/bin/env bash
# Supplemental adapter preservation only; no child-process or live Save acceptance.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
mkdir -p out/foundry/editor-adapter-migration
run_dir=$(mktemp -d out/foundry/editor-adapter-migration/run.XXXXXXXX)
# Preserve the command status and show fresh logs even on failure. No logs are reused.
show_logs() {
    local status=$?
    trap - EXIT
    for log in "$run_dir"/*.stdout "$run_dir"/*.stderr; do
        if [[ -f "$log" ]]; then cat "$log" || true; fi
    done
    exit "$status"
}
trap show_logs EXIT
cargo test --locked -p desktop_service --no-default-features --features desktop_v0_dev --test editor_adapter_migration -- --test-threads=1 --nocapture >"$run_dir/legacy.stdout" 2>"$run_dir/legacy.stderr"
cargo test --locked -p artifact_editor --no-default-features --features host_editor_adapter_assertions_v0_dev --test editor_adapter_migration -- --test-threads=1 --nocapture >"$run_dir/native.stdout" 2>"$run_dir/native.stderr"
python3 - "$run_dir" <<'PY_CLASSIFY'
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
expected = {
    "legacy": ("adapter_legacy_selected_snapshot_continuation_and_policies",),
    "native": (
        "adapter_native_actual_text_max_rejection_is_atomic",
        "adapter_native_actual_view_max_rejection_is_atomic",
        "adapter_native_selected_snapshot_continuation_and_policies",
    ),
}
markers = (
    "ADAPTER_ASSERT actual-text-max hidden-owner-atomic PASS",
    "ADAPTER_ASSERT actual-view-max hidden-owner-atomic PASS",
)
for role, names in expected.items():
    leaves = []
    for stream in ("stdout", "stderr"):
        path = root / (role + "." + stream)
        if not path.is_file() or path.stat().st_size > 8388608:
            raise SystemExit("EDITOR_ADAPTER_MIGRATION: FAIL bounded fresh log")
        leaves.append(path.read_text(encoding="utf8", errors="strict"))
    stdout, stderr = leaves
    if "INCOMPLETE" in stdout + stderr:
        raise SystemExit("EDITOR_ADAPTER_MIGRATION: FAIL incomplete fixture")
    observed = re.findall(r"^test (\w+) \.\.\. ok$", stdout, re.M)
    summary = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;.*$", stdout, re.M)
    all_test_rows = re.findall(r"^test (?!result:)(.+)$", stdout, re.M)
    actual_markers = tuple(line for line in stderr.splitlines() if line.startswith("ADAPTER_ASSERT "))
    if (tuple(observed) != names or len(all_test_rows) != len(names)
            or summary != [str(len(names))]
            or actual_markers != (() if role == "legacy" else markers)
            or "ADAPTER_ASSERT " in stdout):
        raise SystemExit("EDITOR_ADAPTER_MIGRATION: FAIL exact tests or owner markers")
print("EDITOR_ADAPTER_MIGRATION: PASS tests=4 scope=adapter-preservation")
PY_CLASSIFY
