"""Gate-first controls for an additive isolated editor lane and safe cache restoration."""
import importlib.util
import itertools
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / (name + '.py'))
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    spec.loader.exec_module(module); return module
lanes = load('ci_lanes')
policy = load('change_policy')
EDITOR = ('desktop-launch', 'desktop-editor-host', 'editor-save-schema', 'editor-preview-codec',
          'editor-native-save-codec', 'desktop-editor-store', 'editor-native-read',
          'editor-native-preview', 'desktop-editor-task', 'editor-adapter-migration', 'editor-reliability')


class StreamliningTests(unittest.TestCase):
    def test_editor_partition_preserves_complete_serial_inventory(self):
        editor = lanes.lane_plan('editor')
        self.assertEqual([s.name for s in editor[:3]], ['compiler-cache-prerequisites', 'install-toolchain', 'codegen'])
        self.assertEqual(tuple(s.name for s in editor[3:]), EDITOR)
        self.assertFalse(any(s.name in ("proof-image", "proof-prerequisites") for s in editor))
        inventory = [s.name for lane in ('host', 'editor', 'agent', 'qemu')
                     for s in lanes.lane_plan(lane) if s in lanes.extended_plan()]
        self.assertCountEqual(inventory, [s.name for s in lanes.extended_plan()])
        self.assertEqual(len(inventory), len(set(inventory)))
        self.assertEqual(len(lanes.preflight_plan()), 56)

    def test_five_explicit_required_results(self):
        states = ('success', 'failure', 'cancelled', 'skipped', '', 'unknown')
        for results in itertools.product(states, repeat=5):
            self.assertEqual(policy.foundry_allowed('success', 'true', *results), results == ('success',) * 5)
            self.assertEqual(policy.foundry_allowed('success', 'false', *results), results == ('skipped',) * 5)
        with self.assertRaises(TypeError): policy.foundry_allowed('success', 'true', *(['success'] * 4))

    def test_workflow_editor_required_and_compiler_only_fallback(self):
        source = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertIn('  editor:\n', source)
        self.assertIn('needs: [changes, quality, host-docker, agent-task, qemu, editor]', source)
        self.assertIn('EDITOR: ${{ needs.editor.result }}', source)
        self.assertIn('"$QEMU" "$EDITOR"', source)
        self.assertIn('ci_lanes.py lane editor', source)
        self.assertIn('foundry-compile-v2-', source)
        # Prior archives supply compiler candidates only; inner keys still verify all inputs.
        self.assertIn('restore-keys:', source.split('Cache isolated compiler targets', 1)[1])
        self.assertNotIn('path: out/desktop', source)
        import re
        block = source.split('Cache isolated compiler targets', 1)[1].split('- name:', 1)[0]
        self.assertEqual(re.findall(r'(?m)^\s+path: (.+)$', block), ['out/ci-optimization/compiler-cache'])
        self.assertIn("${{ runner.os }}-${{ runner.arch }}-foundry-compile-v2-${{ hashFiles('rust-toolchain.toml') }}-", block)
        for forbidden in ('out/desktop', 'out/foundry/timings', 'out/evidence'):
            self.assertNotIn(forbidden, block)


if __name__ == '__main__': unittest.main()
