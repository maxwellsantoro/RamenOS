#!/usr/bin/env python3
"""A2.3: direct trusted-broker assertions, bypassing the shared JSON front end."""

import base64
import json
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch
import lt_backend
from lt_backend import LinuxTask, TaskError
from linux_sandbox import SandboxFailure

FIXTURE = Path(os.environ["RAMEN_TASK_LT_FIXTURE"])
WORKER = Path(os.environ["RAMEN_TASK_VALIDATOR_WORKER"])
OBSERVATIONS = []


class BackendTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "store"
        self.task = LinuxTask(FIXTURE, self.root, WORKER, 7)
        self.n = 1

    def tearDown(self):
        OBSERVATIONS.append(
            {
                "case": self.id(),
                "pins": self.task.pins,
                "worker": self.task.worker_id,
                "revision": self.task.state["revision"],
                "runs": self.task.state["runs"],
            }
        )
        self.task.close()
        self.tmp.cleanup()

    def call(self, operation, **fields):
        self.n += 1
        return self.task.execute(
            {
                "schema_version": 1,
                "request_id": str(self.n),
                "call": {"operation": operation, **fields},
            }
        )

    def grant(self, rights=None, lifetime=60000):
        return self.call(
            "request_grant",
            policy_cap=self.task.bootstrap["policy_cap"],
            task_id="17",
            resource="resource:0000000000000001",
            rights=rights or ["read", "stage", "validate", "commit", "observe"],
            lifetime_ms=lifetime,
        )["result"]["task_cap"]

    def stage(self, cap, data=b'{"enabled":true,"label":"keep"}'):
        return self.call(
            "stage_candidate",
            task_cap=cap,
            bytes_base64=base64.b64encode(data).decode(),
        )["result"]["candidate_cap"]

    def test_subscription_shapes_and_grant_binding_bypass_the_codec(self):
        cap = self.grant(["observe"])

        def invoke(call):
            return self.task.execute(
                {"schema_version": 2, "request_id": "1000", "call": call}
            )

        subscribe = dict(
            operation="subscribe_task", task_cap=cap, event_types=["output_changed"]
        )
        for invalid in [
            dict(subscribe, task_cap={}),
            dict(subscribe, event_types=[{}]),
            dict(subscribe, event_types=["output_changed", "output_changed"]),
        ]:
            self.assertEqual(invoke(invalid)["status"], "invalid")
            self.assertFalse(self.task.poisoned)
        sub = invoke(subscribe)["result"]["subscription_cap"]
        self.assertEqual(
            invoke(dict(operation="poll_task", task_cap=cap, subscription_cap=[]))[
                "status"
            ],
            "invalid",
        )
        self.assertFalse(self.task.poisoned)
        other = self.grant(["observe"])
        denied = invoke(
            dict(operation="poll_task", task_cap=other, subscription_cap=sub)
        )
        self.assertEqual((denied["status"], denied["result"]), ("denied", None))
        self.assertEqual(
            invoke(dict(operation="poll_task", task_cap=cap, subscription_cap=sub))[
                "result"
            ]["event_types"],
            [],
        )

    def test_direct_authority_expiry_foreign_context_and_sealing(self):
        cap = self.grant(["read"])
        for op, fields in [
            ("stage_candidate", {"bytes_base64": "e30="}),
            ("read_input", {"resource": "resource:00000000000003e7"}),
            ("get_receipt", {"commit_request_id": "99"}),
        ]:
            r = self.call(op, task_cap=cap, **fields)
            self.assertEqual((r["status"], r["result"]), ("denied", None))
        r = self.call(
            "read_input",
            task_cap=self.task.bootstrap["policy_cap"],
            resource="resource:0000000000000001",
        )
        self.assertEqual(r["status"], "denied")
        tiny = self.grant(["observe"], 1)
        time.sleep(0.005)
        self.assertEqual(
            self.call("get_task_state", task_cap=tiny)["status"], "expired"
        )
        self.task.close()
        self.task = LinuxTask(FIXTURE, self.root, WORKER, 8)
        r = self.call(
            "request_grant",
            policy_cap=self.task.bootstrap["policy_cap"],
            task_id="17",
            resource="resource:0000000000000001",
            rights=["read"],
            lifetime_ms=100,
        )
        self.assertEqual((r["status"], r["result"]), ("denied", None))

    def test_commit_validation_revision_retry_restart_and_revoke(self):
        cap = self.grant()
        state = self.call("get_task_state", task_cap=cap)["result"]["state"]
        candidate = self.stage(cap)
        fields = dict(
            task_cap=cap,
            candidate_cap=candidate,
            expected_revision="0",
            expected_content_id=state["content_id"],
        )
        self.assertEqual(
            self.call("commit_candidate", **fields)["status"], "validation_failed"
        )
        bad = self.stage(cap, b"{}")
        r = self.call(
            "validate_candidate",
            task_cap=cap,
            candidate_cap=bad,
            validator_id=state["validator_id"],
        )
        self.assertEqual(r["status"], "ok", (r, self.task.state["runs"]))
        self.assertEqual(r["result"]["outcome"], "invalid")
        self.assertEqual(
            self.call(
                "validate_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                validator_id="sha256:" + "0" * 64,
            )["status"],
            "denied",
        )
        self.assertEqual(
            self.call(
                "validate_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                validator_id=state["validator_id"],
            )["result"]["outcome"],
            "valid",
        )
        self.assertEqual(
            self.call("commit_candidate", **dict(fields, expected_revision="1"))[
                "status"
            ],
            "conflict",
        )
        req = {
            "schema_version": 1,
            "request_id": "100",
            "call": dict(operation="commit_candidate", **fields),
        }
        receipt = self.task.execute(req)
        self.assertEqual(receipt["status"], "ok")
        self.assertEqual(self.task.execute(req), receipt)
        changed = json.loads(json.dumps(req))
        changed["call"]["expected_revision"] = "1"
        self.assertEqual(self.task.execute(changed)["status"], "request_reuse")
        self.assertEqual(
            self.call("get_task_state", task_cap=cap)["result"]["state"]["revision"],
            "1",
        )
        # A second valid commit uses a smaller request ID. Recovery must replay
        # revisions, not lexical JSON key order; stale/ABA preconditions still fail.
        req2 = json.loads(json.dumps(req))
        req2["request_id"] = "3"
        req2["call"]["expected_revision"] = "1"
        req2["call"]["expected_content_id"] = receipt["result"]["content_id"]
        self.assertEqual(self.task.execute(req2)["result"]["revision"], "2")
        req3 = json.loads(json.dumps(req2))
        req3["request_id"] = "2"
        self.assertEqual(self.task.execute(req3)["status"], "conflict")
        self.call(
            "revoke_grant", policy_cap=self.task.bootstrap["policy_cap"], task_cap=cap
        )
        self.assertEqual(self.call("get_task_state", task_cap=cap)["status"], "denied")
        fresh = self.grant()
        new_request = json.loads(json.dumps(req2))
        new_request["request_id"] = "77"
        new_request["call"]["task_cap"] = fresh
        new_request["call"]["expected_revision"] = "2"
        self.assertEqual(self.task.execute(new_request)["status"], "validation_failed")
        self.task.close()
        self.task = LinuxTask(FIXTURE, self.root, WORKER, 7)
        renewed = self.grant(["commit"])
        req["call"]["task_cap"] = renewed
        self.assertEqual(self.task.execute(req), receipt)
        lookup = self.call("get_receipt", task_cap=renewed, commit_request_id="100")
        self.assertEqual(lookup["result"]["revision"], "1")
        self.assertTrue(all(r["removed"] for r in self.task.state["runs"]))
        # Existing receipt lookup cannot roll the accepted revision back.
        self.assertEqual(self.task.state["revision"], 2)
        self.task.close()
        journal = self.root / "journal.json"
        data = json.loads(journal.read_bytes())
        data["receipts"]["100"]["validation"]["outcome"] = "invalid"
        journal.write_text(json.dumps(data))
        with self.assertRaises(TaskError):
            LinuxTask(FIXTURE, self.root, WORKER, 7)

    def test_uncertain_journal_write_poison_and_explicit_recovery(self):
        cap = self.grant()
        candidate = self.stage(cap)
        state = self.call("get_task_state", task_cap=cap)["result"]["state"]
        r = self.call(
            "validate_candidate",
            task_cap=cap,
            candidate_cap=candidate,
            validator_id=state["validator_id"],
        )
        self.assertEqual(r["status"], "ok", r)
        req = {
            "schema_version": 1,
            "request_id": "55",
            "call": dict(
                operation="commit_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                expected_revision="0",
                expected_content_id=state["content_id"],
            ),
        }
        real_atomic = lt_backend.atomic

        def uncertain(path, data):
            real_atomic(path, data)
            if path.name == "journal.json":
                raise OSError("injected failure after replace")

        with patch("lt_backend.atomic", uncertain):
            self.assertEqual(self.task.execute(req)["status"], "io")
        self.assertEqual(self.call("get_task_state", task_cap=cap)["status"], "io")
        self.task.close()
        self.task = LinuxTask(FIXTURE, self.root, WORKER, 7)
        renewed = self.grant(["commit"])
        req["call"]["task_cap"] = renewed
        self.assertEqual(self.task.execute(req)["result"]["revision"], "1")
        self.assertEqual(self.task.state["revision"], 1)

    def test_unconfirmed_container_cleanup_requires_reconciliation(self):
        cap = self.grant()
        candidate = self.stage(cap)
        failure = SandboxFailure(
            "creation_not_confirmed",
            {"created": False, "removed": True, "injected": True},
        )
        with patch("lt_backend.Sandbox.run", side_effect=failure):
            response = self.call(
                "validate_candidate",
                task_cap=cap,
                candidate_cap=candidate,
                validator_id=self.task.pins["validator"],
            )
        self.assertEqual(response["status"], "io")
        self.assertEqual(self.call("get_task_state", task_cap=cap)["status"], "io")
        self.task.close()
        with self.assertRaises(TaskError):
            LinuxTask(FIXTURE, self.root, WORKER, 7)

    def test_writer_lock_tamper_and_malformed_direct_calls(self):
        with self.assertRaises((OSError, TaskError)):
            LinuxTask(FIXTURE, self.root, WORKER, 7)
        cap = self.grant()
        r = self.call("stage_candidate", task_cap=cap, bytes_base64="e30=", domain_id=7)
        self.assertEqual((r["status"], r["result"]), ("invalid", None))
        candidate = self.stage(cap)
        content = self.task.state["candidates"][candidate]["content_id"]
        self.task.blob_path(content).write_bytes(b"forged")
        r = self.call(
            "validate_candidate",
            task_cap=cap,
            candidate_cap=candidate,
            validator_id=self.task.pins["validator"],
        )
        self.assertEqual(r["status"], "io")
        self.assertEqual(self.call("get_task_state", task_cap=cap)["status"], "io")


if __name__ == "__main__":
    result = unittest.main(exit=False).result
    if not result.wasSuccessful():
        raise SystemExit(1)
    evidence = Path(os.environ["RAMEN_TASK_ADAPTER_EVIDENCE_DIR"])
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / "report.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "environment": "linux",
                "arm": "LT",
                "claim": "scripted-independent-linux-transactions",
                "fixture_partition": "development",
                "model_comparison": False,
                "a2_conformant": False,
                "target_kernel_enforcement": False,
                "direct_backend_cases": OBSERVATIONS,
            },
            indent=2,
        )
        + "\n"
    )
