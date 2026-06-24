#!/usr/bin/env python3
"""Negative tests for ContextGrantV0."""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def remove_hash(grant: dict[str, Any]) -> None:
    del grant["granted_context"][0]["sha256"]


def missing_file(grant: dict[str, Any]) -> None:
    grant["granted_context"][0]["path"] = "tools/hil/missing_context.sh"


def changed_file(grant: dict[str, Any]) -> None:
    grant["granted_context"][0]["sha256"] = "sha256:" + "0" * 64


def source_outside_scope(grant: dict[str, Any]) -> None:
    for entry in grant["granted_context"]:
        if entry["access"] == "read":
            entry["access"] = "patch"
            return
    raise ValueError("fixture has no read-only entry")


def absent_required_context(grant: dict[str, Any]) -> None:
    grant["granted_context"].pop()


def new_path_outside_scope(grant: dict[str, Any]) -> None:
    grant["authorized_new_paths"] = ["docs/not_in_scope.md"]


CASES: list[tuple[str, Callable[[dict[str, Any]], None]]] = [
    ("unhashbound_context", remove_hash),
    ("missing_granted_file", missing_file),
    ("changed_granted_file", changed_file),
    ("source_path_outside_scope", source_outside_scope),
    ("patch_plan_missing_required_context", absent_required_context),
    ("authorized_new_path_outside_scope", new_path_outside_scope),
]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--grant", default="out/org/context_grant.json")
    parser.add_argument("--work-dir", default="out/org/context-grant-negative")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    source = load_json(root / args.grant)
    work_dir = root / args.work_dir
    work_dir.mkdir(parents=True, exist_ok=True)
    failures: list[str] = []
    for name, mutate in CASES:
        value = copy.deepcopy(source)
        mutate(value)
        case_path = work_dir / f"{name}.json"
        write_json(case_path, value)
        result = subprocess.run(
            [
                sys.executable,
                "tools/org/validate_context_grant.py",
                "--root",
                str(root),
                "--grant",
                str(case_path.relative_to(root)),
            ],
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if result.returncode == 0:
            failures.append(f"{name}: validator accepted bad grant\n{result.stdout}")
        else:
            print(f"negative_case={name} rejected")
    if failures:
        print("\n".join(failures))
        return 1
    print("context_grant_negative=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
