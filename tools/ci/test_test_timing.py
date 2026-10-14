"""Diagnostic stream timing keeps the real tests and their output unchanged."""
import importlib.util
from pathlib import Path
import time
import os
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('test_timing', HERE / 'test_timing.py')
timing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(timing)


class TimingTests(unittest.TestCase):
    def test_split_live_prefix_and_completion(self):
        observer = timing.TestTiming(['first', 'second'])
        observer('stdout', b'running 2 tests\ntest fir')
        observer('stdout', b'st ... ')
        time.sleep(.02)
        observer('stdout', b'ok\ntest second ... ok\n')
        report = observer.report()
        self.assertEqual([r['case'] for r in report['cases']], ['first', 'second'])
        self.assertGreater(report['cases'][0]['elapsed_seconds'], .01)
        self.assertEqual(report['cases'][0]['outcome'], 'ok')
        self.assertFalse(report['acceptance'])

    def test_unknown_and_incomplete_are_diagnostics(self):
        observer = timing.TestTiming(['first'])
        observer('stdout', b'test other ... ok\ntest first ... ')
        report = observer.report()
        self.assertEqual(report['cases'][0]['outcome'], 'incomplete')
        self.assertEqual(report['unknown_cases'], ['other'])
        self.assertFalse(report['acceptance'])

    def test_command_owner_supports_live_observer(self):
        source = (HERE / 'editor_store_runner.py').read_text()
        self.assertIn('stream_observer', source)
        source = (HERE / 'desktop_editor_task_result.py').read_text()
        self.assertIn('stream_observer=case_timing', source)
        self.assertIn('phase-timings.json', source)

    def test_stderr_bounds_duplicates_are_diagnostic_only(self):
        observer = timing.TestTiming(['first'])
        observer('stderr', b'test first ... FAILED\n')
        self.assertEqual(observer.report()['cases'], [])
        observer('stdout', b'x' * 4097 + b'\ntest first ... ok\ntest first ... ok\n')
        report = observer.report()
        self.assertEqual(report['cases'][0]['outcome'], 'ok')
        self.assertIsNotNone(report['diagnostic_error'])
        self.assertFalse(report['acceptance'])

    def test_actual_owned_child_output_and_reap_unchanged(self):
        spec = importlib.util.spec_from_file_location('timing_owner', HERE / 'editor_store_runner.py')
        owner = importlib.util.module_from_spec(spec); spec.loader.exec_module(owner)
        parent = HERE.parents[1] / 'out/ci-optimization/timing-tests'; parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as folder:
            observer = timing.TestTiming(['first'])
            runner = owner.OwnedCommand(owner.Budget(Path(folder)), folder, dict(os.environ))
            argv = [sys.executable, '-c', 'import time;print("test first ... ",end="",flush=True);time.sleep(.03);print("ok")']
            plain = runner.run(argv, 'plain.log', timeout=3)
            observed = runner.run(argv, 'observed.log', timeout=3, stream_observer=observer)
            self.assertEqual((plain['stdout'], plain['stderr'], plain['status']),
                             (observed['stdout'], observed['stderr'], observed['status']))
            self.assertTrue(observed['process']['wait_reaped'])
            self.assertGreater(observer.report()['cases'][0]['elapsed_seconds'], .01)
            def broken(*_): raise ValueError('diagnostic observer only')
            result = runner.run(argv, 'broken.log', timeout=3, stream_observer=broken)
            self.assertEqual(result['status'], 0); self.assertTrue(result['process']['wait_reaped'])
            self.assertEqual(result['stdout'], plain['stdout'])


if __name__ == '__main__': unittest.main()
