#!/usr/bin/env python3
"""Five-case default-off NativePreview Read1 Foundry gate.

Consumes independently frozen source pins and the reviewed bounded profile.
Reuse only the reviewed Store runner's finite held-child/file primitives. This
runner records actual test outcomes, not a universal wire/pixel/lease transcript.
It signals only its still-held children and makes no descendant containment claim.
"""
import hashlib
import os
from pathlib import Path
import re
import secrets
import stat
import sys
import tomllib
import types

REPO = Path(__file__).absolute().parents[2]
GATE = 'foundry-editor-native-preview-read-ui1-1c'
GATE_SOURCE = 'tools/foundry/desktop_editor_native_preview_read_gate.py'
REGISTRY = 'tools/foundry/editor_native_preview_read_sources_v0.json'
REGISTRY_SHA = '0814e72acf4470e62ac742fe4017a58900a1e201ab7daed351cf63a98731b7b7'
CONTRACT = 'docs/contracts/editor-native-preview-read-v0.json'
CONTRACT_SHA = '2267e4fe80ac02a8676cfccaa194a8395f70a2926526d829fdcac8b37b6b6252'
OBSERVED_CONTRACT_SHA = '2f312a9a2f38504c576606a186d53b108f104c7fdd76519d092d2547d0d29c1c'
CODEC_CONTRACT = 'docs/contracts/editor-native-preview-codec-v0.json'
CODEC_CONTRACT_SHA = '92f31fe946c549f9ec76ce4cd7fc0fedfb515a46a7127e22de46a56325a32249'
PRIMITIVES = 'tools/ci/editor_store_runner.py'
PRIMITIVES_SHA = '532e02ce89574aea07f865320a3678b3ecc0e6a49218771bd03ed77f55cb05ff'
BASE_REGISTRY = 'tools/foundry/editor_native_read_sources_v0.json'
BASE_REGISTRY_SHA = '67888adceacf5e08d84e6fd0efec4d24832c546bc35dfc49ba422b13e727c24e'
ASSERTION_PINS = {
    'services/store_service/tests/editor_native_preview_read.rs': 'bd011ebebe64ce3ec61d90f7e2d1a747246a8cbaf6cc8d37b25e666247498923',
    'services/store_service/tests/editor_native_preview_read_support/mod.rs': 'cff606f92e6706f629945aa05644a90ffdba4d8920235dcb93e331a224417e62',
}  # Independently accepted assertion bytes, never test-reported pins.
IMPLEMENTATION_PATHS = ['services/desktop/src/editor_dev/host.rs', 'services/desktop/src/editor_dev/mod.rs', 'services/desktop/src/editor_dev/native_authority.rs', 'services/desktop/src/editor_dev/native_preview.rs', 'services/store_service/src/editor_store/mod.rs', 'services/store_service/src/editor_store/native_preview.rs']
CACHE_HELPER = 'tools/ci/build_cache.py'
CACHE_HELPER_SHA = '7910c405847716c411c4fb070225b7cf48cce1aebca50267a241b2d881a17743'  # Root freezes reviewed helper and source registry before opt-in use.

INVENTORY_HELPER = 'tools/ci/editor_source_inventory.py'
INVENTORY_HELPER_SHA = 'b13e7bda229678467e31ecea01577d341b30a8e1c19aa276e43937bdf9974639'

FEATURE = 'editor_native_preview_v0_dev'
BASE_FEATURE = 'editor_native_read_v0_dev'
TARGET = 'editor_native_preview_read'
CASES = (
    'native_preview_current_selection',
    'native_preview_fresh_approval_activation',
    'native_preview_pin_cancel_expiry_race',
    'native_preview_init_exposure_failure',
    'native_preview_caps_versions_actual_joins',
)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def pin(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value) is not None,
            'unresolved or malformed independently selected SHA256')
    return value


