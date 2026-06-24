#!/usr/bin/env python3
"""Negative cases for HumanDirectiveV0 validation.

Each case builds a fixture directive and asserts the expected pass/fail.
Exit 0 only when every expectation holds.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_human_directive import validate_directive

SHA = "0123456789abcdef0123456789abcdef01234567"


def write(path: Path, obj: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2) + "\n", encoding="utf-8")


def directive(**overrides: Any) -> dict[str, Any]:
    obj: dict[str, Any] = {
        "schema_version": 1,
        "packet_kind": "human_directive_v0",
        "directive_id": "HD-TEST",
        "repo_sha": SHA,
        "from_role": "Founder/Vision Channel",
        "authority": "vision_input",
        "directive": "try this architecture",
        "proposal_target": "board",
        "constraints": ["board turns this into a work order"],
        "claim_boundary": "vision input only; no merge, no release, no public support authority",
    }
    obj.update(overrides)
    return obj


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--work-dir", default="out/org/directive-negative")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    work = root / args.work_dir

    cases: list[tuple[str, dict[str, Any], bool]] = [
        ("good", directive(), False),
        ("missing_merge_denial", directive(claim_boundary="vision input only; no release, no public support authority"), True),
        ("wrong_from_role", directive(from_role="Architect"), True),
        ("wrong_target", directive(proposal_target="implementer"), True),
        ("wrong_authority", directive(authority="merge"), True),
    ]

    results: list[dict[str, Any]] = []
    all_ok = True
    for name, obj, expect_error in cases:
        path = work / f"{name}.json"
        write(path, obj)
        errors = validate_directive(root, path)
        got_error = bool(errors)
        ok = got_error == expect_error
        all_ok = all_ok and ok
        results.append({"case": name, "expect_error": expect_error, "got_error": got_error, "ok": ok, "errors": errors})

    print(json.dumps({"status": "pass" if all_ok else "fail", "cases": results}, indent=2))
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
