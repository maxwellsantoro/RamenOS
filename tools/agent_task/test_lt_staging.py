#!/usr/bin/env python3
"""Portable transaction semantics; no Linux containment claim."""
import base64
import tempfile
import unittest
from pathlib import Path
from lt_backend import LinuxTask, TaskError, digest


class StagingTests(unittest.TestCase):
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