def load_primitives(repo_fd):
    """Hash admitted held bytes before executing the fixed reviewed helper.

    No ignored out dependency, artifact-selected path, or pre-pin path import.
    This function is proposed runtime code and is not executed during drafting.
    """
    parent = os.dup(repo_fd)
    try:
        for part in ('tools', 'ci'):
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=parent)
            os.close(parent)
            parent = child
        fd = os.open('editor_store_runner.py', os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK,
                     dir_fd=parent)
        try:
            before = os.fstat(fd)
            identity = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns, m.st_mode)
            require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 65536,
                    'reviewed primitive regular byte bound')
            raw = bytearray()
            while len(raw) < before.st_size:
                block = os.read(fd, min(65536, before.st_size - len(raw)))
                require(bool(block) and len(raw) + len(block) <= 65536, 'primitive pre-growth/truncation')
                raw.extend(block)
            require(not os.read(fd, 1) and identity(os.fstat(fd)) == identity(before),
                    'primitive changed while held')
        finally:
            os.close(fd)
    finally:
        os.close(parent)
    require(hashlib.sha256(raw).hexdigest() == pin(PRIMITIVES_SHA), 'reviewed primitive source pin')
    module = types.ModuleType('native_preview_gate_primitives')
    module.__file__ = str(REPO / PRIMITIVES)
    exec(compile(bytes(raw), module.__file__, 'exec'), module.__dict__)
    return module


def registry_paths(h, raw):
    value = h.strict_json(raw)
    h.require(type(value) is dict and set(value) == {'schema_version', 'paths'}
              and type(value['schema_version']) is int and value['schema_version'] == 1,
              'source registry closed shape')
    paths = value['paths']
    h.require(type(paths) is list and 1 <= len(paths) <= 1024
              and all(type(name) is str for name in paths)
              and paths == sorted(set(paths)), 'exact sorted source closure')
    for name in paths:
        h.relative_parts(name)
    return paths


def load_source_inventory(h, repo_fd):
    raw, _, sha = h.source_read(repo_fd, INVENTORY_HELPER, 65536, True)
    h.require(sha == INVENTORY_HELPER_SHA, 'reviewed source inventory adapter pin')
    import types
    module = types.ModuleType('native_gate_source_inventory')
    module.__file__ = INVENTORY_HELPER
    exec(compile(raw, INVENTORY_HELPER, 'exec'), module.__dict__)
    return module


def freeze_sources(h, repo_fd, run):
    raw, _, sha = h.source_read(repo_fd, REGISTRY, 65536, True)
    h.require(sha == pin(REGISTRY_SHA), 'independently frozen NPR source registry')
    paths = registry_paths(h, raw)
    baseline_raw, _, baseline_sha = h.source_read(repo_fd, BASE_REGISTRY, 65536, True)
    h.require(baseline_sha == BASE_REGISTRY_SHA, 'existing NativeRead closure pin')
    baseline = registry_paths(h, baseline_raw)
    required = {GATE_SOURCE, REGISTRY, BASE_REGISTRY, PRIMITIVES, CONTRACT, CODEC_CONTRACT,
                'services/store_service/Cargo.toml', 'services/desktop/Cargo.toml',
                'services/desktop/tests/fixtures/ascii8x16_v0.bin',
                'artifact_store_schema/src/editor_preview.rs', 'rust-toolchain.toml',
                'Cargo.toml', 'Cargo.lock', 'justfile', 'tools/ci/foundry_ci_extended.sh'}
    required.update(ASSERTION_PINS)
    h.require(type(IMPLEMENTATION_PATHS) is list and 1 <= len(IMPLEMENTATION_PATHS) <= 8
              and all(type(p) is str for p in IMPLEMENTATION_PATHS)
              and IMPLEMENTATION_PATHS == sorted(set(IMPLEMENTATION_PATHS)),
              'actual implementation source placement unresolved')
    for name in IMPLEMENTATION_PATHS:
        h.relative_parts(name)
    required.update(IMPLEMENTATION_PATHS)
    h.require((set(baseline) | required) <= set(paths), 'closure omitted baseline/new affected source')
    inventory = load_source_inventory(h, repo_fd)
    closure = inventory.validate(h, repo_fd, paths)
    rows = []
    for name in paths:
        _, size, sha = h.source_read(repo_fd, name)
        if name == INVENTORY_HELPER:
            h.require(sha == INVENTORY_HELPER_SHA, 'captured source admission adapter pin')
        if name == inventory.COLLECTOR:
            h.require(sha == inventory.COLLECTOR_SHA, 'captured transitive collector pin')
        if name in ASSERTION_PINS:
            h.require(sha == pin(ASSERTION_PINS[name]), 'final accepted assertion source pin')
        row = {'relative_path': name, 'byte_len': size, 'sha256': sha}
        h.require(len(h.compact(row)) <= 512 and len(rows) < 1024, 'source rows before growth')
        rows.append(row)
    h.require(inventory.validate(h, repo_fd, paths) == closure,
              'transitive compilation input set changed while captured')
    encoded = h.compact(rows)
    h.require(len(encoded) <= 524288, 'actual source manifest byte bound')
    path = run / 'source-manifest.json'
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    try:
        h.write_all(fd, encoded)
        os.fsync(fd)
    finally:
        os.close(fd)
    manifest_sha = h.digest(encoded)
    return h.read_manifest(path, manifest_sha), manifest_sha


