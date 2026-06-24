#!/usr/bin/env python3
"""Negative cases for the A3 merge gate (MergeRequestV0).

Each case builds a fixture merge request and asserts the expected pass/fail.
Exit 0 only when every expectation holds (good passes, each bad is rejected).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from validate_merge import validate_merge_request

SHA = "0123456789abcdef0123456789abcdef01234567"


def write(path: Path, obj: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2) + "\n", encoding="utf-8")


def work_order(wid: str, requires: list[str] | None = None) -> dict[str, Any]:
    obj: dict[str, Any] = {
        "schema_version": 1,
        "packet_kind": "work_order_v0",
        "work_order_id": wid,
        "repo_sha": SHA,
        "role": "Implementer",
        "authority_level": "A2",
        "task": "namespace research slices away from OS slices and add a drift check",
        "scope": ["docs/research/SLICE_NAMESPACING.md"],
        "context_refs": ["docs/research/SLICE_NAMESPACING.md"],
        "constraints": ["no authority expansion"],
        "required_gates": ["just foundry-org-governance-g0"],
        "claim_level_allowed": "scaffold",
        "rollback_plan": "revert",
    }
    if requires is not None:
        obj["requires_rq"] = requires
    return obj


def vote(vid: str, proposal: str, approve: bool = True) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "packet_kind": "board_vote_v0",
        "vote_id": vid,
        "proposal_id": proposal,
        "repo_sha": SHA,
        "role": "Reviewer",
        "vote": "approve" if approve else "reject",
        "claim_checked": "scope and authority preserved",
        "evidence": {
            "design_evidence_refs": ["docs/research/SLICE_NAMESPACING.md"],
            "gate_evidence_refs": [],
            "claim_evidence_refs": [],
            "hil_evidence_refs": [],
            "release_evidence_refs": [],
        },
        "blocking_conditions": [],
    }


def merge_request(
    mid: str,
    wo_ref: str,
    vote_ref: str,
    *,
    implementer: str = "Implementer",
    reviewer: str = "Reviewer",
    gate_result: str = "PASS",
    boundary: str = (
        "A3 conditional merge of scaffold docs only; no release, no hardware actuation, "
        "and no public support authority; LOOP-LOCAL until configured"
    ),
    honest_outcome: str = "PASS/LOOP-LOCAL",
    protection: bool = False,
    credentials: bool = False,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "packet_kind": "merge_request_v0",
        "merge_request_id": mid,
        "repo_sha": SHA,
        "work_order_ref": wo_ref,
        "vote_ref": vote_ref,
        "implementer_role": implementer,
        "reviewer_role": reviewer,
        "required_gates": ["just foundry-org-governance-g0"],
        "gate_results": {"results": {"just foundry-org-governance-g0": gate_result}},
        "claim_boundary": boundary,
        "remote_merge_precondition": {
            "branch_protection_configured": protection,
            "merge_credentials_available": credentials,
            "honest_outcome": honest_outcome,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--work-dir", default="out/org/merge-negative")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    work = root / args.work_dir
    work_rel = args.work_dir

    write(work / "work_order_good.json", work_order("WO-GOOD"))
    write(work / "vote_good.json", vote("BV-GOOD", "WO-GOOD"))
    write(work / "work_order_rq.json", work_order("WO-RQ", requires=["RQ-0001"]))
    write(work / "vote_rq.json", vote("BV-RQ", "WO-RQ"))

    cases: list[tuple[str, dict[str, Any], bool]] = [
        ("good", merge_request("MR-GOOD", f"{work_rel}/work_order_good.json", f"{work_rel}/vote_good.json"), False),
        ("same_role", merge_request("MR-SAME", f"{work_rel}/work_order_good.json", f"{work_rel}/vote_good.json", implementer="Implementer", reviewer="Implementer"), True),
        ("gate_not_pass", merge_request("MR-GATE", f"{work_rel}/work_order_good.json", f"{work_rel}/vote_good.json", gate_result="FAIL"), True),
        ("overclaim_merge", merge_request("MR-OVER", f"{work_rel}/work_order_good.json", f"{work_rel}/vote_good.json", honest_outcome="PASS/MERGE"), True),
        ("missing_denial", merge_request("MR-DENIAL", f"{work_rel}/work_order_good.json", f"{work_rel}/vote_good.json", boundary="merge docs only; LOOP-LOCAL"), True),
        ("requires_open_rq", merge_request("MR-RQ", f"{work_rel}/work_order_rq.json", f"{work_rel}/vote_rq.json"), True),
    ]

    results: list[dict[str, Any]] = []
    all_ok = True
    for name, mr, expect_error in cases:
        mr_path = work / f"{name}.json"
        write(mr_path, mr)
        errors = validate_merge_request(root, mr_path)
        got_error = bool(errors)
        ok = got_error == expect_error
        all_ok = all_ok and ok
        results.append({"case": name, "expect_error": expect_error, "got_error": got_error, "ok": ok, "errors": errors})

    print(json.dumps({"status": "pass" if all_ok else "fail", "cases": results}, indent=2))
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
