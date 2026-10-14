#!/usr/bin/env python3
"""The actual Cargo graph keeps Oracle providers opt-in in both boot crates."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class BootFeatureAuthority(unittest.TestCase):
    def graph(self, package, *features):
        result = subprocess.run(
            ["cargo", "tree", "--locked", "-p", package, "-e", "normal,build",
             "--prefix", "none", "--format", "{p}|{f}", *features],
            cwd=ROOT, capture_output=True, text=True, timeout=60,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [line.rsplit("|", 1)[1].removesuffix(" (*)")
                for line in result.stdout.splitlines() if line.startswith("kernel v")]
        self.assertTrue(rows, "resolved kernel node must exist")
        return set(rows[0].split(","))

    def test_default_boot_graph_has_no_test_protocols(self):
        for package in ("kernel_uefi", "kernel_aarch64"):
            with self.subTest(package=package):
                self.assertNotIn("test_protocols", self.graph(package))

    def test_explicit_oracle_graph_still_has_test_protocols(self):
        for package in ("kernel_uefi", "kernel_aarch64"):
            with self.subTest(package=package):
                self.assertIn("test_protocols",
                              self.graph(package, "--features", "kernel/test_protocols"))


if __name__ == "__main__":
    unittest.main()
