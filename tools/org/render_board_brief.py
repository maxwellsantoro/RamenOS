#!/usr/bin/env python3
"""Render a human-readable RamenOrg board brief from validated packets."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


EVIDENCE_BUCKETS = [
    "design_evidence_refs",
    "gate_evidence_refs",
    "claim_evidence_refs",
    "hil_evidence_refs",
    "release_evidence_refs",
]


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"expected JSON object: {path}")
    return payload


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def resolve_repo_path(root: Path, rel: str) -> Path:
    if not rel or Path(rel).is_absolute():
        raise ValueError(f"artifact path must be repository-relative: {rel!r}")
    path = (root / rel).resolve()
    try:
        path.relative_to(root)
    except ValueError as exc:
        raise ValueError(f"artifact path escapes repository root: {rel}") from exc
    return path


def require_validation_pass(path: Path) -> dict[str, Any]:
    report = load_json(path)
    if report.get("status") != "pass":
        raise ValueError(f"packet validation report is not pass: {path}")
    return report


def verify_validation_freshness(root: Path, packet_dir: Path, report: dict[str, Any]) -> None:
    checked = report.get("checked")
    hashes = report.get("artifact_hashes")
    if not isinstance(checked, list) or not checked or not all(isinstance(item, str) for item in checked):
        raise ValueError("validation report checked list is missing or invalid")
    if not isinstance(hashes, dict):
        raise ValueError("validation report artifact_hashes map is missing")

    checked_set = set(checked)
    current_packets = {
        str(path.relative_to(root)) for path in sorted(packet_dir.glob("*.json"))
    }
    if not current_packets:
        raise ValueError(f"no packet artifacts found in {packet_dir}")
    if not current_packets.issubset(checked_set):
        stale = sorted(current_packets - checked_set)
        raise ValueError(f"packet artifacts were not validated: {', '.join(stale)}")

    for rel in checked:
        expected = hashes.get(rel)
        if not isinstance(expected, str):
            raise ValueError(f"validation report has no sha256 for {rel}")
        path = resolve_repo_path(root, rel)
        if not path.is_file():
            raise ValueError(f"validated artifact is missing: {rel}")
        actual = sha256_file(path)
        if actual != expected:
            raise ValueError(f"validated artifact hash mismatch: {rel}")


def verify_context_grant_freshness(root: Path, grant: dict[str, Any]) -> None:
    entries = grant.get("granted_context")
    if not isinstance(entries, list) or not entries:
        raise ValueError("context grant has no granted_context entries")
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("context grant entry must be an object")
        rel = entry.get("path")
        expected = entry.get("sha256")
        if not isinstance(rel, str) or not isinstance(expected, str):
            raise ValueError("context grant entry must include path and sha256")
        path = resolve_repo_path(root, rel)
        if not path.is_file():
            raise ValueError(f"granted context is missing: {rel}")
        if sha256_file(path) != expected:
            raise ValueError(f"granted context hash mismatch: {rel}")


def find_board_packet(packet_dir: Path) -> tuple[Path, dict[str, Any]]:
    boards: list[tuple[Path, dict[str, Any]]] = []
    for path in sorted(packet_dir.glob("*.json")):
        packet = load_json(path)
        if packet.get("packet_kind") == "board_packet_v0":
            boards.append((path, packet))
    if len(boards) != 1:
        raise ValueError(f"expected exactly one board packet in {packet_dir}, found {len(boards)}")
    return boards[0]


def require_one_ref(packet: dict[str, Any], field: str) -> str:
    refs = packet.get(field)
    if not isinstance(refs, list) or len(refs) != 1 or not isinstance(refs[0], str):
        raise ValueError(f"{field} must contain exactly one string ref")
    return refs[0]


def md_list(items: list[Any], empty: str = "- none") -> list[str]:
    if not items:
        return [empty]
    return [f"- {item}" for item in items]


def evidence_lines(vote: dict[str, Any]) -> list[str]:
    evidence = vote.get("evidence", {})
    if not isinstance(evidence, dict):
        evidence = {}
    lines: list[str] = []
    for bucket in EVIDENCE_BUCKETS:
        lines.append(f"### {bucket}")
        refs = evidence.get(bucket, [])
        lines.extend(md_list(refs if isinstance(refs, list) else []))
        lines.append("")
    return lines


def render_brief(
    validation_report_rel: str,
    validation_status: str,
    current_task_rel: str,
    context_grant_rel: str,
    context_grant: dict[str, Any],
    board_rel: str,
    board: dict[str, Any],
    work_order_rel: str,
    work_order: dict[str, Any],
    handoff_rel: str,
    handoff: dict[str, Any],
    vote_rel: str,
    vote: dict[str, Any],
) -> str:
    lines = [
        "# Current Board Brief",
        "",
        "Generated from validated RamenOrg packets. This brief is read-only and grants no authority beyond the referenced work order.",
        "",
        "## Intake Binding",
        f"- Validation report: {validation_report_rel}",
        f"- Packet validation status: {validation_status}",
        f"- Current task ref: {current_task_rel}",
        f"- Context grant: {context_grant_rel}",
        f"- Repo SHA: {board.get('repo_sha', '')}",
        f"- Board packet ref: {board_rel}",
        f"- Work order ref: {work_order_rel}",
        f"- Handoff ref: {handoff_rel}",
        f"- Vote ref: {vote_rel}",
        "",
        "## Active Task",
        f"- Active track: {board.get('active_track', '')}",
        f"- Active task: {board.get('active_task', '')}",
        f"- Repo SHA: {board.get('repo_sha', '')}",
        f"- Board packet: {board_rel}",
        f"- Work order: {work_order_rel}",
        f"- Handoff: {handoff_rel}",
        f"- Vote: {vote_rel}",
        "",
        "## Authority Boundary",
        f"- Authority level: {board.get('authority_level', '')}",
        f"- Claim level allowed: {work_order.get('claim_level_allowed', '')}",
        f"- Claim boundary: {board.get('claim_boundary', '')}",
        "- Allowed authority: read packets, follow the bounded work order, and report gate evidence.",
        "- Forbidden authority: no merge, no release, no self-approval, no HIL actuation, and no public support authority.",
        "",
        "## Required Gates",
        f"- Next gate: {board.get('next_gate', '')}",
    ]
    lines.extend(md_list(work_order.get("required_gates", [])))
    lines.extend(
        [
            "",
            "## Context Refs",
            *md_list(board.get("context_refs", [])),
            "",
            "## Granted Context",
            f"- Context grant: {context_grant_rel}",
            *[
                f"- [{entry.get('access', '')}] {entry.get('path', '')} | {entry.get('sha256', '')}"
                for entry in context_grant.get("granted_context", [])
            ],
            "### Authorized New Paths",
            *md_list(context_grant.get("authorized_new_paths", [])),
            "",
            "## Not Granted / Out of Scope",
            *md_list(context_grant.get("not_granted", [])),
            f"- Expansion policy: {context_grant.get('context_expansion_policy', '')}",
            "",
            "## Evidence Refs",
            *evidence_lines(vote),
            "## Handoff",
            f"- From role: {handoff.get('from_role', '')}",
            f"- To role: {handoff.get('to_role', '')}",
            f"- Requested output: {handoff.get('requested_output', '')}",
            f"- Rollback plan: {work_order.get('rollback_plan', '')}",
            "",
            "## Allowed Next-Agent Actions",
            f"- Act as {handoff.get('to_role', '')} for this work order only.",
            f"- Work only on: {work_order.get('task', '')}.",
            f"- Stay within scope: {', '.join(str(item) for item in work_order.get('scope', []))}.",
            "- Use the context refs and evidence refs above before changing files.",
            "- Run the required gates and report their result.",
            "- Preserve the authority boundary and claim level.",
            "- Use only granted context; request missing files with context_expansion_request.",
            "- Do not merge, release, actuate HIL hardware, or make public support claims.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--packet-dir", default="out/org/examples")
    parser.add_argument("--validation-report", default="out/org/packet_validation.json")
    parser.add_argument("--context-grant", default="out/org/context_grant.json")
    parser.add_argument("--out", default="out/org/current_board_brief.md")
    parser.add_argument("--manifest-out", default="out/org/intake_manifest.json")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    packet_dir = root / args.packet_dir
    validation_report_path = resolve_repo_path(root, args.validation_report)
    report = require_validation_pass(validation_report_path)
    verify_validation_freshness(root, packet_dir, report)

    board_path, board = find_board_packet(packet_dir)
    board_rel = str(board_path.relative_to(root))
    work_order_rel = require_one_ref(board, "work_order_refs")
    handoff_rel = require_one_ref(board, "handoff_refs")
    vote_rel = require_one_ref(board, "vote_refs")
    current_task_rel = board.get("current_task_ref")
    if not isinstance(current_task_rel, str) or not current_task_rel:
        raise ValueError("board current_task_ref must be a non-empty string")

    required_refs = {board_rel, work_order_rel, handoff_rel, vote_rel, current_task_rel}
    checked_refs = set(report.get("checked", []))
    missing_refs = sorted(required_refs - checked_refs)
    if missing_refs:
        raise ValueError(f"intake artifact refs were not validated: {', '.join(missing_refs)}")

    work_order = load_json(resolve_repo_path(root, work_order_rel))
    handoff = load_json(resolve_repo_path(root, handoff_rel))
    vote = load_json(resolve_repo_path(root, vote_rel))
    context_grant = load_json(resolve_repo_path(root, args.context_grant))
    verify_context_grant_freshness(root, context_grant)
    if context_grant.get("repo_sha") != board.get("repo_sha"):
        raise ValueError("context grant repo_sha must match board packet")
    if context_grant.get("work_order_id") != work_order.get("work_order_id"):
        raise ValueError("context grant work_order_id must match work order")
    if context_grant.get("authority_level") != work_order.get("authority_level"):
        raise ValueError("context grant authority_level must match work order")

    brief = render_brief(
        args.validation_report,
        str(report["status"]),
        current_task_rel,
        args.context_grant,
        context_grant,
        board_rel,
        board,
        work_order_rel,
        work_order,
        handoff_rel,
        handoff,
        vote_rel,
        vote,
    )
    out_path = resolve_repo_path(root, args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(brief, encoding="utf-8")

    # Close the read/render window before binding the output bundle.
    verify_validation_freshness(root, packet_dir, report)
    verify_context_grant_freshness(root, context_grant)
    report_hashes = report["artifact_hashes"]

    def artifact(path_rel: str, validated: bool = False) -> dict[str, str]:
        digest = report_hashes[path_rel] if validated else sha256_file(resolve_repo_path(root, path_rel))
        return {"path": path_rel, "sha256": digest}

    manifest = {
        "schema_version": 1,
        "manifest_kind": "intake_manifest_v0",
        "repo_sha": board.get("repo_sha", ""),
        "packet_validation_status": report["status"],
        "artifacts": {
            "brief": artifact(args.out),
            "board_packet": artifact(board_rel, validated=True),
            "work_order": artifact(work_order_rel, validated=True),
            "handoff": artifact(handoff_rel, validated=True),
            "vote": artifact(vote_rel, validated=True),
            "current_task": artifact(current_task_rel, validated=True),
            "validation_report": artifact(args.validation_report),
            "context_grant": artifact(args.context_grant),
        },
        "granted_context": context_grant["granted_context"],
        "authorized_new_paths": context_grant.get("authorized_new_paths", []),
    }
    manifest_path = resolve_repo_path(root, args.manifest_out)
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    print(
        json.dumps(
            {
                "status": "pass",
                "brief": args.out,
                "board_packet": board_rel,
                "intake_manifest": args.manifest_out,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
