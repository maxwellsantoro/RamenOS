#!/usr/bin/env python3
"""Gate-first: issued rights are finite projections, never full ambient envelopes."""

import copy
import unittest
from requestable_authority import RIGHTS, matrix_summary


def sample(policy):
    rows = []
    for mask in range(1, 32):
        rights = [r for n, r in enumerate(RIGHTS) if mask & (1 << n)]
        granted = mask & ~policy == 0
        rows.append(
            dict(
                mask=mask,
                requested=rights,
                lifetime_ms=60000,
                status="ok" if granted else "denied",
                issued=rights if granted else None,
            )
        )
    return rows


class ProjectionTests(unittest.TestCase):
    def test_full_and_attenuated_policy_keep_unexercised_rights(self):
        full = matrix_summary(sample(31), 31)
        narrow = matrix_summary(sample(17), 17)
        self.assertEqual(full["issued_masks"], list(range(1, 32)))
        self.assertEqual(narrow["issued_masks"], [1, 16, 17])
        self.assertIn("validator.execute", full["requestable_projection"])
        self.assertNotIn("validator.execute", narrow["requestable_projection"])
        self.assertEqual(full["data_effects_from_issuance"], [])
        self.assertEqual(full["whole_authority_relation"], "unknown")
        self.assertFalse(full["continuous_envelope_certified"])

    def test_missing_duplicate_or_malformed_requests_fail_closed(self):
        for fault in (
            "missing",
            "duplicate",
            "mask_bool",
            "extra",
            "life",
            "wrong_right",
        ):
            rows = sample(31)
            if fault == "missing":
                rows.pop()
            elif fault == "duplicate":
                rows[-1] = copy.deepcopy(rows[0])
            elif fault == "mask_bool":
                rows[0]["mask"] = True
            elif fault == "extra":
                rows[0]["ambient"] = []
            elif fault == "life":
                rows[0]["lifetime_ms"] = True
            else:
                rows[0]["requested"] = ["observe"]
            with self.assertRaises(ValueError):
                matrix_summary(rows, 31)

    def test_escalation_redaction_or_false_denial_invalidates_projection(self):
        for fault in ("escalation", "disclosure", "extra_right", "false_denial"):
            rows = sample(17)
            if fault == "escalation":
                rows[1].update(status="ok", issued=["stage"])
            elif fault == "disclosure":
                rows[1]["issued"] = []
            elif fault == "extra_right":
                rows[0]["issued"].append("commit")
            else:
                rows[0].update(status="denied", issued=None)
            with self.assertRaises(ValueError):
                matrix_summary(rows, 17)


if __name__ == "__main__":
    unittest.main()
