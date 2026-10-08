#!/usr/bin/env python3
"""Gate-first: issued rights are finite projections, never full ambient envelopes."""

import copy
import unittest
from requestable_authority import RIGHTS, matrix_summary, host_canary_summary


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


def canary_sample():
    records, traces = [], {}
    resource = "resource:0000000000000001"
    for arm in ("RT", "LT", "LS"):
        actor = dict(role="contained-python-shell-consumer" if arm == "LS"
                     else "trusted-python-evaluator-host-consumer", pid=123,
                     uid=65534 if arm == "LS" else 1000, gid=65534 if arm == "LS" else 1000,
                     namespaces={n: n + ":[123]" for n in ("mnt", "pid", "net")})
        traces[arm] = []
        for phase, case, status in (
            ("before_expiry", "short_read", "ok"),
            ("after_expiry", "expired_first_read", "expired"),
            ("after_revocation", "revoked_renewed_read", "denied"),
        ):
            records.append(dict(schema_version=1, arm=arm, phase=phase, actor=copy.deepcopy(actor),
                                backend_case=case, outcome="blocked" if arm == "LS" else "allowed",
                                content_sha256=None if arm == "LS" else "a" * 64,
                                denial_errno=2 if arm == "LS" else None))
            cap = "short" if phase != "after_revocation" else "renewed"
            traces[arm].append(dict(case=case, request=dict(schema_version=1, request_id=case,
                                                          call=dict(operation="read_input", task_cap=cap, resource=resource)),
                                    response=dict(schema_version=1, request_id=case, status=status,
                                                  result={"bytes_base64": "Y29uZmln"} if status == "ok" else None),
                                    elapsed_ms=1))
        for case, operation, result in (
            ("short_grant", "request_grant", dict(task_cap="short", expires_at_ms="500", generation="1")),
            ("expiry_clock", "get_task_state", dict(state=dict(now_ms="501"))),
            ("generation_revoke", "revoke_grant", dict(generation="2")),
        ):
            fields = dict(operation=operation)
            if case == "short_grant":
                fields.update(policy_cap="policy", task_id="17", resource=resource,
                              rights=list(RIGHTS), lifetime_ms=500)
            if case == "generation_revoke": fields["task_cap"] = "renewed"
            traces[arm].append(dict(case=case,
                                    request=dict(schema_version=1, request_id=case, call=fields),
                                    response=dict(schema_version=1, request_id=case, status="ok", result=result), elapsed_ms=1))
        for case, cap, generation in (("renewed_grant", "renewed", "1"),
                                      ("policy_after_revocation", "fresh", "2")):
            traces[arm].append(dict(case=case,
                                    request=dict(schema_version=1, request_id=case,
                                                 call=dict(operation="request_grant", policy_cap="policy",
                                                           task_id="17", resource=resource,
                                                           rights=list(RIGHTS) if case == "renewed_grant" else ["read", "observe"],
                                                           lifetime_ms=60000)),
                                    response=dict(schema_version=1, request_id=case, status="ok",
                                                  result=dict(task_cap=cap, generation=generation)), elapsed_ms=1))
        for case, cap in (("renewed_read", "renewed"), ("fresh_generation_read", "fresh")):
            traces[arm].append(dict(case=case,
                                    request=dict(schema_version=1, request_id=case,
                                                 call=dict(operation="read_input", task_cap=cap, resource=resource)),
                                    response=dict(schema_version=1, request_id=case, status="ok",
                                                  result={"bytes_base64": "Y29uZmln"}), elapsed_ms=2))
    return records, traces