def feature_exclusion(h, commands, cargo, target, package, namespace, present, absent):
    """Build with OLD NativeRead enabled; only the new NPR API must be absent."""
    built = commands.run(cargo + ['build', '--locked', '--no-default-features', '-p', package,
                                 '--features', BASE_FEATURE, '--lib', '--message-format=json'],
                         package + '-old-read-build.log')
    rlib, info = h.artifact(h.successful(built), package, 'lib', False, target, '.rlib')
    dependency_dir = rlib.parent if rlib.parent.name == 'deps' else rlib.parent / 'deps'
    h.require(dependency_dir.is_dir() and not dependency_dir.is_symlink(), 'actual old feature dependencies')
    probe_dir = target / ('npr-exclusion-' + secrets.token_hex(16))
    h.private_directory(probe_dir, create=True)
    source = probe_dir / 'probe.rs'
    raw = (f'use {package}::{namespace}::{{{present}, {absent}}};\n'
           f'fn main() {{ let _: Option<{present}> = None; }}\n').encode('ascii')
    fd = os.open(source, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    try:
        h.write_all(fd, raw)
        os.fsync(fd)
    finally:
        os.close(fd)
    try:
        h.unchanged_binary(rlib, info)
        result = commands.run(['rustc', cargo[1], '--edition=2021', '--error-format=json', str(source),
                               '--extern', package + '=' + str(rlib), '-L', 'dependency=' + str(dependency_dir),
                               '-o', str(probe_dir / 'probe')], package + '-npr-exclusion.log')
        h.require(result['status'] != 0, 'new NPR API available with old feature alone')
        h.require(len(result['stdout']) == 0, 'unexpected exclusion stdout')
        diagnostics = []
        for line in result['stderr'].splitlines():
            h.require(0 < len(line) <= 262144 and len(diagnostics) < 64, 'bounded rustc diagnostic rows')
            value = h.strict_json(line)
            h.require(type(value) is dict, 'rustc JSON diagnostic')
            diagnostics.append(value)
        errors = [d for d in diagnostics if d.get('level') == 'error' and d.get('code') is not None]
        h.require(len(errors) == 1 and type(errors[0].get('code')) is dict
                  and errors[0]['code'].get('code') == 'E0432'
                  and absent in errors[0].get('message', '')
                  and namespace in errors[0]['message'],
                  'exact new-symbol unresolved import required; old-symbol/backend failure is not exclusion')
        h.require(all(d.get('level') != 'error' or d in errors
                      or d.get('message') == 'aborting due to 1 previous error' for d in diagnostics),
                  'unrelated exclusion compiler failure')
        h.unchanged_binary(rlib, info)
        h.require(not (probe_dir / 'probe').exists(), 'unexpected exclusion executable')
        return {'package': package, 'enabled_feature': BASE_FEATURE,
                'present_symbol': present, 'absent_symbol': absent,
                'actual_rlib': info, 'actual_compiler_process': result['process'],
                'actual_log': result['log'], 'error_code': 'E0432'}
    finally:
        os.unlink(source)
        os.rmdir(probe_dir)  # Only this exact fresh owned probe; unexpected output fails closed.


def sanitized_environment(target, h):
    env = dict(os.environ)
    exact = set(h.EXPORTS) | {'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                             'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOC', 'RUSTDOCFLAGS',
                             'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_TARGET_DIR'}
    prefixes = ('RAMEN_DESKTOP_', 'CARGO_BUILD_', 'CARGO_PROFILE_', 'CARGO_TARGET_')
    for key in tuple(env):
        if key in exact or key.startswith(prefixes):
            env.pop(key)
    env['CARGO_TARGET_DIR'] = str(target)
    return env


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
    require(len(sys.argv) == 1, 'no artifact-selected gate arguments or unittest filters')
    # Resolve all placeholders BEFORE output creation, imports or any children.
    pin(REGISTRY_SHA)
    pin(CONTRACT_SHA)
    for value in ASSERTION_PINS.values():
        pin(value)
    require(type(IMPLEMENTATION_PATHS) is list and bool(IMPLEMENTATION_PATHS),
            'implementation paths not frozen')
    compiler_cache = None
    repo_fd = os.open(REPO, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        h = load_primitives(repo_fd)
        parent = REPO / 'out/desktop'
        parent.mkdir(parents=True, exist_ok=True)
        run = parent / ('native-preview-read-' + secrets.token_hex(16))
        h.private_directory(run, create=True)
        evidence = run / 'evidence'
        target = run / 'target'
        h.private_directory(evidence, create=True)
        h.private_directory(target, create=True)
        budget = h.Budget(evidence)
        rows, manifest_sha = freeze_sources(h, repo_fd, run)
        h.check_sources(repo_fd, rows)
        _, _, sha = h.source_read(repo_fd, CONTRACT)
        h.require(sha == CONTRACT_SHA, 'actual frozen NPR contract')
        _, _, sha = h.source_read(repo_fd, CODEC_CONTRACT)
        h.require(sha == CODEC_CONTRACT_SHA, 'actual frozen pure data codec contract')
        own, _, _ = h.source_read(repo_fd, GATE_SOURCE, keep=True)
        _, _, running_sha, _ = h.regular(Path(__file__), h.SOURCE_CAP)
        h.require(h.digest(own) == running_sha, 'must execute the exact maintained accepted runner')
        raw, _, _ = h.source_read(repo_fd, 'rust-toolchain.toml', 8192, True)
        channel = tomllib.loads(raw.decode('utf8', 'strict'))['toolchain']['channel']
        h.require(type(channel) is str and re.fullmatch(r'nightly-\d{4}-\d{2}-\d{2}', channel), 'pinned toolchain')
        commands = h.OwnedCommand(budget, str(REPO), sanitized_environment(target, h))
        rustc_profile = h.successful(commands.run(['rustc', '+' + channel, '-vV'], 'rustc.log')).decode('utf8', 'strict')
        cargo_profile = h.successful(commands.run(['cargo', '+' + channel, '--version'], 'cargo-version.log')).decode('utf8', 'strict')
        if cache_switch(h):
            cache_helper = load_build_cache(h, repo_fd, rows)
            compiler_cache = cache_helper.GateCache(REPO, manifest_sha, rustc_profile, cargo_profile, commands.env)
        def phase_target(phase, features):
            selected = compiler_cache.target_for(phase, features) if compiler_cache is not None else target
            commands.env['CARGO_TARGET_DIR'] = str(selected)
            return selected
        cargo = ['cargo', '+' + channel]
        exclusions = [
            feature_exclusion(h, commands, cargo, phase_target('exclusion-desktop', [BASE_FEATURE]), 'desktop_service', 'editor_dev',
                              'RegistryWitness', 'StoreSelectionEnrollment'),
            feature_exclusion(h, commands, cargo, phase_target('exclusion-store', [BASE_FEATURE]), 'store_service', 'editor_store',
                              'NativeReadFixture', 'NativePreviewActivation'),
        ]
        target = phase_target('enabled', [FEATURE])
        built = commands.run(cargo + ['test', '--locked', '--no-default-features', '-p', 'store_service',
                                      '--features', FEATURE, '--test', TARGET, '--no-run', '--message-format=json'],
                             'native-preview-build.log')
        binary, binary_info = h.artifact(h.successful(built), TARGET, 'test', True, target)
        h.successful(commands.run(cargo + ['clippy', '--locked', '--no-default-features', '-p', 'desktop_service',
                                          '--features', FEATURE, '--lib', '--', '-D', 'warnings'],
                                  'desktop-preview-clippy.log'))
        h.successful(commands.run(cargo + ['clippy', '--locked', '--no-default-features', '-p', 'store_service',
                                          '--features', FEATURE, '--test', TARGET, '--', '-D', 'warnings'],
                                  'store-preview-clippy.log'))
        h.check_sources(repo_fd, rows)
        h.unchanged_binary(binary, binary_info)
        listed_record = commands.run([str(binary), '--list'], 'native-preview-list.log', test=True)
        listed = h.successful(listed_record).decode('utf8', 'strict')
        h.require(sorted(re.findall(r'^(\w+): test$', listed, re.M)) == sorted(CASES)
                  and re.search(r'^5 tests, 0 benchmarks$', listed, re.M) is not None
                  and ': benchmark' not in listed, 'exact five actual test inventory')
        h.unchanged_binary(binary, binary_info)
        tested = commands.run([str(binary), '--test-threads=1'], 'native-preview-tests.log',
                              test=True, observe_birth=True)
        # Preserve actual held command/process/log observations even when the
        # behavioral binary fails; this record is never an acceptance result.
        budget.publish('command-observations.json', {
            'schema_version': 1, 'status': 'ACTUAL_COMMANDS_BEFORE_OUTCOME_ACCEPTANCE',
            'gate': GATE, 'source_manifest_sha256': manifest_sha,
            'test_binary': binary_info, 'commands': commands.observations,
        }, 131072)
        output = h.successful(tested).decode('utf8', 'strict')
        h.require(sorted(re.findall(r'^test (\w+) \.\.\. (\w+)$', output, re.M))
                  == sorted((c, 'ok') for c in CASES), 'five actual complete outcomes, no extra/ignored cases')
        h.require(re.search(r'^test result: ok\. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;.*$',
                            output, re.M) is not None, 'exact unfiltered five-case completion')
        h.require(type(tested['process']['start_identity']) is str
                  and tested['process']['wait_reaped'] is True
                  and tested['process']['exit_code_or_null'] == 0
                  and tested['process']['term_signal_or_null'] is None, 'actual retained test native birth/reap')
        h.unchanged_binary(binary, binary_info)
        h.check_sources(repo_fd, rows)
        # Actual process/log primitives cap eleven commands (structural max16),
        # individual normal logs1MiB/test logs8MiB, aggregate other4MiB + tests16MiB.
        # Result/source are actual admitted bytes, never test-supplied provenance.
        retained_binary = None
        if compiler_cache is not None:
            retained_binary = cache_helper.retain_binary(binary, binary_info, run / 'test-binary')
            h.unchanged_binary(binary, binary_info)
            h.check_sources(repo_fd, rows)
            compiler_cache.check_contexts()
        result = {
            'schema_version': 1, 'status': 'PASS', 'gate': GATE,
            'scope': 'native_store_preview_read_host_fixture',
            'contract_sha256': CONTRACT_SHA, 'codec_contract_sha256': CODEC_CONTRACT_SHA,
            'source_registry_sha256': REGISTRY_SHA, 'source_manifest_sha256': manifest_sha,
            'sources': rows, 'test_binary': binary_info, 'test_process': tested['process'],
            'retained_test_binary': retained_binary,
                  'compiler_cache': {'enabled': compiler_cache is not None, 'cached_acceptance': False,
                               'phases': compiler_cache.records if compiler_cache is not None else []},
            'commands': commands.observations, 'feature_exclusions': exclusions, 'cases': list(CASES),
            'limits': {'commands': 16, 'command_seconds': 600, 'normal_log_bytes': 1048576,
                       'test_log_bytes': 8388608, 'combined_test_log_bytes': 16777216,
                       'other_runner_bytes': 4194304, 'runner_files': 36,
                       'source_rows': 1024, 'source_manifest_bytes': 524288},
            'claims': {'five_assertion_log_evidence': True, 'host_fixture': True,
                       'universal_wire_transcript_consumer': False, 'exported_pixel_transcript': False,
                       'exported_lease_transcript': False, 'full_ui1_1c': False, 'save': False,
                       'mutation_permit': False, 'io2_execution': False, 'target': False,
                       'device': False, 'power_loss_durability': False, 'containment': False},
            'limitations': ['Assertion PASS and reviewed source only; no independent exported wire/pixel/lease transcript validator.',
                            'Only actual held CLI/test children are supervised; Cargo descendants are not claimed contained.',
                            'Owned service thread joins/root cleanup are assertions in the source-pinned binary, not runner-native PID evidence.',
                            'The 600s per-command deadline is finite but there is no additional whole-gate wall deadline in this inherited profile.'],
        }
        budget.publish('result.json', result, 131072)
        print('FOUNDRY_DESKTOP_EDITOR_NATIVE_PREVIEW_READ: PASS scope=host evidence=' + str(evidence))
        return 0
    finally:
        if compiler_cache is not None:
            compiler_cache.close()
        os.close(repo_fd)


if __name__ == '__main__':
    try:
        sys.exit(run_gate())
    except (OSError, ValueError, KeyError, IndexError, RecursionError):
        print('NativePreview gate: INCOMPLETE or failed validation; draft/unresolved pins are never PASS', file=sys.stderr)
        sys.exit(1)
