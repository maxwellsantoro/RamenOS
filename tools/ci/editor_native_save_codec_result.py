#!/usr/bin/env python3
"""Eight-case pure Save data gate: finite actual compiler and native test evidence."""
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
GATE = 'foundry-editor-native-save-codec-ui1-1c'
GATE_SOURCE = 'tools/ci/editor_native_save_codec_result.py'
SHELL = 'tools/ci/foundry_editor_native_save_codec_ui1_1c.sh'
REGISTRY = 'tools/foundry/editor_native_save_codec_sources_v0.json'
REGISTRY_SHA = '75034cfceb7207b394cedf4992d33fd2ec12709a0eb0e1ff6e264bcb7e3bd96a'
CONTRACT = 'docs/contracts/editor-native-save-codec-v0.json'
CONTRACT_SHA = 'd646b73b52772589e0aa3ac02ecea214f9b85e76d2a320f6d11f86f11f222969'
ASSERTIONS = 'artifact_store_schema/tests/editor_native_save_codec.rs'
ASSERTIONS_SHA = '2a98d569813172129b8ced6548571be6c95cd11f1383820c3bdced1eb263cd59'
PRIMITIVES = 'tools/ci/editor_store_runner.py'
PRIMITIVES_SHA = '532e02ce89574aea07f865320a3678b3ecc0e6a49218771bd03ed77f55cb05ff'
CASES = (
    'native_save_grants_literal_le_464',
    'native_save_grants_phase_versions_and_rights',
    'native_save_grants_handles_resources_epochs_expiry',
    'native_save_chrome_literal_le_248',
    'native_save_chrome_idle_and_unknown_matrix',
    'native_save_chrome_receipt_and_original_binding',
    'native_save_chrome_successor_and_epoch_limits',
    'native_save_read_codecs_remain_distinct',
)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def pin(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value), 'unresolved source pin')
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


def check_inventory(text, listing):
    if listing:
        entries = re.findall(r'^([^\n]+): (test|benchmark)$', text, re.MULTILINE)
        require(sorted(entries) == sorted((case, 'test') for case in CASES), 'exact eight-case inventory')
        require(re.findall(r'^(\d+) tests, (\d+) benchmarks$', text, re.MULTILINE) == [('8', '0')], 'exact list summary')
    else:
        entries = re.findall(r'^test ([^\n]+) \.\.\. (\S+)$', text, re.MULTILINE)
        require(sorted(entries) == sorted((case, 'ok') for case in CASES), 'eight unskipped outcomes')
        require(re.findall(r'^running (\d+) tests$', text, re.MULTILINE) == ['8'], 'exact execution count')
        summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s$', text, re.MULTILINE)
        require(summaries == [('8', '0', '0', '0', '0')], 'exact unfiltered completion')


def self_test():
    listing = ''.join(c + ': test\n' for c in CASES) + '\n8 tests, 0 benchmarks\n'
    tested = 'running 8 tests\n' + ''.join('test ' + c + ' ... ok\n' for c in CASES) + 'test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'
    check_inventory(listing, True)
    check_inventory(tested, False)
    negatives = [(listing.replace(CASES[0], CASES[1]), True),
                 (listing + 'extra: test\n', True),
                 (listing.replace(': test', ': benchmark', 1), True),
                 (listing.replace('8 tests', '7 tests'), True),
                 (tested.replace('... ok', '... ignored', 1), False),
                 (tested.replace('0 filtered out', '1 filtered out'), False),
                 (tested.replace('test result: ok.', 'test result: FAILED.'), False),
                 (tested + 'test extra ... ok\n', False),
                 (tested.replace('running 8', 'running 7'), False)]
    for text, is_list in negatives:
        try:
            check_inventory(text, is_list)
        except ValueError:
            continue
        raise ValueError('negative inventory admitted')
    print('SAVE_CODEC_CONSUMER_SELF_TEST: PASS negative_cases=9')


def freeze_sources(h, repo_fd, budget):
    raw, _, sha = h.source_read(repo_fd, REGISTRY, 65536, True)
    require(sha == pin(REGISTRY_SHA), 'frozen source registry')
    value = h.strict_json(raw)
    require(type(value) is dict and set(value) == {'schema_version', 'paths'} and type(value['schema_version']) is int and value['schema_version'] == 1, 'registry shape')
    paths = value['paths']
    require(type(paths) is list and 1 <= len(paths) <= 128 and all(type(p) is str for p in paths) and paths == sorted(set(paths)), 'source closure inventory')
    expected = {GATE_SOURCE, SHELL, REGISTRY, CONTRACT, ASSERTIONS, PRIMITIVES,
                'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'artifact_store_schema/Cargo.toml',
                'idl/portals/desktop_editor_session_v1.toml', 'idl/portals/desktop_artifact_v1.toml',
                'justfile', 'tools/ci/ci_lanes.py'}
    expected.update(str(p.relative_to(REPO)) for p in (REPO / 'artifact_store_schema/src').rglob('*.rs'))
    require(set(paths) == expected, 'all compiled schema source inputs required')
    rows = []
    fixed = {CONTRACT: CONTRACT_SHA, ASSERTIONS: ASSERTIONS_SHA, PRIMITIVES: PRIMITIVES_SHA}
    for name in paths:
        _, size, sha = h.source_read(repo_fd, name)
        if name in fixed:
            require(sha == pin(fixed[name]), 'reviewed frozen bytes: ' + name)
        rows.append({'relative_path': name, 'byte_len': size, 'sha256': sha})
    require(len(h.compact(rows)) <= 524288, 'manifest bound')
    budget.publish('source-manifest.json', rows, 524288)
    return rows, h.digest(h.compact(rows))


