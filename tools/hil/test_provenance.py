"""Run-bound provenance must fail closed without accessing hardware."""
import json
import os
from pathlib import Path
import tempfile
import unittest

import provenance as p


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.efi = self.root / "BOOTX64.EFI"
        self.init = self.root / "init.img"
        self.efi.write_bytes(b"efi")
        self.init.write_bytes(p.build_init_image("nvme_boot", 4096))
        self.expected = {
            "schema_version": 1, "git_sha": "a" * 40, "machine_id": "test-target",
            "init_profile": "init-nvme-boot", "kernel_build_id": "b" * 64,
            "storage_manifest_sha256": "c" * 64,
            "kernel_efi_sha256": p.digest(self.efi), "init_img_sha256": p.digest(self.init),
            "kernel_efi": str(self.efi), "init_img": str(self.init),
        }
        self.manifest = self.root / "provenance.json"
        self.manifest.write_text(json.dumps(self.expected))
        self.log = self.root / "serial.log"
        self.markers = {key: self.expected[key] for key in p.MARKER_KEYS if key != "boot_epoch_nonce"}
        self.markers["boot_epoch_nonce"] = "0000000000000001"
        self.write_serial()

    def write_serial(self):
        self.log.write_text("RAMEN OS S0 boot\n" + "".join(f"hil_evidence: {k}={v}\n" for k, v in self.markers.items()) + "persistent_storage: nvme_boot ok\n")

    def validate(self):
        return p.validate_serial(self.log, self.manifest, "1", True)

    def test_bound_record_and_leading_zero_nonce(self):
        self.assertEqual(self.validate()["boot_epoch_nonce"], "0000000000000001")
        self.markers["boot_epoch_nonce"] = "0123456789abcdef"
        self.write_serial()
        p.validate_serial(self.log, self.manifest, "0123456789abcdef", True)

    def test_unknown_mismatch_missing_duplicate_and_stale(self):
        original = self.markers.copy()
        for key in p.MARKER_KEYS:
            for value in ["unknown", "", "wrong"]:
                with self.subTest(key=key, value=value):
                    self.markers = {**original, key: value}
                    self.write_serial()
                    with self.assertRaises(ValueError): self.validate()
        self.markers = original
        self.write_serial()
        with self.assertRaises(ValueError): p.validate_serial(self.log, self.manifest, "2", True)
        with self.assertRaises(ValueError): p.validate_serial(self.log, self.manifest, "", True)
        with self.log.open("a") as f: f.write("hil_evidence: machine_id=test-target\n")
        with self.assertRaises(ValueError): self.validate()
        self.write_serial()
        with self.log.open("a") as f: f.write("RAMEN OS S0 boot\n")
        with self.assertRaises(ValueError): self.validate()

    def test_build_manifest_uses_the_target_init_profile_name(self):
        import subprocess
        env = os.environ.copy()
        for key in ["git_sha", "machine_id", "storage_manifest_sha256", "kernel_build_id", "init_img_sha256"]:
            env["RAMEN_" + key.upper()] = self.expected[key]
        result = subprocess.run(["python3", str(Path(p.__file__)), "build", str(self.manifest),
                                 str(self.efi), str(self.init), "nvme_boot"], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.validate()

    def test_final_artifact_mutation_rejected(self):
        self.efi.write_bytes(b"other build")
        with self.assertRaises(ValueError): self.validate()

    def controller(self):
        control = self.root / "controller.log"
        control.write_text("run-id=test-run\n")
        return {"schema_version": 1, "evidence_kind": "hil_appliance_run_v0",
                "evidence_level": "PASS/HIL-APPLIANCE", "serial_input_kind": "live_device",
                "result": "pass", "run_id": "test-run", "appliance_id": "test-pi",
                "target_id": "test-target", "serial_log": str(self.log), "serial_log_sha256": p.digest(self.log),
                "controller_log": str(control), "controller_log_sha256": p.digest(control),
                "target_hil_evidence_markers": self.markers, "started_at_unix_ms": 1, "ended_at_unix_ms": 2}

    def test_controller_bound_to_run_target_and_transcript(self):
        record = self.controller()
        path = self.root / "controller.json"
        path.write_text(json.dumps(record))
        p.validate_controller(path, self.log, "test-run", "test-pi", "test-target", self.markers)
        for key in ["run_id", "appliance_id", "target_id", "serial_log_sha256", "controller_log_sha256", "serial_input_kind", "result"]:
            bad = {**record, key: "wrong"}
            path.write_text(json.dumps(bad))
            with self.subTest(key=key), self.assertRaises(ValueError):
                p.validate_controller(path, self.log, "test-run", "test-pi", "test-target", self.markers)
        with self.assertRaises((ValueError, OSError)):
            p.validate_controller(self.root / "missing.json", self.log, "test-run", "test-pi", "test-target", self.markers)

    def test_evidence_emission_requires_matching_controller_and_artifacts(self):
        from unittest.mock import patch
        controller = self.root / "controller.json"
        controller.write_text(json.dumps(self.controller()))
        out = self.root / "gate.json"
        env = {"RAMEN_HIL_GRADUATION": "1", "RAMEN_HIL_APPLIANCE": "1",
               "RAMEN_HIL_EXPECTED_BUILD": str(self.manifest), "RAMEN_HIL_EXPECTED_NONCE": "1",
               "RAMEN_HIL_RUN_ID": "test-run", "RAMEN_HIL_APPLIANCE_ID": "test-pi",
               "RAMEN_HIL_CONTROLLER_EVIDENCE": str(controller), "RAMEN_HIL_SERIAL_DEV": "/dev/null"}
        args = (out, "s13_7", "PASS/METAL", self.log, "persistent_storage: nvme_boot ok", self.efi, self.init)
        with patch.dict(os.environ, env, clear=True):
            p.emit(*args, "appliance-mediated")
            payload = json.loads(out.read_text())
            self.assertEqual(payload["target_hil_evidence_markers"], self.markers)
            self.assertEqual(payload["kernel_efi_sha256"], p.digest(self.efi))
            os.environ["RAMEN_HIL_CONTROLLER_EVIDENCE"] = ""
            with self.assertRaises(ValueError): p.emit(*args, "appliance-mediated")
            os.environ["RAMEN_HIL_APPLIANCE"] = "0"
            p.emit(*args, "operator-golden-machine")
            self.assertFalse(json.loads(out.read_text())["appliance"]["enabled"])
            os.environ["RAMEN_HIL_SERIAL_LOG"] = str(self.log)
            with self.assertRaises(ValueError): p.emit(*args, "operator-golden-machine")


    def test_success_must_follow_provenance_in_the_same_boot(self):
        from unittest.mock import patch
        marker = "persistent_storage: nvme_boot ok"
        record = self.log.read_text().replace(marker + "\n", "")
        env = {"RAMEN_HIL_GRADUATION": "1", "RAMEN_HIL_EXPECTED_BUILD": str(self.manifest),
               "RAMEN_HIL_EXPECTED_NONCE": "1", "RAMEN_HIL_SERIAL_DEV": "/dev/null"}
        for text in [marker + "\n" + record + "persistent_storage: nvme_boot failed reason=not_nvme\n",
                     record.replace("RAMEN OS S0 boot\n", "RAMEN OS S0 boot\n" + marker + "\n"),
                     record + marker + "\npersistent_storage: nvme_boot failed reason=not_nvme\n"]:
            self.log.write_text(text)
            with patch.dict(os.environ, env, clear=True), self.assertRaises(ValueError):
                p.emit(self.root / "gate.json", "s13_7", "PASS/METAL", self.log, marker,
                       self.efi, self.init, "operator-golden-machine")


class LiveGateTests(unittest.TestCase):
    setUp = ProvenanceTests.setUp
    write_serial = ProvenanceTests.write_serial

    def test_graduation_appliance_gate_captures_one_bound_boot(self):
        import pty
        import subprocess
        import threading
        import time
        root = Path(__file__).resolve().parents[2]
        master, slave = pty.openpty()
        self.addCleanup(os.close, master)
        self.addCleanup(os.close, slave)
        evidence = self.root / "capture"
        evidence.mkdir()
        env = {k: v for k, v in os.environ.items() if not k.startswith("RAMEN_HIL_")}
        env.update(RAMEN_HIL_APPLIANCE="1", RAMEN_HIL_GRADUATION="1",
                   RAMEN_HIL_SERIAL_DEV=os.ttyname(slave), RAMEN_HIL_EXPECTED_BUILD=str(self.manifest),
                   RAMEN_HIL_EXPECTED_NONCE="1", RAMEN_HIL_TARGET_ID="test-target",
                   RAMEN_HIL_APPLIANCE_ID="test-pi", RAMEN_HIL_RUN_ID="test-run",
                   RAMEN_HIL_EVIDENCE_DIR=str(evidence), RAMEN_HIL_CAPTURE_TIMEOUT_S="1")
        stopped = threading.Event()
        def feed():
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and not stopped.wait(0.02):
                if (evidence / "test-run.serial.log").exists():
                    os.write(master, self.log.read_bytes())
                    return
        thread = threading.Thread(target=feed)
        thread.start()
        try:
            # Skip nested invocation of this same suite; the gate's fixture block
            # runs the provenance unit class selected below, excluding this test.
            result = subprocess.run(["bash", str(root / "tools/ci/foundry_hil_appliance_s12_4.sh")],
                                    cwd=root, env=env, capture_output=True, text=True, timeout=20)
        finally:
            stopped.set()
            thread.join()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        p.validate_controller(evidence / "test-run.json", self.log, "test-run", "test-pi", "test-target", self.markers)

    def test_prepared_image_gate_reuses_identity_without_rebuilding(self):
        import pty
        import subprocess
        import threading
        import time
        root = Path(__file__).resolve().parents[2]
        self.expected["init_profile"] = "init-hil-boot"
        self.init.write_bytes(p.build_init_image("hil_boot", 4096))
        self.expected["init_img_sha256"] = p.digest(self.init)
        self.manifest.write_text(json.dumps(self.expected))
        self.markers["init_profile"] = "init-hil-boot"
        self.markers["init_img_sha256"] = p.digest(self.init)
        self.write_serial()
        text = self.log.read_text().replace("persistent_storage: nvme_boot ok", "golden_machine: gop_probe ok\ngolden_machine: gop_fill ok\ngolden_machine: hil_boot ok")
        self.log.write_text(text)
        master, slave = pty.openpty()
        self.addCleanup(os.close, master)
        self.addCleanup(os.close, slave)
        capture = self.root / "capture"
        capture.mkdir()
        commands = self.root / "bin"
        commands.mkdir()
        cargo = commands / "cargo"
        cargo.write_text("#!/bin/sh\necho unexpected rebuild >&2\nexit 99\n")
        cargo.chmod(0o755)
        env = {k: v for k, v in os.environ.items() if not k.startswith("RAMEN_HIL_")}
        env.update(RAMEN_HIL_APPLIANCE="1", RAMEN_HIL_GOLDEN_MACHINE="1", RAMEN_HIL_GRADUATION="1",
                   RAMEN_HIL_SERIAL_DEV=os.ttyname(slave), RAMEN_HIL_EXPECTED_BUILD=str(self.manifest),
                   RAMEN_HIL_EXPECTED_NONCE="1", RAMEN_HIL_TARGET_ID="test-target",
                   RAMEN_HIL_APPLIANCE_ID="test-pi", RAMEN_HIL_RUN_ID="test-run",
                   RAMEN_HIL_EVIDENCE_DIR=str(capture), RAMEN_HIL_LOG_DIR=str(capture), RAMEN_HIL_BOOT_TIMEOUT_S="1",
                   PATH=str(commands) + os.pathsep + os.environ["PATH"])
        stopped = threading.Event()
        def feed():
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and not stopped.wait(0.02):
                if (capture / "test-run_s12_2.serial.log").exists():
                    os.write(master, self.log.read_bytes())
                    return
        thread = threading.Thread(target=feed)
        thread.start()
        try:
            result = subprocess.run(["bash", str(root / "tools/ci/foundry_s12_hil_boot_s12_2.sh")],
                                    cwd=root, env=env, capture_output=True, text=True, timeout=20)
        finally:
            stopped.set()
            thread.join()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        evidence = json.loads((capture / "foundry_s12_hil_boot_s12_2.json").read_text())
        self.assertEqual(evidence["kernel_build_id"], self.expected["kernel_build_id"])
        self.assertEqual(evidence["claim_path"], "appliance-mediated")
        self.assertEqual(evidence["appliance"]["target_id"], "test-target")


if __name__ == "__main__":
    unittest.main()
