#!/usr/bin/env python3
"""Gate-first process-orchestration tests; fake services prove no Store security.

Run from repository root: python3 tools/ci/test_s7_signature_startup.py.
Repository discovery uses cwd.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import unittest


ROOT = Path.cwd()
GATE = ROOT / "tools/ci/foundry_s7_store_signature_security.sh"

FAKE_CARGO = r'''#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
trace = Path(os.environ['S7_FIXTURE_TRACE'])
with trace.open('a') as output:
    output.write('cargo:' + sys.argv[1] + '\n')
if sys.argv[1] == 'build':
    if os.environ.get('S7_FIXTURE_MODE') == 'build_failure':
        print('fixture compiler failure', file=sys.stderr)
        sys.exit(42)
    time.sleep(float(os.environ.get('S7_FIXTURE_COMPILE_DELAY', '0')))
    print(json.dumps({'reason':'compiler-artifact', 'target':{'name':'store_service','kind':['bin']},
                     'profile':{'test':False}, 'executable':os.environ['S7_FIXTURE_SERVICE']}))
    print(json.dumps({'reason':'build-finished','success':True}))
else:
    time.sleep(float(os.environ.get('S7_FIXTURE_COMPILE_DELAY', '0')))
    os.execv(os.environ['S7_FIXTURE_SERVICE'], [os.environ['S7_FIXTURE_SERVICE']])
'''

FAKE_SERVICE = r'''#!/usr/bin/env python3
import os, sys, time
from pathlib import Path
mode = os.environ.get('S7_FIXTURE_MODE', '')
case = ('dev' if os.environ.get('RAMEN_STORE_DEV_MODE') == '1' else
        'keys' if os.environ.get('RAMEN_STORE_TRUSTED_KEYS') else 'no_keys')
with Path(os.environ['S7_FIXTURE_TRACE']).open('a') as output:
    output.write('service:' + case + '\n')
if case == 'no_keys':
    print('SECURITY ERROR: RAMEN_STORE_TRUSTED_KEYS not set', flush=True)
    if mode != 'missing_marker':
        print('ABORTING', flush=True)
    if mode != 'production_alive':
        sys.exit(0 if mode == 'zero_abort' else 1)
elif case == 'dev':
    print('WARNING: RAMEN_STORE_DEV_MODE IS ENABLED; SECURITY RISK; AllowUnsigned', flush=True)
    if mode == 'dev_early_exit':
        sys.exit(7)
else:
    print('loaded 1 trusted keys; RequireSignature', flush=True)
# Bound the deliberate alive-service fixture even if the gate under test
# exits without cleanup. This is fixture lifetime, never a production timeout.
time.sleep(2)
sys.exit(99)
'''

FAKE_SLEEP = "#!/usr/bin/env python3\nimport time\ntime.sleep(0.25)\n"


class SignatureStartupTests(unittest.TestCase):
    def exercise(self, *, mode="", compile_delay=0):
        # Only private fixture files and processes are created. A held process
        # group bounds a timed-out gate; each fake service also self-terminates.
        with tempfile.TemporaryDirectory(prefix="ramenos-s7-signature-test-") as temporary:
            fixture = Path(temporary)
            tools = fixture / "tools/ci"
            tools.mkdir(parents=True)
            shutil.copyfile(GATE, tools / GATE.name)
            shutil.copyfile(ROOT / "tools/ci/cargo_artifact.py", tools / "cargo_artifact.py")
            schema = fixture / "artifact_store_schema/src"
            schema.mkdir(parents=True)
            shutil.copyfile(ROOT / "artifact_store_schema/src/signature.rs", schema / "signature.rs")
            bins = fixture / "fixture-bin"
            bins.mkdir()
            for name, source in (("cargo", FAKE_CARGO), ("store_service", FAKE_SERVICE), ("sleep", FAKE_SLEEP)):
                path = bins / name
                path.write_text(source)
                path.chmod(0o700)
            trace = fixture / "trace"
            env = dict(os.environ, PATH=str(bins) + os.pathsep + os.environ["PATH"],
                       S7_FIXTURE_TRACE=str(trace), S7_FIXTURE_SERVICE=str(bins / "store_service"),
                       S7_FIXTURE_MODE=mode, S7_FIXTURE_COMPILE_DELAY=str(compile_delay))
            env.pop("RAMEN_STORE_DEV_MODE", None)
            env.pop("RAMEN_STORE_TRUSTED_KEYS", None)
            process = subprocess.Popen(["bash", str(tools / GATE.name)], cwd=fixture,
                                       env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                       text=True, start_new_session=True)
            try:
                output, _ = process.communicate(timeout=12)
                status = process.returncode
            finally:
                if process.poll() is None:
                    # The unreaped live parent still owns this numeric identity.
                    # Never signal a group after communicate/poll reaps it.
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait(timeout=2)
            calls = trace.read_text().splitlines() if trace.exists() else []
            return status, output, calls

    def test_compile_delay_finishes_before_all_three_direct_service_probes(self):
        # Fake compiler takes longer than the fixture's accelerated 2-second
        # service window. Production sleep/deadline values are never changed.
        status, output, calls = self.exercise(compile_delay=0.75)
        self.assertEqual(status, 0, output)
        self.assertIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)
        self.assertEqual(calls, ["cargo:build", "service:no_keys", "service:dev", "service:keys"])

    def test_failed_build_cannot_launch_or_claim_service_security(self):
        status, output, calls = self.exercise(mode="build_failure")
        self.assertNotEqual(status, 0, output)
        self.assertEqual(calls, ["cargo:build"])
        self.assertNotIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)
        self.assertIn("fixture compiler failure", output)

    def test_zero_exit_with_abort_text_is_rejected(self):
        status, output, calls = self.exercise(mode="zero_abort")
        self.assertNotEqual(status, 0, output)
        self.assertEqual(calls, ["cargo:build", "service:no_keys"])
        self.assertNotIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)

    def test_running_production_service_is_rejected_despite_abort_text(self):
        status, output, calls = self.exercise(mode="production_alive")
        self.assertNotEqual(status, 0, output)
        self.assertEqual(calls, ["cargo:build", "service:no_keys"])
        self.assertNotIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)

    def test_dev_early_exit_is_rejected_despite_warning_and_policy_text(self):
        status, output, calls = self.exercise(mode="dev_early_exit")
        self.assertNotEqual(status, 0, output)
        self.assertEqual(calls, ["cargo:build", "service:no_keys", "service:dev"])
        self.assertNotIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)

    def test_nonzero_exit_without_required_abort_marker_is_rejected(self):
        status, output, calls = self.exercise(mode="missing_marker")
        self.assertNotEqual(status, 0, output)
        self.assertEqual(calls, ["cargo:build", "service:no_keys"])
        self.assertNotIn("FOUNDRY_S7_STORE_SIGNATURE_SECURITY: PASS", output)


if __name__ == "__main__":
    unittest.main()
