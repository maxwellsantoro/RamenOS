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

    def test_sanitized_compiler_environment(self):
        e = dev.sanitized_environment({'RUSTFLAGS':'bad','RUSTC_WRAPPER':'bad','CARGO_BUILD_RUSTFLAGS':'bad','RAMEN_DESKTOP_X':'bad','PATH':'ok'},self.root)
        self.assertNotIn('RUSTFLAGS',e);self.assertNotIn('RUSTC_WRAPPER',e)
        self.assertNotIn('CARGO_BUILD_RUSTFLAGS',e);self.assertNotIn('RAMEN_DESKTOP_X',e)
        self.assertEqual(e['PATH'],'ok');self.assertEqual(e['CARGO_TARGET_DIR'],str(self.root))


if __name__ == '__main__': unittest.main()
