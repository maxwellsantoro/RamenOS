"""Pure source admission regressions; never compile or start product processes."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


COLLECTOR = load('task_source_fixture', 'tools/ci/native_save_task_sources.py')
READ = load('read_source_fixture', 'tools/foundry/desktop_editor_native_read_gate.py')
PREVIEW = load('preview_source_fixture', 'tools/foundry/desktop_editor_native_preview_read_gate.py')


class SourceInventoryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / 'repo'
        self.repo.mkdir()
        fd = os.open(ROOT, os.O_RDONLY | os.O_DIRECTORY)
        try:
            names = set(COLLECTOR.source_names(fd))
        finally:
            os.close(fd)
        for gate in (READ, PREVIEW):
            names.update(json.loads((ROOT / gate.REGISTRY).read_bytes())['paths'])
        # The inventory adapter is not part of the original collector's extras.
        for name in ('tools/ci/editor_source_inventory.py',):
            if (ROOT / name).exists():
                names.add(name)
        for name in names:
            target = self.repo / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        self.fd = os.open(self.repo, os.O_RDONLY | os.O_DIRECTORY)
        self.addCleanup(os.close, self.fd)
        self.sequence = 0

    def freeze(self, gate):
        self.sequence += 1
        run = Path(self.tmp.name) / str(self.sequence)
        run.mkdir()
        if gate is READ:
            return gate.freeze_sources(self.fd, run)
        return gate.freeze_sources(gate.load_primitives(self.fd), self.fd, run)

    def test_registered_in_existing_quality_tooling_gate(self):
        gate = (ROOT / 'tools/ci/foundry_ci_optimization.sh').read_text()
        self.assertEqual(gate.count('python3 tools/ci/test_editor_source_inventory.py'), 1)

    def test_dependency_and_included_fixture_mutations_change_both_digests(self):
        for name in ('services/desktop_editor_core/src/lib.rs',
                     'services/desktop_editor_core/tests/fixtures/ascii8x16_v0.bin'):
            path = self.repo / name
            original = path.read_bytes()
            for gate in (READ, PREVIEW):
                with self.subTest(source=name, gate=gate.REGISTRY):
                    rows, before = self.freeze(gate)
                    self.assertTrue(name in {row['relative_path'] for row in rows}, name)
                    path.write_bytes(original + b'\n')
                    try:
                        _, after = self.freeze(gate)
                        self.assertNotEqual(before, after)
                    finally:
                        path.write_bytes(original)

    def test_future_transitive_path_dependency_omission_fails_closed(self):
        manifest = self.repo / 'services/desktop_editor_core/Cargo.toml'
        manifest.write_text(manifest.read_text().replace('[dependencies]',
                            '[dependencies]\nnew_editor_dep = { path = "../new_editor_dep" }'))
        crate = self.repo / 'services/new_editor_dep'
        (crate / 'src').mkdir(parents=True)
        (crate / 'Cargo.toml').write_text('[package]\nname="new_editor_dep"\nversion="0.0.0"\n')
        (crate / 'src/lib.rs').write_text('pub const NEW: u8 = 1;\n')
        for gate in (READ, PREVIEW):
            with self.subTest(gate=gate.REGISTRY):
                with self.assertRaises((ValueError, gate.h.Incomplete) if gate is READ else
                                       (ValueError, PREVIEW.load_primitives(self.fd).Incomplete)):
                    self.freeze(gate)

    def test_explicit_cargo_target_outside_collector_is_rejected(self):
        manifest = self.repo / 'services/desktop_editor_core/Cargo.toml'
        original = manifest.read_text()
        (manifest.parent / 'custom.rs').write_text('pub const CUSTOM: u8 = 1;\n')
        for kind in ('lib', 'bin', 'test', 'bench', 'example'):
            header = '[lib]' if kind == 'lib' else '[[' + kind + ']]'
            manifest.write_text(original + '\n' + header + '\nname="custom"\npath="custom.rs"\n')
            for gate in (READ, PREVIEW):
                with self.subTest(kind=kind, gate=gate.REGISTRY):
                    with self.assertRaisesRegex(ValueError, 'explicit Cargo target outside collected source layout'):
                        self.freeze(gate)
        manifest.write_text(original)

    def test_new_literal_include_outside_crate_requires_registry_review(self):
        source = self.repo / 'services/desktop_editor_core/src/lib.rs'
        source.write_bytes(source.read_bytes() + b'\nconst EXTRA: &str = include_str!("../../../new-fixture.txt");\n')
        (self.repo / 'new-fixture.txt').write_text('new compiled input\n')
        for gate in (READ, PREVIEW):
            with self.subTest(gate=gate.REGISTRY):
                with self.assertRaisesRegex(ValueError, 'transitive compilation input absent'):
                    self.freeze(gate)

    def test_missing_literal_include_fails_closed(self):
        (self.repo / 'services/desktop_editor_core/tests/fixtures/ascii8x16_v0.bin').unlink()
        for gate in (READ, PREVIEW):
            with self.subTest(gate=gate.REGISTRY):
                with self.assertRaises((OSError, ValueError, READ.h.Incomplete)):
                    self.freeze(gate)


if __name__ == '__main__':
    unittest.main()
