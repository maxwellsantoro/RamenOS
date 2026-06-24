#!/usr/bin/env python3
"""Negative freshness tests for the RamenOrg agent intake bundle."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"expected JSON object: {path}")
    return payload


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def run(command: list[str], root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--packet-dir", default="out/org/examples")
    parser.add_argument("--work-dir", default="out/org/intake-negative")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    source_dir = root / args.packet_dir
    work_dir = root / args.work_dir
    packet_dir = work_dir / "packets"
    if work_dir.exists():
        shutil.rmtree(work_dir)
    shutil.copytree(source_dir, packet_dir)

    packet_rel_dir = packet_dir.relative_to(root)
    work_order_path: Path | None = None
    for path in sorted(packet_dir.glob("*.json")):
        packet = load_json(path)
        if packet.get("packet_kind") == "board_packet_v0":
            for field in ["work_order_refs", "handoff_refs", "vote_refs"]:
                packet[field] = [str(packet_rel_dir / Path(packet[field][0]).name)]
            write_json(path, packet)
        elif packet.get("packet_kind") == "work_order_v0":
            work_order_path = path

    if work_order_path is None:
        print("freshness_test=fail reason=missing_work_order")
        return 1

    report_rel = str((work_dir / "packet_validation.json").relative_to(root))
    validation = run(
        [
            sys.executable,
            "tools/org/validate_packets.py",
            "--root",
            str(root),
            "--schema-dir",
            "schemas/org",
            "--packet-dir",
            str(packet_rel_dir),
            "--current-task",
            "docs/org/current_task.yaml",
            "--current-task-schema",
            "schemas/org/current_task_v0.schema.json",
            "--out",
            report_rel,
        ],
        root,
    )
    if validation.returncode != 0:
        print(validation.stdout)
        print("freshness_test=fail reason=fixture_validation_failed")
        return 1

    work_order_path.write_bytes(work_order_path.read_bytes() + b"\n")
    render = run(
        [
            sys.executable,
            "tools/org/render_board_brief.py",
            "--root",
            str(root),
            "--packet-dir",
            str(packet_rel_dir),
            "--validation-report",
            report_rel,
            "--out",
            str((work_dir / "brief.md").relative_to(root)),
            "--manifest-out",
            str((work_dir / "manifest.json").relative_to(root)),
        ],
        root,
    )
    if render.returncode == 0 or "validated artifact hash mismatch" not in render.stdout:
        print(render.stdout)
        print("freshness_test=fail reason=stale_report_accepted")
        return 1

    print("negative_case=stale_validation_report rejected")
    print("intake_freshness_negative=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
