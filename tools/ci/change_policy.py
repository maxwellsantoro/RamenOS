"""Fail-closed change classification and merge decision used by CI and tests."""
import argparse
from pathlib import PurePosixPath
import subprocess


def requires_foundry(paths):
    def documentation(path):
        p = PurePosixPath(path)
        # Org packets are checked by the mandatory governance job.
        return ((p.suffix == ".md" and not path.startswith(".github/"))
                or (path.startswith("docs/") and p.suffix in {".yaml", ".yml", ".json"}))
    return any(not documentation(path) for path in paths)


def merge_allowed(changes, os_code, foundry, governance):
    return (changes == "success" and governance == "success"
            and os_code in {"true", "false"}
            and (foundry == "success" if os_code == "true" else foundry in {"success", "skipped"}))


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    classify = sub.add_parser("classify")
    classify.add_argument("base")
    classify.add_argument("head")
    merge = sub.add_parser("merge")
    for name in ["changes", "os_code", "foundry", "governance"]:
        merge.add_argument(name)
    args = parser.parse_args()
    if args.command == "classify":
        raw = subprocess.check_output(["git", "diff", "--no-renames", "--name-only", "-z", args.base, args.head, "--"])
        paths = [p.decode("utf-8", errors="strict") for p in raw.split(b"\0") if p]
        print("true" if requires_foundry(paths) else "false")
    elif not merge_allowed(args.changes, args.os_code, args.foundry, args.governance):
        raise SystemExit("merge-gate: FAIL")
    else:
        print("merge-gate: PASS")


if __name__ == "__main__":
    main()
