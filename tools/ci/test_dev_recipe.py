#!/usr/bin/env python3
"""Recipe parameters remain data before the validating developer CLI runs."""
from pathlib import Path
import shlex
import os
import json
import sys
import tempfile
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class RecipeTests(unittest.TestCase):
    def invoke_development_recipe(self, recipe, mode, jobs=None):
        scratch = ROOT/'out/ci-optimization/recipe-tests'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as temp:
            root = Path(temp); capture = root/'argv.json'; sentinel=root/'injected'
            binary=root/'python3'
            binary.write_text('#!'+sys.executable+'\nimport sys,os,json,pathlib\n'
                              'pathlib.Path(os.environ["RECIPE_CAPTURE"]).write_text(json.dumps(sys.argv[1:]))\n')
            binary.chmod(0o700)
            package = "unknown'; touch " + str(sentinel) + "; #"
            target = 'lib$(false)'; features = 'x y'
            args = [recipe, package, target, features]
            if jobs is not None: args.append(jobs)
            result = subprocess.run(['just', *args], cwd=ROOT,
                                    env=dict(os.environ, PATH=str(root)+os.pathsep+os.environ['PATH'], RECIPE_CAPTURE=str(capture)),
                                    capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            expected = ['tools/ci/dev_check.py','--package',package,'--test',target,'--features',features]
            if mode is not None: expected += ['--mode',mode]
            if jobs is not None and jobs != '': expected += ['--jobs',jobs]
            self.assertEqual(json.loads(capture.read_text()), expected)
            self.assertFalse(sentinel.exists())

    def test_development_parameters_are_shell_quoted(self):
        self.invoke_development_recipe('dev-check', None)

    def test_fast_development_modes_and_jobs_remain_quoted_data(self):
        for recipe,mode in [('dev-test','test'),('dev-compile','check'),('dev-lint','lint')]:
            with self.subTest(recipe=recipe):
                self.invoke_development_recipe(recipe, mode)
                self.invoke_development_recipe(recipe, mode, '')
                self.invoke_development_recipe(recipe, mode, "3'; false; #")
        self.invoke_development_recipe('dev-check', None, '4')

    def test_lane_parameter_is_shell_quoted(self):
        lane = "quality'; false; #"
        result = subprocess.run(['just', '--dry-run', 'ci-lane', lane], cwd=ROOT,
                                capture_output=True, text=True, check=True)
        self.assertEqual(shlex.split(result.stderr.strip()),
                         ['python3', 'tools/ci/ci_lanes.py', 'lane', lane])


if __name__ == '__main__':
    unittest.main()
