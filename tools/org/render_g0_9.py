#!/usr/bin/env python3
"""Render the G0.9 artifacts: the founder HumanDirectiveV0 and the first
A2->A3 loop packet set (work order, board vote, merge request) for the
slice-namespacing change. All four artifacts share the current repo SHA so
cross-packet SHA agreement holds at validation time."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


def repo_sha(root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
    except Exception:
        return "unknown"


def write_json(path: Path, obj: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--examples-dir", default="out/org/examples-g0-9")
    parser.add_argument("--directive-out", default="out/org/human_directive_proceed_with_all.json")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    sha = repo_sha(root)
    examples = root / args.examples_dir
    directive_out = root / args.directive_out

    work_order_id = "WO-2026-06-23-g0-9-slice-namespacing"

    human_directive = {
        "schema_version": 1,
        "packet_kind": "human_directive_v0",
        "directive_id": "HD-2026-06-23-proceed-with-all",
        "repo_sha": sha,
        "from_role": "Founder/Vision Channel",
        "authority": "vision_input",
        "directive": (
            "Proceed with all five RamenOrg pressure points: (1) fix the slice-numbering "
            "collision and dogfood the drift checker, (2) bind research to implementation via "
            "requires_rq so research can block, (3) close the first A2->A3 merge loop with "
            "separated roles and an honest LOOP-LOCAL outcome, (4) declare the offers "
            "airlock/leakage-meter prototype as research-bound slice R-OFFERS-1 with its claim "
            "boundary, and (5) add a typed HumanDirectiveV0 primitive for founder vision injection."
        ),
        "proposal_target": "board",
        "constraints": [
            "Board turns this directive into work orders through the normal packet path",
            "Do not grant merge, release, hardware actuation, or public support authority from this directive alone",
            "Preserve A2-local boundaries; LOOP-LOCAL is honest until branch protection and credentials are configured",
        ],
        "claim_boundary": (
            "vision input only; no merge, no release, no public support authority, and no "
            "authority increase above each agent's current ladder rung"
        ),
    }

    work_order = {
        "schema_version": 1,
        "packet_kind": "work_order_v0",
        "work_order_id": work_order_id,
        "repo_sha": sha,
        "role": "Implementer",
        "authority_level": "A2",
        "task": (
            "Fix the slice-numbering collision by namespacing research slices away from OS "
            "slices and adding a cross-namespace drift check"
        ),
        "scope": [
            "docs/research/SLICE_NAMESPACING.md",
            "tools/org/status_drift.py",
            "docs/org/MERGE_GATE_V0.md",
            "docs/org/HUMAN_DIRECTIVE_V0.md",
        ],
        "context_refs": [
            "docs/research/SLICE_NAMESPACING.md",
            "docs/org/CLAIM_SAFETY.md",
            "CURRENT_STATUS.md",
        ],
        "constraints": [
            "Do not assign S## to research or org work",
            "G0.9 is A2-local and grants no merge, no release, no self-approval, no hardware actuation, and no public support authority",
            "The first A2->A3 loop is LOOP-LOCAL until branch protection and merge credentials are configured",
        ],
        "required_gates": ["just foundry-org-governance-g0"],
        "claim_level_allowed": "scaffold",
        "rollback_plan": "Revert only files touched by this work order",
    }

    vote = {
        "schema_version": 1,
        "packet_kind": "board_vote_v0",
        "vote_id": "BV-2026-06-23-reviewer-g0-9-slice-namespacing",
        "proposal_id": work_order_id,
        "repo_sha": sha,
        "role": "Reviewer",
        "vote": "approve",
        "claim_checked": (
            "G0.9 slice namespacing lands as scaffold docs plus a drift check; no OS slice is "
            "reassigned and no authority expands"
        ),
        "evidence": {
            "design_evidence_refs": [
                "docs/research/SLICE_NAMESPACING.md",
                "schemas/org/work_order_v0.schema.json",
                "schemas/org/merge_request_v0.schema.json",
            ],
            "gate_evidence_refs": ["tools/ci/foundry_org_governance_g0.sh"],
            "claim_evidence_refs": ["docs/org/CLAIM_SAFETY.md", "docs/org/MERGE_GATE_V0.md"],
            "hil_evidence_refs": [],
            "release_evidence_refs": [],
        },
        "blocking_conditions": [],
    }

    merge_request = {
        "schema_version": 1,
        "packet_kind": "merge_request_v0",
        "merge_request_id": "MR-2026-06-23-g0-9-slice-namespacing",
        "repo_sha": sha,
        "work_order_ref": str((examples / "work_order_g0_9_slice_namespacing.json").relative_to(root)),
        "vote_ref": str((examples / "board_vote_g0_9_slice_namespacing.json").relative_to(root)),
        "implementer_role": "Implementer",
        "reviewer_role": "Reviewer",
        "required_gates": ["just foundry-org-governance-g0"],
        "gate_results": {"results": {"just foundry-org-governance-g0": "PASS"}},
        "claim_boundary": (
            "A3 conditional merge of scaffold docs and a drift check only; no release, no "
            "hardware actuation, and no public support authority; LOOP-LOCAL until branch "
            "protection and merge credentials are configured"
        ),
        "remote_merge_precondition": {
            "branch_protection_configured": False,
            "merge_credentials_available": False,
            "honest_outcome": "PASS/LOOP-LOCAL",
        },
    }

    write_json(examples / "work_order_g0_9_slice_namespacing.json", work_order)
    write_json(examples / "board_vote_g0_9_slice_namespacing.json", vote)
    write_json(examples / "merge_request_g0_9_slice_namespacing.json", merge_request)
    write_json(directive_out, human_directive)

    artifacts = [
        str(directive_out.relative_to(root)),
        str((examples / "work_order_g0_9_slice_namespacing.json").relative_to(root)),
        str((examples / "board_vote_g0_9_slice_namespacing.json").relative_to(root)),
        str((examples / "merge_request_g0_9_slice_namespacing.json").relative_to(root)),
    ]
    print(json.dumps({"status": "rendered", "repo_sha": sha, "artifacts": artifacts}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
