#!/usr/bin/env python3
"""Recipe parameters remain data before the validating developer CLI runs."""
from pathlib import Path
import shlex
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class RecipeTests(unittest.TestCase):
    def test_development_parameters_are_shell_quoted(self):
        package = "unknown'; touch /tmp/ramen-should-not-run; #"
        target = 'lib$(false)'
        features = 'x y'
        result = subprocess.run(['just', '--dry-run', 'dev-check', package, target, features],
                                cwd=ROOT, capture_output=True, text=True, check=True)
        self.assertEqual(shlex.split(result.stderr.strip()),
                         ['python3', 'tools/ci/dev_check.py', '--package', package,
                          '--test', target, '--features', features])

    def test_lane_parameter_is_shell_quoted(self):
        lane = "quality'; false; #"
        result = subprocess.run(['just', '--dry-run', 'ci-lane', lane], cwd=ROOT,
                                capture_output=True, text=True, check=True)
        self.assertEqual(shlex.split(result.stderr.strip()),
                         ['python3', 'tools/ci/ci_lanes.py', 'lane', lane])


if __name__ == '__main__':
    unittest.main()
