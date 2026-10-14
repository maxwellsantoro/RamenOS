"""Compiler retirement controls preserve evidence and refuse uncertain owners."""
import importlib.util
import json
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / (name + '.py'))
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module); return module
control = load('compiler_cache_control')
cache = load('build_cache')


class CacheControlTests(unittest.TestCase):
    def setUp(self):
        scratch = ROOT / 'out/ci-optimization/control-tests'; scratch.mkdir(parents=True, exist_ok=True)
        self.tmp = tempfile.TemporaryDirectory(dir=scratch)
        self.repo = Path(self.tmp.name); (self.repo / 'Cargo.lock').write_text('lock')
        (self.repo / 'out/ci-optimization').mkdir(parents=True)
        self.root = self.repo / 'out/ci-optimization/compiler-cache'
        self.inputs = cache.key_inputs(self.repo, 'a' * 64, 'enabled', [], 'rustc', 'cargo',
                                       {'CARGO_HOME': str(self.repo / 'home')})
    def tearDown(self): self.tmp.cleanup()

    def completed(self):
        with cache.CacheLease(self.root, self.inputs) as lease:
            (lease.target / 'compiled').write_bytes(b'compiler')
            self.key = lease.key
        return control.inspect_cache(self.repo, 'acceptance')['entries'][0]

    def test_missing_inspection_does_not_create_directories(self):
        self.assertFalse(self.root.exists())
        self.assertEqual(control.inspect_cache(self.repo, 'acceptance')['entries'], [])
        self.assertFalse(self.root.exists())

    def test_completed_retirement_preserves_evidence_and_next_allocation(self):
        row = self.completed(); evidence = self.repo / 'out/evidence'; evidence.mkdir()
        (evidence / 'result.json').write_text('preserve')
        control.retire(self.repo, 'acceptance', self.key, row['context_sha256'])
        self.assertFalse((self.root / self.key).exists())
        self.assertEqual((evidence / 'result.json').read_text(), 'preserve')
        with cache.CacheLease(self.root, self.inputs) as lease: self.assertFalse(lease.metadata['hit'])

    def test_active_and_quarantined_entries_are_not_retired(self):
        with cache.CacheLease(self.root, self.inputs) as lease:
            key = lease.key
            (lease.target / 'compiled').write_bytes(b'held')
            digest = hashlib.sha256((lease.target.parent / 'context.json').read_bytes()).hexdigest()
            with self.assertRaises((ValueError, OSError)): control.retire(self.repo, 'acceptance', key, digest)
            self.assertEqual((lease.target / 'compiled').read_bytes(), b'held')
            self.assertTrue((lease.target.parent / '.inflight').exists())
        try:
            with cache.CacheLease(self.root, self.inputs): raise RuntimeError('interrupted')
        except RuntimeError: pass
        row = control.inspect_cache(self.repo, 'acceptance')['entries'][0]
        self.assertEqual(row['state'], 'quarantined')
        with self.assertRaises(ValueError): control.retire(self.repo, 'acceptance', key, row['context_sha256'])
        self.assertTrue((self.root / key / '.inflight').exists())

    def test_context_mismatch_and_symlink_refuse_before_removal(self):
        row = self.completed()
        with self.assertRaises(ValueError): control.retire(self.repo, 'acceptance', self.key, '0' * 64)
        external = self.repo / 'external'; external.write_text('outside')
        (self.root / self.key / 'target/link').symlink_to(external)
        with self.assertRaises(ValueError): control.retire(self.repo, 'acceptance', self.key, row['context_sha256'])
        self.assertEqual(external.read_text(), 'outside')
        self.assertTrue((self.root / self.key / 'target/compiled').exists())

    def test_unknown_entry_fields_and_path_traversal_deny(self):
        row = self.completed()
        (self.root / self.key / 'evidence.json').write_text('unknown ownership')
        with self.assertRaises(ValueError): control.retire(self.repo, 'acceptance', self.key, row['context_sha256'])
        with self.assertRaises(ValueError): control.retire(self.repo, 'acceptance', '../evidence', row['context_sha256'])

    def test_missing_lock_is_never_created_and_enumeration_is_bounded(self):
        self.completed(); entry = self.root / self.key
        (entry / 'owner.lock').unlink()
        with self.assertRaises(FileNotFoundError): control.inspect_cache(self.repo, 'acceptance')
        self.assertFalse((entry / 'owner.lock').exists())
        fd = control.os.open(entry, control.os.O_RDONLY | control.os.O_DIRECTORY)
        try:
            with self.assertRaises(ValueError): control.bounded_names(fd, 1)
        finally: control.os.close(fd)

    def test_failed_retirement_quarantines_compiler_entry(self):
        row = self.completed()
        with patch.object(control, 'remove_tree', side_effect=OSError('interrupted deletion')):
            with self.assertRaises(OSError): control.retire(self.repo, 'acceptance', self.key, row['context_sha256'])
        self.assertTrue((self.root / self.key / '.inflight').exists())
        with self.assertRaises(cache.CacheDenied):
            with cache.CacheLease(self.root, self.inputs): pass


if __name__ == '__main__': unittest.main()
