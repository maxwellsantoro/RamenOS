#!/usr/bin/env python3
"""The proxy assertion consumes one successful real command's output."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]


class BrokerGateTests(unittest.TestCase):
    def fixture(self, folder, mode):
        root = Path(folder)
        gate = 'tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh'
        for path in [gate, 'docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md',
                     'kernel_api/src/generated/domain_manager_v1.generated.rs',
                     'runtime_supervisor/src/native_wasm_runner.rs', 'services/domain_manager/src/broker.rs']:
            destination = root / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO / path, destination)
        (root / 'services/kernel_harness_proxy').mkdir(parents=True)
        binary = root / 'bin'
        binary.mkdir()
        cargo = binary / 'cargo'
        cargo.write_text('''#!/usr/bin/env python3
import os,sys
from pathlib import Path
if 'kernel_harness_proxy' in sys.argv:
    with open(os.environ['CALLS'], 'a') as calls: calls.write('proxy\\n')
    print('9c0de4419f03f426' if os.environ['MODE'] != 'missing' else 'different')
    if os.environ['MODE'] == 'failure': raise SystemExit(7)
''')
        cargo.chmod(0o755)
        env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ['PATH'],
                   MODE=mode, CALLS=str(root / 'calls'))
        result = subprocess.run(['bash', str(root / gate)], env=env, capture_output=True, text=True)
        return result, (root / 'calls').read_text().splitlines()

    def test_proxy_run_once_and_checks_sentinel(self):
        with tempfile.TemporaryDirectory() as folder:
            result, calls = self.fixture(folder, 'success')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(calls, ['proxy'])

    def test_command_failure_with_valid_sentinel_is_not_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            result, calls = self.fixture(folder, 'failure')
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('code=PROXY_ROUNDTRIP', result.stderr)
            self.assertIn('9c0de4419f03f426', result.stderr)
            self.assertEqual(calls, ['proxy'])

    def test_success_without_sentinel_is_not_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            result, calls = self.fixture(folder, 'missing')
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('code=SHA256_PREFIX_MISMATCH', result.stderr)
            self.assertEqual(calls, ['proxy'])


if __name__ == '__main__':
    unittest.main()
