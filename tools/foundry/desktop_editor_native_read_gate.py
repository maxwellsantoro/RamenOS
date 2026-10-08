#!/usr/bin/env python3
"""Nine-case default-off host NativeRead gate; no device or full UI1.1c claim.

Reuse the reviewed bounded file/process primitives of the existing Store runner.
Source selection comes from the root-owned registry, never test observations.
Only held child processes may be signalled. This is host evidence, not containment.
"""
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import sys
import tomllib

REPO = Path(__file__).absolute().parents[2]
helper_path = REPO / 'tools/ci/editor_store_runner.py'
spec = importlib.util.spec_from_file_location('native_read_gate_primitives', helper_path)
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)
REGISTRY = 'tools/foundry/editor_native_read_sources_v0.json'
REGISTRY_SHA = '67888adceacf5e08d84e6fd0efec4d24832c546bc35dfc49ba422b13e727c24e'
CONTRACT_SHA = 'b28134e814f25b1b51ee1124509e971fed38b70c6682fe6d79a8baa13512f2bd'
CASES = (
    'native_read_positive', 'native_read_identity_denials',
    'native_read_control_denials', 'native_read_constructor_recheck',
    'native_read_legacy_descriptor_denials', 'native_read_once_deadline',
    'native_read_retirement', 'native_read_owned_handles',
    'native_read_counters_domains',
)
CACHE_HELPER = 'tools/ci/build_cache.py'
CACHE_HELPER_SHA = '7910c405847716c411c4fb070225b7cf48cce1aebca50267a241b2d881a17743'  # Root freezes reviewed helper and source registry before opt-in use.

INVENTORY_HELPER = 'tools/ci/editor_source_inventory.py'
INVENTORY_HELPER_SHA = 'b13e7bda229678467e31ecea01577d341b30a8e1c19aa276e43937bdf9974639'

FEATURE = 'editor_native_read_v0_dev'


def load_source_inventory(h, repo_fd):
    raw, _, sha = h.source_read(repo_fd, INVENTORY_HELPER, 65536, True)
    h.require(sha == INVENTORY_HELPER_SHA, 'reviewed source inventory adapter pin')
    import types
    module = types.ModuleType('native_gate_source_inventory')
    module.__file__ = INVENTORY_HELPER
    exec(compile(raw, INVENTORY_HELPER, 'exec'), module.__dict__)
    return module


def freeze_sources(repo_fd, run):
    raw, _, pin = h.source_read(repo_fd, REGISTRY, 65536, True)
    h.require(pin == REGISTRY_SHA, 'reviewed source registry pin')
    registry = h.strict_json(raw)
    h.require(type(registry) is dict and set(registry) == {'schema_version', 'paths'}
              and type(registry['schema_version']) is int and registry['schema_version'] == 1,
              'registry shape')
    paths = registry['paths']
    h.require(type(paths) is list and 1 <= len(paths) <= 1024
              and all(type(p) is str for p in paths) and paths == sorted(set(paths)),
              'registry exact inventory')
    inventory = load_source_inventory(h, repo_fd)
    closure = inventory.validate(h, repo_fd, paths)
    rows = []
    for name in paths:
        _, size, sha = h.source_read(repo_fd, name)
        if name == INVENTORY_HELPER:
            h.require(sha == INVENTORY_HELPER_SHA, 'captured source admission adapter pin')
        if name == inventory.COLLECTOR:
            h.require(sha == inventory.COLLECTOR_SHA, 'captured transitive collector pin')
        row = {'relative_path': name, 'byte_len': size, 'sha256': sha}
        h.require(len(h.compact(row)) <= 512, 'source record bound')
        rows.append(row)
    h.require(inventory.validate(h, repo_fd, paths) == closure,
              'transitive compilation input set changed while captured')
    raw = h.compact(rows)
    h.require(len(raw) <= 524288, 'manifest bound')
    destination = run / 'source-manifest.json'
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        h.write_all(fd, raw)
        os.fsync(fd)
    finally:
        os.close(fd)
    #The common reader also checks baseline Cargo/schema/consumer dependencies.
    return h.read_manifest(destination, h.digest(raw)), h.digest(raw)


