"""Host-only regressions for CI policy, efivar encoding and HIL opt-in."""
import importlib.util
import json
import os
import re
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
    def test_executable_docs_require_foundry_before_prose_exemption(self):
        spec = importlib.util.spec_from_file_location("ci_policy", ROOT / "tools/ci/change_policy.py")
        policy = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(policy)
        for path in ["docs/contracts/editor-native-preview-read-v0.json",
                     "docs/contracts/new-contract.md", "docs/fixtures/new.yaml",
                     "docs/new-executable-input.json", "docs/DESKTOP_EDITOR_WIRE_V1.md",
                     "docs/HIL_APPLIANCE_EVIDENCE_V0.md", "CONSTITUTION.md",
                     "EVIDENCE_LEVELS.md", "drivers/reference_vaults/virtio-net/README.md",
                     "drivers/reference_vaults/virtio-blk/README.md"]:
            with self.subTest(path=path):
                self.assertTrue(policy.requires_foundry([path]), path)
        self.assertFalse(policy.requires_foundry([
            "README.md", "CURRENT_STATUS.md", "docs/INDEX.md", "docs/research/RESEARCH_PROGRAM.md",
            "docs/org/current_task.yaml", "docs/org/tasks/new-packet.json"]))

    def test_execution_consumed_markdown_is_not_prose(self):
        spec = importlib.util.spec_from_file_location("ci_policy", ROOT / "tools/ci/change_policy.py")
        policy = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(policy)
        # Audit literal inputs in OS gate tooling. Governance alone owns org
        # packet/prose checks, so its requirements do not activate expensive lanes.
        paths = set()
        for directory in (ROOT / "tools/ci", ROOT / "tools/foundry"):
            for source in directory.iterdir():
                if (source.suffix not in {".py", ".sh"} or source.name.startswith("test_")
                        or source.name in {"change_policy.py", "foundry_org_governance_g0.sh"}):
                    continue
                for line in source.read_text().splitlines():
                    if not line.lstrip().startswith("#"):
                        paths.update(re.findall(r"docs/[A-Za-z0-9_./-]+\.md", line))
                        paths.update(re.findall(r"[\"']([A-Z][A-Z0-9_]*\.md)[\"']", line))
            for registry in directory.glob("*sources*v*.json"):
                paths.update(path for path in json.loads(registry.read_bytes())["paths"]
                             if path.endswith(".md"))
        # Contract source pins can name root documents rather than docs/**.
        # Free-form historical prose references are intentionally not input pins.
        def pinned_markdown(value):
            if type(value) is dict:
                if (type(value.get("path")) is str and value["path"].endswith(".md")
                        and "sha256" in value):
                    paths.add(value["path"])
                for child in value.values():
                    pinned_markdown(child)
            elif type(value) is list:
                for child in value:
                    pinned_markdown(child)
        for contract in (ROOT / "docs/contracts").rglob("*.json"):
            pinned_markdown(json.loads(contract.read_bytes()))
        self.assertIn("CONSTITUTION.md", paths)
        self.assertIn("EVIDENCE_LEVELS.md", paths)
        for path in paths:
            with self.subTest(path=path):
                self.assertTrue(policy.requires_foundry([path]), path)

    def test_contract_modification_and_deletion_cli(self):
        for name in ("docs/contracts/editor-native-preview-read-v0.json", "CONSTITUTION.md"):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                def git(*args):
                    return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.DEVNULL).decode().strip()
                git("init", "-q")
                git("config", "user.name", "CI fixture")
                git("config", "user.email", "fixture@example.invalid")
                contract = root / name
                contract.parent.mkdir(parents=True, exist_ok=True)
                contract.write_text('{"version":1}\n')
                git("add", ".")
                git("commit", "-qm", "baseline")
                base = git("rev-parse", "HEAD")
                for operation in ("modify", "delete"):
                    if operation == "modify":
                        contract.write_text('{"version":2}\n')
                    else:
                        contract.unlink()
                    git("add", "-A")
                    git("commit", "-qm", operation)
                    result = subprocess.run(["python3", str(ROOT / "tools/ci/change_policy.py"),
                                             "classify", base, "HEAD"], cwd=root,
                                            capture_output=True, text=True, timeout=30)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    with self.subTest(path=name, operation=operation):
                        self.assertEqual(result.stdout.strip(), "true", operation)

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
