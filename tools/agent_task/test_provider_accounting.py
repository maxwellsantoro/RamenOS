#!/usr/bin/env python3
"""Offline synthetic assertions; no model/provider invocation or real release."""

import copy
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

from fixture_bank import create, release_record
from provider_accounting import (
    AccountingError,
    accounting_report,
    freeze_study,
    study_digest,
    validate_study,
)


class ProviderAccountingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.manifest = create(
            Path(self.temp.name) / "bank", b"synthetic-validator", seed=b"x" * 32,
            synthetic=True,
        )
        self.release = release_record(self.manifest, "pilot", "scripted-contract-test")
        self.ids = [i["instance_id"] for i in self.manifest["instances"]
                    if i["partition"] == "pilot"][:2]
        self.settings = dict(
            fixture_ids=self.ids, provider="synthetic-provider", model="synthetic-v1",
            rate_schedule_id="synthetic-rate-v1", input_usd_micros_per_million=3,
            output_usd_micros_per_million=7, context_policy_sha256="a" * 64,
            max_input_tokens=100, max_output_tokens=100, funded_ceiling_usd_micros=100,
        )
        self.study = freeze_study(self.manifest, self.release, **self.settings)

    def tearDown(self):
        self.temp.cleanup()

    def rows(self):
        return [dict(
            schema_version=1, accounting_study_sha256=study_digest(self.study),
            **a, outcome="completed", failure_reason=None, input_tokens=10,
            output_tokens=20, model_calls=1, visible_context_bytes=123,
            transcript_sha256="b" * 64,
        ) for a in self.study["attempts"]]

    def test_freeze_is_deterministic_and_copies_inputs(self):
        self.assertEqual(self.study, freeze_study(self.manifest, self.release, **self.settings))
        self.assertEqual(len(self.study["attempts"]), 6)
        self.assertEqual({a["arm"] for a in self.study["attempts"]}, {"RT", "LT", "LS"})
        self.assertEqual(len({a["attempt_id"] for a in self.study["attempts"]}), 6)
        self.assertEqual(self.study["bank_kind"], "synthetic-contract-bank")
        self.assertEqual(self.study["release"]["study_sha256"], "0" * 64)
        before = copy.deepcopy(self.study)
        self.ids.clear()
        self.release["purpose"] = "changed"
        self.assertEqual(self.study, before)
        self.assertEqual(len(study_digest(self.study)), 64)
        validate_study(self.study, self.manifest)

    def test_complete_report_counts_all_arms_and_integer_cost(self):
        rows = self.rows()
        report = accounting_report(self.study, self.manifest, rows)
        self.assertTrue(report["complete"])
        self.assertTrue(report["ceiling_certified"])
        self.assertEqual(report["planned_attempts"], 6)
        self.assertEqual(report["recorded_attempts"], 6)
        self.assertEqual(report["completed_attempts"], 6)
        self.assertEqual(report["cost_usd_micros"], 6)
        self.assertEqual(report["known_cost_lower_bound_usd_micros"], 6)
        self.assertEqual(report["known_input_tokens"], 60)
        self.assertEqual(report["known_output_tokens"], 120)
        self.assertEqual(report, accounting_report(self.study, self.manifest, rows[::-1]))
        rows[0]["input_tokens"] = 99
        self.assertEqual(report["records"][0]["input_tokens"], 10)

    def test_failure_and_unknown_usage_remain_in_denominator(self):
        rows = self.rows()
        rows[0].update(outcome="failed", failure_reason="provider-timeout",
                       input_tokens=10, output_tokens=None)
        report = accounting_report(self.study, self.manifest, rows)
        self.assertEqual(report["failed_attempts"], 1)
        self.assertEqual(report["recorded_attempts"], 6)
        self.assertEqual(report["unknown_usage_attempts"], 1)
        self.assertIsNone(report["cost_usd_micros"])
        self.assertFalse(report["ceiling_certified"])
        self.assertEqual(report["known_input_tokens"], 60)
        self.assertEqual(report["known_output_tokens"], 100)
        self.assertEqual(report["known_cost_lower_bound_usd_micros"], 6)
        self.assertIsNone(report["records"][0]["output_tokens"])
        rows[0].update(input_tokens=None)
        self.assertEqual(accounting_report(self.study, self.manifest, rows)
                         ["known_cost_lower_bound_usd_micros"], 5)

    def test_missing_rows_are_pending_never_certified(self):
        for rows in ([], self.rows()[:-1]):
            report = accounting_report(self.study, self.manifest, rows)
            self.assertFalse(report["complete"])
            self.assertFalse(report["ceiling_certified"])
            self.assertIsNone(report["cost_usd_micros"])
            self.assertEqual(len(report["pending_attempt_ids"]), 6 - len(rows))
            self.assertEqual(report["planned_attempts"], 6)

    def test_zero_call_failed_predispatch_is_retained(self):
        rows = self.rows()
        rows[0].update(outcome="failed", failure_reason="launch-failed", model_calls=0,
                       input_tokens=0, output_tokens=0)
        report = accounting_report(self.study, self.manifest, rows)
        self.assertEqual(report["failed_attempts"], 1)
        self.assertEqual(report["cost_usd_micros"], 5)
        self.assertTrue(report["ceiling_certified"])
        for mutate in (dict(input_tokens=None), dict(input_tokens=1), dict(outcome="completed", failure_reason=None)):
            invalid = copy.deepcopy(rows)
            invalid[0].update(mutate)
            with self.assertRaises(AccountingError):
                accounting_report(self.study, self.manifest, invalid)

    def test_over_budget_observations_are_retained(self):
        rows = self.rows()
        rows[0].update(outcome="failed", failure_reason="token-budget-exhausted",
                       input_tokens=101, output_tokens=102)
        report = accounting_report(self.study, self.manifest, rows)
        self.assertEqual(report["records"][0]["input_tokens"], 101)
        self.assertEqual(len(report["budget_breaches"]), 2)
        self.assertFalse(report["ceiling_certified"])
        self.assertEqual(report["failed_attempts"], 1)
        settings = dict(self.settings, funded_ceiling_usd_micros=5)
        study = freeze_study(self.manifest, self.release, **settings)
        rows = self.rows()
        for row in rows:
            row["accounting_study_sha256"] = study_digest(study)
        report = accounting_report(study, self.manifest, rows)
        self.assertFalse(report["ceiling_certified"])
        self.assertEqual(report["budget_breaches"][0]["kind"], "funded-ceiling")
        study = freeze_study(self.manifest, self.release,
                             **dict(self.settings, funded_ceiling_usd_micros=6))
        for row in rows:
            row["accounting_study_sha256"] = study_digest(study)
        self.assertTrue(accounting_report(study, self.manifest, rows)["ceiling_certified"])

    def test_exact_integer_rounding_and_zero_frozen_rates(self):
        study = freeze_study(self.manifest, self.release,
                             **dict(self.settings, input_usd_micros_per_million=0,
                                    output_usd_micros_per_million=0))
        rows = self.rows()
        for row in rows:
            row["accounting_study_sha256"] = study_digest(study)
        self.assertEqual(accounting_report(study, self.manifest, rows)["cost_usd_micros"], 0)
        rows[0]["output_tokens"] = None
        report = accounting_report(study, self.manifest, rows)
        self.assertIsNone(report["cost_usd_micros"])
        self.assertFalse(report["ceiling_certified"])
        # A whole million tokens at 3 micro USD costs exactly 3; no floats.
        study = freeze_study(self.manifest, self.release,
                             **dict(self.settings, max_input_tokens=1000000))
        rows = self.rows()
        for row in rows:
            row.update(accounting_study_sha256=study_digest(study), input_tokens=1000000,
                       output_tokens=0)
        self.assertEqual(accounting_report(study, self.manifest, rows)["cost_usd_micros"], 18)

    def test_strict_study_and_release_binding(self):
        for mutate in (
            lambda s: s.update(schema_version=True),
            lambda s: s.update(extra="unrecognized"),
            lambda s: s.update(provider=""),
            lambda s: s.update(max_input_tokens=-1),
            lambda s: s.update(output_usd_micros_per_million=1.2),
            lambda s: s.update(context_policy_sha256="invalid"),
            lambda s: s.update(bank_sha256="c" * 64),
            lambda s: s["release"].update(study_sha256="c" * 64),
            lambda s: s["release"].update(bank_commit="a" * 40),
            lambda s: s["release"].update(purpose="pilot-comparison"),
            lambda s: s["release"].update(partition="final"),
            lambda s: s["fixture_ids"].append(s["fixture_ids"][0]),
            lambda s: s["fixture_ids"].append(self.manifest["instances"][0]["instance_id"]),
            lambda s: s["attempts"][0].update(arm="LS"),
            lambda s: s["attempts"].pop(),
        ):
            study = copy.deepcopy(self.study)
            mutate(study)
            with self.assertRaises(AccountingError):
                validate_study(study, self.manifest)
        changed = dict(self.settings, rate_schedule_id="synthetic-rate-v2")
        self.assertNotEqual(study_digest(self.study), study_digest(
            freeze_study(self.manifest, self.release, **changed)))

    def test_malformed_freeze_inputs_fail_before_schedule_expansion(self):
        for settings in (
            dict(self.settings, fixture_ids=self.ids * 10000),
            dict(self.settings, fixture_ids=[{}]),
            dict(self.settings, fixture_ids=[]),
            dict(self.settings, fixture_ids=[self.ids[0], self.ids[0]]),
            dict(self.settings, input_usd_micros_per_million=1 << 63),
        ):
            with patch("provider_accounting._schedule") as schedule:
                with self.assertRaises(AccountingError):
                    freeze_study(self.manifest, self.release, **settings)
                schedule.assert_not_called()
        for manifest, release in (({}, self.release), (None, self.release),
                                  (self.manifest, {}), (self.manifest, None)):
            with self.assertRaises(AccountingError):
                freeze_study(manifest, release, **self.settings)

    def test_strict_rows_duplicates_and_wrong_study_are_denied(self):
        for mutate in (
            dict(schema_version=True), dict(extra=1), dict(input_tokens=True),
            dict(input_tokens=1.0), dict(output_tokens=-1), dict(model_calls=1 << 63),
            dict(visible_context_bytes=-1), dict(input_tokens=1 << 63),
            dict(accounting_study_sha256="c" * 64), dict(arm="other"),
            dict(instance_id="c" * 32), dict(attempt_id="c" * 64),
            dict(outcome="retry"), dict(failure_reason="unexpected"),
            dict(outcome="failed", failure_reason=None), dict(transcript_sha256="bad"),
        ):
            rows = self.rows()
            rows[0].update(mutate)
            with self.assertRaises(AccountingError):
                accounting_report(self.study, self.manifest, rows)
        with self.assertRaises(AccountingError):
            accounting_report(self.study, self.manifest, self.rows() + self.rows()[:1])


if __name__ == "__main__":
    unittest.main()
