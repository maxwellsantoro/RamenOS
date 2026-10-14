"""Gate-first developer-loop scope/process tests using finite synthetic children."""
import importlib.util
import os
from pathlib import Path
import sys
import json
import signal
import subprocess
import time
import tempfile
import unittest

HERE = Path(__file__).parent
spec = importlib.util.spec_from_file_location('dev_check', HERE / 'dev_check.py')
dev = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dev)
SCRATCH = HERE.parents[1] / 'out/ci-optimization/cache-tests'


class DevCheckTests(unittest.TestCase):
    def setUp(self):
        SCRATCH.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=SCRATCH)
        self.root = Path(self.temp.name)

    def tearDown(self): self.temp.cleanup()

    def test_valid_scope_has_actual_build_lint_tests(self):
        scope = dev.select_scope(HERE.parents[1], 'store_service', 'editor_native_read', ['editor_native_read_v0_dev'])
        cmds = dev.commands(scope, 'nightly-2026-02-08')
        self.assertEqual([c[2] for c in cmds], ['test', 'clippy', 'test'])
        self.assertIn('--no-run', cmds[0]); self.assertIn('--locked', cmds[0])
        self.assertEqual(cmds[-1][-1], '--test-threads=1')
        self.assertIn('--no-default-features', cmds[0])
        self.assertEqual(scope['label'], 'development_not_acceptance')

    def test_modes_select_only_requested_cargo_work(self):
        scope = dev.select_scope(HERE.parents[1], 'store_service', 'editor_native_read', ['editor_native_read_v0_dev'])
        for mode, verbs in [('all', ['test','clippy','test']), ('check',['check']), ('test',['test']), ('lint',['clippy'])]:
            with self.subTest(mode=mode):
                commands = dev.commands(scope, 'nightly-2026-02-08', mode=mode, jobs=3)
                self.assertEqual([c[2] for c in commands], verbs)
                for command in commands:
                    self.assertIn('--locked', command)
                    self.assertIn('--no-default-features', command)
                    self.assertEqual(command[command.index('--test')+1], 'editor_native_read')
                    self.assertEqual(command[command.index('--jobs')+1], '3')
                if mode == 'test':
                    self.assertNotIn('--no-run', commands[0])
                    self.assertEqual(commands[0][-1], '--test-threads=1')
        for mode in ('build', '', 'test;echo'):
            with self.assertRaises(dev.DevelopmentDenied):
                dev.commands(scope, 'nightly-2026-02-08', mode=mode)

    def test_jobs_are_bounded_and_preserve_approved_override(self):
        self.assertEqual(dev.resolve_jobs(None, {}), 2)
        self.assertEqual(dev.resolve_jobs(None, {'CARGO_BUILD_JOBS':'7'}), 7)
        self.assertEqual(dev.resolve_jobs('3', {'CARGO_BUILD_JOBS':'7'}), 3)
        self.assertEqual(dev.resolve_jobs('4', {'CARGO_BUILD_JOBS':'invalid'}), 4)
        for value in ('0','33','-1','auto','002','2.0','2;echo',' 2','2 ','', True):
            with self.subTest(jobs=value):
                with self.assertRaises(dev.DevelopmentDenied): dev.resolve_jobs(value, {})
                with self.assertRaises(dev.DevelopmentDenied): dev.resolve_jobs(None, {'CARGO_BUILD_JOBS':value})
        scope = dev.select_scope(HERE.parents[1], 'kernel_api', 'lib', [])
        for jobs in (0,33,-1,True,'3'):
            with self.assertRaises(dev.DevelopmentDenied):
                dev.commands(scope, 'nightly-2026-02-08', jobs=jobs)
        actual = dev.sanitized_environment({'CARGO_BUILD_JOBS':'7','RUSTC_WRAPPER':'hook'}, self.root)
        self.assertEqual(actual['CARGO_BUILD_JOBS'], '7')
        self.assertNotIn('RUSTC_WRAPPER', actual)
        override = dev.sanitized_environment({'CARGO_BUILD_JOBS':'invalid'}, self.root, jobs=3)
        self.assertEqual(override['CARGO_BUILD_JOBS'], '3')

    def test_compilation_identity_excludes_only_target_and_mode(self):
        first = dev.select_scope(HERE.parents[1], 'store_service', 'editor_native_read', ['editor_native_read_v0_dev'])
        second = dev.select_scope(HERE.parents[1], 'store_service', 'lib', ['editor_native_read_v0_dev'])
        self.assertEqual(dev.compilation_identity(first), dev.compilation_identity(second))
        features = dev.select_scope(HERE.parents[1], 'store_service', 'lib', ['editor_native_save_v0_dev'])
        package = dev.select_scope(HERE.parents[1], 'kernel_api', 'lib', [])
        self.assertNotEqual(dev.compilation_identity(first), dev.compilation_identity(features))
        self.assertNotEqual(dev.compilation_identity(first), dev.compilation_identity(package))
        self.assertNotIn('test', dev.compilation_identity(first))
        self.assertNotIn('mode', dev.compilation_identity(first))

    def test_compilation_cache_tracks_actual_compiler_config_and_lock(self):
        (self.root/'Cargo.lock').write_text('initial lock')
        env = {'CARGO_HOME':str(self.root/'cargo-home'), 'PATH':'pinned-path'}
        identity = {'package':'fixture','features':[], 'default_features':False}
        digest = dev.hashlib.sha256(dev.cache.compact(identity)).hexdigest()
        def key(**overrides):
            return dev.cache.key_for(dev.cache.key_inputs(self.root, digest, 'development-fixture', overrides.get('features', []), overrides.get('rustc','compiler one'), 'cargo one', overrides.get('env',env), scope='development'))
        original = key()
        self.assertNotEqual(original, key(rustc='compiler two'))
        self.assertNotEqual(original, key(features=['std']))
        self.assertNotEqual(original, key(env=dict(env, RUSTFLAGS='--cfg changed')))
        config = self.root/'.cargo';config.mkdir()
        (config/'config.toml').write_text('[build]\nincremental=false\n')
        changed = key()
        self.assertNotEqual(original, changed)
        (self.root/'Cargo.lock').write_text('successor lock')
        self.assertNotEqual(changed, key())

    def test_unknown_injected_and_missing_required_scopes_rejected(self):
        for p, t, f in [('unknown', 'test', []), ('store_service;echo', 'editor_native_read', []),
                        ('store_service', '../editor_native_read', []),
                        ('store_service', 'editor_native_read', ['bad']),
                        ('store_service', 'editor_native_read', []),
                        ('store_service', 'editor_native_read', ['editor_native_read_v0_dev;echo'])]:
            with self.subTest(package=p,test=t,features=f):
                with self.assertRaises(dev.DevelopmentDenied): dev.select_scope(HERE.parents[1],p,t,f)

    def test_feature_order_canonical_and_duplicate_denied(self):
        a = dev.select_scope(HERE.parents[1], 'artifact_store_schema', 'editor_preview_codec', ['std'])
        self.assertEqual(a['features'], ['std'])
        with self.assertRaises(dev.DevelopmentDenied):
            dev.select_scope(HERE.parents[1], 'artifact_store_schema', 'editor_preview_codec', ['std','std'])

    def test_workspace_lib_scope_and_target_rejections(self):
        scope = dev.select_scope(HERE.parents[1], 'kernel_api', 'lib', [])
        cmds = dev.commands(scope, 'nightly-2026-02-08')
        self.assertIn('--lib', cmds[0]); self.assertNotIn('--test', cmds[0])
        for package in ('kernel_uefi', 'kernel_aarch64'):
            with self.assertRaisesRegex(dev.DevelopmentDenied, 'target'):
                dev.select_scope(HERE.parents[1], package, 'lib', [])

    def test_actual_success_and_exit_propagation(self):
        ok = dev.run_command([sys.executable, '-c', 'print("actual")'], self.root, dict(os.environ), timeout=2)
        self.assertEqual(ok['exit_code'],0); self.assertTrue(ok['reaped']); self.assertIn('actual',ok['stdout'])
        bad = dev.run_command([sys.executable, '-c', 'raise SystemExit(7)'], self.root, dict(os.environ), timeout=2)
        self.assertEqual(bad['exit_code'],7); self.assertTrue(bad['reaped'])
        result = dev.run_sequence([[sys.executable,'-c','raise SystemExit(9)'],[sys.executable,'-c','print("not run")']],self.root,dict(os.environ),timeout=2)
        self.assertEqual(result['exit_code'],9);self.assertEqual(len(result['commands']),1)
        self.assertEqual(result['label'],'development_not_acceptance')

    def test_timeout_and_output_limit_stop_owned_child(self):
        with self.assertRaises(dev.DevelopmentDenied):
            dev.run_command([sys.executable,'-c','import time; time.sleep(2)'],self.root,dict(os.environ),timeout=0.2)
        with self.assertRaises(dev.DevelopmentDenied):
            dev.run_command([sys.executable,'-c','print("x"*4096)'],self.root,dict(os.environ),timeout=2,output_cap=512)

    def child_tree_script(self):
        script = self.root / 'tree.py'
        script.write_text("import subprocess,sys,signal,pathlib\n"
                          "p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'])\n"
                          "pathlib.Path(sys.argv[1]).write_text(str(p.pid))\n"
                          "def stop(s,f):\n p.wait(timeout=2);raise SystemExit(128+s)\n"
                          "signal.signal(signal.SIGTERM,stop)\n"
                          "print('ready',flush=True)\n"
                          "p.wait()\n")
        return [sys.executable, str(script), str(self.root / 'grandchild.pid')]

    def assert_grandchild_gone(self):
        pid = int((self.root / 'grandchild.pid').read_text())
        with self.assertRaises(ProcessLookupError): os.kill(pid, 0)

    def test_timeout_cleans_own_child_and_grandchild(self):
        with self.assertRaises(dev.DevelopmentDenied) as caught:
            dev.run_command(self.child_tree_script(), self.root, dict(os.environ), timeout=0.8)
        self.assertTrue(caught.exception.observation['reaped'])
        self.assert_grandchild_gone()

    def test_parent_term_and_int_clean_group_and_abandon_target(self):
        for sig in (signal.SIGTERM, signal.SIGINT):
            with self.subTest(signal=sig):
                pidfile = self.root / 'grandchild.pid'
                if pidfile.exists(): pidfile.unlink()
                driver = self.root / ('outer-' + str(sig) + '.py')
                driver.write_text("import sys,json,pathlib\nsys.path.insert(0," + repr(str(HERE)) + ")\n"
                                  "import dev_check as d\n"
                                  "with d.interruption_handlers():\n"
                                  " root=pathlib.Path(" + repr(str(self.root)) + ")\n"
                                  " (root/'Cargo.lock').write_bytes(b'lock')\n"
                                  " inputs=d.cache.key_inputs(root,'a'*64,'signal-" + str(sig) + "',[],'rustc','cargo',{'CARGO_HOME':str(root/'home')},scope='development')\n"
                                  " try:\n"
                                  "  with d.cache.CacheLease(root/'signal-cache',inputs) as lease:\n"
                                  "   r=d.run_sequence(" + repr([self.child_tree_script()]) + ",root,dict(d.os.environ),timeout=5)\n"
                                  "   r['cache_key']=lease.key\n"
                                  "   if r['exit_code']: raise d.DevelopmentDenied('failed command')\n"
                                  " except d.DevelopmentDenied: pass\n"
                                  " (root/'outer-result.json').write_text(json.dumps(r))\n"
                                  " raise SystemExit(r['exit_code'])\n")
                p = subprocess.Popen([sys.executable,str(driver)],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
                try:
                    deadline = time.monotonic() + 2
                    while not pidfile.exists() and time.monotonic() < deadline: time.sleep(0.01)
                    self.assertTrue(pidfile.exists()); p.send_signal(sig)
                    self.assertEqual(p.wait(timeout=3),128+sig)
                    r=json.loads((self.root/'outer-result.json').read_text())
                    self.assertTrue(r['commands'][0]['reaped']); self.assertEqual(r['exit_code'],128+sig)
                    self.assert_grandchild_gone()
                    self.assertTrue((self.root / 'signal-cache' / r['cache_key'] / '.inflight').is_file())
                    inputs=dev.cache.key_inputs(self.root,'a'*64,'signal-'+str(sig),[],'rustc','cargo',{'CARGO_HOME':str(self.root/'home')},scope='development')
                    with self.assertRaises(dev.cache.CacheDenied):
                        with dev.cache.CacheLease(self.root/'signal-cache',inputs): pass
                finally:
                    if p.poll() is None: p.kill(); p.wait(timeout=2)
                    p.stderr.close()

    def test_cli_reaped_failure_then_success_reuses_warm_target(self):
        repo=self.root/'workspace';repo.mkdir()
        (repo/'Cargo.toml').write_text('[workspace]\nmembers=["fixture"]\n')
        (repo/'Cargo.lock').write_text('# synthetic fixture lock\n')
        (repo/'rust-toolchain.toml').write_text('[toolchain]\nchannel="nightly-2026-02-08"\n')
        pkg=repo/'fixture';(pkg/'src').mkdir(parents=True)
        (pkg/'Cargo.toml').write_text('[package]\nname="fixture_pkg"\nversion="0.0.0"\n')
        (pkg/'src/lib.rs').write_text('// synthetic; never compiled\n')
        outcome=repo/'outcome';outcome.write_text('7')
        bins=self.root/'bin';bins.mkdir()
        for name in ('cargo','rustc'):
            script=bins/name
            script.write_text('#!'+sys.executable+'\nimport sys,pathlib\n'
                              'args=sys.argv[1:]\n'
                              'if "--version" in args or "-vV" in args: print("synthetic tool profile");raise SystemExit(0)\n'
                              'if "test" in args and "--no-run" not in args: raise SystemExit(int(pathlib.Path('+repr(str(outcome))+').read_text()))\n'
                              'print("synthetic command ran")\n')
            script.chmod(0o700)
        driver=self.root/'mock-cli.py'
        driver.write_text('import sys,pathlib\nsys.path.insert(0,'+repr(str(HERE))+')\n'
                          'import dev_check as d\nd.REPO=pathlib.Path('+repr(str(repo))+')\n'
                          'sys.argv=["dev-check","--package","fixture_pkg","--test","lib"]\n'
                          'with d.interruption_handlers(): raise SystemExit(d.main())\n')
        env=dict(os.environ,PATH=str(bins)+os.pathsep+os.environ['PATH'],CARGO_HOME=str(self.root/'cargo-home'))
        first=subprocess.run([sys.executable,str(driver)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=5)
        self.assertEqual(first.returncode,7)
        outcome.write_text('0')
        second=subprocess.run([sys.executable,str(driver)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=5)
        self.assertEqual(second.returncode,0,second.stderr)
        results=[json.loads(p.read_text()) for p in (repo/'out/ci-optimization').glob('dev-check-*/result.json')]
        self.assertEqual(sorted(r['exit_code'] for r in results),[0,7])
        self.assertEqual(len({r['cache']['key_sha256'] for r in results}),1)
        self.assertEqual(sorted(r['cache']['hit'] for r in results),[False,True])
        self.assertTrue(all(r['acceptance'] is False and len(r['commands'])==3 for r in results))

    def test_cli_modes_targets_and_changed_source_share_warm_compilation(self):
        repo=self.root/'workspace';repo.mkdir()
        (repo/'Cargo.toml').write_text('[workspace]\nmembers=["fixture"]\n')
        (repo/'Cargo.lock').write_text('# synthetic fixture lock\n')
        (repo/'rust-toolchain.toml').write_text('[toolchain]\nchannel="nightly-2026-02-08"\n')
        pkg=repo/'fixture';(pkg/'src').mkdir(parents=True);(pkg/'tests').mkdir()
        (pkg/'Cargo.toml').write_text('[package]\nname="fixture_pkg"\nversion="0.0.0"\n')
        source=pkg/'src/lib.rs';source.write_text('// source one; synthetic only\n')
        for name in ('first','second'): (pkg/'tests'/(name+'.rs')).write_text('// synthetic test\n')
        outcome=repo/'outcome';outcome.write_text('7')
        calls=repo/'calls.jsonl'
        tool_calls=repo/'all-tools.jsonl'
        bins=self.root/'bin';bins.mkdir()
        for name in ('cargo','rustc'):
            script=bins/name
            script.write_text('#!'+sys.executable+'\nimport sys,pathlib,os,json\n'
                              'args=sys.argv[1:]\n'
                              'with pathlib.Path('+repr(str(tool_calls))+').open("a") as f: f.write(json.dumps({"tool":pathlib.Path(sys.argv[0]).name,"argv":args})+"\\n")\n'
                              'if "--version" in args or "-vV" in args: print("synthetic tool profile");raise SystemExit(0)\n'
                              'with pathlib.Path('+repr(str(calls))+').open("a") as f: f.write(json.dumps({"argv":args,"target":os.environ["CARGO_TARGET_DIR"],"jobs":os.environ["CARGO_BUILD_JOBS"],"source":pathlib.Path('+repr(str(source))+').read_text()})+"\\n")\n'
                              'raise SystemExit(int(pathlib.Path('+repr(str(outcome))+').read_text()))\n')
            script.chmod(0o700)
        driver=self.root/'mock-cli.py'
        driver.write_text('import sys,pathlib\nsys.path.insert(0,'+repr(str(HERE))+')\n'
                          'import dev_check as d\nd.REPO=pathlib.Path('+repr(str(repo))+')\n'
                          'with d.interruption_handlers(): raise SystemExit(d.main())\n')
        env=dict(os.environ,PATH=str(bins)+os.pathsep+os.environ['PATH'],CARGO_HOME=str(self.root/'cargo-home'),CARGO_BUILD_JOBS='3')
        runs=[('test','first',7),('check','lib',0),('lint','second',0),('test','second',0)]
        for index,(mode,target,status) in enumerate(runs):
            outcome.write_text(str(status))
            if index == 1: source.write_text('// source two; changed inputs require a fresh Cargo invocation\n')
            result=subprocess.run([sys.executable,str(driver),'--package','fixture_pkg','--test',target,'--mode',mode],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=5)
            self.assertEqual(result.returncode,status,result.stderr)
        records=[json.loads(p.read_text()) for p in (repo/'out/ci-optimization').glob('dev-check-*/result.json')]
        self.assertEqual(len(records),4)
        self.assertEqual(len({r['cache']['key_sha256'] for r in records}),1)
        self.assertEqual(sorted(r['cache']['hit'] for r in records),[False,True,True,True])
        self.assertEqual({(r['mode'],r['scope']['test']) for r in records},{(mode,target) for mode,target,_ in runs})
        for record in records:
            self.assertFalse(record['acceptance'])
            self.assertEqual(record['jobs'],3)
            self.assertEqual(len(record['commands']),1)
            actual=record['commands'][0]
            self.assertTrue(actual['reaped']);self.assertIsNone(actual['fault'])
            self.assertEqual(actual['mode'],record['mode'])
            self.assertEqual(actual['selected_target'],record['scope']['test'])
        actual_calls=[json.loads(row) for row in calls.read_text().splitlines()]
        self.assertEqual(len(actual_calls),4)
        self.assertEqual(len({r['target'] for r in actual_calls}),1)
        self.assertEqual({r['jobs'] for r in actual_calls},{'3'})
        self.assertIn('source one',actual_calls[0]['source'])
        self.assertTrue(all('source two' in r['source'] for r in actual_calls[1:]))
        for call, (mode,target,_) in zip(actual_calls, runs):
            self.assertEqual(call['argv'][1], {'test':'test','check':'check','lint':'clippy'}[mode])
            self.assertEqual(call['argv'][call['argv'].index('--jobs')+1], '3')
            if target == 'lib': self.assertIn('--lib',call['argv'])
            else: self.assertEqual(call['argv'][call['argv'].index('--test')+1],target)
        all_tools_before = tool_calls.read_text()
        denied=subprocess.run([sys.executable,str(driver),'--package','fixture_pkg','--mode','test','--jobs','0'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=5)
        self.assertNotEqual(denied.returncode,0)
        self.assertEqual(len(calls.read_text().splitlines()),4, 'invalid jobs must not start a Cargo child')
        self.assertEqual(tool_calls.read_text(),all_tools_before, 'invalid jobs must not start profile children either')

    def test_sanitized_compiler_environment(self):
        e = dev.sanitized_environment({'RUSTFLAGS':'bad','RUSTC_WRAPPER':'bad','CARGO_BUILD_RUSTFLAGS':'bad','RAMEN_DESKTOP_X':'bad','PATH':'ok'},self.root)
        self.assertNotIn('RUSTFLAGS',e);self.assertNotIn('RUSTC_WRAPPER',e)
        self.assertNotIn('CARGO_BUILD_RUSTFLAGS',e);self.assertNotIn('RAMEN_DESKTOP_X',e)
        self.assertEqual(e['PATH'],'ok');self.assertEqual(e['CARGO_TARGET_DIR'],str(self.root))


if __name__ == '__main__': unittest.main()
