"""Gate-first controls for compilation reuse; assertions always execute afresh."""
import importlib.util
import os
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('acceptance_build', HERE / 'acceptance_build.py')
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class AcceptanceBuildTests(unittest.TestCase):
    def test_closed_opt_in_and_jobs(self):
        self.assertEqual(build.settings({}), (False, '2'))
        self.assertEqual(build.settings({'RAMEN_FOUNDRY_BUILD_CACHE': '1', 'CARGO_BUILD_JOBS': '3'}), (True, '3'))
        for env in ({'RAMEN_FOUNDRY_BUILD_CACHE': 'yes'}, {'CARGO_BUILD_JOBS': '0'},
                    {'CARGO_BUILD_JOBS': '33'}, {'CARGO_BUILD_JOBS': '02'}, {'CARGO_BUILD_JOBS': 'x'}):
            with self.subTest(env=env), self.assertRaises(ValueError): build.settings(env)

    def test_phase_identity_reuses_only_identical_features(self):
        calls = []
        cache = types.SimpleNamespace(target_for=lambda phase, features: calls.append((phase, features)) or Path('/cache') / phase)
        obj = build.Compilation.__new__(build.Compilation)
        obj.commands = types.SimpleNamespace(env={})
        obj.cache = cache; obj.targets = {}; obj.fresh_target = Path('/fresh'); obj.closed = False; obj.finished = False
        self.assertEqual(obj.phase('enabled', ['f']), obj.phase('enabled', ['f']))
        self.assertEqual(len(calls), 1)
        obj.phase('exclusion', [])
        self.assertEqual(len(calls), 2)
        with self.assertRaises(ValueError): obj.phase('enabled', ['other'])
        for phase, features in [('bad/path', []), ('enabled', ['f', 'f']), ('ok', ['bad/feature'])]:
            with self.subTest(phase=phase), self.assertRaises(ValueError): obj.phase(phase, features)

    def test_close_without_acceptance_never_completes_cache(self):
        actions = []
        obj = build.Compilation.__new__(build.Compilation)
        obj.cache = types.SimpleNamespace(check_contexts=lambda: actions.append('completed'), close=lambda: actions.append('closed')); obj.closed = False; obj.finished = False
        obj.close(); self.assertEqual(actions, ['closed'])
        with self.assertRaises(ValueError): obj.finish()
        success = build.Compilation.__new__(build.Compilation)
        success.cache = obj.cache; success.closed = False; success.finished = False
        success.finish()
        with self.assertRaises(ValueError): success.phase('late', [])
        success.close(accepted=True); self.assertEqual(actions, ['closed', 'completed', 'closed'])

    def test_context_failure_does_not_complete(self):
        actions = []
        def deny(): raise ValueError('context changed')
        obj = build.Compilation.__new__(build.Compilation)
        obj.cache = types.SimpleNamespace(check_contexts=deny, close=lambda: actions.append('closed')); obj.closed = False; obj.finished = False
        with self.assertRaises(ValueError): obj.finish()
        obj.close(); self.assertEqual(actions, ['closed'])

    def test_held_helper_pin_rejects_changed_bytes(self):
        raw = (HERE / 'build_cache.py').read_bytes()
        h = types.SimpleNamespace(source_read=lambda *a: (raw + b'\n', len(raw) + 1, '0' * 64))
        with self.assertRaises(ValueError): build.load_cache(h, 0, [{'relative_path': build.CACHE_SOURCE, 'sha256': build.CACHE_SHA}])

    def test_task_and_regression_use_profile_and_separate_exclusion(self):
        task = (HERE / 'desktop_editor_task_result.py').read_text()
        regression = (HERE / 'editor_reliability_gate.py').read_text()
        for source in (task, regression): self.assertIn('Compilation(', source)
        self.assertIn("phase('exclusion-app', [])", task)
        self.assertIn("phase('manifest-verifier', ['std'])", task)
        self.assertIn('compilation.finish()', task)
        self.assertIn('compilation.finish()', regression)


if __name__ == '__main__': unittest.main()
