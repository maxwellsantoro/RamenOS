#!/usr/bin/env python3
"""Gate-first fresh firmware and bounded single-boot observation controls."""
import json
import sys
import os
import signal
import subprocess
import time
import tempfile
import unittest
from unittest import mock
from pathlib import Path
import qemu_boot_observation as boot


class BootObservationTests(unittest.TestCase):
    def test_fresh_vars_copy_never_reuses_existing_mutable_state(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            template = root / "template.fd"
            template.write_bytes(b"firmware vars")
            first = root / "run-a.fd"
            second = root / "run-b.fd"
            boot.prepare_vars(template, first)
            first.write_bytes(b"mutated by first boot")
            boot.prepare_vars(template, second)
            self.assertEqual(second.read_bytes(), template.read_bytes())
            with self.assertRaises(boot.ObservationError):
                boot.prepare_vars(template, first)
            self.assertEqual(first.read_bytes(), b"mutated by first boot")

    def fixture(self, root, program, timeout=.2, markers=("complete",)):
        firmware = root / "code.fd"
        firmware.write_bytes(b"firmware code")
        kernel = root / "kernel.efi"
        kernel.write_bytes(b"kernel")
        image = root / "init.img"
        image.write_bytes(b"init")
        serial = root / "serial.log"
        command = [sys.executable, "-c", program, str(serial), str(firmware)]
        return boot.observe_boot(command, root, serial, list(markers), timeout,
                                 {"code": firmware, "kernel": kernel, "init": image}, None, None)

    def test_marker_success_still_requires_actual_exit_reap_and_fresh_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = self.fixture(root, "import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text('RAMEN OS S0 boot\\ncomplete\\n'); time.sleep(10)")
            self.assertTrue(result["success"])
            self.assertTrue(result["reaped"])
            self.assertIsInstance(result["exit_code"], int)
            self.assertIsNotNone(result["markers"]["complete"])
            self.assertLess(result["markers"]["complete"], result["deadline_monotonic_ns"])
            self.assertEqual(result["launches"], 1)
            self.assertEqual(result["inputs_before"], result["inputs_after"])

    def test_timeout_keeps_failure_serial_and_one_actual_reap(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            actual_spawn = boot.subprocess.Popen
            serial = root / "serial.log"
            def ready_spawn(*args, **kwargs):
                process = actual_spawn(*args, **kwargs)
                try:
                    # This negative control tests retention of actual child
                    # output. Make that precondition explicit without extending
                    # the observer's original 50 ms marker deadline.
                    deadline = time.monotonic() + 3
                    while not serial.exists() or serial.read_text() != "firmware only":
                        self.assertLess(time.monotonic(), deadline)
                        self.assertIsNone(process.poll())
                        time.sleep(.005)
                    return process
                except BaseException:
                    process.kill()
                    process.wait(timeout=5)
                    raise
            with mock.patch.object(boot.subprocess, 'Popen', ready_spawn):
                result = self.fixture(root, "import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text('firmware only'); time.sleep(10)", timeout=.05)
            self.assertEqual(result["deadline_seconds"], .05)
            self.assertFalse(result["success"])
            self.assertEqual(result["failure"], "marker-timeout")
            self.assertTrue(result["reaped"])
            self.assertEqual(result["launches"], 1)
            self.assertEqual((root / "serial.log").read_text(), "firmware only")

    def test_late_marker_and_early_exit_cannot_manufacture_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = self.fixture(Path(temporary), "import pathlib,sys,time; time.sleep(.1); pathlib.Path(sys.argv[1]).write_text('complete')", timeout=.025)
            self.assertFalse(result["success"])
        with tempfile.TemporaryDirectory() as temporary:
            result = self.fixture(Path(temporary), "import sys; sys.exit(23)")
            self.assertFalse(result["success"])
            self.assertEqual(result["exit_code"], 23)
            self.assertTrue(result["reaped"])

    def test_every_marker_and_immutable_input_must_hold(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = self.fixture(Path(temporary), "import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text('complete'); time.sleep(10)", timeout=.05, markers=("complete", "missing"))
            self.assertFalse(result["success"])
        with tempfile.TemporaryDirectory() as temporary:
            result = self.fixture(Path(temporary), "import pathlib,sys,time; pathlib.Path(sys.argv[2]).write_bytes(b'changed'); pathlib.Path(sys.argv[1]).write_text('complete'); time.sleep(10)")
            self.assertFalse(result["success"])
            self.assertEqual(result["failure"], "immutable-input-changed")

    def test_existing_serial_log_is_denied_before_spawn(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "serial.log").write_text("complete from old boot")
            with self.assertRaises(boot.ObservationError):
                self.fixture(root, "raise Exception('must not execute')")

    def test_valid_marker_followed_by_unrequested_exit23_is_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            actual_read = boot.serial_bytes
            def terminal_read(path):
                time.sleep(.04)  # Observe the real child's natural terminal state.
                return actual_read(path)
            with mock.patch.object(boot, 'serial_bytes', terminal_read):
                result = self.fixture(Path(temporary), "import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('complete'); sys.exit(23)", timeout=.5)
            self.assertFalse(result['success'])
            self.assertTrue(result['reaped'])
            self.assertEqual(result['exit_code'], 23)

    def test_sigterm_reaps_actual_boot_owner_without_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ('code.fd', 'kernel.efi', 'init.img'): (root / name).write_bytes(b'fixture')
            pid_file = root / 'child.pid'
            child = "import os,pathlib,time; pathlib.Path(" + repr(str(pid_file)) + ").write_text(str(os.getpid())); time.sleep(30)"
            program = "import qemu_boot_observation as b; from pathlib import Path; r=Path(" + repr(str(root)) + "); b.observe_boot(" + repr([sys.executable, '-c', child]) + ",r,r/'serial.log',['complete'],30,{'code':r/'code.fd','kernel':r/'kernel.efi','init':r/'init.img'},None,None)"
            env = dict(os.environ, PYTHONPATH=str(Path(boot.__file__).parent))
            owned = subprocess.Popen([sys.executable, '-c', program], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            deadline = time.monotonic() + 3
            while not pid_file.exists():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.01)
            child_pid = int(pid_file.read_text())
            owned.send_signal(signal.SIGTERM)
            owned.communicate(timeout=12)
            self.assertNotEqual(owned.returncode, 0)
            with self.assertRaises(ProcessLookupError): os.kill(child_pid, 0)

    def test_natural_boot_parent_cannot_leave_same_group_descendant(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            pid_file = root / 'descendant.pid'
            sleeping = "import pathlib,os,time; pathlib.Path(" + repr(str(pid_file)) + ").write_text(str(os.getpid())); time.sleep(30)"
            parent = "import pathlib,sys,subprocess,time; p=subprocess.Popen(" + repr([sys.executable, '-c', sleeping]) + "); path=pathlib.Path(" + repr(str(pid_file)) + ");\nwhile not path.exists(): time.sleep(.01)\npathlib.Path(sys.argv[1]).write_text('complete')\n"
            actual_spawn = boot.subprocess.Popen
            actual_read = boot.serial_bytes
            held = []
            def held_spawn(*args, **kwargs):
                process = actual_spawn(*args, **kwargs)
                if not held: held.append(process)
                return process
            def after_natural_terminal(path):
                deadline = time.monotonic() + 1.5
                while held[0].returncode is None and boot.terminal_observation(held[0]) is None:
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(.005)
                return actual_read(path)
            with mock.patch.object(boot.subprocess, 'Popen', held_spawn), mock.patch.object(boot, 'serial_bytes', after_natural_terminal):
                result = self.fixture(root, parent, timeout=2)
            self.assertTrue(result['success'])
            self.assertTrue(result['terminal_before_cleanup'])
            self.assertTrue(result['group_stop_before_reap'])
            descendant = int(pid_file.read_text())
            deadline = time.monotonic() + 3
            while True:
                try: os.kill(descendant, 0)
                except ProcessLookupError: break
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.01)


if __name__ == "__main__":
    unittest.main()
