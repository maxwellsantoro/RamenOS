#!/usr/bin/env python3
"""Keep unrelated Store tests free of the agent fixture's WAT parser."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]

class StoreTestDependencies(unittest.TestCase):
    def test_wat_is_optional_and_only_agent_feature_admits_it(self):
        manifest = tomllib.loads((ROOT/'services/store_service/Cargo.toml').read_text())
        self.assertNotIn('wat', manifest.get('dev-dependencies', {}))
        self.assertEqual(manifest['dependencies']['wat'], {'version':'1','optional':True})
        self.assertIn('dep:wat', manifest['features']['agent_task_v1_dev'])
        for feature, dependencies in manifest['features'].items():
            if feature != 'agent_task_v1_dev':
                self.assertNotIn('dep:wat', dependencies)
        self.assertEqual(manifest['features']['default'], [])

    def test_agent_example_and_native_test_feature_requirements_preserved(self):
        manifest = tomllib.loads((ROOT/'services/store_service/Cargo.toml').read_text())
        examples = {row['name']:row for row in manifest['example']}
        self.assertEqual(examples['export_agent_task_fixture']['required-features'], ['agent_task_v1_dev'])
        tests = {row['name']:row for row in manifest['test']}
        for name, feature in [('editor_store','editor_store_v0_dev'), ('editor_native_read','editor_native_read_v0_dev'), ('editor_native_preview_read','editor_native_preview_v0_dev'), ('editor_reliability','editor_native_save_v0_dev')]:
            self.assertEqual(tests[name]['required-features'], [feature])
        source=(ROOT/'services/store_service/tests/agent_task_service.rs').read_text()
        self.assertIn('#![cfg(feature = "agent_task_v1_dev")]', source)

if __name__ == '__main__': unittest.main()
