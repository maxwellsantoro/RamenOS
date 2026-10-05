#!/usr/bin/env python3
"""Strict Cargo output selector fixtures for the two Store gates."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

from cargo_artifact import select_executable


class CargoArtifactTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="cargo-artifact-fixture-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / "server with spaces"
        self.binary.write_text("#!/bin/sh\nexit 0\n")
        self.binary.chmod(0o700)
        self.artifact = {"reason": "compiler-artifact",
                         "target": {"name": "store_service", "kind": ["bin"]},
                         "profile": {"test": False}, "executable": str(self.binary)}
        self.finish = {"reason": "build-finished", "success": True}

    def write(self, rows):
        path = self.root / "build.jsonl"
        path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        return path

    def test_one_actual_non_test_bin_and_same_name_library(self):
        library = copy.deepcopy(self.artifact)
        library["target"]["kind"] = ["lib"]
        library["executable"] = None
        path = self.write([library, self.artifact, self.finish])
        self.assertEqual(select_executable(path, "store_service"), str(self.binary))
        other = copy.deepcopy(self.artifact)
        other["target"]["name"] = "runtime_supervisor"
        path = self.write([self.artifact, other, self.finish])
        self.assertEqual(select_executable(path, "runtime_supervisor"), str(self.binary))

    def test_wrong_missing_duplicate_and_malformed_artifacts(self):
        nonexecutable = self.root / "nonexecutable"
        nonexecutable.write_text("fixture")
        cases = {"duplicate": [self.artifact, self.artifact, self.finish],
                 "missing": [self.finish], "missing_finish": [self.artifact],
                 "duplicate_finish": [self.artifact, self.finish, self.finish],
                 "failed_finish": [self.artifact, {"reason": "build-finished", "success": False}],
                 "scalar": [self.artifact, 7, self.finish]}
        for name, update in {
            "wrong_name": {"target": {"name": "other", "kind": ["bin"]}},
            "wrong_kind": {"target": {"name": "store_service", "kind": ["lib"]}},
            "ambiguous_kind": {"target": {"name": "store_service", "kind": ["bin", "lib"]}},
            "malformed_target": {"target": []},
            "malformed_profile": {"profile": {}},
            "test": {"profile": {"test": True}},
            "string_test": {"profile": {"test": "false"}},
            "null_executable": {"executable": None},
            "empty_executable": {"executable": ""},
            "relative_executable": {"executable": "server"},
            "missing_executable": {"executable": str(self.root / "missing")},
            "nonexecutable": {"executable": str(nonexecutable)},
            "control_path": {"executable": str(self.binary) + "\n"},
        }.items():
            cases[name] = [dict(self.artifact, **update), self.finish]
        for name, rows in cases.items():
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    select_executable(self.write(rows), "store_service")

    def test_invalid_json_keys_constants_and_truncation(self):
        valid = json.dumps(self.artifact)
        for name, raw in {
            "malformed": valid + "\n{bad\n",
            "duplicate_key": valid.replace('"test": false', '"test": false, "test": false') + "\n" + json.dumps(self.finish) + "\n",
            "nonfinite": valid + '\n{"reason":"other","value":NaN}\n' + json.dumps(self.finish) + "\n",
            "truncated": valid[:-1],
        }.items():
            with self.subTest(name=name):
                path = self.root / "invalid.jsonl"
                path.write_text(raw)
                with self.assertRaises(ValueError):
                    select_executable(path, "store_service")


if __name__ == "__main__":
    unittest.main()