def default_exclusion(commands, cargo, target, package, feature, namespace, symbol):
    built = commands.run(cargo + ['build', '--locked', '--no-default-features', '-p', package,
                                 '--features', feature, '--lib', '--message-format=json'],
                         package + '-baseline-build.log')
    rlib, _ = h.artifact(h.successful(built), package, 'lib', False, target, '.rlib')
    dependencies = rlib.parent if rlib.parent.name == 'deps' else rlib.parent / 'deps'
    probe_dir = target / ('native-read-exclusion-' + secrets.token_hex(16))
    os.mkdir(probe_dir, 0o700)
    probe = probe_dir / 'probe.rs'
    fd = os.open(probe, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        h.write_all(fd, f'use {package}::{namespace}::{symbol};\nfn main() {{}}\n'.encode())
    finally:
        os.close(fd)
    result = commands.run(['rustc', cargo[1], '--edition=2021', '--error-format=json', str(probe),
                           '--extern', package + '=' + str(rlib), '-L', 'dependency=' + str(dependencies),
                           '-o', str(probe_dir / 'probe')], package + '-native-exclusion.log')
    h.require(result['status'] != 0, 'native API available without native feature')
    diagnostics = [h.strict_json(line) for line in result['stderr'].splitlines()]
    errors = [d for d in diagnostics if d.get('level') == 'error' and d.get('code') is not None]
    h.require(len(errors) == 1 and errors[0]['code']['code'] == 'E0432'
              and (package + '::' + namespace) in errors[0]['message'],
              'exclusion must be exact unresolved native API')
    h.require(all(d.get('level') != 'error' or d in errors
                  or d.get('message') == 'aborting due to 1 previous error' for d in diagnostics),
              'unrelated exclusion failure')
    os.unlink(probe)
    os.rmdir(probe_dir)


def load_build_cache(h, repo_fd, rows):
    # No path import: execute only held, bounded bytes in the frozen source closure.
    expected = CACHE_HELPER_SHA
    h.require(type(expected) is str and re.fullmatch('[0-9a-f]{64}', expected),
              'compiler cache helper pin unresolved')
    h.require(any(row['relative_path'] == CACHE_HELPER and row['sha256'] == expected for row in rows),
              'compiler cache helper absent from independently selected source closure')
    raw, _, sha = h.source_read(repo_fd, CACHE_HELPER, 65536, True)
    h.require(sha == expected, 'reviewed compiler cache helper mismatch')
    import types
    module = types.ModuleType('native_gate_build_cache')
    module.__file__ = str(REPO / CACHE_HELPER)
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


def cache_switch(h):
    value = os.environ.get('RAMEN_FOUNDRY_BUILD_CACHE', '0')
    h.require(value in ('0', '1'), 'compiler cache opt-in must be 0 or 1')
    return value == '1'


def run_gate():
    h.require(len(sys.argv) == 1, 'gate takes no artifact-selected arguments')
    parent = REPO / 'out/desktop'
    parent.mkdir(parents=True, exist_ok=True)
    run = parent / ('native-read-' + secrets.token_hex(16))
    os.mkdir(run, 0o700)
    evidence = run / 'evidence'
    h.private_directory(evidence, create=True)
    #A fresh run-owned target prevents Cargo cache reuse across source snapshots.
    target = run / 'target'
    h.private_directory(target, create=True)
    repo_fd = os.open(REPO, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    budget = h.Budget(evidence)
    compiler_cache = None
    try:
        rows, pin = freeze_sources(repo_fd, run)
        h.check_sources(repo_fd, rows)
        _, _, sha = h.source_read(repo_fd, 'docs/contracts/editor-native-read-v0.json')
        h.require(sha == CONTRACT_SHA, 'frozen native Read contract')
        own_raw, _, _ = h.source_read(repo_fd, 'tools/foundry/desktop_editor_native_read_gate.py', keep=True)
        _, _, own_sha, _ = h.regular(Path(__file__), h.SOURCE_CAP)
        h.require(h.digest(own_raw) == own_sha, 'executing gate source mismatch')
        raw, _, _ = h.source_read(repo_fd, 'rust-toolchain.toml', 8192, True)
        channel = tomllib.loads(raw.decode())['toolchain']['channel']
        h.require(type(channel) is str and re.fullmatch(r'nightly-\d{4}-\d{2}-\d{2}', channel), 'pinned toolchain')
        env = dict(os.environ)
        for key in (*h.EXPORTS, 'RAMEN_DESKTOP_EDITOR_NATIVE_READ_GATE_EVIDENCE',
                    'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                    'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTC',
                    'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER',
                    'CARGO_BUILD_RUSTFLAGS', 'RUSTDOC', 'RUSTDOCFLAGS', 'CARGO_BUILD_RUSTDOC'):
            env.pop(key, None)
        env['CARGO_TARGET_DIR'] = str(target)
        commands = h.OwnedCommand(budget, str(REPO), env)
        rustc_profile = h.successful(commands.run(['rustc', '+' + channel, '-vV'], 'rustc.log')).decode('utf8', 'strict')
        cargo_profile = h.successful(commands.run(['cargo', '+' + channel, '--version'], 'cargo-version.log')).decode('utf8', 'strict')
        if cache_switch(h):
            cache_helper = load_build_cache(h, repo_fd, rows)
            compiler_cache = cache_helper.GateCache(REPO, pin, rustc_profile, cargo_profile, commands.env)
        def phase_target(phase, features):
            selected = compiler_cache.target_for(phase, features) if compiler_cache is not None else target
            commands.env['CARGO_TARGET_DIR'] = str(selected)
            return selected
        cargo = ['cargo', '+' + channel]
        default_exclusion(commands, cargo, phase_target('exclusion-desktop', ['desktop_v0_dev']), 'desktop_service', 'desktop_v0_dev', 'editor_dev', 'RegistryWitness')
        default_exclusion(commands, cargo, phase_target('exclusion-store', ['editor_store_v0_dev']), 'store_service', 'editor_store_v0_dev', 'editor_store', 'NativeReadFixture')
        target = phase_target('enabled', [FEATURE])
        built = commands.run(cargo + ['test', '--locked', '--no-default-features', '-p', 'store_service',
                                      '--features', FEATURE, '--test', 'editor_native_read', '--no-run',
                                      '--message-format=json'], 'native-build.log')
        binary, binary_info = h.artifact(h.successful(built), 'editor_native_read', 'test', True, target)
        h.successful(commands.run(cargo + ['clippy', '--locked', '--no-default-features', '-p', 'desktop_service',
                                          '--features', FEATURE, '--lib', '--', '-D', 'warnings'],
                                  'desktop-native-clippy.log'))
        h.successful(commands.run(cargo + ['clippy', '--locked', '--no-default-features', '-p', 'store_service',
                                          '--features', FEATURE, '--test', 'editor_native_read', '--', '-D', 'warnings'],
                                  'native-clippy.log'))
        h.check_sources(repo_fd, rows)
        listed = h.successful(commands.run([str(binary), '--list'], 'native-list.log', test=True)).decode()
        h.require(sorted(re.findall(r'^(\w+): test$', listed, re.M)) == sorted(CASES)
                  and '9 tests, 0 benchmarks' in listed and ': benchmark' not in listed, 'exact nine actual tests')
        h.unchanged_binary(binary, binary_info)
        tested = commands.run([str(binary), '--test-threads=1'], 'native-tests.log', test=True, observe_birth=True)
        output = h.successful(tested).decode('utf8', 'strict')
        h.require(sorted(re.findall(r'^test (\w+) \.\.\. (\w+)$', output, re.M)) == sorted((c, 'ok') for c in CASES),
                  'nine actual outcomes')
        h.require('test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' in output,
                  'nine actual completion')
        h.unchanged_binary(binary, binary_info)
        h.check_sources(repo_fd, rows)
        retained_binary = None
        if compiler_cache is not None:
            retained_binary = cache_helper.retain_binary(binary, binary_info, run / 'test-binary')
            h.unchanged_binary(binary, binary_info)
            h.check_sources(repo_fd, rows)
            compiler_cache.check_contexts()
        result = {'schema_version': 1, 'status': 'PASS', 'scope': 'native_read_host_fixture',
                  'contract_sha256': CONTRACT_SHA, 'source_manifest_sha256': pin, 'sources': rows,
                  'test_binary': binary_info, 'test_process': tested['process'], 'commands': commands.observations,
                  'retained_test_binary': retained_binary,
                  'compiler_cache': {'enabled': compiler_cache is not None, 'cached_acceptance': False,
                                     'phases': compiler_cache.records if compiler_cache is not None else []},
                  'cases': list(CASES), 'claims': {'full_ui1_1c': False, 'save': False, 'device': False,
                                                'target': False, 'containment': False},
                  'limitations': 'Nine executable assertions and reviewed implementation; no universal transcript consumer. Optional fixture exports are not collected by this gate.'}
        budget.publish('result.json', result, 131072)
        print('FOUNDRY_DESKTOP_EDITOR_NATIVE_READ: PASS scope=host evidence=' + str(evidence))
        return 0
    finally:
        if compiler_cache is not None:
            compiler_cache.close()
        os.close(repo_fd)


if __name__ == '__main__':
    try:
        sys.exit(run_gate())
    except (h.Incomplete, OSError, ValueError, KeyError, IndexError, RecursionError):
        print('NativeRead gate: INCOMPLETE or failed validation', file=sys.stderr)
        sys.exit(1)
