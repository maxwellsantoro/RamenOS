#!/usr/bin/env python3
"""Check that RamenOS planning docs agree on the active track.

This is the first G0 governance check: catch the exact class of drift where an
agent-facing instruction file points at a stale "now" while NEXT_TASKS has moved
on.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


REQUIRED_FILES = [
    "AGENTS.md",
    "CURRENT_STATUS.md",
    "NEXT_TASKS.md",
    "ROADMAP.md",
    "docs/plans/2026-06-23-research-backed-ramenorg.md",
    "docs/plans/2026-06-23-g0-1-board-packet-validators.md",
    "docs/plans/2026-06-23-g0-2-active-task-cross-packet.md",
    "docs/plans/2026-06-23-g0-3-current-task-negative-fixtures.md",
    "docs/plans/2026-06-23-g0-3-1-governance-label-claim-boundary.md",
    "docs/plans/2026-06-23-g0-4-read-only-steward-heartbeat.md",
    "docs/plans/2026-06-23-g0-5-agent-intake-freshness-binding.md",
    "docs/plans/2026-06-23-g0-6-intake-only-agent-trial.md",
    "docs/plans/2026-06-23-g0-7-bounded-context-grant.md",
    "docs/plans/2026-06-23-g0-8-bounded-implementation-trial.md",
    "docs/plans/2026-06-23-g0-8-1-implementation-authority-serial-claim-hygiene.md",
    "docs/org/current_task.yaml",
    "docs/org/CURRENT_TASK_V0.md",
    "docs/org/ORG_CONSTITUTION.md",
    "docs/org/ROLE_CHARTER.md",
    "docs/org/AUTHORITY_LEVELS.md",
    "docs/org/HEARTBEATS.md",
    "docs/org/WORK_ORDER_V0.md",
    "docs/org/HANDOFF_PACKET_V0.md",
    "docs/org/BOARD_VOTE_V0.md",
    "docs/org/BOARD_PACKET_V0.md",
    "docs/org/BOARD_BRIEF_V0.md",
    "docs/org/INTAKE_BUNDLE_V0.md",
    "docs/org/CONTEXT_GRANT_V0.md",
    "docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md",
    "docs/org/trials/2026-06-23-g0-7-bounded-context-patch-plan.md",
    "docs/org/trials/2026-06-23-g0-8-bounded-implementation-trial.md",
    "docs/org/trials/2026-06-23-g0-8-1-authority-serial-claim-hygiene.md",
    "docs/org/CLAIM_SAFETY.md",
    "docs/research/INDEX.md",
    "docs/research/RESEARCH_PROGRAM.md",
    "docs/research/questions/RQ-0001-offer-boundaries.md",
    "docs/research/questions/RQ-0002-ai-org-kernel.md",
    "schemas/org/work_order_v0.schema.json",
    "schemas/org/handoff_packet_v0.schema.json",
    "schemas/org/board_vote_v0.schema.json",
    "schemas/org/board_packet_v0.schema.json",
    "schemas/org/current_task_v0.schema.json",
    "schemas/org/intake_manifest_v0.schema.json",
    "schemas/org/context_grant_v0.schema.json",
    "tools/org/render_board_packet.py",
    "tools/org/render_board_brief.py",
    "tools/org/render_context_grant.py",
    "tools/org/validate_context_grant.py",
    "tools/org/validate_intake_manifest.py",
    "tools/org/validate_packets.py",
    "tools/org/test_validate_packets.py",
    "tools/org/test_intake_bundle.py",
    "tools/org/test_context_grant.py",
    "docs/plans/2026-06-23-g0-9-first-a2-to-a3-loop.md",
    "docs/org/trials/2026-06-23-g0-9-first-a2-to-a3-loop.md",
    "docs/org/HUMAN_DIRECTIVE_V0.md",
    "docs/org/MERGE_GATE_V0.md",
    "docs/research/SLICE_NAMESPACING.md",
    "docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md",
    "schemas/org/human_directive_v0.schema.json",
    "schemas/org/merge_request_v0.schema.json",
    "tools/org/validate_human_directive.py",
    "tools/org/validate_merge.py",
    "tools/org/render_g0_9.py",
    "tools/org/test_validate_merge.py",
    "tools/org/test_validate_human_directive.py",
]


def read(root: Path, rel: str) -> str:
    return (root / rel).read_text(encoding="utf-8")


def first_match(pattern: str, text: str) -> str:
    match = re.search(pattern, text, flags=re.MULTILINE)
    return match.group(1).strip() if match else ""


def add_check(checks: list[dict[str, Any]], name: str, ok: bool, detail: str) -> None:
    checks.append({"name": name, "ok": ok, "detail": detail})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--out", help="optional JSON report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    checks: list[dict[str, Any]] = []

    for rel in REQUIRED_FILES:
        add_check(checks, f"required_file:{rel}", (root / rel).is_file(), rel)

    missing = [c["detail"] for c in checks if c["name"].startswith("required_file:") and not c["ok"]]
    if missing:
        report = {"status": "fail", "checks": checks, "missing": missing}
        if args.out:
            out_path = Path(args.out)
            out_path.parent.mkdir(parents=True, exist_ok=True)
            out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report, indent=2))
        return 1

    agents = read(root, "AGENTS.md")
    current = read(root, "CURRENT_STATUS.md")
    next_tasks = read(root, "NEXT_TASKS.md")
    roadmap = read(root, "ROADMAP.md")

    next_now = first_match(r"^\*\*Now:\*\*\s*(.+)$", next_tasks)
    agents_now = first_match(r"^- \*\*Now:\*\*\s*(.+)$", agents)

    add_check(
        checks,
        "next_tasks_now_mentions_hil_appliance",
        "HIL appliance" in next_now and "serial observer" in next_tasks,
        next_now,
    )
    add_check(
        checks,
        "agents_now_mentions_hil_appliance",
        "HIL appliance" in agents_now and "S13.2 virtio-blk Oracle capture" not in agents_now,
        agents_now,
    )
    add_check(
        checks,
        "current_status_mentions_active_hil_appliance",
        "S12.4" in current and "HIL appliance" in current,
        "CURRENT_STATUS.md active track should mention S12.4 HIL appliance",
    )
    add_check(
        checks,
        "roadmap_mentions_parallel_governance_track",
        "G0 Org Kernel" in roadmap and "Research Office" in roadmap,
        "ROADMAP.md should mention G0 Org Kernel and Research Office",
    )
    add_check(
        checks,
        "next_tasks_tracks_g0_and_research",
        "G0.8.1 implementation authority" in next_tasks and "RQ-0001" in next_tasks and "RQ-0002" in next_tasks,
        "NEXT_TASKS.md should track G0.8.1, RQ-0001, and RQ-0002",
    )
    current_task = read(root, "docs/org/current_task.yaml")
    add_check(
        checks,
        "current_task_yaml_tracks_s12_4_1",
        "task_id: S12.4.1" in current_task and "next_gate: just hil-appliance" in current_task,
        "docs/org/current_task.yaml should name S12.4.1 and just hil-appliance",
    )
    add_check(
        checks,
        "current_task_yaml_keeps_a2_local_boundary",
        "authority_level: A2" in current_task
        and "no merge, no release, no self-approval, no HIL actuation, and no public support authority" in current_task,
        "docs/org/current_task.yaml should keep A2-local/no-authority-expansion boundary",
    )

    stale_active = "Current active execution track (authoritative):\n- **Now:** S13.2 virtio-blk Oracle capture" in agents
    add_check(
        checks,
        "agents_no_stale_active_s13_2",
        not stale_active,
        "AGENTS.md active track must not point at completed S13.2",
    )

    # Slice namespacing (G0.9): research-bound slices must not reuse OS S-numbers.
    slices_dir = root / "docs/research/slices"
    if slices_dir.is_dir():
        for slice_file in sorted(slices_dir.glob("*.md")):
            text = slice_file.read_text(encoding="utf-8")
            heading = first_match(r"^# (.+)$", text)
            add_check(
                checks,
                f"research_slice_not_os_number:{slice_file.name}",
                not re.match(r"^S\d", heading),
                f"{slice_file.name} heading '{heading}' must use a research namespace, not an OS S-number",
            )
    offers_slice = read(root, "docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md")
    add_check(
        checks,
        "offers_airlock_is_r_offers_1",
        offers_slice.startswith("# R-OFFERS-1"),
        "offers airlock prototype must be namespaced R-OFFERS-1, not S12.0",
    )
    namespacing = read(root, "docs/research/SLICE_NAMESPACING.md")
    add_check(
        checks,
        "slice_namespacing_declares_three_namespaces",
        "`S##`" in namespacing and "`R-<PROGRAM>-<n>`" in namespacing and "`G#`" in namespacing,
        "SLICE_NAMESPACING.md must declare the S## / R-<PROGRAM>-<n> / G# namespaces",
    )

    status = "pass" if all(check["ok"] for check in checks) else "fail"
    report = {
        "status": status,
        "active_next_tasks": next_now,
        "active_agents": agents_now,
        "checks": checks,
    }

    if args.out:
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    print(json.dumps(report, indent=2))
    return 0 if status == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