class HostCanaryTests(unittest.TestCase):
    def test_named_consumer_difference_keeps_whole_inclusion_unknown(self):
        rows, traces = canary_sample()
        report = host_canary_summary(rows, "a" * 64, traces)
        self.assertEqual(report["observations"], 9)
        self.assertEqual(report["phases"], ["before_expiry", "after_expiry", "after_revocation"])
        self.assertEqual(report["allowed_arms"], ["RT", "LT"])
        self.assertEqual(report["blocked_arms"], ["LS"])
        self.assertEqual(report["whole_authority_relation"], "unknown")
        self.assertFalse(report["continuous_envelope_certified"])
        self.assertFalse(report["adapter_authority_measured"])
        self.assertFalse(report["model_interface_authority_measured"])
        self.assertEqual(report, host_canary_summary(rows[::-1], "a" * 64, traces))

    def test_missing_duplicate_malformed_actor_or_canary_fail_closed(self):
        for fault in ("missing", "duplicate", "hash", "errno", "pid", "role", "ns", "extra", "bool", "actor_change", "leak"):
            rows, traces = canary_sample()
            if fault == "missing": rows.pop()
            elif fault == "duplicate": rows[-1] = copy.deepcopy(rows[0])
            elif fault == "hash": rows[0]["content_sha256"] = "b" * 64
            elif fault == "errno": rows[-1]["denial_errno"] = 5
            elif fault == "pid": rows[0]["actor"]["pid"] = 0
            elif fault == "role": rows[0]["actor"]["role"] = "rt-adapter"
            elif fault == "ns": rows[0]["actor"]["namespaces"]["mnt"] = "unknown"
            elif fault == "extra": rows[0]["host_io"] = "inferred"
            elif fault == "bool": rows[0]["schema_version"] = True
            elif fault == "actor_change": rows[0]["actor"]["pid"] = 999
            else: rows[-1]["content_sha256"] = "a" * 64
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                host_canary_summary(rows, "a" * 64, traces)

    def test_grant_caps_and_revocation_transitions_are_bound(self):
        for case, where, field, altered in (
            ("short_grant", "result", "task_cap", "unrelated-short"),
            ("short_read", "call", "task_cap", "unrelated-active"),
            ("revoked_renewed_read", "call", "task_cap", "unrelated-revoked"),
            ("renewed_grant", "result", "task_cap", "unrelated-renewed"),
            ("generation_revoke", "call", "task_cap", "unrelated-revoke-target"),
            ("policy_after_revocation", "result", "task_cap", "unrelated-fresh"),
            ("fresh_generation_read", "call", "task_cap", "unrelated-live-cap"),
            ("policy_after_revocation", "result", "generation", "1"),
            ("renewed_grant", "result", "generation", "2"),
        ):
            rows, traces = canary_sample()
            item = next(item for item in traces["RT"] if item["case"] == case)
            item["response" if where == "result" else "request"][where][field] = altered
            with self.subTest(case=case, field=field), self.assertRaises(ValueError):
                host_canary_summary(rows, "a" * 64, traces)

    def test_read_resources_are_bound_to_each_grant_and_positive_witness(self):
        for arm in ("RT", "LT", "LS"):
            for case in ("short_grant", "renewed_grant", "policy_after_revocation",
                         "short_read", "expired_first_read", "renewed_read",
                         "revoked_renewed_read", "fresh_generation_read"):
                for altered in ("resource:0000000000000999", None, True):
                    rows, traces = canary_sample()
                    item = next(item for item in traces[arm] if item["case"] == case)
                    item["request"]["call"]["resource"] = altered
                    with self.subTest(arm=arm, case=case, resource=altered), self.assertRaises(ValueError):
                        host_canary_summary(rows, "a" * 64, traces)
            # Matching active/expired off-resource probes must not evade the
            # binding to the actual issued short-grant resource.
            rows, traces = canary_sample()
            for item in traces[arm]:
                if item["case"] in ("short_read", "expired_first_read"):
                    item["request"]["call"]["resource"] = "resource:0000000000000999"
            with self.subTest(arm=arm, case="paired-off-resource"), self.assertRaises(ValueError):
                host_canary_summary(rows, "a" * 64, traces)

    def test_renewed_read_must_succeed_with_the_actual_renewed_capability(self):
        for arm in ("RT", "LT", "LS"):
            for fault in ("missing", "denied", "wrong_cap"):
                rows, traces = canary_sample()
                item = next(item for item in traces[arm] if item["case"] == "renewed_read")
                if fault == "missing":
                    traces[arm].remove(item)
                elif fault == "denied":
                    item["response"].update(status="denied", result=None)
                else:
                    item["request"]["call"]["task_cap"] = "unrelated-renewed"
                with self.subTest(arm=arm, fault=fault), self.assertRaises(ValueError):
                    host_canary_summary(rows, "a" * 64, traces)

    def test_missing_actual_backend_denial_redaction_or_positive_witness_fails(self):
        for fault in ("redaction", "false_denial", "missing", "positive", "request_cap", "response_id", "clock", "generation"):
            rows, traces = canary_sample()
            if fault == "redaction": traces["RT"][1]["response"]["result"] = {}
            elif fault == "false_denial": traces["LT"][2]["response"]["status"] = "ok"
            elif fault == "missing": traces["LS"].pop(1)
            elif fault == "positive": traces["RT"][-1]["response"]["status"] = "denied"
            elif fault == "request_cap": traces["RT"][1]["request"]["call"]["task_cap"] = "different"
            elif fault == "clock": traces["RT"][4]["response"]["result"]["state"]["now_ms"] = "1"
            elif fault == "generation": traces["RT"][5]["response"]["result"]["generation"] = "1"
            else: traces["RT"][0]["response"]["request_id"] = "different"
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                host_canary_summary(rows, "a" * 64, traces)


if __name__ == "__main__":
    unittest.main()
