#!/usr/bin/env python3
"""Fail-closed evidence assertions, independent of backend implementations."""

import copy
import unittest
from authority_manifest import build_manifest, compare, validate


class EvidenceTests(unittest.TestCase):
    def sample(self):
        return build_manifest(
            "RT",
            [
                {
                    "case": "read",
                    "phase": "granted",
                    "elapsed_ms": 1,
                    "tuple_id": "config.read.grant",
                    "outcome": "allowed",
                    "forbidden": False,
                    "channel": "task",
                    "status": "ok",
                }
            ],
            {"source": "test", "runtime": {"kind": "trusted-host-client"}},
        )

    def test_unknowns_block_inclusion_claims(self):
        manifest = self.sample()
        validate(manifest)
        self.assertEqual(compare(manifest, manifest)["relation"], "unknown")
        self.assertFalse(manifest["narrower_claim_eligible"])

    def test_a_forbidden_success_invalidates_mapping(self):
        with self.assertRaises(ValueError):
            build_manifest(
                "RT",
                [
                    {
                        "case": "private",
                        "phase": "granted",
                        "elapsed_ms": 1,
                        "tuple_id": "workspace_b.read",
                        "outcome": "allowed",
                        "forbidden": True,
                        "channel": "probe",
                        "status": "ok",
                    }
                ],
                {},
            )

    def test_available_authority_requires_observation_and_attempts_are_not_effects(
        self,
    ):
        m = self.sample()
        bad = copy.deepcopy(m)
        bad["entries"]["workspace_b.read"]["availability"] = "available"
        with self.assertRaises(ValueError):
            validate(bad)
        m = build_manifest(
            "RT",
            [
                {
                    "case": "denial",
                    "phase": "granted",
                    "elapsed_ms": 1,
                    "tuple_id": "workspace_b.read",
                    "outcome": "blocked",
                    "forbidden": True,
                    "channel": "probe",
                    "status": "denied",
                }
            ],
            {},
        )
        self.assertEqual(m["exercised"], [])
        self.assertEqual(m["forbidden_probes"], {"attempts": 1, "successful": 0})

    def test_tampered_universe_times_and_provenance_fail_closed(self):
        for fault in (
            "tuple",
            "time",
            "provenance",
            "unknown",
            "fields",
            "version_type",
        ):
            m = self.sample()
            if fault == "tuple":
                m["entries"]["config.read.grant"]["tuple"]["scope"] = "*"
            elif fault == "time":
                m["observations"][0]["elapsed_ms"] = -1
            elif fault == "provenance":
                m["entries"]["config.read.grant"]["probe_refs"] = ["missing"]
            elif fault == "unknown":
                m["unknown"] = []
            elif fault == "version_type":
                m["schema_version"] = True
            else:
                m["secret"] = "unrecognized"
            with self.assertRaises(ValueError):
                validate(m)

    def test_probe_effects_are_separate_from_task_effects_and_status_is_checked(self):
        m = self.sample()
        o = m["observations"]
        o[0]["channel"] = "probe"
        m = build_manifest("RT", o, {})
        self.assertEqual(m["exercised"], [])
        self.assertEqual(m["probe_exercised"], ["config.read.grant"])
        o[0]["status"] = "denied"
        with self.assertRaises(ValueError):
            build_manifest("RT", o, {})


if __name__ == "__main__":
    unittest.main()
