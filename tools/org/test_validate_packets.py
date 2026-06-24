#!/usr/bin/env python3
"""Negative tests for the RamenOrg packet validator."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable


PacketMap = dict[str, dict[str, Any]]


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"expected object: {path}")
    return payload


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def copy_case(root: Path, source_dir: Path, case_dir: Path) -> PacketMap:
    if case_dir.exists():
        shutil.rmtree(case_dir)
    case_dir.mkdir(parents=True)
    packets: PacketMap = {}
    for source in sorted(source_dir.glob("*.json")):
        target = case_dir / source.name
        shutil.copy2(source, target)
        packets[source.name] = load_json(target)

    rel_dir = case_dir.relative_to(root)
    for packet in packets.values():
        if packet.get("packet_kind") == "board_packet_v0":
            for field in ["work_order_refs", "handoff_refs", "vote_refs"]:
                packet[field] = [str(rel_dir / Path(ref).name) for ref in packet[field]]
    for name, payload in packets.items():
        write_json(case_dir / name, payload)
    return packets


def run_validator(root: Path, packet_dir: Path, current_task: str = "docs/org/current_task.yaml") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "tools/org/validate_packets.py",
            "--root",
            str(root),
            "--schema-dir",
            "schemas/org",
            "--packet-dir",
            str(packet_dir.relative_to(root)),
            "--current-task",
            current_task,
            "--current-task-schema",
            "schemas/org/current_task_v0.schema.json",
        ],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def packet_by_kind(packets: PacketMap, kind: str) -> dict[str, Any]:
    for packet in packets.values():
        if packet.get("packet_kind") == kind:
            return packet
    raise KeyError(kind)


def mutate_mismatched_sha(packets: PacketMap) -> None:
    packet_by_kind(packets, "work_order_v0")["repo_sha"] = "sha256:bad"


def mutate_missing_evidence(packets: PacketMap) -> None:
    vote = packet_by_kind(packets, "board_vote_v0")
    for refs in vote["evidence"].values():
        refs.clear()


def mutate_unknown_gate(packets: PacketMap) -> None:
    packet_by_kind(packets, "work_order_v0")["required_gates"] = ["hil-appliance"]
    packet_by_kind(packets, "handoff_packet_v0")["required_gates"] = ["hil-appliance"]


def mutate_a3_authority(packets: PacketMap) -> None:
    packet_by_kind(packets, "work_order_v0")["authority_level"] = "A3"
    packet_by_kind(packets, "board_packet_v0")["authority_level"] = "A3"


def mutate_stale_hil_claim(packets: PacketMap) -> None:
    packet_by_kind(packets, "work_order_v0")["task"] = "HIL serial observer without required proof constraint"
    packet_by_kind(packets, "work_order_v0")["constraints"] = [
        "Do not claim PASS/METAL from stale serial logs",
        "Pi GPIO UART is TTL-only; do not wire directly to RS-232",
    ]


def mutate_wrong_handoff_work_order(packets: PacketMap) -> None:
    packet_by_kind(packets, "handoff_packet_v0")["work_order_id"] = "WO-wrong"


def mutate_vote_proposal_mismatch(packets: PacketMap) -> None:
    packet_by_kind(packets, "board_vote_v0")["proposal_id"] = "WO-wrong"


def mutate_vote_sha_mismatch(packets: PacketMap) -> None:
    packet_by_kind(packets, "board_vote_v0")["repo_sha"] = "sha256:bad"


def mutate_too_many_refs(packets: PacketMap) -> None:
    board = packet_by_kind(packets, "board_packet_v0")
    board["work_order_refs"].append(board["work_order_refs"][0])


def mutate_missing_release_public_support_denial(packets: PacketMap) -> None:
    packet_by_kind(packets, "board_packet_v0")["claim_boundary"] = (
        "A2-local only; no merge and no HIL actuation"
    )


def mutate_pass_metal_without_hil_evidence(packets: PacketMap) -> None:
    packet_by_kind(packets, "work_order_v0")["claim_level_allowed"] = "PASS/METAL"
    packet_by_kind(packets, "board_vote_v0")["evidence"]["hil_evidence_refs"] = []


NEGATIVE_CASES: list[tuple[str, Callable[[PacketMap], None]]] = [
    ("mismatched_sha", mutate_mismatched_sha),
    ("missing_evidence", mutate_missing_evidence),
    ("unknown_gate_syntax", mutate_unknown_gate),
    ("a3_authority", mutate_a3_authority),
    ("stale_hil_claim", mutate_stale_hil_claim),
    ("wrong_handoff_work_order_id", mutate_wrong_handoff_work_order),
    ("vote_proposal_mismatch", mutate_vote_proposal_mismatch),
    ("vote_sha_mismatch", mutate_vote_sha_mismatch),
    ("too_many_work_order_refs", mutate_too_many_refs),
    ("missing_release_public_support_denial", mutate_missing_release_public_support_denial),
    ("pass_metal_without_hil_evidence", mutate_pass_metal_without_hil_evidence),
]


def write_case_packets(case_dir: Path, packets: PacketMap) -> None:
    for name, payload in packets.items():
        write_json(case_dir / name, payload)


def run_packet_negative_cases(root: Path, source_dir: Path, work_dir: Path) -> list[str]:
    failures: list[str] = []
    for name, mutate in NEGATIVE_CASES:
        case_dir = work_dir / name
        packets = copy_case(root, source_dir, case_dir)
        mutate(packets)
        write_case_packets(case_dir, packets)
        result = run_validator(root, case_dir)
        if result.returncode == 0:
            failures.append(f"{name}: validator accepted bad packets\n{result.stdout}")
        else:
            print(f"negative_case={name} rejected")
    return failures


def run_current_task_negative_case(root: Path, work_dir: Path) -> list[str]:
    failures: list[str] = []
    bad_task = work_dir / "bad_current_task_missing_active_task.yaml"
    source = root / "docs/org/current_task.yaml"
    lines = [
        line
        for line in source.read_text(encoding="utf-8").splitlines()
        if not line.startswith("active_task:")
    ]
    bad_task.write_text("\n".join(lines) + "\n", encoding="utf-8")
    result = subprocess.run(
        [
            sys.executable,
            "tools/org/validate_packets.py",
            "--root",
            str(root),
            "--schema-dir",
            "schemas/org",
            "--current-task",
            str(bad_task.relative_to(root)),
            "--current-task-schema",
            "schemas/org/current_task_v0.schema.json",
            "--skip-packets",
        ],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode == 0:
        failures.append(f"bad_current_task_missing_active_task: validator accepted bad current task\n{result.stdout}")
    else:
        print("negative_case=bad_current_task_missing_active_task rejected")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--packet-dir", default="out/org/examples")
    parser.add_argument("--work-dir", default="out/org/negative")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    source_dir = root / args.packet_dir
    work_dir = root / args.work_dir
    work_dir.mkdir(parents=True, exist_ok=True)

    failures = []
    failures.extend(run_packet_negative_cases(root, source_dir, work_dir))
    failures.extend(run_current_task_negative_case(root, work_dir))

    if failures:
        print("\n".join(failures))
        return 1
    print("negative_cases=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
