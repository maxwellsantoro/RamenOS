#!/usr/bin/env python3
"""Failure cases exercise the external watchdog, frames and visible byte ledger."""

import sys
import tempfile
import unittest
from pathlib import Path

from evaluator_session import Session, Limits, SessionStopped

BOOT = "print('{\"ready\":true}', flush=True);"


class SessionTests(unittest.TestCase):
    def make(self, code, **limits):
        return Session(
            [sys.executable, "-u", "-c", code],
            Limits(**limits),
            visible=[("task", b"repair"), ("tools", b"descriptions")],
        )

    def test_bytes_and_fresh_context(self):
        code = (
            BOOT + "\nimport sys\nfor line in sys.stdin: print(line.strip(),flush=True)"
        )
        with self.make(code) as s:
            self.assertEqual(s.call({"hello": "é"}), {"hello": "é"})
            s.finish("receipt supplied by consumer")
            ledger = s.evidence()
            self.assertEqual(
                ledger["context_bytes"],
                sum(r["bytes"] for r in ledger["visible_ledger"]),
            )
            self.assertEqual(ledger["requests"], 1)
        with self.make(code) as fresh:
            self.assertEqual(fresh.requests, 0)
            self.assertNotIn("é", fresh.context().decode())

    def test_count_and_context_preflight(self):
        code = BOOT + "\nimport sys\nfor line in sys.stdin: print('{}',flush=True)"
        with self.make(code, max_requests=1) as s:
            s.call({})
            with self.assertRaises(SessionStopped):
                s.call({})
            self.assertEqual(s.requests, 1)
            self.assertEqual(s.reason, "request_limit")
        with self.make(code, context_bytes=100) as s:
            with self.assertRaises(SessionStopped):
                s.call({"oversize": "x" * 200})
            self.assertEqual(s.requests, 0)
            self.assertEqual(s.reason, "context_limit")

    def test_partial_reply_has_absolute_deadline(self):
        code = (
            BOOT
            + "\nimport sys,time\nsys.stdin.readline()\nfor i in range(100):\n print('x',end='',flush=True); time.sleep(.04)"
        )
        with self.make(code, reply_ms=150) as s:
            with self.assertRaises(SessionStopped):
                s.call({})
            self.assertEqual(s.reason, "reply_timeout")
        self.assertTrue(s.evidence()["process_cleanup"]["leader_reaped"])

    def test_whole_session_budget_and_bad_frames(self):
        import time

        with self.make(
            BOOT + "import time; time.sleep(30)", wall_ms=150, reply_ms=150
        ) as s:
            time.sleep(0.2)
            with self.assertRaises(SessionStopped):
                s.call({})
            self.assertEqual(s.reason, "session_timeout")
        for tail, reason in (
            ("print('x'*2000,flush=True)", "response_limit"),
            ("print('not-json',flush=True)", "response_invalid"),
            ('print(\'{"x":1,"x":2}\',flush=True)', "response_invalid"),
            ("print('{\"x\":NaN}',flush=True)", "response_invalid"),
            ("sys.exit(3)", "response_eof"),
            ("print('{}\\n{}',flush=True)", "unsolicited_output"),
        ):
            with self.make(
                BOOT + "\nimport sys\nsys.stdin.readline()\n" + tail,
                response_bytes=1024,
            ) as s:
                with self.assertRaises(SessionStopped):
                    s.call({})
                self.assertEqual(s.reason, reason)

    def test_reply_context_overflow_is_retained_and_terminal(self):
        code = (
            BOOT
            + "\nimport sys\nsys.stdin.readline();print('{\"data\":\"'+'x'*200+'\"}',flush=True)"
        )
        with self.make(code, context_bytes=100) as s:
            with self.assertRaises(SessionStopped):
                s.call({})
            self.assertEqual(s.reason, "context_limit")
            self.assertEqual(s.requests, 1)
            self.assertEqual(s.evidence()["rejected_visible"]["role"], "response")

    def test_startup_failure_and_limits_are_retained(self):
        with self.assertRaises(SessionStopped) as caught:
            self.make("import time;time.sleep(30)", wall_ms=150, reply_ms=150)
        self.assertEqual(caught.exception.evidence["outcome"], "session_timeout")
        self.assertTrue(caught.exception.evidence["process_cleanup"]["leader_reaped"])
        with self.make(
            BOOT + "\nimport sys\nfor line in sys.stdin: print('{}',flush=True)",
            request_bytes=64,
        ) as s:
            with self.assertRaises(SessionStopped):
                s.call({"oversize": "x" * 100})
            self.assertEqual(s.reason, "request_limit_bytes")
            self.assertEqual(s.requests, 0)
        code = (
            BOOT
            + "\nimport sys,time\nsys.stdin.readline();sys.stderr.write('x'*5000);sys.stderr.flush();time.sleep(.1);print('{}',flush=True)"
        )
        with self.make(code) as s:
            with self.assertRaises(SessionStopped):
                s.call({})
            self.assertEqual(s.reason, "stderr_limit")

    def test_unsolicited_output_on_close_is_terminal(self):
        code = (
            BOOT
            + "\nimport sys\nsys.stdin.readline();print('{}',flush=True);sys.stdin.read();print('{}',flush=True)"
        )
        with self.make(code) as s:
            s.call({})
        self.assertEqual(s.reason, "unsolicited_output")

    def test_separate_process_group_descendant_cleanup(self):
        if not Path("/proc/self/task").exists():
            self.skipTest("Linux process evidence")
        with tempfile.TemporaryDirectory() as tmp:
            pidfile = Path(tmp) / "pid"
            code = (
                "import subprocess,sys,time\np=subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)'],start_new_session=True)\nopen("
                + repr(str(pidfile))
                + ",'w').write(str(p.pid))\n"
                + BOOT
                + "time.sleep(30)"
            )
            with self.make(code) as s:
                pid = int(pidfile.read_text())
            proc = Path("/proc") / str(pid)
            if proc.exists():
                self.assertEqual(
                    (proc / "stat").read_text().rsplit(")", 1)[1].split()[0], "Z"
                )
            self.assertTrue(s.evidence()["process_cleanup"]["leader_reaped"])


if __name__ == "__main__":
    unittest.main()
