"""Host-only regressions for CI policy, efivar encoding and HIL opt-in."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def run_script(name, args=(), env=None):
    clean = {k: v for k, v in os.environ.items() if not k.startswith("RAMEN_HIL_")}
    clean.update(env or {})
    return subprocess.run(["bash", str(ROOT / name), *args], cwd=ROOT, env=clean,
                          capture_output=True, text=True, timeout=30)


class FirmwareTests(unittest.TestCase):
    def test_exact_binary_records_and_replacement(self):
        with tempfile.TemporaryDirectory() as tmp:
            shim = Path(tmp) / "python-shim"
            shim.mkdir()
            (shim / "sitecustomize.py").write_text(
                "import os, errno\ndef unsupported(fd):\n    raise OSError(errno.EINVAL, 'efivarfs has no fsync')\nos.fsync = unsupported\n")
            env = {"RAMEN_HIL_EFIVAR_DIR": tmp, "PYTHONPATH": str(shim)}
            for slot, byte in [("A", 0), ("B", 1)]:
                result = run_script("tools/hil/set_ramenos_ab_slot.sh", [slot], env)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(next(Path(tmp).glob("RamenAbSlot-*")).read_bytes(),
                                 bytes([7, 0, 0, 0, 1, byte, 1]))
            for nonce in ["1", "0123456789abcdef"]:
                result = run_script("tools/hil/set_ramenos_boot_nonce.sh", [nonce], env)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(next(Path(tmp).glob("RamenBootNonce-*")).read_bytes(),
                                 bytes([7, 0, 0, 0]) + int(nonce, 16).to_bytes(8, "little"))
            before = next(Path(tmp).glob("RamenBootNonce-*")).read_bytes()
            for nonce in ["0", "-1", "10000000000000000", "unknown"]:
                self.assertNotEqual(run_script("tools/hil/set_ramenos_boot_nonce.sh", [nonce], env).returncode, 0)
                self.assertEqual(next(Path(tmp).glob("RamenBootNonce-*")).read_bytes(), before)


class CiTests(unittest.TestCase):
    def test_classifier_and_merge_fail_closed(self):
        spec = importlib.util.spec_from_file_location("ci_policy", ROOT / "tools/ci/change_policy.py")
        policy = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(policy)
        self.assertFalse(policy.requires_foundry(["README.md", "docs/org/current_task.yaml"]))
        for path in ["tools/hil/build_usb_boot_image.sh", "tools/init/build_init_image.py",
                     "justfile", ".github/workflows/ci.yml", "kernel/src/lib.rs", "new-unknown-file"]:
            self.assertTrue(policy.requires_foundry([path]), path)
        for changes in ["failure", "cancelled", "skipped", ""]:
            self.assertFalse(policy.merge_allowed(changes, "false", "skipped", "success"))
        for output in ["", "bogus"]:
            self.assertFalse(policy.merge_allowed("success", output, "skipped", "success"))
        self.assertTrue(policy.merge_allowed("success", "false", "skipped", "success"))
        self.assertTrue(policy.merge_allowed("success", "true", "success", "success"))
        self.assertFalse(policy.merge_allowed("success", "true", "skipped", "success"))
        self.assertFalse(policy.merge_allowed("success", "false", "skipped", "failure"))
        result = subprocess.run(["python3", str(ROOT / "tools/ci/change_policy.py"), "classify", "no-such-ref", "HEAD"], cwd=ROOT, capture_output=True)
        self.assertNotEqual(result.returncode, 0)


class ApplianceTests(unittest.TestCase):
    def test_opt_in_captures_serial_under_inherited_live_environment(self):
        import pty
        import threading
        master, slave = pty.openpty()
        self.addCleanup(os.close, master)
        self.addCleanup(os.close, slave)
        with tempfile.TemporaryDirectory() as tmp:
            stopped = threading.Event()
            def feed():
                while not stopped.wait(0.05):
                    os.write(master, b"RAMEN OS synthetic serial fixture\n")
            thread = threading.Thread(target=feed)
            thread.start()
            try:
                result = run_script("tools/ci/foundry_hil_appliance_s12_4.sh", env={
                    "RAMEN_HIL_APPLIANCE": "1", "RAMEN_HIL_SERIAL_DEV": os.ttyname(slave),
                    "RAMEN_HIL_EVIDENCE_DIR": tmp, "RAMEN_HIL_RUN_ID": "synthetic-capture",
                    "RAMEN_HIL_CAPTURE_TIMEOUT_S": "0.5", "RAMEN_HIL_APPLIANCE_ID": "test-pi",
                })
            finally:
                stopped.set()
                thread.join()
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            payload = json.loads((Path(tmp) / "synthetic-capture.json").read_text())
            self.assertEqual(payload["serial_input_kind"], "live_device")
            self.assertGreater(Path(payload["serial_log"]).stat().st_size, 0)
            self.assertEqual(payload["appliance_id"], "test-pi")
            self.assertEqual(payload["evidence_level"], "PASS/HIL-APPLIANCE")
            # The same run ID cannot overwrite valid evidence.
            repeated = run_script("tools/hil/appliance_capture_serial.sh", env={
                "RAMEN_HIL_EVIDENCE_DIR": tmp, "RAMEN_HIL_RUN_ID": "synthetic-capture",
                "RAMEN_HIL_SERIAL_DEV": os.ttyname(slave),
            })
            self.assertNotEqual(repeated.returncode, 0)
            self.assertIn("RUN_ID_REUSED", repeated.stderr)


if __name__ == "__main__":
    unittest.main()
