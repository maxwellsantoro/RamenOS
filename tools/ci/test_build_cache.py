"""Gate-first compiler-cache unit controls; no Foundry or Cargo execution."""
import copy
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).parent
spec = importlib.util.spec_from_file_location('build_cache', HERE / 'build_cache.py')
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)
SCRATCH = HERE.parents[1] / 'out/ci-optimization/cache-tests'


def inputs():
    return {'schema_version': 1, 'manifest_sha256': 'a' * 64,
            'toolchain': {'rustc': 'rustc pinned host', 'cargo': 'cargo pinned'},
            'platform': {'system': 'test', 'machine': 'test'},
            'repo': '/private/repo', 'lock_sha256': 'b' * 64,
            'config': [], 'environment': [], 'phase': 'enabled',
            'features': ['editor_native_read_v0_dev'], 'default_features': False,
            'profile': 'dev', 'scope': 'acceptance'}


class BuildCacheTests(unittest.TestCase):
    def setUp(self):
        SCRATCH.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=SCRATCH)
        self.root = Path(self.temp.name) / 'cache'

    def tearDown(self):
        self.temp.cleanup()

    def test_exact_key_changes_for_every_input(self):
        base = inputs(); original = cache.key_for(base)
        mutations = {'manifest_sha256': 'c' * 64, 'lock_sha256': 'd' * 64,
                     'toolchain': {'rustc': 'different', 'cargo': 'cargo pinned'},
                     'platform': {'system': 'other', 'machine': 'test'},
                     'config': [{'path': '/config', 'sha256': 'e' * 64}],
                     'environment': [['RUSTFLAGS', 'c' * 64]],
                     'phase': 'exclusion-desktop', 'features': ['different'],
                     'default_features': True, 'profile': 'release',
                     'repo': '/other/repo', 'scope': 'development'}
        for key, value in mutations.items():
            with self.subTest(key=key):
                changed = copy.deepcopy(base); changed[key] = value
                self.assertNotEqual(cache.key_for(changed), original)
        self.assertEqual(cache.key_for(dict(reversed(list(base.items())))), original)

    def test_input_snapshot_cannot_change_after_key_selection(self):
        data=inputs(); expected=cache.key_for(data)
        lease=cache.CacheLease(self.root,data)
        data['phase']='mutated'
        with lease:
            self.assertEqual(lease.key,expected)
            self.assertEqual(lease.metadata['phase'],'enabled')

    def test_first_miss_then_hit_preserves_target(self):
        with cache.CacheLease(self.root, inputs()) as lease:
            self.assertFalse(lease.metadata['hit']); target = lease.target
            (target / 'owned-output').write_bytes(b'compiled')
        with cache.CacheLease(self.root, inputs()) as lease:
            self.assertTrue(lease.metadata['hit']); self.assertEqual(lease.target, target)
            self.assertEqual((target / 'owned-output').read_bytes(), b'compiled')

    def test_feature_phases_do_not_alias(self):
        a = inputs(); b = inputs(); b['phase'] = 'exclusion-desktop'
        with cache.CacheLease(self.root, a) as first:
            with cache.CacheLease(self.root, b) as second:
                self.assertNotEqual(first.target, second.target)
                (first.target / 'lib').write_bytes(b'enabled')
                self.assertFalse((second.target / 'lib').exists())

    def test_exclusive_lock_rejects_second_owner(self):
        with cache.CacheLease(self.root, inputs()):
            with self.assertRaises(cache.CacheDenied):
                with cache.CacheLease(self.root, inputs()):
                    self.fail('second owner admitted')
        with cache.CacheLease(self.root, inputs()) as lease:
            self.assertTrue(lease.metadata['hit'])

    def test_symlink_root_and_ancestor_rejected(self):
        real = Path(self.temp.name) / 'real'; real.mkdir(mode=0o700)
        self.root.symlink_to(real, target_is_directory=True)
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, inputs()): pass
        nested = self.root / 'nested'
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(nested, inputs()): pass

    def test_insecure_root_and_target_rejected(self):
        self.root.mkdir(mode=0o755)
        self.root.chmod(0o755)  # Explicitly insecure even under a private umask.
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, inputs()): pass
        self.root.chmod(0o700)
        with cache.CacheLease(self.root, inputs()) as lease: target = lease.target
        target.chmod(0o777)
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, inputs()): pass

    def test_changed_context_fails_closed(self):
        with cache.CacheLease(self.root, inputs()) as lease: entry = lease.target.parent
        p = entry / 'context.json'; p.write_bytes(b'{}')
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, inputs()): pass

    def test_failed_body_abandons_marker_and_denies_reuse(self):
        with self.assertRaisesRegex(RuntimeError, 'work failed'):
            with cache.CacheLease(self.root, inputs()): raise RuntimeError('work failed')
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, inputs()): pass

    def test_gate_failure_abandons_all_phase_markers(self):
        repo = Path(self.temp.name); (repo / 'Cargo.lock').write_bytes(b'lock')
        g = cache.GateCache(repo, 'a' * 64, 'rustc', 'cargo', {'CARGO_HOME':str(repo / 'home')})
        target = g.target_for('enabled', ['std']); g.close()
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(g.root, cache.key_inputs(repo,'a'*64,'enabled',['std'],'rustc','cargo',g.environment)): pass
        self.assertTrue((target.parent / '.inflight').exists())

    def test_retained_binary_survives_target_change_and_rejects_wrong_pin(self):
        source=Path(self.temp.name)/'actual-binary';source.write_bytes(b'actual fixture bytes');source.chmod(0o700)
        info={'path':str(source),'byte_len':source.stat().st_size,'sha256':hashlib.sha256(source.read_bytes()).hexdigest()}
        retained=Path(self.temp.name)/'test-binary'
        record=cache.retain_binary(source,info,retained)
        self.assertEqual(record['sha256'],info['sha256']);self.assertEqual(retained.read_bytes(),b'actual fixture bytes')
        source.write_bytes(b'different later cached output')
        self.assertEqual(retained.read_bytes(),b'actual fixture bytes')
        wrong=Path(self.temp.name)/'wrong-copy'
        with self.assertRaises(cache.CacheDenied): cache.retain_binary(source,info,wrong)
        self.assertFalse(wrong.exists())
        with self.assertRaises(cache.CacheDenied): cache.retain_binary(source,info,retained)
        self.assertEqual(retained.read_bytes(),b'actual fixture bytes')

    def test_config_lock_mutation_and_nested_shapes(self):
        repo = Path(self.temp.name)
        (repo / 'Cargo.lock').write_bytes(b'lock-v1')
        (repo / '.cargo').mkdir(mode=0o700)
        cfg = repo / '.cargo/config.toml'; cfg.write_bytes(b'[build]\njobs=1\n')
        env = {'CARGO_HOME': str(repo / 'cargo-home')}
        a = cache.key_inputs(repo, 'a' * 64, 'enabled', ['std'], 'rustc', 'cargo', env)
        cfg.write_bytes(b'[build]\njobs=2\n')
        b = cache.key_inputs(repo, 'a' * 64, 'enabled', ['std'], 'rustc', 'cargo', env)
        self.assertNotEqual(cache.key_for(a), cache.key_for(b))
        (repo / 'Cargo.lock').write_bytes(b'lock-v2')
        c = cache.key_inputs(repo, 'a' * 64, 'enabled', ['std'], 'rustc', 'cargo', env)
        self.assertNotEqual(cache.key_for(b), cache.key_for(c))
        bad = inputs(); bad['config'] = [{'path': '/config', 'sha256': 'not-a-pin'}]
        with self.assertRaises(cache.CacheDenied): cache.key_for(bad)
        bad = inputs(); bad['environment'] = [['RUSTFLAGS', 'unhashed']]
        with self.assertRaises(cache.CacheDenied): cache.key_for(bad)

    def test_configuration_snapshot_reuses_key_rows_and_rejects_insecure_absence(self):
        repo = Path(self.temp.name)
        (repo / 'Cargo.lock').write_bytes(b'lock')
        home = repo / 'cargo-home'; home.mkdir(mode=0o700)
        env = {'CARGO_HOME': str(home)}
        before = sorted(p.name for p in repo.iterdir())
        rows = cache.configuration_snapshot(repo, env)
        self.assertEqual(rows, cache.key_inputs(repo, 'a'*64, 'enabled', [], 'rustc', 'cargo', env)['config'])
        self.assertIn({'path': str(home / 'config.toml'), 'sha256': None}, rows)
        self.assertEqual(sorted(p.name for p in repo.iterdir()), before)
        self.assertEqual(list(home.iterdir()), [])
        home.chmod(0o775)
        with self.assertRaisesRegex(cache.CacheDenied, 'writable cache ancestor'):
            cache.configuration_snapshot(repo, env)
        self.assertEqual(list(home.iterdir()), [])
        home.chmod(0o700)
        (repo / '.cargo').mkdir(mode=0o700)
        cfg = repo / '.cargo/config.toml'; cfg.write_bytes(b'[build]\njobs=1\n'); cfg.chmod(0o664)
        with self.assertRaisesRegex(cache.CacheDenied, 'regular shape'):
            cache.configuration_snapshot(repo, env)

    def test_check_inputs_cli_readonly_denial_and_closed_arguments(self):
        home = Path(self.temp.name) / 'cargo-home'; home.mkdir(mode=0o700)
        env = {'PATH': '', 'HOME': str(Path(self.temp.name)), 'CARGO_HOME': str(home),
               'RAMEN_FOUNDRY_BUILD_CACHE': '1', 'PYTHONDONTWRITEBYTECODE': '1'}
        argv = [sys.executable, str(HERE / 'build_cache.py'), '--check-inputs']
        result = subprocess.run(argv, env=env, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(b'BUILD_CACHE_INPUTS: PASS', result.stdout)
        self.assertEqual(list(home.iterdir()), [])
        home.chmod(0o775)
        result = subprocess.run(argv, env=env, capture_output=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'writable cache ancestor', result.stderr)
        self.assertEqual(list(home.iterdir()), [])
        # Disabled means no lock/config read, even with this denied path.
        env['RAMEN_FOUNDRY_BUILD_CACHE'] = '0'
        result = subprocess.run(argv, env=env, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0)
        self.assertIn(b'BUILD_CACHE_INPUTS: INFO disabled', result.stdout)
        env['RAMEN_FOUNDRY_BUILD_CACHE'] = '2'
        self.assertNotEqual(subprocess.run(argv, env=env, capture_output=True, timeout=5).returncode, 0)
        env['RAMEN_FOUNDRY_BUILD_CACHE'] = '0'
        self.assertNotEqual(subprocess.run(argv + ['--extra'], env=env, capture_output=True, timeout=5).returncode, 0)
        self.assertNotEqual(subprocess.run(argv[:-1], env=env, capture_output=True, timeout=5).returncode, 0)
        for args in (['--check-input'], ['--check-inputs', '--check-inputs'], ['--help']):
            self.assertNotEqual(subprocess.run(argv[:-1] + args, env=env, capture_output=True, timeout=5).returncode, 0)

    def test_process_umask_guard_restores_caller_even_on_denial(self):
        from unittest.mock import patch
        for mask in (0o002, 0o022, 0o077):
            original = os.umask(mask)
            try:
                if mask == 0o002:
                    with self.assertRaisesRegex(cache.CacheDenied, 'umask'):
                        cache.check_process_umask()
                else:
                    self.assertEqual(cache.check_process_umask(), mask)
                # Query also resets to the same mask; returned prior value proves restoration.
                self.assertEqual(os.umask(mask), mask)
            finally:
                os.umask(original)
        with patch.dict(os.environ, {'RAMEN_FOUNDRY_BUILD_CACHE': '0'}, clear=True), \
             patch.object(sys, 'argv', [str(HERE / 'build_cache.py'), '--check-inputs']), \
             patch.object(cache.os, 'umask', side_effect=AssertionError('disabled query')):
            self.assertEqual(cache.main(), 0)

    def test_gate_cache_rejects_unsafe_umask_before_directory_creation(self):
        repo = Path(self.temp.name); (repo / 'Cargo.lock').write_bytes(b'lock')
        original = os.umask(0o002)
        try:
            with self.assertRaisesRegex(cache.CacheDenied, 'umask'):
                cache.GateCache(repo, 'a'*64, 'rustc', 'cargo', {})
            self.assertFalse((repo / 'out').exists())
            self.assertEqual(os.umask(0o002), 0o002)
        finally:
            os.umask(original)

    def test_check_inputs_cli_umask_002_denied_022_077_pass(self):
        home = Path(self.temp.name) / 'cargo-home'; home.mkdir(mode=0o700)
        env = {'PATH': '', 'CARGO_HOME': str(home), 'HOME': str(Path(self.temp.name)),
               'RAMEN_FOUNDRY_BUILD_CACHE': '1', 'PYTHONDONTWRITEBYTECODE': '1'}
        argv = [sys.executable, str(HERE / 'build_cache.py'), '--check-inputs']
        for mask in (0o002, 0o022, 0o077):
            result = subprocess.run(argv, env=env, capture_output=True, timeout=5,
                                    preexec_fn=lambda: os.umask(mask))
            if mask == 0o002:
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'umask', result.stderr)
            else:
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(b'BUILD_CACHE_INPUTS: PASS', result.stdout)
            self.assertEqual(list(home.iterdir()), [])
        env['RAMEN_FOUNDRY_BUILD_CACHE'] = '0'
        result = subprocess.run(argv, env=env, capture_output=True, timeout=5,
                                preexec_fn=lambda: os.umask(0o002))
        self.assertEqual(result.returncode, 0)
        self.assertIn(b'INFO disabled', result.stdout)

    def test_invalid_keys_and_entry_cap_rejected(self):
        bad = inputs(); bad['manifest_sha256'] = 'unresolved'
        with self.assertRaises(cache.CacheDenied): cache.key_for(bad)
        bad = inputs(); bad['unexpected'] = True
        with self.assertRaises(cache.CacheDenied): cache.key_for(bad)
        with cache.CacheLease(self.root, inputs(), max_entries=1): pass
        other = inputs(); other['phase'] = 'other'
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, other, max_entries=1): pass


if __name__ == '__main__': unittest.main()
