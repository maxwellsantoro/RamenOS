#!/usr/bin/env python3
"""Validate a HumanDirectiveV0 artifact (typed founder vision injection).

A directive grants vision input only. It must not authorize merge, release,
hardware actuation, or public support, and it must target the board (which turns
it into a WorkOrderV0 through the normal packet path).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_packets import fail, load_json, sha256_file, validate_object

DIRECTIVE_DENIALS = ["no merge", "no release"]


def validate_directive(root: Path, path: Path, errors: list[str] | None = None) -> list[str]:
    errors = [] if errors is None else errors
    schema = load_json(root / "schemas/org/human_directive_v0.schema.json")
    directive = load_json(path)
    validate_object(errors, path, directive, schema)
    text = str(directive.get("claim_boundary", "")).lower()
    for denial in DIRECTIVE_DENIALS:
        if denial not in text:
            fail(errors, path, f"claim_boundary missing denial: {denial}")
    if directive.get("proposal_target") != "board":
        fail(errors, path, "a human directive must target the board, not bind work directly")
    if directive.get("from_role") != "Founder/Vision Channel":
        fail(errors, path, "from_role must be Founder/Vision Channel")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--schema", default="schemas/org/human_directive_v0.schema.json")
    parser.add_argument("--directive", required=True)
    parser.add_argument("--out", help="optional JSON report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    dpath = root / args.directive
    errors: list[str] = []
    if not dpath.is_file():
        errors.append(f"missing directive: {dpath}")
    else:
        validate_directive(root, dpath, errors)

    status = "pass" if not errors else "fail"
    report: dict[str, Any] = {
        "status": status,
        "directive": args.directive,
        "errors": errors,
    }
    if dpath.is_file():
        report["hash"] = sha256_file(dpath)
    if args.out:
        out_path = Path(args.out)
        if not out_path.is_absolute():
            out_path = root / out_path
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if status == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
