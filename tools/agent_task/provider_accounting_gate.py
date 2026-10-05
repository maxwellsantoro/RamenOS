#!/usr/bin/env python3
"""Deterministic offline consumer of synthetic provider-accounting records."""

import argparse
import copy
import hashlib
import tempfile
from pathlib import Path

from fixture_bank import create, encoded, release_record
from provider_accounting import accounting_report, freeze_study, study_digest


def run():
    with tempfile.TemporaryDirectory(prefix="ramen-synthetic-accounting-") as temp:
        manifest = create(
            Path(temp) / "bank", b"synthetic-accounting-validator-not-executable",
            seed=b"s" * 32, synthetic=True,
        )
        release = release_record(manifest, "pilot", "scripted-contract-test")
        fixture_ids = [i["instance_id"] for i in manifest["instances"]
                       if i["partition"] == "pilot"][:2]
        study = freeze_study(
            manifest, release, fixture_ids=fixture_ids, provider="synthetic-provider",
            model="synthetic-model-v1", rate_schedule_id="synthetic-uncached-rate-v1",
            input_usd_micros_per_million=3, output_usd_micros_per_million=7,
            context_policy_sha256=hashlib.sha256(b"synthetic-context-policy-v1").hexdigest(),
            max_input_tokens=100, max_output_tokens=100, funded_ceiling_usd_micros=6,
        )
        rows = [dict(
            schema_version=1, accounting_study_sha256=study_digest(study), **attempt,
            outcome="completed", failure_reason=None, input_tokens=10, output_tokens=20,
            model_calls=1, visible_context_bytes=123,
            transcript_sha256=hashlib.sha256(b"synthetic-visible-transcript").hexdigest(),
        ) for attempt in study["attempts"]]
        rows[1].update(outcome="failed", failure_reason="synthetic-provider-failure")
        complete = accounting_report(study, manifest, rows)
        unknown_rows = copy.deepcopy(rows)
        unknown_rows[2].update(outcome="failed", failure_reason="synthetic-unknown-usage",
                               output_tokens=None)
        unknown = accounting_report(study, manifest, unknown_rows)
        over_rows = copy.deepcopy(rows)
        over_rows[0].update(outcome="failed", failure_reason="synthetic-token-budget",
                            input_tokens=101)
        over_budget = accounting_report(study, manifest, over_rows)
        incomplete = accounting_report(study, manifest, rows[:-1])
        # Give a known cost ceiling violation its own retained case.
        expensive_rows = copy.deepcopy(rows)
        expensive_rows[0].update(outcome="failed", failure_reason="synthetic-cost-budget",
                                 input_tokens=1000000)
        funded_violation = accounting_report(study, manifest, expensive_rows)
        checks = {
            "complete_three_arm_schedule": complete["complete"] and complete["recorded_attempts"] == 6
                and {r["arm"] for r in complete["records"]} == {"RT", "LT", "LS"},
            "failed_row_retained": complete["failed_attempts"] == 1,
            "known_ceiling_estimate": complete["ceiling_certified"] and complete["cost_usd_micros"] == 6,
            "unknown_usage_retained": unknown["unknown_usage_attempts"] == 1
                and unknown["records"][2]["output_tokens"] is None
                and unknown["cost_usd_micros"] is None and not unknown["ceiling_certified"],
            "token_breach_retained": over_budget["records"][0]["input_tokens"] == 101
                and bool(over_budget["budget_breaches"]) and not over_budget["ceiling_certified"],
            "cost_breach_retained": any(b["kind"] == "funded-ceiling"
                for b in funded_violation["budget_breaches"]) and not funded_violation["ceiling_certified"],
            "incomplete_schedule_pending": not incomplete["complete"]
                and len(incomplete["pending_attempt_ids"]) == 1
                and incomplete["cost_usd_micros"] is None and not incomplete["ceiling_certified"],
            "deterministic_record_order": complete == accounting_report(study, manifest, rows[::-1]),
        }
        if not all(checks.values()):
            raise RuntimeError("synthetic accounting assertion failed")
        source_hashes = {
            name: hashlib.sha256((Path(__file__).parent / name).read_bytes()).hexdigest()
            for name in ("provider_accounting.py", "provider_accounting_gate.py",
                         "test_provider_accounting.py", "fixture_bank.py")
        }
        return dict(
            schema_version=1, result="PASS", evidence_scope="offline-synthetic-accounting",
            synthetic=True, model_calls_performed=0, network_calls_performed=0,
            claims=dict(real_hidden_bank=False, real_study_release=False, model_comparison=False,
                        provider_attestation=False, provider_billing=False, funded_execution=False,
                        linux_controls=False, full_a2=False),
            source_sha256=source_hashes, accounting_study_sha256=study_digest(study),
            study=study, checks=checks,
            cases=dict(complete_with_failure=complete, unknown_usage=unknown,
                       token_budget_breach=over_budget, funded_ceiling_breach=funded_violation,
                       incomplete_schedule=incomplete),
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    report = run()
    args.evidence.mkdir(parents=True, exist_ok=True)
    (args.evidence / "report.json").write_bytes(encoded(report) + b"\n")
    print("PASS offline synthetic provider accounting; Linux controls/reconciliation INCOMPLETE")


if __name__ == "__main__":
    main()
