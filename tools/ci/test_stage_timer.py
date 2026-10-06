#!/usr/bin/env python3
"""Regression assertions for real stage timing and failure propagation."""
import contextlib
import importlib.util
import io
import json
import os
import signal
import time
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

TIMER = Path(__file__).with_name('stage_timer.py')


class StageTimerTests(unittest.TestCase):
    def run_stage(self, destination, *command, label='fixture'):
        return subprocess.run([sys.executable, str(TIMER), '--directory', str(destination),
                               '--label', label, '--', *command], capture_output=True, text=True)

    def test_actual_success_and_failure_are_unique_records(self):
        with tempfile.TemporaryDirectory() as folder:
            self.assertEqual(self.run_stage(folder, sys.executable, '-c', 'pass').returncode, 0)
            self.assertEqual(self.run_stage(folder, sys.executable, '-c', 'raise SystemExit(7)').returncode, 7)
            rows = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
            self.assertEqual(len(rows), 2)
            self.assertEqual(sorted(r['exit_code'] for r in rows), [0, 7])
            for row in rows:
                self.assertEqual(row['label'], 'fixture')
                self.assertGreater(row['elapsed_seconds'], 0)
                self.assertGreater(row['process_id'], 0)
                self.assertEqual(row['argv'][0], sys.executable)
                self.assertEqual(row['schema_version'], 1)
                self.assertIn(row['status'], ['PASS', 'FAIL'])
            self.assertFalse(list(Path(folder).glob('*.tmp')))

    def test_exec_failure_is_recorded_and_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.run_stage(folder, '/nonexistent/ramen-stage-test')
            self.assertEqual(result.returncode, 127)
            row, = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
            self.assertEqual(row['exit_code'], 127)
            self.assertEqual(row['status'], 'FAIL')
            self.assertIsNone(row['process_id'])

    def test_invalid_label_cannot_escape_directory_or_execute(self):
        with tempfile.TemporaryDirectory() as folder:
            marker = Path(folder) / 'executed'
            result = self.run_stage(folder, sys.executable, '-c',
                                    f'open({str(marker)!r}, "w").close()', label='../escape')
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(marker.exists())
            self.assertFalse(list(Path(folder).glob('*.json')))

    def test_interruption_reaps_owned_child_and_records_failure(self):
        with tempfile.TemporaryDirectory() as folder:
            marker = Path(folder) / 'child'
            process = subprocess.Popen([sys.executable, str(TIMER), '--directory', folder,
                                        '--label', 'interrupted', '--', sys.executable, '-c',
                                        f'import os,time; open({str(marker)!r}, "w").write(str(os.getpid())); time.sleep(30)'],
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 10
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(marker.exists())
                child = int(marker.read_text())
                process.send_signal(signal.SIGTERM)
                process.communicate(timeout=15)
                self.assertEqual(process.returncode, 143)
                with self.assertRaises(ProcessLookupError):
                    os.kill(child, 0)
                row, = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
                self.assertEqual(row['process_id'], child)
                self.assertEqual(row['status'], 'FAIL')
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate()

    def test_uncertain_reap_has_bounded_waits_and_failure_record(self):
        spec = importlib.util.spec_from_file_location('timer_fixture', TIMER)
        timer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(timer)
        waits = []
        class Child:
            pid = 99999999
            def poll(self): return None
            def wait(self, timeout=None):
                waits.append(timeout)
                if timeout is None: raise AssertionError('unbounded wait')
                raise subprocess.TimeoutExpired('fixture', timeout)
        def spawn(*_args, **_kwargs):
            signal.raise_signal(signal.SIGTERM)
            return Child()
        with tempfile.TemporaryDirectory() as folder:
            with patch.object(sys, 'argv', [str(TIMER), '--directory', folder, '--label', 'uncertain', '--', 'fixture']), patch.object(timer.subprocess, 'Popen', side_effect=spawn), patch.object(timer.os, 'killpg'), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(timer.main(), 143)
            row, = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
            self.assertFalse(row['wait_reaped'])
            self.assertTrue(row['cleanup_uncertain'])
            self.assertIsNone(row['child_returncode'])
            self.assertEqual(row['status'], 'FAIL')
            self.assertTrue(waits and all(0 < w <= 10 for w in waits))

    def test_signal_failure_records_cleanup_uncertainty(self):
        spec = importlib.util.spec_from_file_location('timer_fixture', TIMER)
        timer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(timer)
        class Child:
            pid = 99999999
            def poll(self): return None
        def spawn(*_args, **_kwargs):
            signal.raise_signal(signal.SIGTERM)
            return Child()
        with tempfile.TemporaryDirectory() as folder:
            with patch.object(sys, 'argv', [str(TIMER), '--directory', folder, '--label', 'signal-fault', '--', 'fixture']), patch.object(timer.subprocess, 'Popen', side_effect=spawn), patch.object(timer.os, 'killpg', side_effect=OSError('fixture signal failure')), contextlib.redirect_stdout(io.StringIO()):
                self.assertNotEqual(timer.main(), 0)
            row, = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
            self.assertFalse(row['wait_reaped'])
            self.assertTrue(row['cleanup_uncertain'])
            self.assertEqual(row['status'], 'FAIL')

    def test_child_signal_is_failure_and_returned(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.run_stage(folder, sys.executable, '-c',
                                    'import os,signal; os.kill(os.getpid(),signal.SIGTERM)')
            self.assertEqual(result.returncode, 143)
            row, = [json.loads(p.read_text()) for p in Path(folder).glob('*.json')]
            self.assertEqual(row['child_returncode'], -15)
            self.assertEqual(row['status'], 'FAIL')


if __name__ == '__main__':
    unittest.main()
