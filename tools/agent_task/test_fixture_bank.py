#!/usr/bin/env python3
"""Gate-first tests for frozen, disjoint fixtures and explicit partition access."""

import copy
import tempfile
import unittest
from pathlib import Path

from fixture_bank import BankError, create, verify, select_fixture, release_record


class BankTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "private"
        self.manifest = create(
            self.root, b"validator-test", seed=b"x" * 32, synthetic=True
        )

    def tearDown(self):
        self.temp.cleanup()

    def test_disjoint_complete_and_reproducible(self):
        self.assertEqual(len(verify(self.root)["instances"]), 45)
        self.assertEqual(
            len({i["instance_id"] for i in self.manifest["instances"]}), 45
        )
        other = Path(self.temp.name) / "second"
        self.assertEqual(
            create(other, b"validator-test", seed=b"x" * 32, synthetic=True),
            self.manifest,
        )
        self.assertEqual(
            len({i["input_sha256"] for i in self.manifest["instances"]}), 45
        )

    def test_selection_and_release_are_bound(self):
        for partition in ("pilot", "final"):
            item = next(
                i for i in self.manifest["instances"] if i["partition"] == partition
            )
            with self.assertRaises(BankError):
                select_fixture(self.root, item["instance_id"], partition)
            release = release_record(self.manifest, partition, "scripted-contract-test")
            selected = select_fixture(
                self.root, item["instance_id"], partition, release=release
            )
            self.assertEqual(selected[0].name, "inputs")
            altered = dict(release, bank_sha256="0" * 64)
            with self.assertRaises(BankError):
                select_fixture(
                    self.root, item["instance_id"], partition, release=altered
                )
            altered = dict(release, schema_version=True)
            with self.assertRaises(BankError):
                select_fixture(
                    self.root, item["instance_id"], partition, release=altered
                )
            with self.assertRaises(BankError):
                select_fixture(self.root, item["instance_id"], "development")

    def test_tampering_and_symlinks_fail_closed(self):
        item = self.manifest["instances"][0]
        fixture, _ = select_fixture(self.root, item["instance_id"], "development")
        p = fixture / "config.json"
        p.write_bytes(b"{}")
        with self.assertRaises(BankError):
            verify(self.root)
        p.unlink()
        p.symlink_to(fixture / "schema.json")
        with self.assertRaises(BankError):
            verify(self.root)

    def test_manifest_forgery_and_extra_fields(self):
        from fixture_bank import validate_manifest

        for mutate in (
            lambda m: m["instances"].append(copy.deepcopy(m["instances"][0])),
            lambda m: m["instances"][1].update(
                input_sha256=m["instances"][0]["input_sha256"]
            ),
            lambda m: m.update(schema_version=True),
            lambda m: m["instances"][0].update(oracle="leak"),
        ):
            m = copy.deepcopy(self.manifest)
            mutate(m)
            with self.assertRaises(BankError):
                validate_manifest(m)

    def test_private_oracle_is_not_in_inputs(self):
        for item in self.manifest["instances"]:
            fixture = self.root / item["partition"] / item["instance_id"] / "inputs"
            self.assertEqual(
                set(p.name for p in fixture.iterdir()),
                {
                    "config.json",
                    "schema.json",
                    "notes.txt",
                    "policy.json",
                    "validator.wasm",
                },
            )
            self.assertEqual(
                (fixture.parent / "oracle.json").stat().st_mode & 0o777, 0o600
            )
            self.assertNotIn("oracle", item["task_text"])

    def test_real_release_requires_commit_and_study_binding(self):
        other = Path(self.temp.name) / "operator-private"
        m = create(other, b"validator-test", seed=b"y" * 32)
        with self.assertRaises(BankError):
            release_record(m, "final", "frozen-final-comparison")
        release = release_record(
            m,
            "final",
            "frozen-final-comparison",
            bank_commit="a" * 40,
            study_sha256="b" * 64,
        )
        item = next(i for i in m["instances"] if i["partition"] == "final")
        select_fixture(other, item["instance_id"], "final", release=release)


if __name__ == "__main__":
    unittest.main()
