#!/usr/bin/env python3
"""Gate-first admission/outcome controls for Cargo-selected direct execution."""
import json
import tempfile
import sys
import os
import signal
import subprocess
import time
import unittest
from unittest import mock
from pathlib import Path
import selected_test_runner as runner


def rows(*items):
    return "\n".join(json.dumps(item) for item in items)


def artifact(name="store_service", kind="lib", package="fixture#store_service@1", executable="/target/unit"):
    return {"reason": "compiler-artifact", "package_id": package,
            "target": {"name": name, "kind": [kind], "src_path": "/fixture/src/lib.rs", "test": True},
            "profile": {"test": True}, "executable": executable}


def discovered(*names):
    return rows({"type": "suite", "event": "discovery"},
                *({"type": "test", "event": "discovered", "name": name, "ignore": False} for name in names),
                {"type": "suite", "event": "completed", "tests": len(names), "benchmarks": 0, "total": len(names), "ignored": 0})


def outcomes(*names, event="ok"):
    return rows({"type": "suite", "event": "started", "test_count": len(names)},
                *({"type": "test", "event": "started", "name": name} for name in names),
                *({"type": "test", "event": event, "name": name} for name in names),
                {"type": "suite", "event": "ok" if event == "ok" else "failed", "passed": len(names) if event == "ok" else 0,
                 "failed": 0 if event == "ok" else len(names), "ignored": 0, "measured": 0, "filtered_out": 0})


