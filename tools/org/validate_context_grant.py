#!/usr/bin/env python3
"""Validate ContextGrantV0 structure, freshness, and scope."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from render_board_brief import load_json, resolve_repo_path, sha256_file
from render_board_packet import git_sha, load_current_task, require_list, require_scalar
from render_context_grant import path_in_scope
from validate_packets import validate_object


def validate_grant(root: Path, grant_path: Path, schema_path: Path, task_path: Path) -> list[str]:
    errors: list[str] = []
    grant = load_json(grant_path)
    schema = load_json(schema_path)
    task = load_current_task(task_path)
    validate_object(errors, grant_path, grant, schema)
    if errors:
        return errors

    expected_identity = {
        "task_id": require_scalar(task, "task_id"),
        "work_order_id": require_scalar(task, "work_order_id"),
        "repo_sha": git_sha(root),
        "authority_level": require_scalar(task, "authority_level"),
    }
    for field, expected in expected_identity.items():
        if grant.get(field) != expected:
            errors.append(f"{grant_path}: {field} does not match current task/repository")

    refs = require_list(task, "context_grant_refs")
    scope = require_list(task, "scope")
    context_refs = set(require_list(task, "context_refs"))
    entries = grant.get("granted_context", [])
    paths = [str(entry.get("path", "")) for entry in entries]
    if paths != refs:
        errors.append(f"{grant_path}: granted_context paths must exactly match current_task.context_grant_refs")
    if len(paths) != len(set(paths)):
        errors.append(f"{grant_path}: granted_context paths must be unique")
    if grant.get("required_for_patch_plan") != refs:
        errors.append(f"{grant_path}: required_for_patch_plan must exactly match context_grant_refs")

    authorized_new_paths = [str(item) for item in task.get("authorized_new_paths", [])]
    if grant.get("authorized_new_paths") != authorized_new_paths:
        errors.append(f"{grant_path}: authorized_new_paths must match current task")
    for rel in authorized_new_paths:
        if not path_in_scope(rel, scope):
            errors.append(f"{grant_path}: authorized new path is outside work-order scope: {rel}")
        try:
            path = resolve_repo_path(root, rel)
        except ValueError as exc:
            errors.append(f"{grant_path}: {exc}")
            continue
        if path.exists():
            errors.append(f"{grant_path}: authorized new path already exists and should be granted context: {rel}")

    for entry in entries:
        rel = str(entry.get("path", ""))
        expected_access = "patch" if path_in_scope(rel, scope) else "read"
        if entry.get("access") != expected_access:
            errors.append(f"{grant_path}: access for {rel} must be {expected_access}")
        if expected_access == "read" and rel not in context_refs:
            errors.append(f"{grant_path}: read context is not declared in current_task.context_refs: {rel}")
        try:
            path = resolve_repo_path(root, rel)
        except ValueError as exc:
            errors.append(f"{grant_path}: {exc}")
            continue
        if not path.is_file():
            errors.append(f"{grant_path}: granted file is missing: {rel}")
            continue
        if sha256_file(path) != entry.get("sha256"):
            errors.append(f"{grant_path}: granted file hash mismatch: {rel}")

    if "context_expansion_request" not in str(grant.get("context_expansion_policy", "")):
        errors.append(f"{grant_path}: context expansion policy must require context_expansion_request")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--current-task", default="docs/org/current_task.yaml")
    parser.add_argument("--schema", default="schemas/org/context_grant_v0.schema.json")
    parser.add_argument("--grant", default="out/org/context_grant.json")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    errors = validate_grant(
        root,
        resolve_repo_path(root, args.grant),
        resolve_repo_path(root, args.schema),
        resolve_repo_path(root, args.current_task),
    )
    print(json.dumps({"status": "fail" if errors else "pass", "errors": errors}, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
