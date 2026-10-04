#!/usr/bin/env python3
"""Portable transaction semantics; no Linux containment claim."""
import base64
import tempfile
import unittest
from pathlib import Path
from lt_backend import LinuxTask, TaskError, digest


class StagingTests(unittest.TestCase):
    def test_last_observation_freshness_does_not_authorize_commit(self):
        task = LinuxTask.__new__(LinuxTask)
        task.subscriptions = {}
        task.grants = {}
        task.now = lambda: 10
        task.state = {"generation": "1", "revision": 0, "content_id": digest(b"input")}
        task.pins = {name: digest(name.encode()) for name in ("schema", "policy", "validator")}
        task.bootstrap = {"resources": []}
        task.load = lambda content_id: b"input"
        task.authorize = lambda cap, right: {"bits": 31, "expires": 100}
        for outcome, truncated, wall, guest in [
            ("valid", False, 0, 0), ("invalid", False, 0, 0),
            ("timeout", False, 0, 0), ("host_failure", False, 0, 0),
            ("valid", True, 0, 0), ("valid", False, 100000, 100000),
        ]:
            record = dict(generation="1", valid_until_ms="11", outcome=outcome,
                          diagnostics_truncated=truncated,
                          wall_elapsed_ms=str(wall), guest_elapsed_ms=str(guest))
            task.state["last_validation"] = record
            def state():
                return task.dispatch("1", {"operation": "get_task_state", "task_cap": "cap:0000000000000001"})["state"]
            self.assertTrue(state()["validation_current"])
            self.assertEqual(task.valid(record), outcome == "valid" and not truncated and wall == 0)
            record["valid_until_ms"] = "10"
            self.assertFalse(state()["validation_current"])
            record["valid_until_ms"] = "11"
            record["generation"] = "0"
            self.assertFalse(state()["validation_current"])
        self.assertFalse(task.validation_current(None))

    def test_duplicate_stage_preserves_validation_and_unique_capacity(self):
        # Exercise the actual dispatch/CAS implementation with a minimal in-memory
        # journal. No platform or sandbox bypass is presented as containment proof.
        with tempfile.TemporaryDirectory() as directory:
            task = LinuxTask.__new__(LinuxTask)
            task.state = {"candidates": {}, "generation": "1"}
            task.grants = {}
            task.subscriptions = {}
            task.now = lambda: 1
            task.authorize = lambda cap, right: None
            task.save = lambda: None
            task.mint = lambda: "cap:" + format(len(task.state["candidates"]) + 1, "016x")
            def put(data):
                content = digest(data)
                (Path(directory) / content.split(":")[1]).write_bytes(data)
                return content
            task.put = put
            def stage(data):
                return task.dispatch("1", {"operation": "stage_candidate", "task_cap": "cap:0000000000000001", "bytes_base64": base64.b64encode(data).decode()})
            first = stage(b"candidate")
            marker = {"outcome": "valid"}
            task.state["candidates"][first["candidate_cap"]]["validation"] = marker
            for _ in range(70):
                self.assertEqual(stage(b"candidate"), first)
            self.assertIs(task.state["candidates"][first["candidate_cap"]]["validation"], marker)
            self.assertEqual(len(task.state["candidates"]), 1)
            for index in range(63):
                stage(f"unique {index}".encode())
            self.assertEqual(len(task.state["candidates"]), 64)
            self.assertEqual(stage(b"candidate"), first)
            with self.assertRaisesRegex(TaskError, "capacity"):
                stage(b"sixty fifth unique candidate")


if __name__ == "__main__":
    unittest.main()
