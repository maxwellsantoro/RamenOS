#!/usr/bin/env python3
"""Validate RamenOrg packet examples without external dependencies."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

from render_board_packet import load_current_task


SCHEMA_BY_KIND = {
    "work_order_v0": "work_order_v0.schema.json",
    "handoff_packet_v0": "handoff_packet_v0.schema.json",
    "board_vote_v0": "board_vote_v0.schema.json",
    "board_packet_v0": "board_packet_v0.schema.json",
}

EVIDENCE_BUCKETS = [
    "design_evidence_refs",
    "gate_evidence_refs",
    "claim_evidence_refs",
    "hil_evidence_refs",
    "release_evidence_refs",
]

CLAIM_BOUNDARY_DENIALS = [
    "no merge",
    "no release",
    "no self-approval",
    "no hil actuation",
    "no public support authority",
]


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def fail(errors: list[str], path: Path, msg: str) -> None:
    errors.append(f"{path}: {msg}")


def type_matches(value: Any, expected: str) -> bool:
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "boolean":
        return isinstance(value, bool)
    return True


def validate_value(errors: list[str], path: Path, field_path: str, value: Any, schema: dict[str, Any]) -> None:
    expected_type = schema.get("type")
    if expected_type and not type_matches(value, expected_type):
        fail(errors, path, f"{field_path} must be {expected_type}")
        return

    if "const" in schema and value != schema["const"]:
        fail(errors, path, f"{field_path} must equal {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        fail(errors, path, f"{field_path} must be one of {schema['enum']!r}")
    if isinstance(value, str) and schema.get("minLength", 0) and len(value) < schema["minLength"]:
        fail(errors, path, f"{field_path} must not be empty")
    if isinstance(value, str) and "pattern" in schema and not re.fullmatch(schema["pattern"], value):
        fail(errors, path, f"{field_path} must match {schema['pattern']!r}")
    if isinstance(value, int) and "minimum" in schema and value < schema["minimum"]:
        fail(errors, path, f"{field_path} must be >= {schema['minimum']}")
    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            fail(errors, path, f"{field_path} must contain at least {schema['minItems']} item(s)")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            fail(errors, path, f"{field_path} must contain at most {schema['maxItems']} item(s)")
        item_schema = schema.get("items")
        if item_schema:
            for index, item in enumerate(value):
                validate_value(errors, path, f"{field_path}[{index}]", item, item_schema)
    if isinstance(value, dict):
        validate_object(errors, path, value, schema, field_path)


def validate_object(
    errors: list[str],
    path: Path,
    obj: dict[str, Any],
    schema: dict[str, Any],
    field_path: str = "$",
) -> None:
    if schema.get("additionalProperties") is False:
        allowed = set(schema.get("properties", {}).keys())
        for key in obj:
            if key not in allowed:
                fail(errors, path, f"{field_path}.{key} is not allowed")
    for key in schema.get("required", []):
        if key not in obj:
            fail(errors, path, f"{field_path}.{key} is required")
    for key, value in obj.items():
        prop_schema = schema.get("properties", {}).get(key)
        if prop_schema:
            validate_value(errors, path, f"{field_path}.{key}", value, prop_schema)


def rel_exists(root: Path, rel: str) -> bool:
    return (root / rel).exists()


def gate_exists(root: Path, gate_ref: str) -> bool:
    if gate_ref.startswith("just "):
        recipe = gate_ref.split(maxsplit=1)[1].strip()
        justfile = (root / "justfile").read_text(encoding="utf-8")
        return re.search(rf"^{re.escape(recipe)}:", justfile, re.M) is not None
    candidate = root / gate_ref
    if gate_ref.endswith(".sh") or gate_ref.startswith("tools/"):
        return candidate.exists() and candidate.is_file() and os.access(candidate, os.X_OK)
    return False


def check_refs(errors: list[str], root: Path, path: Path, refs: list[str], field: str) -> None:
    for ref in refs:
        if not rel_exists(root, ref):
            fail(errors, path, f"{field} ref does not exist: {ref}")


def evidence_refs(packet: dict[str, Any]) -> list[str]:
    evidence = packet.get("evidence", {})
    if not isinstance(evidence, dict):
        return []
    refs: list[str] = []
    for bucket in EVIDENCE_BUCKETS:
        bucket_refs = evidence.get(bucket, [])
        if isinstance(bucket_refs, list):
            refs.extend(str(ref) for ref in bucket_refs)
    return refs


def hil_evidence_refs(packet: dict[str, Any]) -> list[str]:
    evidence = packet.get("evidence", {})
    if not isinstance(evidence, dict):
        return []
    refs = evidence.get("hil_evidence_refs", [])
    if not isinstance(refs, list):
        return []
    return [str(ref) for ref in refs]


def claims_pass_metal(value: Any) -> bool:
    return "PASS/METAL" in str(value).upper()


def validate_claim_boundary(errors: list[str], path: Path, boundary: Any) -> None:
    text = str(boundary).lower()
    missing = [denial for denial in CLAIM_BOUNDARY_DENIALS if denial not in text]
    if missing:
        fail(errors, path, f"claim_boundary missing denial(s): {', '.join(missing)}")


def custom_validate(errors: list[str], root: Path, path: Path, packet: dict[str, Any]) -> None:
    kind = packet.get("packet_kind")
    if kind in {"work_order_v0", "handoff_packet_v0", "board_packet_v0"}:
        check_refs(errors, root, path, packet.get("context_refs", []), "context_refs")

    if kind in {"work_order_v0", "handoff_packet_v0"}:
        for gate in packet.get("required_gates", []):
            if not gate_exists(root, gate):
                fail(errors, path, f"required gate does not exist: {gate}")

    if kind == "work_order_v0":
        authority = packet.get("authority_level")
        if authority not in {"A0", "A1", "A2"}:
            fail(errors, path, "G0.8.1 examples must not grant authority above A2-local")
        task_blob = " ".join([packet.get("task", ""), *packet.get("constraints", [])]).lower()
        if "hil" in task_blob and "evidence" not in task_blob:
            fail(errors, path, "HIL work orders must mention evidence constraints")

    if kind == "handoff_packet_v0":
        if packet.get("from_role") == packet.get("to_role"):
            fail(errors, path, "from_role and to_role must differ")
        for index, claim in enumerate(packet.get("claims", [])):
            source = claim.get("source", "")
            if source and not rel_exists(root, source):
                fail(errors, path, f"claims[{index}].source does not exist: {source}")

    if kind == "board_vote_v0":
        vote = packet.get("vote")
        refs = evidence_refs(packet)
        if vote == "approve" and not refs:
            fail(errors, path, "approve votes require at least one typed evidence ref")
        if vote == "block" and not packet.get("blocking_conditions"):
            fail(errors, path, "block votes require blocking_conditions")
        check_refs(errors, root, path, refs, "evidence")

    if kind == "board_packet_v0":
        if packet.get("authority_level") not in {"A0", "A1", "A2"}:
            fail(errors, path, "G0.8.1 board packets must remain A0/A1/A2-local")
        check_refs(errors, root, path, [packet.get("current_task_ref", "")], "current_task_ref")
        if not gate_exists(root, packet.get("next_gate", "")):
            fail(errors, path, f"next_gate does not exist: {packet.get('next_gate')}")
        for field in ["work_order_refs", "handoff_refs", "vote_refs"]:
            check_refs(errors, root, path, packet.get(field, []), field)
        validate_claim_boundary(errors, path, packet.get("claim_boundary", ""))


def validate_current_task(
    errors: list[str],
    root: Path,
    task_path: Path,
    schema_path: Path,
) -> dict[str, Any] | None:
    if not task_path.is_file():
        errors.append(f"missing current task: {task_path}")
        return None
    if not schema_path.is_file():
        errors.append(f"missing current task schema: {schema_path}")
        return None
    try:
        task = load_current_task(task_path)
    except Exception as exc:
        errors.append(f"{task_path}: failed to parse current task: {exc}")
        return None
    schema = load_json(schema_path)
    validate_object(errors, task_path, task, schema)
    check_refs(errors, root, task_path, task.get("context_refs", []), "context_refs")
    check_refs(errors, root, task_path, task.get("context_grant_refs", []), "context_grant_refs")
    for gate in task.get("required_gates", []):
        if not gate_exists(root, gate):
            fail(errors, task_path, f"required gate does not exist: {gate}")
    if not gate_exists(root, task.get("next_gate", "")):
        fail(errors, task_path, f"next_gate does not exist: {task.get('next_gate')}")
    for item in task.get("claims", []):
        claim, sep, source = str(item).partition("|")
        if not sep:
            fail(errors, task_path, f"claim entry must use 'claim | source': {item}")
        elif not rel_exists(root, source.strip()):
            fail(errors, task_path, f"claim source does not exist: {source.strip()}")
    for bucket in EVIDENCE_BUCKETS:
        check_refs(errors, root, task_path, task.get(bucket, []), bucket)
    validate_claim_boundary(errors, task_path, task.get("claim_boundary", ""))
    if claims_pass_metal(task.get("claim_level_allowed")) and not task.get("hil_evidence_refs", []):
        fail(errors, task_path, "PASS/METAL claims require hil_evidence_refs")
    return task


def load_ref_packet(
    errors: list[str],
    root: Path,
    board_path: Path,
    rel: str,
    packets_by_rel: dict[str, dict[str, Any]],
) -> dict[str, Any] | None:
    packet = packets_by_rel.get(rel)
    if packet is not None:
        return packet
    target = root / rel
    if not target.is_file():
        fail(errors, board_path, f"referenced packet missing: {rel}")
        return None
    loaded = load_json(target)
    if not isinstance(loaded, dict):
        fail(errors, board_path, f"referenced packet is not an object: {rel}")
        return None
    packets_by_rel[rel] = loaded
    return loaded


def cross_validate(
    errors: list[str],
    root: Path,
    packets_by_rel: dict[str, dict[str, Any]],
    packet_paths_by_rel: dict[str, Path],
) -> None:
    for board_rel, board in packets_by_rel.items():
        if board.get("packet_kind") != "board_packet_v0":
            continue
        board_path = packet_paths_by_rel.get(board_rel, root / board_rel)
        work_refs = board.get("work_order_refs", [])
        handoff_refs = board.get("handoff_refs", [])
        vote_refs = board.get("vote_refs", [])
        if not work_refs or not handoff_refs or not vote_refs:
            continue
        work_order = load_ref_packet(errors, root, board_path, work_refs[0], packets_by_rel)
        handoff = load_ref_packet(errors, root, board_path, handoff_refs[0], packets_by_rel)
        vote = load_ref_packet(errors, root, board_path, vote_refs[0], packets_by_rel)
        if not work_order or not handoff or not vote:
            continue

        same_sha = {board.get("repo_sha"), work_order.get("repo_sha"), handoff.get("repo_sha"), vote.get("repo_sha")}
        if len(same_sha) != 1:
            fail(errors, board_path, "repo_sha must match across board packet, work order, handoff, and vote")
        if vote.get("proposal_id") != work_order.get("work_order_id"):
            fail(errors, board_path, "vote.proposal_id must equal work_order.work_order_id")
        if handoff.get("work_order_id") != work_order.get("work_order_id"):
            fail(errors, board_path, "handoff.work_order_id must equal work_order.work_order_id")
        if handoff.get("task") != work_order.get("task"):
            fail(errors, board_path, "handoff.task must equal work_order.task")
        if board.get("active_task") != work_order.get("task"):
            fail(errors, board_path, "board_packet.active_task must equal work_order.task")
        if set(handoff.get("required_gates", [])) != set(work_order.get("required_gates", [])):
            fail(errors, board_path, "handoff.required_gates must match work_order.required_gates")
        if work_order.get("authority_level") != board.get("authority_level"):
            fail(errors, board_path, "board_packet.authority_level must equal work_order.authority_level")
        if claims_pass_metal(work_order.get("claim_level_allowed")) and not hil_evidence_refs(vote):
            fail(errors, board_path, "PASS/METAL claims require vote.evidence.hil_evidence_refs")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--schema-dir", default="schemas/org")
    parser.add_argument("--packet-dir", default="out/org/examples")
    parser.add_argument("--current-task")
    parser.add_argument("--current-task-schema", default="schemas/org/current_task_v0.schema.json")
    parser.add_argument("--skip-packets", action="store_true")
    parser.add_argument("--out", help="optional JSON validation report")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    schema_dir = root / args.schema_dir
    packet_dir = root / args.packet_dir
    errors: list[str] = []
    checked: list[str] = []
    artifact_hashes: dict[str, str] = {}
    packets_by_rel: dict[str, dict[str, Any]] = {}
    packet_paths_by_rel: dict[str, Path] = {}

    if args.current_task:
        task_path = root / args.current_task
        schema_path = root / args.current_task_schema
        validate_current_task(errors, root, task_path, schema_path)
        task_rel = str(task_path.relative_to(root))
        checked.append(task_rel)
        if task_path.is_file():
            artifact_hashes[task_rel] = sha256_file(task_path)

    schemas: dict[str, dict[str, Any]] = {}
    for kind, name in SCHEMA_BY_KIND.items():
        schema_path = schema_dir / name
        if not schema_path.is_file():
            errors.append(f"missing schema: {schema_path}")
            continue
        schemas[kind] = load_json(schema_path)

    if not args.skip_packets:
        packets = sorted(packet_dir.glob("*.json"))
        if not packets:
            errors.append(f"no packets found in {packet_dir}")

        for packet_path in packets:
            packet_rel = str(packet_path.relative_to(root))
            checked.append(packet_rel)
            artifact_hashes[packet_rel] = sha256_file(packet_path)
            packet = load_json(packet_path)
            if not isinstance(packet, dict):
                fail(errors, packet_path, "packet must be an object")
                continue
            rel = str(packet_path.relative_to(root))
            packets_by_rel[rel] = packet
            packet_paths_by_rel[rel] = packet_path
            kind = packet.get("packet_kind")
            schema = schemas.get(kind)
            if not schema:
                fail(errors, packet_path, f"unknown packet_kind: {kind!r}")
                continue
            validate_object(errors, packet_path, packet, schema)
            custom_validate(errors, root, packet_path, packet)

        cross_validate(errors, root, packets_by_rel, packet_paths_by_rel)

    status = "pass" if not errors else "fail"
    report = {
        "status": status,
        "checked": checked,
        "artifact_hashes": artifact_hashes,
        "errors": errors,
    }
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