def environment(target, h):
    env = dict(os.environ)
    exact = set(h.EXPORTS) | {'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                             'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOC', 'RUSTDOCFLAGS',
                             'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_TARGET_DIR'}
    for key in tuple(env):
        if key in exact or key.startswith(('RAMEN_DESKTOP_', 'CARGO_BUILD_', 'CARGO_PROFILE_', 'CARGO_TARGET_')):
            env.pop(key)
    env['CARGO_TARGET_DIR'] = str(target)
    env['RUST_TEST_THREADS'] = '1'
    return env


def run_gate():
    require(len(sys.argv) == 1, 'no filters or evidence-selected gate arguments')
    for value in (REGISTRY_SHA, CONTRACT_SHA, ASSERTIONS_SHA, PRIMITIVES_SHA):
        pin(value)
    repo_fd = os.open(REPO, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        repo_identity = (os.fstat(repo_fd).st_dev, os.fstat(repo_fd).st_ino)
        h = load_primitives(repo_fd)
        parent = REPO / 'out/desktop'
        parent.mkdir(parents=True, exist_ok=True)
        run = parent / ('native-save-codec-' + secrets.token_hex(16))
        h.private_directory(run, create=True)
        evidence, target = run / 'evidence', run / 'target'
        h.private_directory(evidence, create=True)
        h.private_directory(target, create=True)
        budget = h.Budget(evidence)
        rows, manifest_sha = freeze_sources(h, repo_fd, budget)
        h.check_sources(repo_fd, rows)
        _, _, own_sha = h.source_read(repo_fd, GATE_SOURCE)
        _, _, executing_sha, _ = h.regular(Path(__file__), h.SOURCE_CAP, False)
        require(own_sha == executing_sha, 'executing maintained runner bytes')
        raw, _, _ = h.source_read(repo_fd, 'rust-toolchain.toml', 8192, True)
        channel = tomllib.loads(raw.decode('utf8'))['toolchain']['channel']
        require(type(channel) is str and re.fullmatch(r'nightly-\d{4}-\d{2}-\d{2}', channel), 'dated pinned compiler')
        commands = h.OwnedCommand(budget, str(REPO), environment(target, h))
        rustc = commands.run(['rustc', '+' + channel, '-vV'], 'rustc.log')
        cargo_version = commands.run(['cargo', '+' + channel, '--version'], 'cargo-version.log')
        h.successful(rustc)
        h.successful(cargo_version)
        own_test = commands.run([sys.executable, str(REPO / GATE_SOURCE), '--self-test'], 'consumer-self-test.log')
        require(h.successful(own_test) == b'SAVE_CODEC_CONSUMER_SELF_TEST: PASS negative_cases=9\n', 'consumer self-test')
        cargo = ['cargo', '+' + channel]
        base = ['--locked', '-p', 'artifact_store_schema', '--no-default-features']
        h.successful(commands.run(cargo + ['check'] + base, 'no-std.log'))
        h.successful(commands.run(cargo + ['clippy'] + base + ['--features', 'std', '--test', 'editor_native_save_codec', '--', '-D', 'warnings'], 'std-clippy.log'))
        built = commands.run(cargo + ['test'] + base + ['--features', 'std', '--test', 'editor_native_save_codec', '--no-run', '--message-format=json'], 'test-build.log')
        binary, binary_info = h.artifact(h.successful(built), 'editor_native_save_codec', 'test', True, target)
        h.check_sources(repo_fd, rows)
        listing = commands.run([str(binary), '--list'], 'test-list.log', test=True)
        check_inventory(h.successful(listing).decode('utf8'), True)
        h.unchanged_binary(binary, binary_info)
        tested = commands.run([str(binary), '--test-threads=1'], 'test.log', test=True)
        check_inventory(h.successful(tested).decode('utf8'), False)
        h.unchanged_binary(binary, binary_info)
        h.check_sources(repo_fd, rows)
        live = os.stat(REPO, follow_symlinks=False)
        require((live.st_dev, live.st_ino) == repo_identity, 'repository identity changed')
        result = dict(schema_version=1, gate=GATE, status='PASS', scope='pure-native-save-data-codec',
                      behavior_cases=list(CASES), behavior_case_count=8, source_count=len(rows),
                      source_manifest_sha256=manifest_sha, assertion_sha256=ASSERTIONS_SHA,
                      contract_sha256=CONTRACT_SHA, actual_test_process=tested['process'],
                      test_binary=binary_info, commands=commands.observations,
                      compiler_stdout_sha256=h.digest(rustc['stdout']), cargo_stdout_sha256=h.digest(cargo_version['stdout']),
                      claims=dict(pure_codec=True, live_save=False, service_authority=False, store_io=False,
                                  current_draft_saved=False, durability=False, target=False, device=False, containment=False))
        budget.publish('result.json', result, 65536)
        print('FOUNDRY_EDITOR_NATIVE_SAVE_CODEC_UI1_1C: PASS scope=pure-native-save-data-codec evidence=' + str(evidence))
        return 0
    finally:
        os.close(repo_fd)


if __name__ == '__main__':
    try:
        if sys.argv[1:] == ['--self-test']:
            self_test()
        else:
            sys.exit(run_gate())
    except (OSError, ValueError, KeyError, IndexError, RecursionError) as error:
        print('Save codec gate: INCOMPLETE or failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
