#!/usr/bin/env python3
"""Render a hash-bound ContextGrantV0 from the active task."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from render_board_brief import resolve_repo_path, sha256_file
from render_board_packet import git_sha, load_current_task, require_list, require_scalar, write_json


def path_in_scope(path: str, scope: list[str]) -> bool:
    return any(path == item or path.startswith(item.rstrip("/") + "/") for item in scope)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--current-task", default="docs/org/current_task.yaml")
    parser.add_argument("--out", default="out/org/context_grant.json")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    task = load_current_task(resolve_repo_path(root, args.current_task))
    refs = require_list(task, "context_grant_refs")
    authorized_new_paths = [str(item) for item in task.get("authorized_new_paths", [])]
    scope = require_list(task, "scope")
    granted = []
    for rel in refs:
        path = resolve_repo_path(root, rel)
        if not path.is_file():
            raise ValueError(f"context grant ref is not a file: {rel}")
        granted.append(
            {
                "path": rel,
                "sha256": sha256_file(path),
                "access": "patch" if path_in_scope(rel, scope) else "read",
            }
        )

    grant = {
        "schema_version": 1,
        "grant_kind": "context_grant_v0",
        "task_id": require_scalar(task, "task_id"),
        "work_order_id": require_scalar(task, "work_order_id"),
        "repo_sha": git_sha(root),
        "authority_level": require_scalar(task, "authority_level"),
        "purpose": "patch" if not authorized_new_paths else "patch_plan",
        "granted_context": granted,
        "authorized_new_paths": authorized_new_paths,
        "required_for_patch_plan": refs,
        "not_granted": [
            "Any existing repository file not listed in granted_context as input context",
            "Credentials, secrets, and external service access",
            "HIL device access, power control, reset control, and actuation",
            "Merge, release, self-approval, and public support authority",
        ],
        "context_expansion_policy": (
            "If additional context is required, emit context_expansion_request with path and reason; "
            "do not produce a patch."
        ),
    }
    write_json(resolve_repo_path(root, args.out), grant)
    print(json.dumps({"status": "pass", "context_grant": args.out, "granted": len(granted)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
