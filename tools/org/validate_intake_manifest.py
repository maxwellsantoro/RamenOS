#!/usr/bin/env python3
"""Validate a hash-bound RamenOrg agent intake manifest."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from render_board_brief import load_json, resolve_repo_path, sha256_file
from validate_packets import validate_object


PACKET_ARTIFACTS = ["board_packet", "work_order", "handoff", "vote"]


def artifact_entry(artifacts: dict[str, Any], name: str) -> tuple[str, str]:
    entry = artifacts.get(name)
    if not isinstance(entry, dict):
        raise ValueError(f"manifest artifact is missing or invalid: {name}")
    path = entry.get("path")
    digest = entry.get("sha256")
    if not isinstance(path, str) or not isinstance(digest, str):
        raise ValueError(f"manifest artifact path/hash is invalid: {name}")
    return path, digest


def validate_manifest(root: Path, manifest_path: Path, schema_path: Path) -> list[str]:
    errors: list[str] = []
    manifest = load_json(manifest_path)
    schema = load_json(schema_path)
    validate_object(errors, manifest_path, manifest, schema)
    if errors:
        return errors

    artifacts = manifest["artifacts"]
    paths: dict[str, str] = {}
    for name in artifacts:
        rel, expected = artifact_entry(artifacts, name)
        paths[name] = rel
        path = resolve_repo_path(root, rel)
        if not path.is_file():
            errors.append(f"{manifest_path}: artifact is missing: {rel}")
            continue
        actual = sha256_file(path)
        if actual != expected:
            errors.append(f"{manifest_path}: artifact hash mismatch: {rel}")

    if errors:
        return errors

    report = load_json(resolve_repo_path(root, paths["validation_report"]))
    if report.get("status") != "pass":
        errors.append(f"{manifest_path}: validation report status must be pass")
        return errors
    report_hashes = report.get("artifact_hashes")
    if not isinstance(report_hashes, dict):
        errors.append(f"{manifest_path}: validation report artifact_hashes is missing")
        return errors

    for name in [*PACKET_ARTIFACTS, "current_task"]:
        rel, digest = artifact_entry(artifacts, name)
        if report_hashes.get(rel) != digest:
            errors.append(f"{manifest_path}: {name} is not bound by the validation report")

    context_grant = load_json(resolve_repo_path(root, paths["context_grant"]))
    granted_context = manifest.get("granted_context")
    if granted_context != context_grant.get("granted_context"):
        errors.append(f"{manifest_path}: granted_context must exactly match context grant")
    if manifest.get("authorized_new_paths") != context_grant.get("authorized_new_paths"):
        errors.append(f"{manifest_path}: authorized_new_paths must exactly match context grant")
    if context_grant.get("repo_sha") != manifest.get("repo_sha"):
        errors.append(f"{manifest_path}: context grant repo_sha does not match manifest")
    if isinstance(granted_context, list):
        for entry in granted_context:
            if not isinstance(entry, dict):
                errors.append(f"{manifest_path}: granted context entry must be an object")
                continue
            rel = entry.get("path")
            digest = entry.get("sha256")
            if not isinstance(rel, str) or not isinstance(digest, str):
                errors.append(f"{manifest_path}: granted context entry is missing path/hash")
                continue
            path = resolve_repo_path(root, rel)
            if not path.is_file():
                errors.append(f"{manifest_path}: granted context file is missing: {rel}")
            elif sha256_file(path) != digest:
                errors.append(f"{manifest_path}: granted context hash mismatch: {rel}")

    packets = {
        name: load_json(resolve_repo_path(root, paths[name])) for name in PACKET_ARTIFACTS
    }
    board = packets["board_packet"]
    expected_refs = {
        "work_order": board.get("work_order_refs", []),
        "handoff": board.get("handoff_refs", []),
        "vote": board.get("vote_refs", []),
    }
    for name, refs in expected_refs.items():
        if refs != [paths[name]]:
            errors.append(f"{manifest_path}: board {name} ref does not match manifest")
    if board.get("current_task_ref") != paths["current_task"]:
        errors.append(f"{manifest_path}: board current_task_ref does not match manifest")

    repo_sha = manifest.get("repo_sha")
    for name, packet in packets.items():
        if packet.get("repo_sha") != repo_sha:
            errors.append(f"{manifest_path}: {name}.repo_sha does not match manifest")

    brief = resolve_repo_path(root, paths["brief"]).read_text(encoding="utf-8")
    required_brief_values = [
        paths["validation_report"],
        paths["current_task"],
        paths["board_packet"],
        paths["work_order"],
        paths["handoff"],
        paths["vote"],
        paths["context_grant"],
        str(repo_sha),
    ]
    if isinstance(granted_context, list):
        required_brief_values.extend(str(entry.get("path", "")) for entry in granted_context)
    required_brief_values.extend(str(item) for item in context_grant.get("authorized_new_paths", []))
    required_brief_values.extend(str(item) for item in context_grant.get("not_granted", []))
    for value in required_brief_values:
        if value not in brief:
            errors.append(f"{manifest_path}: brief is missing intake binding value: {value}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--schema", default="schemas/org/intake_manifest_v0.schema.json")
    parser.add_argument("--manifest", default="out/org/intake_manifest.json")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    errors = validate_manifest(
        root,
        resolve_repo_path(root, args.manifest),
        resolve_repo_path(root, args.schema),
    )
    report = {"status": "fail" if errors else "pass", "errors": errors}
    print(json.dumps(report, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
