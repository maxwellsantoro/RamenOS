#!/usr/bin/env python3
"""Validate a MergeRequestV0 against the A3 conditional-merge preconditions.

This is the closure of the work loop: WorkOrder -> implementer -> reviewer ->
gates -> evidence -> board vote -> merge. It enforces separation of duties,
evidence-bearing votes, green required gates, research-blocks-implementation,
and honest LOOP-LOCAL vs PASS/MERGE labelling. See docs/org/MERGE_GATE_V0.md.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

from validate_packets import (
    evidence_refs,
    fail,
    gate_exists,
    is_doctrine_text,
    load_json,
    rq_supports_implementation,
    sha256_file,
    validate_object,
)

# A3 still denies release, hardware actuation, and public support authority.
MERGE_DENIALS = ["no release", "no public support"]


def load_packet(root: Path, rel: str) -> dict[str, Any] | None:
    target = root / rel
    if not target.is_file():
        return None
    loaded = load_json(target)
    return loaded if isinstance(loaded, dict) else None


def validate_merge_request(root: Path, path: Path, errors: list[str] | None = None) -> list[str]:
    errors = [] if errors is None else errors
    schema = load_json(root / "schemas/org/merge_request_v0.schema.json")
    mr = load_json(path)
    validate_object(errors, path, mr, schema)

    work_order = load_packet(root, mr.get("work_order_ref", ""))
    vote = load_packet(root, mr.get("vote_ref", ""))
    if work_order is None:
        fail(errors, path, f"work_order_ref not found: {mr.get('work_order_ref')}")
    if vote is None:
        fail(errors, path, f"vote_ref not found: {mr.get('vote_ref')}")

    # 1. Separation of duties.
    if mr.get("implementer_role") == mr.get("reviewer_role"):
        fail(errors, path, "implementer_role must differ from reviewer_role (separation of duties)")

    # repo_sha agreement across the merge request and its referenced packets.
    if work_order is not None and vote is not None:
        shas = {mr.get("repo_sha"), work_order.get("repo_sha"), vote.get("repo_sha")}
        if len(shas) != 1:
            fail(errors, path, "repo_sha must match across merge request, work order, and vote")

    # 2. Evidence-bearing board vote, proposal id agreement.
    if vote is not None:
        if vote.get("vote") != "approve":
            fail(errors, path, "referenced vote must be approve")
        elif not evidence_refs(vote):
            fail(errors, path, "approve vote requires at least one typed evidence ref")
        if work_order is not None and vote.get("proposal_id") != work_order.get("work_order_id"):
            fail(errors, path, "vote.proposal_id must equal work_order.work_order_id")

    # 3. Required gates exist and are reported PASS.
    results = (mr.get("gate_results") or {}).get("results", {})
    for gate in mr.get("required_gates", []):
        if not gate_exists(root, gate):
            fail(errors, path, f"required gate does not exist: {gate}")
        elif str(results.get(gate, "")).upper() != "PASS":
            fail(errors, path, f"required gate not reported PASS: {gate}")

    # 4. Research blocks implementation (mirrors work_order enforcement).
    if work_order is not None:
        requires = work_order.get("requires_rq") or []
        doctrine_blob = " ".join(
            [work_order.get("task", ""), *work_order.get("scope", []), *work_order.get("constraints", [])]
        )
        is_doctrine = bool(work_order.get("doctrine_area")) or is_doctrine_text(doctrine_blob)
        if is_doctrine and not requires:
            fail(errors, path, "doctrine-level work requires non-empty requires_rq")
        for rq in requires:
            if not rq_supports_implementation(root, rq):
                fail(errors, path, f"requires_rq not satisfied (open research blocks merge): {rq}")

    # 5. Claim boundary + honest outcome.
    boundary = str(mr.get("claim_boundary", "")).lower()
    for denial in MERGE_DENIALS:
        if denial not in boundary:
            fail(errors, path, f"claim_boundary missing A3 denial: {denial}")
    precondition = mr.get("remote_merge_precondition") or {}
    configured = bool(precondition.get("branch_protection_configured")) and bool(
        precondition.get("merge_credentials_available")
    )
    honest = str(precondition.get("honest_outcome", ""))
    if not configured and honest != "PASS/LOOP-LOCAL":
        fail(errors, path, "honest_outcome must be PASS/LOOP-LOCAL without branch protection + credentials")
    if configured and honest != "PASS/MERGE":
        fail(errors, path, "honest_outcome must be PASS/MERGE once branch protection + credentials are configured")

    return errors


def _origin_repo(root: Path) -> str | None:
    try:
        url = subprocess.check_output(
            ["git", "-C", str(root), "remote", "get-url", "origin"], text=True
        ).strip()
    except Exception:
        return None
    match = re.search(r"github\.com[:/]([^/]+)/([^/]+?)(?:\.git)?$", url)
    return f"{match.group(1)}/{match.group(2)}" if match else None


def _gh_api(repo: str, path: str) -> tuple[Any, str]:
    proc = subprocess.run(
        ["gh", "api", f"repos/{repo}/{path}"], capture_output=True, text=True
    )
    if proc.returncode != 0:
        return None, (proc.stderr or proc.stdout).strip()
    try:
        return json.loads(proc.stdout), ""
    except json.JSONDecodeError as exc:
        return None, f"invalid JSON: {exc}"


def verify_remote(root: Path, pr_number: str, errors: list[str]) -> None:
    """Read ground truth from GitHub instead of trusting self-reported booleans.

    Confirms `main` enforces a pull_request (approvals) rule and a
    required_status_checks rule, and that PR ``pr_number`` has an APPROVED review
    from an actor distinct from the PR author (native separation of duties).
    """
    repo = _origin_repo(root)
    if not repo:
        errors.append("verify-remote: could not resolve origin repo")
        return
    rules, err = _gh_api(repo, "rules/branches/main")
    if rules is None:
        errors.append(f"verify-remote: could not fetch branch rules for main: {err}")
        return
    rule_types = {r.get("type") for r in rules} if isinstance(rules, list) else set()
    if "pull_request" not in rule_types:
        errors.append("verify-remote: main has no pull_request rule (approvals not required)")
    if "required_status_checks" not in rule_types:
        errors.append("verify-remote: main has no required_status_checks rule (CI not required)")
    pr, err = _gh_api(repo, f"pulls/{pr_number}")
    if pr is None:
        errors.append(f"verify-remote: could not fetch PR {pr_number}: {err}")
        return
    author = (pr.get("user") or {}).get("login", "")
    reviews, err = _gh_api(repo, f"pulls/{pr_number}/reviews")
    if reviews is None:
        errors.append(f"verify-remote: could not fetch reviews for PR {pr_number}: {err}")
        return
    distinct_approval = any(
        (r.get("state") == "APPROVED")
        and (r.get("user") or {}).get("login", "").lower() != author.lower()
        for r in reviews
    )
    if not distinct_approval:
        errors.append(
            f"verify-remote: PR {pr_number} has no approval from an actor distinct from author '{author}'"
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".")
    parser.add_argument("--merge-request", required=True)
    parser.add_argument("--out", help="optional JSON report path")
    parser.add_argument(
        "--verify-remote",
        action="store_true",
        help="verify protection + distinct approval on GitHub instead of trusting self-reported booleans",
    )
    parser.add_argument("--pr", help="PR number required by --verify-remote")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    mr_path = root / args.merge_request
    errors: list[str] = []
    if not mr_path.is_file():
        errors.append(f"missing merge request: {mr_path}")
    else:
        validate_merge_request(root, mr_path, errors)

    if args.verify_remote:
        if not args.pr:
            errors.append("verify-remote requires --pr")
        else:
            verify_remote(root, args.pr, errors)

    status = "pass" if not errors else "fail"
    report: dict[str, Any] = {
        "status": status,
        "merge_request": args.merge_request,
        "errors": errors,
    }
    if args.verify_remote and status == "pass":
        report["verified_outcome"] = "PASS/MERGE"
    if mr_path.is_file():
        report["hash"] = sha256_file(mr_path)
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
