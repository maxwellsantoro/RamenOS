#!/usr/bin/env python3
"""Fresh, source-bound R1-R4 host regressions with exact test inventories.

Uses the existing bounded command owner to retain real exits/reaps and logs.
The regression fixtures assert service joins. Their recordings are diagnostics;
the original eighteen-case task and Reader93 remain separate required gates.
"""
import hashlib
import os
from pathlib import Path
import re
import secrets
import sys
import tempfile
import tomllib
import types

ROOT = Path(__file__).resolve().parents[2]
PRIMITIVES = 'tools/ci/editor_store_runner.py'
PRIMITIVES_SHA = '3aeba4be230cfcd972ee8939e183d6e1cb9a14c60ebeec2e96368e16e143a787'
BUILD_PROFILE = 'tools/ci/acceptance_build.py'
BUILD_PROFILE_SHA = 'e8f60acf9871604e998e7f39dcdae26b17bd2ae1ad6dbaecee8a14eaef20f6a9'
COLLECTOR = 'tools/ci/native_save_task_sources.py'
COLLECTOR_SHA = '3fa310f06ee74638944c6dfabd9d70ea2f817b825534f47081a3c45e6b4af671'
REGISTRY = 'tools/ci/editor_reliability_sources_v0.json'
REGISTRY_SHA = '08c06c5c982b91b84739ed2b3c854b69c5d0240354d5d037b1342ebc2ce0c097'
EXTRAS = (REGISTRY, 'tools/ci/editor_reliability_gate.py',
          'tools/ci/foundry_editor_reliability_ui1_1.sh', 'tools/ci/test_ci_lanes.py')
TARGETS = (
    ('artifact_editor', 'desktop_reliability', 'host_native_save_v0_dev',
     'apps/artifact_editor/tests/desktop_reliability.rs', 'test'),
    ('store_service', 'editor_reliability', 'editor_native_save_v0_dev',
     'services/store_service/tests/editor_reliability.rs', 'test'),
    ('artifact_editor', 'artifact_editor', 'host_native_save_v0_dev',
     'apps/artifact_editor/src/native_tests.rs', 'lib'),
)


def admit_registry(h, registry):
    h.require(type(registry) is dict
              and set(registry) == {'schema_version', 'paths', 'assertion_sources', 'targets'}
              and type(registry['schema_version']) is int and registry['schema_version'] == 1,
              'closed reliability registry')
    paths = registry['paths']
    h.require(type(paths) is list and 1 <= len(paths) <= 1024
              and all(type(p) is str for p in paths) and paths == sorted(set(paths)),
              'nonempty unique source inventory')
    pins = registry['assertion_sources']
    h.require(type(pins) is dict and len(TARGETS) <= len(pins) <= 32 and set(pins) <= set(paths)
              and all(type(pin) is str and re.fullmatch('[0-9a-f]{64}', pin) for pin in pins.values()),
              'nonempty bounded assertion pins')
    specs = registry['targets']
    h.require(type(specs) is list and len(specs) == len(TARGETS), 'all regression targets required')
    all_cases = set()
    for spec, (package, target, feature, path, kind) in zip(specs, TARGETS):
        h.require(type(spec) is dict and set(spec) == {'package', 'target', 'features', 'cases', 'kind'}
                  and (spec['package'], spec['target'], spec['features'], spec['kind']) == (package, target, feature, kind)
                  and path in pins, 'exact required-feature regression target and assertion pin')
        cases = spec['cases']
        h.require(type(cases) is list and 1 <= len(cases) <= 32
                  and all(type(case) is str and len(case) <= 128
                          and re.fullmatch('[a-z0-9_]+(?:::[a-z0-9_]+)*', case) for case in cases)
                  and cases == sorted(set(cases)) and not (all_cases & set(cases)),
                  'nonempty unique bounded regression cases')
        all_cases.update(cases)


def held_module(path, pin):
    # Execute only the held bytes checked against the independently reviewed pin.
    fd = os.open(ROOT / path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            raw = stream.read(65537)
        if len(raw) > 65536 or hashlib.sha256(raw).hexdigest() != pin:
            raise ValueError('reviewed helper pin: ' + path)
    finally:
        os.close(fd)
    module = types.ModuleType('reliability_' + Path(path).stem)
    module.__file__ = str(ROOT / path)
    exec(compile(raw, path, 'exec'), module.__dict__)
    return module


def snapshot(h, collector, repo_fd, registry):
    names = sorted(set(collector.source_names(repo_fd)) | set(EXTRAS))
    h.require(registry['paths'] == names, 'reviewed exact compilation/tooling closure')
    rows = []
    for name in names:
        _, size, sha = h.source_read(repo_fd, name)
        if name in registry['assertion_sources']:
            h.require(sha == registry['assertion_sources'][name], 'reviewed assertion source pin')
        rows.append(dict(relative_path=name, byte_len=size, sha256=sha))
    h.require(names == sorted(set(collector.source_names(repo_fd)) | set(EXTRAS)),
              'source closure changed while freezing')
    return rows


def exact_tests(h, stdout, cases, listing=False):
    text = stdout.decode('utf8', 'strict')
    if listing:
        h.require(sorted(re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)) == sorted(cases)
                  and re.findall(r'^(\d+) tests?, 0 benchmarks$', text, re.M) == [str(len(cases))],
                  'exact unfiltered regression test inventory')
    else:
        rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (\w+)$', text, re.M)
        h.require(sorted(rows) == sorted((case, 'ok') for case in cases),
                  'all regression outcomes passed')
        h.require(re.findall(r'^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;.*$',
                             text, re.M) == [str(len(cases))], 'complete regression summary')