class SelectedRunnerTests(unittest.TestCase):
    def test_cargo_keeps_library_and_binary_targets(self):
        actual = runner.parse_artifacts(rows(artifact(), artifact(kind="bin", executable="/target/bin"), {"reason": "build-finished", "success": True}),
                                        "fixture#store_service@1", Path("/fixture"))
        self.assertEqual({row["kind"] for row in actual}, {"lib", "bin"})
        self.assertEqual(len(actual), 2)

    def test_cargo_rejects_missing_malformed_duplicate_and_foreign_sources(self):
        for text in ("bad-json", rows({"reason": "build-finished", "success": True}),
                     rows(artifact(), artifact(), {"reason": "build-finished", "success": True}),
                     rows({**artifact(), "target": {"name": "store_service", "kind": ["lib"], "src_path": "/foreign/lib.rs", "test": True}}, {"reason": "build-finished", "success": True}),
                     rows(artifact()), rows(artifact(), {"reason": "build-finished", "success": False}),
                     rows(artifact(), {"reason": "build-finished", "success": True}, artifact(executable="/target/late"))):
            with self.subTest(text=text), self.assertRaises(runner.RunnerError):
                runner.parse_artifacts(text, "fixture#store_service@1", Path("/fixture"))
        self.assertEqual(len(runner.parse_artifacts(rows(artifact(), artifact(package="foreign", executable="/target/foreign"), {"reason": "build-finished", "success": True}),
                                                   "fixture#store_service@1", Path("/fixture"))), 1)

    def test_discovery_rejects_ignored_duplicate_and_wrong_count(self):
        self.assertEqual(runner.parse_discovery(discovered("a")), {"a": False})
        for text in (discovered("a", "a"), discovered("a").replace('"tests": 1', '"tests": 2')):
            with self.assertRaises(runner.RunnerError):
                runner.parse_discovery(text)
        ignored = discovered("a").replace('"ignore": false', '"ignore": true').replace('"ignored": 0', '"ignored": 1')
        with self.assertRaises(runner.RunnerError):
            runner.select_cases({"bin": runner.parse_discovery(ignored)}, ["a"], [])

    def test_union_retains_matching_lib_and_bin_cases_once(self):
        selected = runner.select_cases({"lib": {"projection_index": False, "other": False},
                                        "bin": {"projection_index": False, "durable_index_roundtrip": False}},
                                       ["projection_index", "durable_index_roundtrip"], [])
        self.assertEqual(selected, {"lib": ["projection_index"], "bin": ["durable_index_roundtrip", "projection_index"]})

    def test_every_selector_and_required_semantic_subset_must_exist(self):
        names = {"unit": {"subscribe_delivery_a": False, "cap_filtered_b": False, "other": False}}
        self.assertEqual(len(runner.select_cases(names, [], ["subscribe_delivery", "cap_filtered"])["unit"]), 3)
        for filters, required in ((["missing"], []), ([], ["missing"]), (["subscribe_delivery"], ["cap_filtered"])):
            with self.assertRaises(runner.RunnerError):
                runner.select_cases(names, filters, required)

    def test_exact_outcomes_and_actual_exit_are_both_required(self):
        runner.validate_outcomes(outcomes("a", "b"), ["a", "b"], 0)
        for text, expected, status in ((outcomes("a"), ["a", "b"], 0),
                                      (outcomes("a", "b"), ["a"], 0),
                                      (outcomes("a", "a"), ["a"], 0),
                                      (outcomes("a", event="failed"), ["a"], 101),
                                      (outcomes("a", event="ignored"), ["a"], 0),
                                      (outcomes("a"), ["a"], 17)):
            with self.subTest(status=status), self.assertRaises(runner.RunnerError):
                runner.validate_outcomes(text, expected, status)

    def test_skipping_diagnostic_cannot_supply_success(self):
        text = outcomes("a").replace('"event": "ok", "name": "a"', '"event": "ok", "name": "a", "stdout": "Skipping absent WASM"')
        with self.assertRaises(runner.RunnerError):
            runner.validate_outcomes(text, ["a"], 0)

    def test_required_input_is_regular_bounded_and_stable(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "hello.wasm"
            with self.assertRaises(runner.RunnerError):
                runner.input_identity(path)
            path.write_bytes(b"fixture")
            original = runner.input_identity(path)
            self.assertEqual(original, runner.input_identity(path))
            path.write_bytes(b"mutated")
            self.assertNotEqual(original, runner.input_identity(path))
            link = path.with_suffix(".link")
            link.symlink_to(path)
            with self.assertRaises(runner.RunnerError):
                runner.input_identity(link)

    def test_owned_command_records_actual_failure_and_reap(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            record = runner.run_owned([sys.executable, "-c", "print('actual output'); raise SystemExit(37)"], root, "probe")
            self.assertEqual(record["exit_code"], 37)
            self.assertTrue(record["reaped"])
            self.assertGreater(record["pid"], 0)
            self.assertEqual((root / "probe.stdout").read_text().strip(), "actual output")

    def test_outcomes_reject_reordered_or_unknown_protocol_events(self):
        actual = [json.loads(row) for row in outcomes("a").splitlines()]
        for altered in ([actual[0], actual[2], actual[1], actual[3]],
                        [actual[0], actual[3], actual[1], actual[2]],
                        [actual[0], {"type": "test", "event": "mystery", "name": "a"}, *actual[1:]]):
            with self.assertRaises(runner.RunnerError):
                runner.validate_outcomes(rows(*altered), ["a"], 0)

    def test_actual_cli_builds_once_runs_each_selected_binary_once_and_docs_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "src"
            source.mkdir()
            (source / "lib.rs").write_text("fixture")
            (source / "main.rs").write_text("fixture")
            commands = root / "commands.jsonl"
            programs = []
            for kind in ("lib", "bin"):
                executable = root / (kind + ".py")
                cases = ["projection_index", "durable_index_roundtrip", "projection"]
                executable.write_text("#!" + sys.executable + "\n" +
                    "import json,sys,pathlib\n" +
                    "cases=" + repr(cases) + "\n" +
                    "with open(" + repr(str(commands)) + ", 'a') as log: log.write(json.dumps([" + repr(kind) + ",sys.argv[1:]])+'\\n')\n" +
                    "if '--list' in sys.argv:\n" +
                    " print(json.dumps({'type':'suite','event':'discovery'}))\n" +
                    " for name in cases: print(json.dumps({'type':'test','event':'discovered','name':name,'ignore':False}))\n" +
                    " print(json.dumps({'type':'suite','event':'completed','tests':3,'benchmarks':0,'total':3,'ignored':0}))\n" +
                    "else:\n" +
                    " chosen=[name for name in cases if name in sys.argv]\n" +
                    " skipped=set(cases)-set(chosen)\n" +
                    " print(json.dumps({'type':'suite','event':'started','test_count':len(chosen)}))\n" +
                    " for name in chosen:\n" +
                    "  print(json.dumps({'type':'test','event':'started','name':name}))\n" +
                    "  print(json.dumps({'type':'test','event':'ok','name':name}))\n" +
                    " print(json.dumps({'type':'suite','event':'ok','passed':len(chosen),'failed':0,'ignored':0,'measured':0,'filtered_out':len(skipped)}))\n")
                executable.chmod(0o700)
                programs.append({"reason": "compiler-artifact", "package_id": "fixture#store_service@1",
                                 "target": {"name": "store_service", "kind": [kind], "src_path": str(source / ("lib.rs" if kind == "lib" else "main.rs")), "test": True, "doctest": kind == "lib"},
                                 "profile": {"test": True}, "features": [], "executable": str(executable)})
            cargo = root / "cargo"
            cargo.write_text("#!" + sys.executable + "\nimport json,sys\n" +
                "with open(" + repr(str(commands)) + ", 'a') as log: log.write(json.dumps(['cargo',sys.argv[1:]])+'\\n')\n" +
                "if 'metadata' in sys.argv: print(" + repr(json.dumps({"packages": [{"name": "store_service", "id": "fixture#store_service@1", "manifest_path": str(root / "Cargo.toml"), "targets": [row['target'] for row in programs]}]})) + ")\n" +
                "elif '--no-run' in sys.argv:\n" + " for row in " + repr(programs) + ": print(json.dumps(row))\n" +
                " print(json.dumps({'reason':'build-finished','success':True}))\n")
            cargo.chmod(0o700)
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'])
            command = [sys.executable, str(Path(runner.__file__)), '--package', 'store_service', '--filter', 'projection_index',
                       '--filter', 'durable_index_roundtrip', '--include-doc-tests', '--output', str(root / 'evidence')]
            result = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            actual = [json.loads(row) for row in commands.read_text().splitlines()]
            self.assertEqual(sum('--no-run' in argv for name, argv in actual if name == 'cargo'), 1)
            self.assertEqual(sum('--doc' in argv for name, argv in actual if name == 'cargo'), 1)
            for kind in ('lib', 'bin'):
                self.assertEqual(sum('--list' not in argv for name, argv in actual if name == kind), 1)
                executed = next(argv for name, argv in actual if name == kind and '--list' not in argv)
                self.assertIn('--exact', executed)
                self.assertNotIn('--skip', executed)
            feature_command = [*command, '--features', 'test_protocols']
            feature_command[feature_command.index('--output') + 1] = str(root / 'feature-evidence')
            feature_result = subprocess.run(feature_command, cwd=root, env=env,
                                            capture_output=True, text=True, timeout=15)
            self.assertEqual(feature_result.returncode, 0, feature_result.stdout + feature_result.stderr)
            feature_summary = json.loads((root / 'feature-evidence' / 'summary.json').read_text())
            self.assertEqual(sum('--doc' in row['argv'] for row in feature_summary['commands']), 1)
            for observed in feature_summary['commands']:
                argv = observed['argv']
                if argv[0] == 'cargo' and argv[1] == 'test':
                    self.assertEqual(argv[argv.index('--features') + 1], 'test_protocols')
            original_cargo = cargo.read_text()
            metadata_text = json.dumps({'packages':[{'name':'store_service', 'id':'fixture#store_service@1', 'manifest_path':str(root/'Cargo.toml'), 'targets':[row['target'] for row in programs]}]})
            bin_only_metadata = json.dumps({'packages':[{'name':'store_service', 'id':'fixture#store_service@1', 'manifest_path':str(root/'Cargo.toml'), 'targets':[programs[1]['target']]}]})
            bin_only_artifacts = [programs[1]]
            cargo.write_text(original_cargo.replace(repr(metadata_text), repr(bin_only_metadata)).replace(repr(programs), repr(bin_only_artifacts)))
            command[-1] = str(root / 'no-lib-evidence')
            no_library = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True, timeout=15)
            self.assertEqual(no_library.returncode, 0, no_library.stdout + no_library.stderr)
            no_library_summary = json.loads((root/'no-lib-evidence'/'summary.json').read_text())
            self.assertFalse(no_library_summary['doctests']['run'])
            actual = [json.loads(row) for row in commands.read_text().splitlines()]
            self.assertEqual(sum('--doc' in argv for name, argv in actual if name == 'cargo'), 2)
            cargo.write_text(original_cargo)
            programs[0]['executable'] = str(root / 'lib.py')
            with open(root / 'lib.py', 'a') as executable: executable.write("\nif '--list' not in sys.argv: raise SystemExit(37)\n")
            command[-1] = str(root / 'failure-evidence')
            self.assertNotEqual(subprocess.run(command, cwd=root, env=env, capture_output=True, timeout=15).returncode, 0)

    def test_sigterm_reaps_actual_owned_child_before_return(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            pid_file = root / 'child.pid'
            child_program = "import os,pathlib,time; pathlib.Path(" + repr(str(pid_file)) + ").write_text(str(os.getpid())); time.sleep(30)"
            program = "import selected_test_runner as r; from pathlib import Path; r.run_owned(" + repr([sys.executable, '-c', child_program]) + ",Path(" + repr(str(root)) + "),'interrupted')"
            env = dict(os.environ, PYTHONPATH=str(Path(runner.__file__).parent))
            owned = subprocess.Popen([sys.executable, '-c', program], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            deadline = time.monotonic() + 3
            while not pid_file.exists():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.01)
            child = int(pid_file.read_text())
            owned.send_signal(signal.SIGTERM)
            owned.communicate(timeout=12)
            self.assertNotEqual(owned.returncode, 0)
            with self.assertRaises(ProcessLookupError): os.kill(child, 0)

    def test_terminal_parent_cannot_leave_descendant_after_owned_cleanup(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            child_pid = root / 'descendant.pid'
            sleeping = "import os,pathlib,time; pathlib.Path(" + repr(str(child_pid)) + ").write_text(str(os.getpid())); time.sleep(30)"
            parent = "import subprocess,sys,time,pathlib; p=subprocess.Popen(" + repr([sys.executable, '-c', sleeping]) + "); deadline=time.monotonic()+3; path=pathlib.Path(" + repr(str(child_pid)) + ");\nwhile not path.exists():\n assert time.monotonic()<deadline\n time.sleep(.01)\n"
            record = runner.run_owned([sys.executable, '-c', parent], root, 'terminal-parent')
            self.assertEqual(record['exit_code'], 0)
            self.assertTrue(record['reaped'])
            self.assertTrue(record['group_stop_before_reap'])
            descendant = int(child_pid.read_text())
            deadline = time.monotonic() + 3
            while True:
                try: os.kill(descendant, 0)
                except ProcessLookupError: break
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.01)

    def test_empty_group_eperm_requires_complete_kernel_group_observation(self):
        process = mock.Mock(pid=4242)
        process.wait.return_value = 0
        with mock.patch.object(runner, 'terminal_observation', return_value=mock.Mock(si_pid=4242)), mock.patch.object(runner.os, 'killpg', side_effect=PermissionError(1, 'EPERM')), mock.patch.object(runner, 'group_snapshot', return_value=[{'pgid':100,'pid':1,'state':'?Es'}, {'pgid':4242,'pid':4242,'state':'Z'}]):
            info = {}
            self.assertEqual(runner.reap(process, info), 0)
            self.assertTrue(info['group_absent_before_reap'])
            self.assertEqual(info['group_observation']['method'], '/bin/ps -axo pgid=,pid=,stat=')
        for observed in ([{'pgid':4242,'pid':4243,'state':'S'}], [{'pgid':4242,'pid':4243,'state':'Z'}], [{'pgid':4242,'pid':4242,'state':'S'}], [{'pgid':4242,'pid':4242,'state':'?Es'}], [{'pgid':4242,'pid':4243,'state':'?Es'}], runner.RunnerError('ps failed')):
            with mock.patch.object(runner, 'terminal_observation', return_value=mock.Mock(si_pid=4242)), mock.patch.object(runner.os, 'killpg', side_effect=PermissionError(1, 'EPERM')), mock.patch.object(runner, 'group_snapshot', side_effect=observed if isinstance(observed, Exception) else None, return_value=observed):
                with self.assertRaises(runner.RunnerError): runner.reap(process, {})

    def test_group_observation_rejects_malformed_truncated_and_nonzero_status(self):
        self.assertEqual(runner.parse_groups(b' 100 200 S\n200 300 Z\n', 0), [{'pgid':100,'pid':200,'state':'S'}, {'pgid':200,'pid':300,'state':'Z'}])
        for data, status in ((b'bad\n', 0), (b'100 200 S\n', 37), (b'100 200 S\n' * 20000, 0), (b'100 200 Sssssssssssssssssssssssssss\n', 0)):
            with self.assertRaises(runner.RunnerError): runner.parse_groups(data, status)

    def test_actual_mixed_library_and_metadata_bound_example_harnesses(self):
        library = artifact()
        library['target']['kind'] = ['cdylib', 'rlib']
        compiled = artifact(name='compiled_example', kind='example', executable='/target/compiled')
        compiled['target']['test'] = False
        tested = artifact(name='tested_example', kind='example', executable='/target/tested')
        roster = [row['target'] for row in [library, compiled, tested]]
        text = rows(library, compiled, tested, {'reason':'build-finished','success':True})
        actual = runner.parse_artifacts(text, 'fixture#store_service@1', Path('/fixture'), metadata_targets=roster)
        self.assertEqual({row['kind'] for row in actual}, {'lib','example'})
        self.assertEqual({row['executable'] for row in actual}, {'/target/unit','/target/tested'})
        inconsistent = [dict(row) for row in roster]
        inconsistent[-1]['test'] = False
        with self.assertRaises(runner.RunnerError):
            runner.parse_artifacts(text, 'fixture#store_service@1', Path('/fixture'), metadata_targets=inconsistent)
        with self.assertRaises(runner.RunnerError):
            runner.parse_artifacts(text, 'fixture#store_service@1', Path('/fixture'), metadata_targets=[])

    def test_explicit_integration_target_preserves_only_its_actual_harness(self):
        unit = artifact()
        example = artifact(name='integration_test', kind='example', executable='/target/example')
        integration = artifact(name='integration_test', kind='test', executable='/target/integration')
        # Explicit --test may override the target's default test=false choice.
        integration['target']['test'] = False
        text = rows(unit, example, integration, {'reason':'build-finished','success':True})
        actual = runner.parse_artifacts(text, 'fixture#store_service@1', Path('/fixture'), 'integration_test',
                                        [row['target'] for row in [unit, example, integration]])
        self.assertEqual([row['executable'] for row in actual], ['/target/integration'])

    def test_command_work_budget_rejects_late_success_reaps_and_allows_next_command(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(runner.RunnerError):
                runner.run_owned([sys.executable, '-c', "import time; print('okay result', flush=True); time.sleep(30)"], root, 'timeout', work_seconds=.05)
            result = json.loads((root / 'timeout.command.json').read_text())
            self.assertTrue(result['reaped'])
            self.assertTrue(result['group_stop_before_reap'])
            self.assertEqual(result['work_seconds'], .05)
            self.assertIn('deadline', result['error'])
            next_result = runner.run_owned([sys.executable, '-c', 'pass'], root, 'next')
            self.assertEqual(next_result['exit_code'], 0)
            self.assertTrue(next_result['reaped'])
            with self.assertRaises(runner.RunnerError):
                runner.run_owned([sys.executable, '-c', 'pass'], root, 'expanded', work_seconds=601)

    def test_late_terminal_observation_cannot_waive_command_budget(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            observed = runner.terminal_observation
            def late_terminal(process):
                # Establish an actual held terminal child before delaying its
                # observation. Interpreter startup need not finish in 60 ms.
                # This fixture bound never extends run_owned's 50 ms budget.
                end = time.monotonic() + 2
                terminal = observed(process)
                while terminal is None and time.monotonic() < end:
                    time.sleep(.005)
                    terminal = observed(process)
                self.assertIsNotNone(terminal, 'fixture child did not exit')
                time.sleep(.06)
                return terminal
            with mock.patch.object(runner, 'terminal_observation', late_terminal):
                with self.assertRaises(runner.RunnerError):
                    runner.run_owned([sys.executable, '-c', 'pass'], root, 'late', work_seconds=.05)
            result = json.loads((root / 'late.command.json').read_text())
            self.assertTrue(result['reaped'])
            self.assertEqual(result['exit_code'], 0)
            self.assertIn('deadline', result['error'])
            self.assertTrue(result['group_stop_before_reap'])
            next_result = runner.run_owned([sys.executable, '-c', 'pass'], root, 'after-late')
            self.assertEqual(next_result['exit_code'], 0)
            self.assertTrue(next_result['reaped'])

    def test_doctest_eligibility_preserves_default_cargo_library_scope(self):
        binary = {'name':'main', 'kind':['bin'], 'doctest':False}
        no_library = runner.doctest_plan([binary])
        self.assertFalse(no_library['run'])
        self.assertEqual(no_library['eligible_libraries'], [])
        disabled = {'name':'lib', 'kind':['lib'], 'doctest':False, 'test':True, 'doc':True}
        self.assertFalse(runner.doctest_plan([binary, disabled])['run'])
        eligible = {'name':'semantic_state', 'kind':['cdylib','rlib'], 'doctest':True, 'test':False, 'doc':False}
        plan = runner.doctest_plan([binary, disabled, eligible])
        self.assertTrue(plan['run'])
        self.assertEqual(plan['eligible_libraries'], [{'name':'semantic_state', 'kinds':['cdylib','rlib']}])
        with self.assertRaises(runner.RunnerError):
            runner.doctest_plan([{'name':'unknown', 'kind':['lib']}])


if __name__ == "__main__":
    unittest.main()
