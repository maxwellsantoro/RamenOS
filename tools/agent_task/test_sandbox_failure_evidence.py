#!/usr/bin/env python3
"""Gate-first operator diagnostics controls; mocked Docker is not enforcement proof.

Root invocation: python3 tools/agent_task/test_sandbox_failure_evidence.py
Frozen new fields: failure_reason, failure_phase, cleanup_failure_reason,
engine_command, engine_returncode, engine_stderr, engine_stderr_truncated.
Engine stderr retains at most 16384 raw bytes before replacement decoding.
These tests exercise maintained Sandbox.run/_engine and LinuxTask.execute/dispatch.
"""
import json
import os
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import linux_sandbox
from linux_sandbox import Sandbox, SandboxFailure
import lt_backend
from lt_backend import LinuxTask, digest


INVOCATION = "1" * 32
CONTAINER_ID = "2" * 64
IMAGE = "sha256:" + "3" * 64


class FailureEvidenceTests(unittest.TestCase):
    def sandbox(self):
        sandbox = Sandbox.__new__(Sandbox)
        sandbox.image = IMAGE
        sandbox.engine = {"fixture": "mocked CLI; no enforcement claim"}
        sandbox.mounts = []
        return sandbox

    def completed(self, args, code=0, stdout=b"", stderr=b""):
        return subprocess.CompletedProcess(args, code, stdout, stderr)

    def run_create_failure(self, create_result, *, cleanup_confirmed=True):
        calls = []

        def engine(args, **kwargs):
            calls.append(args)
            if args[1] == "create":
                if isinstance(create_result, Exception):
                    raise create_result
                return self.completed(args, **create_result)
            if args[1] == "rm":
                return self.completed(args)
            self.assertEqual(args[1:4], ["inspect", "--type", "container"])
            return self.completed(args, code=1 if cleanup_confirmed else 0,
                                  stderr=b"Error: No such container" if cleanup_confirmed else b"")

        with patch.dict(os.environ, {}, clear=True), \
                patch("linux_sandbox.time.monotonic", return_value=100.0), \
                patch("linux_sandbox.subprocess.run", side_effect=engine), \
                patch("linux_sandbox.subprocess.Popen") as popen:
            with self.assertRaises(SandboxFailure) as raised:
                self.sandbox().run(["/validator"], wall_ms=2290, invocation_id=INVOCATION)
        self.assertEqual([args[1] for args in calls], ["create", "rm", "inspect"])
        popen.assert_not_called()
        error = raised.exception
        self.assertEqual(error.reason, "creation_not_confirmed")
        self.assertFalse(error.evidence["created"])
        self.assertEqual(error.evidence["removed"], cleanup_confirmed)
        self.assertEqual(error.evidence["image_id"], IMAGE)
        self.assertEqual(error.evidence["wall_ms"], 2290)
        return error

    def test_create_timeout_retained_but_lt_public_response_redacted_and_poisoned(self):
        error = self.run_create_failure(subprocess.TimeoutExpired(
            ["docker", "create", "secret-argument"], 2.29, stderr=b"operator timeout marker"))
        self.assertEqual(error.evidence["failure_reason"], "timeout")
        self.assertEqual(error.evidence["failure_phase"], "create")
        self.assertEqual(error.evidence["engine_command"], "create")
        self.assertEqual(error.evidence["engine_stderr"], "operator timeout marker")
        self.assertFalse(error.evidence["engine_stderr_truncated"])
        self.assertNotIn("engine_returncode", error.evidence)
        self.assertNotIn("secret-argument", json.dumps(error.evidence))

        # Real LT dispatch catches the actual Sandbox failure above. Only the
        # external sandbox construction/run and CAS backing are mocked here.
        task = LinuxTask.__new__(LinuxTask)
        task.poisoned = False
        task.subscriptions, task.grants = {}, {}
        task.now = lambda: 1
        task.state = {"generation": 1, "runs": [], "last_validation": None}
        task.pins = {key: digest(key.encode()) for key in ("schema", "validator")}
        task.worker_id = digest(b"worker")
        task.sealed_worker = "/unused-fixture-worker"
        task.authorize = lambda cap, right: {"expires": 10000}
        candidate = {"content_id": digest(b"candidate"), "validation": None}
        task.candidate = lambda cap: candidate
        task.save = Mock()
        task.load = lambda *args: b"fixture-bytes"
        request = {"schema_version": 1, "request_id": "7", "call": {
            "operation": "validate_candidate", "task_cap": "cap:0000000000000001",
            "candidate_cap": "cap:0000000000000002", "validator_id": task.pins["validator"]}}
        fake_sandbox = Mock()
        fake_sandbox.run.side_effect = error
        with patch("lt_backend.hash_regular", return_value=task.worker_id), \
                patch("lt_backend.Sandbox", return_value=fake_sandbox):
            response = task.execute(request)
        self.assertEqual(response, {"schema_version": 1, "request_id": "7", "status": "io", "result": None})
        self.assertTrue(task.poisoned)
        self.assertEqual(len(task.state["runs"]), 1)
        retained = task.state["runs"][0]
        self.assertTrue(retained["reconciliation_required"])
        self.assertEqual(retained["engine_stderr"], "operator timeout marker")
        self.assertEqual(retained["failure_reason"], "timeout")
        self.assertEqual(task.execute(request), response)
        self.assertEqual(fake_sandbox.run.call_count, 1)

    def test_create_engine_failure_retains_code_and_bounded_stderr_without_retry(self):
        marker = b"E" * 16384
        error = self.run_create_failure({"code": 125, "stderr": marker + b"discarded-tail"})
        self.assertEqual(error.evidence["failure_reason"], "engine_failure")
        self.assertEqual(error.evidence["failure_phase"], "create")
        self.assertEqual(error.evidence["engine_command"], "create")
        self.assertEqual(error.evidence["engine_returncode"], 125)
        self.assertEqual(error.evidence["engine_stderr"], marker.decode())
        self.assertTrue(error.evidence["engine_stderr_truncated"])
        self.assertNotIn("discarded-tail", json.dumps(error.evidence))

    def test_cleanup_failure_preserves_original_create_cause(self):
        error = self.run_create_failure(subprocess.TimeoutExpired(
            ["docker", "create"], 2.29, stderr=b"first failure"), cleanup_confirmed=False)
        self.assertEqual(error.evidence["failure_reason"], "timeout")
        self.assertEqual(error.evidence["failure_phase"], "create")
        self.assertEqual(error.evidence["cleanup_failure_reason"], "cleanup_failed")
        self.assertEqual(error.evidence["engine_stderr"], "first failure")
        self.assertFalse(error.evidence["removed"])

    def test_nonzero_attached_command_keeps_guest_diagnostic_and_cleanup(self):
        config = {"Config": {"User": "65534:65534"}, "Mounts": [], "HostConfig": {
            "NetworkMode": "none", "ReadonlyRootfs": True, "CapDrop": ["ALL"], "CapAdd": [],
            "Privileged": False, "SecurityOpt": ["no-new-privileges"], "PidsLimit": 32,
            "Memory": 2147483648, "MemorySwap": 2147483648, "NanoCpus": 1000000000,
            "PidMode": "", "IpcMode": "private", "Devices": [], "DeviceRequests": [], "Tmpfs": {}}}
        calls = []

        def engine(args, **kwargs):
            calls.append(args[1])
            if args[1] == "create":
                return self.completed(args, stdout=CONTAINER_ID.encode() + b"\n")
            if args[1] == "inspect" and "rm" not in calls:
                return self.completed(args, stdout=json.dumps([config]).encode())
            return self.completed(args, code=1 if args[1] == "inspect" else 0,
                                  stderr=b"No such container" if args[1] == "inspect" else b"")

        in_read, in_write = os.pipe()
        out_read, out_write = os.pipe()
        err_read, err_write = os.pipe()
        os.close(out_write)
        os.write(err_write, b"normal guest diagnostic")
        os.close(err_write)
        cli = SimpleNamespace(stdin=os.fdopen(in_write, "wb"), stdout=os.fdopen(out_read, "rb"),
                              stderr=os.fdopen(err_read, "rb"), wait=Mock(return_value=7), poll=Mock(return_value=7))
        try:
            with patch.dict(os.environ, {}, clear=True), \
                    patch("linux_sandbox.subprocess.run", side_effect=engine), \
                    patch("linux_sandbox.subprocess.Popen", return_value=cli) as popen:
                with self.assertRaises(SandboxFailure) as raised:
                    self.sandbox().run(["/validator"], invocation_id=INVOCATION)
            error = raised.exception
            self.assertEqual(error.reason, "command_failed")
            self.assertEqual(error.evidence["failure_reason"], "command_failed")
            self.assertEqual(error.evidence["failure_phase"], "attach")
            self.assertEqual(error.evidence["trusted_stderr"], "normal guest diagnostic")
            self.assertEqual(error.evidence["exit_code"], 7)
            self.assertTrue(error.evidence["created"])
            self.assertTrue(error.evidence["removed"])
            self.assertEqual(calls, ["create", "inspect", "rm", "inspect"])
            self.assertEqual(popen.call_count, 1)
        finally:
            os.close(in_read)
            for stream in (cli.stdin, cli.stdout, cli.stderr):
                if not stream.closed:
                    stream.close()


if __name__ == "__main__":
    unittest.main()