def main():
    h = held_module(PRIMITIVES, PRIMITIVES_SHA)
    collector = held_module(COLLECTOR, COLLECTOR_SHA)
    profile = held_module(BUILD_PROFILE, BUILD_PROFILE_SHA)
    profile.settings(os.environ)
    parent = ROOT / 'out/foundry/editor-reliability'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run.', dir=parent))
    budget = h.Budget(run)
    commands = compilation = None
    gate_accepted = False
    repo_fd = os.open(ROOT, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        raw, _, sha = h.source_read(repo_fd, REGISTRY, 65536, True)
        h.require(sha == REGISTRY_SHA, 'independently reviewed reliability registry pin')
        registry = h.strict_json(raw)
        admit_registry(h, registry)
        rows = snapshot(h, collector, repo_fd, registry)
        budget.publish('source-manifest.json', rows, 524288)
        channel = tomllib.loads((ROOT / 'rust-toolchain.toml').read_text())['toolchain']['channel']
        target = run / 'target'
        target.mkdir(mode=0o700)
        cases_root = run / 'cases'
        cases_root.mkdir(mode=0o700)
        nonce = secrets.token_hex(16)
        env = dict(os.environ, RUSTUP_TOOLCHAIN=channel, CARGO_TARGET_DIR=str(target),
                   RAMEN_NATIVE_TASK_EVIDENCE=str(cases_root), RAMEN_NATIVE_TASK_RUN_NONCE=nonce)
        # Fresh compilation shares the host with other work. Bound preparation
        # concurrency without changing any service or assertion deadline.
        env.setdefault('CARGO_BUILD_JOBS', '2')
        commands = h.OwnedCommand(budget, ROOT, env)
        rustc = h.successful(commands.run(['rustc', '+' + channel, '-vV'], 'rustc.log')).decode('utf8', 'strict')
        cargo_version = h.successful(commands.run(['cargo', '+' + channel, '--version'], 'cargo-version.log')).decode('utf8', 'strict')
        compilation = profile.Compilation(h, repo_fd, rows, ROOT, h.digest(h.compact(rows)),
                                          commands, rustc, cargo_version, target, os.environ)
        binaries = []
        retained_binaries = []
        retained_root = run / "retained"
        retained_root.mkdir(mode=0o700)
        for spec in registry['targets']:
            role = spec['target']
            target = compilation.phase('enabled-' + spec['package'], [spec['features']])
            cargo = ['cargo', '+' + channel]
            args = ['--locked', '-p', spec['package'], '--no-default-features',
                    '--features', spec['features']]
            args += ['--test', role] if spec['kind'] == 'test' else ['--lib']
            build = commands.run(cargo + ['test', *args, '--no-run', '--message-format=json'],
                                 role + '-build.log')
            binary, info = h.artifact(h.successful(build), role, spec['kind'], True, target)
            h.successful(commands.run(cargo + ['clippy', *args, '--', '-D', 'warnings'],
                                      role + '-clippy.log'))
            h.check_sources(repo_fd, rows)
            listed = commands.run([str(binary), '--list'], role + '-list.log', test=True)
            exact_tests(h, h.successful(listed), spec['cases'], listing=True)
            tested = commands.run([str(binary), '--test-threads=1'], role + '-test.log',
                                  test=True, observe_birth=True)
            exact_tests(h, h.successful(tested), spec['cases'])
            h.require(b'INCOMPLETE' not in tested['stdout'] + tested['stderr'],
                      'incomplete service cleanup fixture')
            h.unchanged_binary(binary, info)
            if compilation.cache is not None:
                cache_helper = profile.load_cache(h, repo_fd, rows)
                retained = cache_helper.retain_binary(binary, info, retained_root / (role + '-binary'))
                retained_binaries.append(retained)
                h.unchanged_binary(binary, info)
            binaries.append(info)
        h.check_sources(repo_fd, rows)
        h.require(snapshot(h, collector, repo_fd, registry) == rows, 'final source snapshot')
        result = dict(gate='editor-reliability', status='PASS', scope='default-off-host-regressions',
                      source_manifest_sha256=h.digest(h.compact(rows)), source_registry_sha256=REGISTRY_SHA,
                      targets=registry['targets'], binaries=binaries, commands=commands.observations,
                      run_nonce=nonce, original_task_recordings='separate-required-gate',
                      compiler_cache=compilation.metadata(), retained_binaries=retained_binaries,
                      target_execution=False, device=False, process_containment=False)
        compilation.finish()
        budget.publish('result.json', result, 65536)
        gate_accepted = True
        print('EDITOR_RELIABILITY: PASS scope=host-regressions evidence=' + str(run))
        return 0
    except Exception as error:
        budget.publish('failure.json', dict(status='FAIL', reason=str(error),
                       commands=[] if commands is None else commands.observations), 65536)
        print('EDITOR_RELIABILITY: FAIL ' + str(error) + ' evidence=' + str(run), file=sys.stderr)
        return 1
    finally:
        if compilation is not None:
            compilation.close(accepted=gate_accepted)
        os.close(repo_fd)


if __name__ == '__main__':
    raise SystemExit(main())
