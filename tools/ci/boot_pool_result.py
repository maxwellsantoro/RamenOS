#!/usr/bin/env python3
"""Bind a frozen pure boot-pool assertion inventory to source and test artifacts."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

CASES = (
    "valid_final_conventional_pool", "denies_active_and_unmasked_stages",
    "fixed_capacity_and_checked_counts", "validates_every_descriptor_at_capacity",
    "rejects_zero_unaligned_overflow_and_physical_width",
    "rejects_duplicate_and_overlapping_descriptors",
    "accepts_adjacency_unsorted_input_and_ignores_unused_slots",
    "requires_all_retention_reasons", "retention_count_is_bounded",
    "rejects_invalid_retained_ranges", "rejects_uncovered_and_conventional_retention",
    "accepts_conservative_loader_retention_and_shared_reasons",
    "no_reclamation_and_no_eligible_memory_is_explicit",
    "intersects_window_and_excludes_page_zero_and_high_memory",
    "capacity_policy_is_explicit_and_does_not_round_up",
    "selection_is_stable_and_never_combines_descriptors",
    "contains_exactly_selected_frames",
)
REQUIRED = tuple("mm::boot_pool_contract_tests::boot_pool_" + name for name in CASES)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check_list(path):
    listed = re.findall(r"^(.+): (?:test|benchmark)$", path.read_text(), re.MULTILINE)
    if len(listed) != len(REQUIRED) or set(listed) != set(REQUIRED):
        raise ValueError("missing/duplicate/unexpected boot pool assertion")


def result(evidence):
    check_list(evidence / "cases.log")
    log = (evidence / "tests.log").read_text()
    outcomes = re.findall(r"^test (.+) \.\.\. (\w+)$", log, re.MULTILINE)
    if (len(outcomes) != len(REQUIRED) or {name for name, _ in outcomes} != set(REQUIRED)
            or any(state != "ok" for _, state in outcomes)):
        raise ValueError("missing/skipped/failed boot pool assertions")
    summaries = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;", log, re.MULTILINE)
    # Other kernel tests are outside this named module's scope, explicitly filtered.
    if len(summaries) != 1 or summaries[0][:4] != (str(len(REQUIRED)), "0", "0", "0"):
        raise ValueError("unexpected pure assertion summary")
    executables = [row["executable"] for line in (evidence / "build.jsonl").read_text().splitlines()
                   if (row := json.loads(line)).get("reason") == "compiler-artifact"
                   and row.get("target", {}).get("name") == "kernel"
                   and row.get("profile", {}).get("test") is True and row.get("executable")]
    if len(executables) != 1:
        raise ValueError("missing/ambiguous acceptance executable")
    sources = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "kernel/Cargo.toml",
               "kernel_api/Cargo.toml", "docs/BOOT_FRAME_OWNERSHIP_V0.md",
               "tools/ci/boot_pool_result.py", "tools/ci/foundry_boot_frame_pool_run0_0.sh", "justfile"]
    sources += [str(p) for root in ("kernel/src", "kernel_api/src")
                for p in sorted(Path(root).rglob("*.rs"))]
    return dict(schema_version=1, gate="foundry-boot-frame-pool-run0-0", outcome="PASS",
                scope="pure-map-retention-admission", required_cases=list(REQUIRED),
                executed_cases=[name for name, _ in outcomes],
                filtered_kernel_tests=int(summaries[0][4]),
                source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                dirty_diff_sha256=sha(subprocess.check_output(["git", "diff", "HEAD", "--binary"])),
                source_sha256={p: sha(Path(p).read_bytes()) for p in sorted(set(sources))},
                test_executable_sha256=sha(Path(executables[0]).read_bytes()),
                artifact_sha256={p.name: sha(p.read_bytes()) for p in sorted(evidence.iterdir())
                                 if p.is_file() and p.name != "result.json"},
                claims=dict(actual_firmware_exit=False, interrupts_masked=False,
                            raw_efi_conversion=False, complete_retention_collection=False,
                            allocator_installed=False, physical_access=False,
                            qemu_consumer=False, user_mode=False, physical_hardware=False))


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check-list", type=Path)
    group.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    if args.check_list:
        check_list(args.check_list)
    else:
        (args.evidence / "result.json").write_text(json.dumps(result(args.evidence), indent=2) + "\n")


if __name__ == "__main__":
    main()
