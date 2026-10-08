"""Fail-closed change classification and merge decision used by CI and tests."""
import argparse
from pathlib import PurePosixPath
import subprocess


# These Markdown documents are byte-pinned inputs or content assertions of OS
# gates, rather than prose-only references. Keep this list path-based: a deleted
# input must still require Foundry. test_review_boundaries audits gate references
# and reviewed source registries so a new executable document cannot evade it.
EXECUTABLE_DOCUMENTS = frozenset({
    'CONSTITUTION.md',
    'EVIDENCE_LEVELS.md',
    'NEXT_TASKS.md',
    'docs/BOOT_FRAME_OWNERSHIP_V0.md',
    'docs/DESKTOP_EDITOR_HOST_API_V0.md',
    'docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md',
    'docs/DESKTOP_EDITOR_STORE_API_V0.md',
    'docs/DESKTOP_EDITOR_WIRE_V1.md',
    'docs/DESKTOP_SESSION_V1.md',
    'docs/FOUNDRY_CI_OPTIMIZATION_V0.md',
    'docs/HARDWARE_STRATEGY.md',
    'docs/HIL_APPLIANCE_EVIDENCE_V0.md',
    'docs/plans/2026-02-20-s11-driver-factory-mvp.md',
    'docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md',
    'docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md',
    'docs/plans/2026-06-17-s10-5-host-to-target-integration.md',
    'docs/plans/2026-06-21-s12-golden-machine-design.md',
    'docs/plans/2026-06-21-s13-persistent-storage-design.md',
    'docs/plans/2026-06-22-hil-appliance-controller.md',
    'docs/plans/desktop-editor-v0.md',
    'docs/plans/editor-store-transaction-v0.md',
    'docs/plans/posix_runner_remaining_risks.md',
    'drivers/reference_vaults/virtio-blk/README.md',
    'drivers/reference_vaults/virtio-net/README.md',
})


def requires_foundry(paths):
    def documentation(path):
        p = PurePosixPath(path)
        if (path in EXECUTABLE_DOCUMENTS
                or path.startswith('docs/contracts/') or 'fixtures' in p.parts):
            return False
        # Only genuine org packet locations get the structured-data exemption.
        # Unknown JSON/YAML elsewhere defaults to Foundry, including new inputs.
        return ((p.suffix == ".md" and not path.startswith(".github/"))
                or (path.startswith("docs/org/") and p.suffix in {".yaml", ".yml", ".json"}))
    return any(not documentation(path) for path in paths)


def merge_allowed(changes, os_code, foundry, governance):
    return (changes == "success" and governance == "success"
            and os_code in {"true", "false"}
            and (foundry == "success" if os_code == "true" else foundry in {"success", "skipped"}))


def foundry_allowed(changes, os_code, quality, host, agent, qemu):
    """Only docs classification permits all lane jobs to skip."""
    if changes != "success" or os_code not in {"true", "false"}:
        return False
    expected = "success" if os_code == "true" else "skipped"
    return all(result == expected for result in (quality, host, agent, qemu))


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    classify = sub.add_parser("classify")
    classify.add_argument("base")
    classify.add_argument("head")
    merge = sub.add_parser("merge")
    for name in ["changes", "os_code", "foundry", "governance"]:
        merge.add_argument(name)
    foundry = sub.add_parser("foundry")
    for name in ["changes", "os_code", "quality", "host", "agent", "qemu"]:
        foundry.add_argument(name)
    args = parser.parse_args()
    if args.command == "classify":
        raw = subprocess.check_output(["git", "diff", "--no-renames", "--name-only", "-z", args.base, args.head, "--"])
        paths = [p.decode("utf-8", errors="strict") for p in raw.split(b"\0") if p]
        print("true" if requires_foundry(paths) else "false")
    elif args.command == "foundry":
        if not foundry_allowed(args.changes, args.os_code, args.quality, args.host, args.agent, args.qemu):
            raise SystemExit("foundry: FAIL")
        print("foundry: PASS")
    elif not merge_allowed(args.changes, args.os_code, args.foundry, args.governance):
        raise SystemExit("merge-gate: FAIL")
    else:
        print("merge-gate: PASS")


if __name__ == "__main__":
    main()
