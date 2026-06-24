#!/usr/bin/env python3
"""Render read-only RamenOrg board packets from docs/org/current_task.yaml."""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Any


def git_sha(root: Path) -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    except Exception:
        return "unknown"


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def clear_generated_packets(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for path in out_dir.glob("*.json"):
        path.unlink()


def strip_quotes(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def parse_scalar(value: str) -> Any:
    value = strip_quotes(value)
    if value.isdigit():
        return int(value)
    return value


def load_current_task(path: Path) -> dict[str, Any]:
    data: dict[str, Any] = {}
    current_list: str | None = None
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.rstrip()
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line.startswith("  - "):
            if current_list is None:
                raise ValueError(f"list item without key in {path}: {line}")
            data[current_list].append(strip_quotes(line[4:]))
            continue
        if line.startswith(" "):
            raise ValueError(f"unsupported indentation in {path}: {line}")
        key, sep, value = line.partition(":")
        if not sep:
            raise ValueError(f"expected key: value in {path}: {line}")
        key = key.strip()
        value = value.strip()
        if value:
            data[key] = parse_scalar(value)
            current_list = None
        else:
            data[key] = []
            current_list = key
    return data


def require_scalar(config: dict[str, Any], key: str) -> str:
    value = config.get(key)
    if not isinstance(value, str) or not value:
        raise ValueError(f"current task missing scalar key: {key}")
    return value


def require_list(config: dict[str, Any], key: str) -> list[str]:
    value = config.get(key)
    if not isinstance(value, list) or not value:
        raise ValueError(f"current task missing non-empty list key: {key}")
    return [str(item) for item in value]


def optional_list(config: dict[str, Any], key: str) -> list[str]:
    value = config.get(key)
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"current task key must be a list: {key}")
    return [str(item) for item in value]


def claim_entries(config: dict[str, Any]) -> list[dict[str, str]]:
    entries = []
    for item in require_list(config, "claims"):
        claim, sep, source = item.partition("|")
        if not sep:
            raise ValueError(f"claim entries must use 'claim | source': {item}")
        entries.append({"claim": claim.strip(), "source": source.strip()})
    return entries


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--current-task", default="docs/org/current_task.yaml")
    parser.add_argument("--out-dir", default="out/org/examples", help="example packet output directory")
    parser.add_argument(
        "--board-out",
        default="out/org/current_board_packet.json",
        help="copy of the current board packet",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    current_task_rel = args.current_task
    current_task = load_current_task(root / current_task_rel)
    out_dir = root / args.out_dir
    clear_generated_packets(out_dir)
    now_ms = int(time.time() * 1000)
    sha = git_sha(root)
    slug = require_scalar(current_task, "slug")

    work_order_rel = f"{args.out_dir}/work_order_{slug}.json"
    handoff_rel = f"{args.out_dir}/handoff_planner_to_implementer_{slug}.json"
    vote_rel = f"{args.out_dir}/board_vote_foundry_evidence_{slug}.json"
    board_rel = f"{args.out_dir}/board_packet_{slug}.json"
    work_order_id = require_scalar(current_task, "work_order_id")
    active_task = require_scalar(current_task, "active_task")
    required_gates = require_list(current_task, "required_gates")
    constraints = require_list(current_task, "constraints")
    context_refs = require_list(current_task, "context_refs")

    work_order = {
        "schema_version": 1,
        "packet_kind": "work_order_v0",
        "work_order_id": work_order_id,
        "repo_sha": sha,
        "role": require_scalar(current_task, "work_order_role"),
        "authority_level": require_scalar(current_task, "authority_level"),
        "task": active_task,
        "scope": require_list(current_task, "scope"),
        "context_refs": context_refs,
        "constraints": constraints,
        "required_gates": required_gates,
        "claim_level_allowed": require_scalar(current_task, "claim_level_allowed"),
        "rollback_plan": require_scalar(current_task, "rollback_plan"),
    }

    handoff = {
        "schema_version": 1,
        "packet_kind": "handoff_packet_v0",
        "handoff_id": require_scalar(current_task, "handoff_id"),
        "work_order_id": work_order_id,
        "from_role": require_scalar(current_task, "from_role"),
        "to_role": require_scalar(current_task, "to_role"),
        "repo_sha": sha,
        "task": active_task,
        "context_refs": context_refs,
        "claims": claim_entries(current_task),
        "constraints": constraints,
        "requested_output": require_scalar(current_task, "requested_output"),
        "required_gates": required_gates,
    }

    vote = {
        "schema_version": 1,
        "packet_kind": "board_vote_v0",
        "vote_id": require_scalar(current_task, "vote_id"),
        "proposal_id": work_order_id,
        "repo_sha": sha,
        "role": require_scalar(current_task, "vote_role"),
        "vote": "approve",
        "claim_checked": f"{active_task} packets are valid G0.8.1 scaffold artifacts",
        "evidence": {
            "design_evidence_refs": optional_list(current_task, "design_evidence_refs"),
            "gate_evidence_refs": optional_list(current_task, "gate_evidence_refs"),
            "claim_evidence_refs": optional_list(current_task, "claim_evidence_refs"),
            "hil_evidence_refs": optional_list(current_task, "hil_evidence_refs"),
            "release_evidence_refs": optional_list(current_task, "release_evidence_refs"),
        },
        "blocking_conditions": [],
    }

    board_packet = {
        "schema_version": 1,
        "packet_kind": "board_packet_v0",
        "packet_id": require_scalar(current_task, "packet_id"),
        "generated_at_unix_ms": now_ms,
        "repo_sha": sha,
        "authority_level": require_scalar(current_task, "authority_level"),
        "current_task_ref": current_task_rel,
        "active_track": require_scalar(current_task, "active_track"),
        "active_task": active_task,
        "next_gate": require_scalar(current_task, "next_gate"),
        "parallel_tracks": require_list(current_task, "parallel_tracks"),
        "context_refs": context_refs,
        "work_order_refs": [work_order_rel],
        "handoff_refs": [handoff_rel],
        "vote_refs": [vote_rel],
        "claim_boundary": require_scalar(current_task, "claim_boundary"),
    }

    write_json(root / work_order_rel, work_order)
    write_json(root / handoff_rel, handoff)
    write_json(root / vote_rel, vote)
    write_json(root / board_rel, board_packet)
    write_json(root / args.board_out, board_packet)

    print(
        json.dumps(
            {
                "status": "pass",
                "current_task": current_task_rel,
                "board_packet": board_rel,
                "out_dir": args.out_dir,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
