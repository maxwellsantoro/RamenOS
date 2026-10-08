#!/usr/bin/env python3
"""Source-bound UI1.1c native editor task gate.

Root owns eventual tools/ci/desktop_editor_task_result.py, shell, registry,
source pins and recording schema. No output artifact can select trusted code.
Case data and process evidence have separate namespaces/budgets. Build target
is separate, fresh, and is not accepted case/process evidence or a disk sandbox.
Only the pinned primitives' actually held Popen children may be signalled;
service-thread joins are verified by the future semantic reader, never PID data.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import stat
import sys
import time
import tomllib
import types

GATE = 'foundry-desktop-editor-task-ui1-1c'
GATE_SOURCE = 'tools/ci/desktop_editor_task_result.py'
SHELL = 'tools/ci/foundry_desktop_editor_task_ui1_1c.sh'
CONTRACT = 'docs/contracts/editor-native-save-task-v0.json'
CONTRACT_SHA = 'f76a0cdb81ab94f003a484989661293ea2037640d981e8be7228e4fac5d410a9'
PRIMITIVES = 'tools/ci/editor_store_runner.py'
PRIMITIVES_SHA = '532e02ce89574aea07f865320a3678b3ecc0e6a49218771bd03ed77f55cb05ff'
PACKAGE = 'artifact_editor'
FEATURE = 'host_native_save_v0_dev'
TARGET = 'native_task'
SOURCE_COLLECTOR = 'tools/ci/native_save_task_sources.py'
SOURCE_COLLECTOR_SHA = '6323de423cd8476a6c0bca92119587742a5e7a010be3e669706643c2e7434b63'
# ROOT MUST FREEZE THESE AFTER INDEPENDENT ASSERTION/INVENTORY/READER REVIEW.
REGISTRY_PATH = 'tools/ci/native_save_task_sources_v0.json'
REGISTRY_SHA = 'c2db944d5a78469e4310f2ce7e002bb51abff5d4e3d68962172535184eda438a'
ASSERTION_PINS = {'apps/artifact_editor/tests/native_task.rs': '2d6d616b416dbebe85e730ac44bcadd30142f20a359154225ce2b3ebb35fa447', 'apps/artifact_editor/tests/native_task_support/mod.rs': '0a4d62ef0a934b00e2b9b42878212ff82d2530225c91148941fd558c1d0e8228', 'apps/artifact_editor/tests/native_task_support/recording.rs': '69844fffb1b8b3bbb1d7fa9e4bbe2218c59e5e0adb99457383032bf749b848fd', 'apps/artifact_editor/tests/native_task_support/protected_goldens.rs': 'f99add25f2aa9b4bf37074fb7a2ae3fca5f78475775c7fe469f649d9dd438d6c'}
RECORDING_CONTRACT_PATH = 'docs/contracts/editor-native-save-recording-v0.json'
RECORDING_CONTRACT_SHA = 'c011d7ac19bf17ac6aa2fa941f16d6c6fbeac1ebd245eb1b7015d7f94d216bfe'
SOURCE_INVENTORY_PATH = 'docs/contracts/editor-native-save-source-inventory-v0.json'
SOURCE_INVENTORY_SHA = 'cac4f6caac41ed8ab8ae2ff378157cde2264f40d67ef4705c6d6567e4717cfef'
RECORDING_PROPOSAL_SHA = 'b994e45e0841c94508d0c15e3f2f4bb54d4b04f00094fa00af3fc6775d9c5227'
VERIFIER_SOURCE = 'artifact_store_schema/examples/native_manifest_verifier.rs'
VERIFIER_SOURCE_SHA = '8700f70d1686359b3eec77cf3ce5a47c6c744055eb8793b4e74d60a51ae32254'
VERIFIER_OWNER_SOURCE = 'tools/ci/owned_manifest_verifier.py'
VERIFIER_OWNER_SHA = '97ddf6a32ec5ee536e107721afb98195a70130de370f7623fc3881456d534581' # Source and actual component controls independently reviewed.
VERIFIER_REVIEW_SHA = '5c18bcee9d259351fe47b6d45d4ba22d20ce96880e8ff22b5b50a0384fabf036'
VERIFIER_PUBLIC_KEY = 'f80cccdce4ae1c07ae208a2adf99a310ae4207e0306fa0236110b06827bbb8d0'
# Packaging ABI approved in principle; nested recorder schema remains unresolved.
EXPORT_BINDINGS = {'RAMEN_NATIVE_TASK_EVIDENCE': 'cases', 'RAMEN_NATIVE_TASK_RUN_NONCE': 'nonce'}
SEMANTIC_READER_PATH = 'tools/ci/native_save_task_recording.py'
SEMANTIC_READER_SHA = 'f4cc584db229852168f50887feefab2b667b81d8ff00dc255cbc696ef5e994af'
SEMANTIC_SEAM_IMPLEMENTED = True
CASES = ('ui1_1_keyboard_edits_ascii_and_renders_owned_frame', 'ui1_1_ascii_bounds_preserve_prior_draft', 'ui1_1_focus_routes_reserved_keys_to_trusted_chrome', 'ui1_1_focus_epoch_reset_overflow_and_detach', 'ui1_1_surface_freeze_alias_quota_and_chrome_clip', 'ui1_1_wire_schema_descriptor_and_identity_fail_closed', 'ui1_1_editor_grants_are_exact_and_preview_single_use', 'ui1_1_volatile_save_and_reopen_are_explicitly_labeled', 'ui1_1_revoke_paused_key_frame_and_save', 'ui1_1_fault_restart_discards_unsaved_draft_and_old_grants', 'ui1_1_focus_compositor_restart_retires_old_epochs', 'ui1_1_stalled_session_leaves_other_session_usable', 'ui1_1_clock_capacity_and_counter_limits_fail_closed', 'ui1_1_store_commit_reopen_preserves_prior_blob', 'ui1_1_store_same_base_conflict_and_operation_reuse', 'ui1_1_store_lost_reply_reconciles_without_mutation_replay', 'ui1_1_store_recovery_validates_atomic_selection_and_receipt', 'ui1_1_unknown_save_survives_editor_recovery')
CASE_BYTES = 2147483648
CASE_JSON_BYTES = 536870912
CASE_FILES = 40960
CASE_ENTRIES = 41115
CASE_DEPTH = 8
CASE_PATH_BYTES = 256
PROCESS_BYTES = 1073741824
BINARY_BYTES = 536870912
LOG_BYTES = 67108864
RAW_NARROW_CAPS = {'text64': 4160, 'receipt176': 176, 'journal': 131072,
                   'app_frame': 1228800, 'composed_frame': 1454080,
                   'wire88': 88}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha_pin(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value),
            'unresolved independently frozen SHA256')
    return value


def resolved_before_any_children_or_outputs():
    sha_pin(REGISTRY_SHA)
    sha_pin(SOURCE_COLLECTOR_SHA)
    sha_pin(RECORDING_CONTRACT_SHA)
    sha_pin(SEMANTIC_READER_SHA)
    sha_pin(SOURCE_INVENTORY_SHA)
    sha_pin(RECORDING_PROPOSAL_SHA)
    sha_pin(VERIFIER_SOURCE_SHA)
    sha_pin(VERIFIER_OWNER_SHA)
    sha_pin(VERIFIER_REVIEW_SHA)
    sha_pin(VERIFIER_PUBLIC_KEY)
    require(type(REGISTRY_PATH) is str and type(RECORDING_CONTRACT_PATH) is str
            and type(SEMANTIC_READER_PATH) is str, 'unresolved trusted paths')
    require(type(ASSERTION_PINS) is dict and bool(ASSERTION_PINS), 'unfrozen assertions')
    for name, pin in ASSERTION_PINS.items():
        require(type(name) is str and name.startswith('apps/artifact_editor/tests/'), 'assertion path')
        sha_pin(pin)
    require(type(EXPORT_BINDINGS) is dict and bool(EXPORT_BINDINGS), 'unfrozen export ABI')
    # ROOT final closed ABI maps exact env names to cases/nonce only. Never an
    # artifact-selected source path, PID, command, test filter or backend.
    require(all(type(k) is str and re.fullmatch('RAMEN_[A-Z0-9_]{1,100}', k)
                and v in ('cases', 'nonce') for k, v in EXPORT_BINDINGS.items()), 'export bindings')
    require(type(SOURCE_INVENTORY_PATH) is str and type(VERIFIER_OWNER_SOURCE) is str, 'unresolved trusted sources')
    require(SEMANTIC_SEAM_IMPLEMENTED is True, 'recording semantic acceptance seam deferred')


def held_read_at(repo_fd, name, cap, expected=None):
    parts = name.split('/')
    require(1 <= len(parts) <= 32 and all(re.fullmatch('[A-Za-z0-9_.-]{1,128}', p)
            and p not in ('.', '..') for p in parts), 'trusted relative source path')
    parent = os.dup(repo_fd)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=parent)
            os.close(parent)
            parent = child
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                     dir_fd=parent)
        try:
            before = os.fstat(fd)
            identity = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns, m.st_mode, m.st_nlink)
            require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                    and 0 < before.st_size <= cap, 'held regular source admission')
            raw = bytearray()
            while len(raw) < before.st_size:
                block = os.read(fd, min(65536, before.st_size - len(raw)))
                require(bool(block) and len(raw) + len(block) <= cap, 'source pre-growth/truncation')
                raw.extend(block)
            require(not os.read(fd, 1) and identity(os.fstat(fd)) == identity(before), 'source changed')
        finally:
            os.close(fd)
    finally:
        os.close(parent)
    data = bytes(raw)
    if expected is not None:
        require(hashlib.sha256(data).hexdigest() == sha_pin(expected), 'trusted source digest')
    return data


def open_absolute_directory(path):
    require(path.is_absolute() and '..' not in path.parts, 'absolute directory no traversal')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        for part in path.parts[1:]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=fd)
            os.close(fd)
            fd = child
        return fd
    except BaseException:
        os.close(fd)
        raise


def held_absolute_read(path, cap, expected=None):
    require(path.is_absolute() and '..' not in path.parts, 'absolute no-traversal input')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        return held_read_at(fd, '/'.join(path.parts[1:]), cap, expected)
    finally:
        os.close(fd)


def load_primitives(repo_fd, repo):
    raw = held_read_at(repo_fd, PRIMITIVES, 65536, PRIMITIVES_SHA)
    module = types.ModuleType('native_task_pinned_primitives')
    module.__file__ = str(repo / PRIMITIVES)
    # Executes only independently pinned HELD bytes, never a path import.
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


def manifest_rows(h, repo_fd, manifest, manifest_sha):
    raw = held_absolute_read(manifest, 2097152, manifest_sha)
    rows = h.strict_json(raw)
    require(type(rows) is list and 1 <= len(rows) <= 4096, 'bounded compiled-source closure')
    for row in rows:
        require(type(row) is dict and set(row) == {'relative_path', 'byte_len', 'sha256'}, 'source row closed')
        h.relative_parts(row['relative_path'])
        require(type(row['byte_len']) is int and 1 <= row['byte_len'] <= h.SOURCE_CAP, 'source row bytes')
        sha_pin(row['sha256'])
    names = [r['relative_path'] for r in rows]
    require(names == sorted(set(names)) and h.compact(rows) == raw, 'canonical unique sorted rows')
    registry = h.strict_json(held_read_at(repo_fd, REGISTRY_PATH, 524288, REGISTRY_SHA))
    require(type(registry) is dict and set(registry) == {'schema_version', 'paths'}
            and type(registry['schema_version']) is int and registry['schema_version'] == 1
            and type(registry['paths']) is list and registry['paths'] == names, 'exact frozen source set')
    required = {GATE_SOURCE, SHELL, REGISTRY_PATH, SOURCE_COLLECTOR, PRIMITIVES, CONTRACT,
                RECORDING_CONTRACT_PATH, SOURCE_INVENTORY_PATH, SEMANTIC_READER_PATH,
                VERIFIER_SOURCE, VERIFIER_OWNER_SOURCE,
                'rust-toolchain.toml', 'Cargo.toml', 'Cargo.lock',
                'apps/artifact_editor/Cargo.toml', 'apps/artifact_editor/src/lib.rs',
                'services/desktop/Cargo.toml', 'services/store_service/Cargo.toml',
                'artifact_store_core/Cargo.toml', 'artifact_store_schema/Cargo.toml',
                'kernel_api/Cargo.toml', 'idl/portals/desktop_artifact_v1.toml',
                'services/desktop/tests/fixtures/ascii8x16_v0.bin',
                'justfile', 'tools/ci/foundry_ci_extended.sh'} | set(ASSERTION_PINS)
    require(required <= set(names), 'compiled/trusted source omitted')
    check_source_set(h, repo_fd, names)
    # Exact root registry review must cover all source/build scripts/generated
    # Rust and IDL/font/test-support inputs for these six compiled crates, plus
    # their transitive workspace path deps. Presence of one file/root is not proof.
    for row in rows:
        actual = held_read_at(repo_fd, row['relative_path'], h.SOURCE_CAP)
        require((len(actual), h.digest(actual)) == (row['byte_len'], row['sha256']), 'source mismatch')
        if row['relative_path'] in ASSERTION_PINS:
            require(row['sha256'] == ASSERTION_PINS[row['relative_path']], 'reviewed assertion bytes')
    held_read_at(repo_fd, CONTRACT, 262144, CONTRACT_SHA)
    held_read_at(repo_fd, RECORDING_CONTRACT_PATH, 1048576, RECORDING_CONTRACT_SHA)
    held_read_at(repo_fd, SEMANTIC_READER_PATH, h.SOURCE_CAP, SEMANTIC_READER_SHA)
    held_read_at(repo_fd, SOURCE_INVENTORY_PATH, 1048576, SOURCE_INVENTORY_SHA)
    held_read_at(repo_fd, VERIFIER_SOURCE, h.SOURCE_CAP, VERIFIER_SOURCE_SHA)
    held_read_at(repo_fd, VERIFIER_OWNER_SOURCE, h.SOURCE_CAP, VERIFIER_OWNER_SHA)
    return rows


def check_source_set(h, repo_fd, names):
    raw = held_read_at(repo_fd, SOURCE_COLLECTOR, 65536, SOURCE_COLLECTOR_SHA)
    collector = types.ModuleType('native_task_trusted_source_collector')
    exec(compile(raw, SOURCE_COLLECTOR, 'exec'), collector.__dict__)
    require(collector.source_names(repo_fd) == names, 'actual compilation source set changed')


def check_sources(h, repo_fd, rows):
    check_source_set(h, repo_fd, [row['relative_path'] for row in rows])
    for row in rows:
        raw = held_read_at(repo_fd, row['relative_path'], h.SOURCE_CAP)
        require((len(raw), h.digest(raw)) == (row['byte_len'], row['sha256']), 'post-source mismatch')


def clean_env(target):
    env = dict(os.environ)
    for key in tuple(env):
        if key.startswith(('CARGO_', 'RUST', 'RAMEN_DESKTOP_', 'RAMEN_NATIVE_TASK_', 'LD_', 'DYLD_')) or key == 'RAMEN_FOUNDRY_BUILD_CACHE':
            env.pop(key)
    # No Cargo config/env build hooks or ambient backend/export. Root must have
    # installed this pinned toolchain/dependency lock already. Offline forbids downloads.
    env['CARGO_TARGET_DIR'] = str(target)
    env['RUST_BACKTRACE'] = '1'
    return env


def absent_cargo_configs(repo, admitted_home=None):
    cargo_home = admitted_home if admitted_home is not None else Path(os.environ.get('CARGO_HOME', str(Path.home() / '.cargo'))).absolute()
    candidates = [(p / '.cargo' / name) for p in (repo, *repo.parents)
                  for name in ('config', 'config.toml')]
    candidates += [cargo_home / name for name in ('config', 'config.toml')]
    for path in candidates:
        try:
            os.lstat(path)
        except FileNotFoundError:
            continue
        raise ValueError('ambient Cargo config not admitted by draft profile')
    return cargo_home


def default_off_probe(h, commands, cargo, target):
    built = commands.run(cargo + ['build', '--locked', '--offline', '--no-default-features',
                                 '-p', PACKAGE, '--lib', '--message-format=json'], 'default-app-build.log')
    rlib, info = h.artifact(h.successful(built), PACKAGE, 'lib', False, target, '.rlib')
    probe = target / ('default-off-probe-' + secrets.token_hex(16))
    h.private_directory(probe, create=True)
    path = probe / 'probe.rs'
    raw = b'use artifact_editor::NativeArtifactEditor;\nfn main(){let _:Option<NativeArtifactEditor>=None;}\n'
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    try:
        h.write_all(fd, raw)
    finally:
        os.close(fd)
    dependency_dir = rlib.parent if rlib.parent.name == 'deps' else rlib.parent / 'deps'
    h.unchanged_binary(rlib, info)
    tested = commands.run(['rustc', cargo[1], '--edition=2021', '--error-format=json', str(path),
                          '--extern', 'artifact_editor=' + str(rlib), '-L', 'dependency=' + str(dependency_dir),
                          '-o', str(probe / 'probe')], 'default-off-probe.log')
    require(tested['status'] != 0 and not tested['stdout'], 'default-off must reject actual Save symbol')
    diagnostics = []
    for line in tested['stderr'].splitlines():
        require(0 < len(line) <= 262144 and len(diagnostics) < 64, 'bounded default-off diagnostics')
        row = h.strict_json(line)
        require(type(row) is dict, 'compiler diagnostic object')
        diagnostics.append(row)
    errors = [x for x in diagnostics if x.get('level') == 'error' and x.get('code') is not None]
    require(len(errors) == 1 and type(errors[0]['code']) is dict
            and errors[0]['code'].get('code') == 'E0432'
            and 'NativeArtifactEditor' in errors[0].get('message', ''), 'actual missing API E0432 only')
    require(all(x.get('level') != 'error' or x in errors
                or x.get('message') == 'aborting due to 1 previous error' for x in diagnostics),
            'unrelated default-off failure')
    require(not (probe / 'probe').exists(), 'unexpected default app probe executable')
    h.unchanged_binary(rlib, info)
    return {'actual_default_rlib': info, 'compiler': tested['process'], 'log': tested['log']}


def scan_process_namespace(budget, root, whole_deadline):
    """Observe only root-created ordinary files; no artifact path authority."""
    fd = open_absolute_directory(root)
    rows = []
    total = 0
    names = set()
    try:
        with os.scandir(fd) as stream:
            for entry in stream:
                require(len(names) < 36 and entry.name in budget.names, 'unexpected process artifact/count')
                names.add(entry.name)
        require(names == budget.names, 'process artifact missing')
        for name in sorted(names):
            require(time.monotonic() < whole_deadline, 'process scan original deadline')
            info = os.stat(name, dir_fd=fd, follow_symlinks=False)
            require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_uid == os.geteuid()
                    and not info.st_mode & 0o077, 'nonprivate/nonregular process artifact')
            cap = BINARY_BYTES if name in ('test-binary', 'manifest-verifier-binary') else LOG_BYTES if name.endswith('.log') else 8388608 if name == 'manifest-verifier-observations.json' else 2097152
            require(0 < info.st_size <= cap, 'process artifact byte cap')
            leaf = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=fd)
            try:
                identity = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns,
                                      m.st_mode, m.st_nlink, m.st_uid)
                require(identity(os.fstat(leaf)) == identity(info), 'process leaf changed')
                amount = 0
                sha = hashlib.sha256()
                while amount < info.st_size:
                    require(time.monotonic() < whole_deadline, 'process read original deadline')
                    block = os.read(leaf, min(65536, info.st_size - amount))
                    require(bool(block) and amount + len(block) <= cap, 'process pre-growth/truncation')
                    amount += len(block)
                    sha.update(block)
                require(not os.read(leaf, 1) and identity(os.fstat(leaf)) == identity(info),
                        'process file changed')
            finally:
                os.close(leaf)
            total += amount
            require(total <= PROCESS_BYTES, 'observed process namespace1GiB')
            rows.append({'relative_path': name, 'byte_len': amount, 'sha256': sha.hexdigest()})
    finally:
        os.close(fd)
    require(total == budget.other + budget.test, 'root charged process bytes mismatch')
    return rows


def load_trusted_module(h, repo_fd, repo, path, expected, module_name):
    raw = held_read_at(repo_fd, path, h.SOURCE_CAP, expected)
    module = types.ModuleType(module_name)
    module.__file__ = str(repo / path)
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


def retain_binary(h, budget, binary, info, name, deadline, verifier_target=None):
    # Only the root-selected Cargo example may have its normal compiler alias.
    if verifier_target is not None:
        require(name == 'manifest-verifier-binary'
                and binary == verifier_target / 'debug/examples/native_manifest_verifier',
                'root selected verifier example path only')
    h.unchanged_binary(binary, info)
    require(time.monotonic() < deadline, 'original binary retention deadline')
    parent = open_absolute_directory(binary.parent)
    source = alias = dest = None
    identity = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns,
                          m.st_mode, m.st_nlink, m.st_uid, m.st_gid)
    try:
        parent_meta = os.fstat(parent)
        require(stat.S_ISDIR(parent_meta.st_mode) and parent_meta.st_uid == os.geteuid()
                and not parent_meta.st_mode & 0o022, 'owned compiler binary parent')
        source = os.open(binary.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                         dir_fd=parent)
        meta = os.fstat(source)
        require(stat.S_ISREG(meta.st_mode) and meta.st_uid == os.geteuid()
                and not meta.st_mode & 0o022 and meta.st_mode & 0o111
                and meta.st_size == info['byte_len'] and 0 < meta.st_size <= BINARY_BYTES,
                'actual selected binary bounded owned executable leaf')
        alias_name = None
        if meta.st_nlink != 1:
            require(verifier_target is not None and meta.st_nlink == 2,
                    'single link or exact verifier compiler pair only')
            candidates = []
            count = 0
            with os.scandir(parent) as entries:
                for entry in entries:
                    require(time.monotonic() < deadline and count < 128,
                            'compiler alias inventory deadline/count')
                    count += 1
                    if re.fullmatch('native_manifest_verifier-[0-9a-f]{16}', entry.name):
                        require(not candidates, 'one compiler verifier alias only')
                        candidates.append(entry.name)
            require(len(candidates) == 1, 'missing compiler verifier alias')
            alias_name = candidates[0]
            alias = os.open(alias_name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                            dir_fd=parent)
            require(identity(os.fstat(alias)) == identity(meta), 'compiler aliases must be same actual inode')
        require(identity(os.stat(binary.name, dir_fd=parent, follow_symlinks=False)) == identity(meta),
                'selected source path identity')
        dest = budget.open(name)
        amount = 0
        sha = hashlib.sha256()
        while amount < meta.st_size:
            require(time.monotonic() < deadline, 'original binary retention deadline')
            block = os.read(source, min(65536, meta.st_size - amount))
            require(bool(block), 'selected binary truncation')
            budget.charge(len(block))
            h.write_all(dest, block)
            amount += len(block)
            sha.update(block)
        require(not os.read(source, 1) and sha.hexdigest() == info['sha256'], 'actual selected binary digest')
        require(identity(os.fstat(source)) == identity(meta)
                and identity(os.stat(binary.name, dir_fd=parent, follow_symlinks=False)) == identity(meta),
                'actual selected binary full identity changed')
        if alias is not None:
            require(identity(os.fstat(alias)) == identity(meta)
                    and identity(os.stat(alias_name, dir_fd=parent, follow_symlinks=False)) == identity(meta),
                    'actual compiler alias identity changed')
        require(identity(os.fstat(parent)) == identity(parent_meta), 'compiler parent identity changed')
        rebound = open_absolute_directory(binary.parent)
        try:
            require(identity(os.fstat(rebound)) == identity(parent_meta), 'compiler parent path changed')
        finally:
            os.close(rebound)
        output = os.fstat(dest)
        require(stat.S_ISREG(output.st_mode) and output.st_nlink == 1
                and output.st_uid == os.geteuid() and stat.S_IMODE(output.st_mode) == 0o600
                and output.st_size == amount, 'private single-link retained executable bytes')
        os.fsync(dest)
        require(time.monotonic() < deadline, 'binary retention publication deadline')
        admission = {'source_policy': 'closed_cargo_example_alias_pair' if alias is not None else 'single_link',
                     'source_link_count': meta.st_nlink, 'source_identity': list(identity(meta)),
                     'compiler_alias_path': str(binary.parent / alias_name) if alias_name is not None else None,
                     'byte_len': amount, 'sha256': sha.hexdigest(), 'retained_file': name}
    finally:
        for fd in (dest, alias, source, parent):
            if fd is not None:
                os.close(fd)
    h.unchanged_binary(binary, info)
    require(time.monotonic() < deadline, 'binary retention final hash deadline')
    return admission


def run(args):
    resolved_before_any_children_or_outputs()
    require(args.repo.is_absolute() and args.run_root.is_absolute()
            and args.source_manifest.is_absolute()
            and all('..' not in p.parts for p in (args.repo, args.run_root, args.source_manifest)),
            'trusted absolute no-traversal CLI paths')
    sha_pin(args.source_manifest_sha256)
    repo_fd = open_absolute_directory(args.repo)
    commands = budget = verifier = None
    verifier_published = False
    published_commands = False
    gate_accepted = False
    command_return_pending = False
    try:
        h = load_primitives(repo_fd, args.repo)
        require(h.platform.system() in ('Linux', 'Darwin'), 'native birth profile Linux/Mac only')
        absent_home = absent_cargo_configs(args.repo)
        # Independent source input admission before child execution or run outputs.
        rows = manifest_rows(h, repo_fd, args.source_manifest, args.source_manifest_sha256)
        self_sha = h.digest(held_absolute_read(Path(__file__).absolute(), h.SOURCE_CAP))
        require(any(x['relative_path'] == GATE_SOURCE and x['sha256'] == self_sha for x in rows),
                'execute exact maintained runner in source closure')
        require(not args.run_root.exists(), 'fresh nonexistent output only')
        h.private_directory(args.run_root, create=True)
        for name in ('process', 'cases', 'target'):
            h.private_directory(args.run_root / name, create=True)
        process_root, case_root, target = (args.run_root / p for p in ('process', 'cases', 'target'))
        class ProcessBudget(h.Budget):
            def __init__(self, root):
                super().__init__(root)
                self.names = set()
            def open(self, name):
                require(name not in self.names, 'exclusive process evidence filename')
                fd = super().open(name)
                self.names.add(name)
                return fd
            def charge(self, count, test=False):
                require(type(count) is int and count >= 0 and self.other + self.test + count <= PROCESS_BYTES,
                        'separate process namespace pre-growth1GiB')
                if test:
                    self.test += count
                else:
                    self.other += count
        budget = ProcessBudget(process_root)
        # Runtime module parameter is a stricter instance policy, not altered source
        # bytes. Normal logs still1MiB; native list/test logs at most64MiB each.
        h.LOG_CAP = LOG_BYTES
        env = clean_env(target)
        env['CARGO_HOME'] = str(absent_home)
        nonce = secrets.token_hex(16)
        for key, value in EXPORT_BINDINGS.items():
            env[key] = str(case_root) if value == 'cases' else nonce
        commands = h.OwnedCommand(budget, str(args.repo), env)
        original_run = commands.run
        whole_deadline = time.monotonic() + 3600
        def bounded_run(argv, name, **kwargs):
            nonlocal command_return_pending
            left = whole_deadline - time.monotonic()
            require(left >= 3, 'whole gate original deadline')
            absent_cargo_configs(args.repo, absent_home)
            kwargs['observe_birth'] = True
            command_return_pending = True
            result = original_run(argv, name, timeout=min(600, left), **kwargs)
            command_return_pending = False
            absent_cargo_configs(args.repo, absent_home)
            return result
        commands.run = bounded_run
        raw = held_read_at(repo_fd, 'rust-toolchain.toml', 8192)
        admitted_toolchain = next((row for row in rows if row['relative_path'] == 'rust-toolchain.toml'), None)
        require(admitted_toolchain is not None
                and (len(raw), h.digest(raw)) == (admitted_toolchain['byte_len'], admitted_toolchain['sha256']),
                'compiler toolchain bytes differ from admitted source row')
        toolchain = tomllib.loads(raw.decode('utf8', 'strict'))['toolchain']['channel']
        require(type(toolchain) is str and re.fullmatch('nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}', toolchain),
                'actual pinned toolchain not env override')
        cargo = ['cargo', '+' + toolchain]
        rustc = h.successful(commands.run(['rustc', '+' + toolchain, '-vV'], 'rustc.log'))
        cargo_version = h.successful(commands.run(cargo + ['--version'], 'cargo-version.log'))
        exclusion = default_off_probe(h, commands, cargo, target)
        build = commands.run(cargo + ['test', '--locked', '--offline', '--no-default-features', '-p', PACKAGE,
                                     '--features', FEATURE, '--test', TARGET, '--no-run', '--message-format=json'],
                             'task-build.log')
        binary, binary_info = h.artifact(h.successful(build), TARGET, 'test', True, target)
        require(binary_info['byte_len'] <= BINARY_BYTES, 'retained native binary512MiB')
        helper_build = commands.run(cargo + ['build', '--locked', '--offline', '-p', 'artifact_store_schema',
                                              '--features', 'std', '--example', 'native_manifest_verifier',
                                              '--message-format=json'], 'manifest-verifier-build.log')
        helper_binary, helper_info = h.artifact(h.successful(helper_build), 'native_manifest_verifier',
                                               'example', False, target)
        check_sources(h, repo_fd, rows)
        h.unchanged_binary(binary, binary_info)
        listed = h.successful(commands.run([str(binary), '--list'], 'task-list.log', test=True)).decode('utf8', 'strict')
        require(sorted(re.findall(r'^(\w+): test$', listed, re.M)) == sorted(CASES)
                and re.search(r'^18 tests, 0 benchmarks$', listed, re.M) and ': benchmark' not in listed,
                'actual exactly18 list withoutcase19')
        tested = commands.run([str(binary), '--test-threads=1'], 'task-tests.log', test=True, observe_birth=True)
        budget.publish('command-observations.json', {'status': 'ACTUAL_COMMANDS_BEFORE_OUTCOME_ACCEPTANCE',
                       'source_manifest_sha256': args.source_manifest_sha256, 'binary': binary_info,
                       'commands': commands.observations}, 2097152)
        published_commands = True
        output = h.successful(tested).decode('utf8', 'strict')
        require(sorted(re.findall(r'^test (\w+) \.\.\. (\w+)$', output, re.M))
                == sorted((name, 'ok') for name in CASES), 'exact18 actual outcomes')
        require(re.search(r'^test result: ok\. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;.*$',
                          output, re.M), 'complete unfiltered task summary')
        proc = tested['process']
        require(type(proc['start_identity']) is str and proc['wait_reaped'] is True
                and proc['exit_code_or_null'] == 0 and proc['term_signal_or_null'] is None, 'actual testbirth/reap')
        check_sources(h, repo_fd, rows)
        h.unchanged_binary(binary, binary_info)
        for package, flags, name in ((PACKAGE, ['--test', TARGET], 'app-clippy.log'),
                                     ('desktop_service', ['--lib'], 'desktop-clippy.log'),
                                     ('store_service', ['--lib'], 'store-clippy.log')):
            feature = FEATURE if package == PACKAGE else 'editor_native_save_v0_dev'
            h.successful(commands.run(cargo + ['clippy', '--locked', '--offline', '--no-default-features',
                                              '-p', package, '--features', feature] + flags + ['--', '-D', 'warnings'], name))
        retain_binary(h, budget, binary, binary_info, 'test-binary', whole_deadline)
        helper_retention = retain_binary(h, budget, helper_binary, helper_info, 'manifest-verifier-binary',
                                         whole_deadline, verifier_target=target)
        reader = load_trusted_module(h, repo_fd, args.repo, SEMANTIC_READER_PATH,
                                     SEMANTIC_READER_SHA, 'native_task_trusted_reader')
        owner = load_trusted_module(h, repo_fd, args.repo, VERIFIER_OWNER_SOURCE,
                                    VERIFIER_OWNER_SHA, 'native_task_trusted_verifier_owner')
        verifier = owner.OwnedManifestVerifier(h, helper_binary, helper_info, str(args.repo), env, whole_deadline)
        verifier_seam = {'callback': verifier, 'source_sha256': VERIFIER_SOURCE_SHA,
                         'binary_sha256': helper_info['sha256'], 'review_sha256': VERIFIER_REVIEW_SHA,
                         'public_key': VERIFIER_PUBLIC_KEY}
        cases_fd = open_absolute_directory(case_root)
        try:
            consistency = reader.validate_recording(cases_fd, rows, nonce,
                held_read_at(repo_fd, RECORDING_CONTRACT_PATH, 1048576, RECORDING_CONTRACT_SHA),
                RECORDING_CONTRACT_SHA,
                held_read_at(repo_fd, SOURCE_INVENTORY_PATH, 1048576, SOURCE_INVENTORY_SHA),
                SOURCE_INVENTORY_SHA, CONTRACT_SHA, RECORDING_PROPOSAL_SHA,
                held_read_at(repo_fd, 'services/desktop/tests/fixtures/ascii8x16_v0.bin', 1536),
                verifier_seam, whole_deadline)
        finally:
            os.close(cases_fd)
        require(type(consistency) is dict and consistency.get('cases') == 18
                and consistency.get('runtime_acceptance_claim') is False,
                'complete consistency report accompanies real eighteen-case execution')
        budget.publish('manifest-verifier-observations.json',
                       {'scope': 'actual_owned_pure_verifier_commands', 'binary': helper_info,
                        'source_sha256': VERIFIER_SOURCE_SHA, 'commands': verifier.observations}, 8388608)
        verifier_published = True
        check_sources(h, repo_fd, rows)
        process_rows = scan_process_namespace(budget, process_root, whole_deadline)
        provenance = {'scope': 'actual_in_process_host_keyboard_editor_store_task',
                      'nonce': nonce, 'sources': rows, 'binary': binary_info, 'process': proc,
                      'os': dict(h.platform.uname()._asdict()), 'python': sys.version,
                      'commands': commands.observations, 'process_files': process_rows, 'default_off': exclusion,
                      'rustc_bytes': rustc.decode('utf8', 'strict'), 'cargo_bytes': cargo_version.decode('utf8', 'strict'),
                      'case_data_consistency': consistency, 'manifest_verifier_binary': helper_info,
                      'manifest_verifier_retention': helper_retention}
        accepted = {'schema_version': 1, 'status': 'PASS', 'gate': GATE,
                    'scope': 'default_off_trusted_in_process_host_native_editor_store_task',
                    'actual_eighteen_unfiltered_runtime_outcomes': True,
                    'source_manifest_sha256': args.source_manifest_sha256,
                    'provenance': provenance,
                    'limitations': ['No separate editor process, target runtime, device IO, physical qualification, containment or release claim.']}
        check_sources(h, repo_fd, rows)
        absent_cargo_configs(args.repo, absent_home)
        require(time.monotonic() < whole_deadline, 'final gate deadline')
        budget.publish('result.json', accepted, 2097152)
        scan_process_namespace(budget, process_root, whole_deadline)
        absent_cargo_configs(args.repo, absent_home)
        require(time.monotonic() < whole_deadline, 'final accepted publication deadline')
        gate_accepted = True
        print('FOUNDRY_DESKTOP_EDITOR_TASK: PASS evidence=' + str(args.run_root))
        return 0
    finally:
        if budget is not None and verifier is not None and not verifier_published:
            budget.publish('manifest-verifier-observations.json',
                           {'scope': 'actual_owned_pure_verifier_commands_incomplete',
                            'commands': verifier.observations}, 8388608)
        if budget is not None and commands is not None and not published_commands:
            # Only actually returned command records, never synthetic completion
            # for an exception/unknown reap. Logs remain failure evidence.
            budget.publish('command-observations.json', {'status': 'INCOMPLETE_BEFORE_ACCEPTANCE',
                           'commands': commands.observations, 'unrecorded_failure_possible': True}, 2097152)
        elif budget is not None and commands is not None and not gate_accepted:
            # Keep the exclusive interim snapshot and preserve later genuinely
            # returned commands even if validation fails after strict checks.
            budget.publish('command-observations-incomplete.json',
                           {'status': 'INCOMPLETE_AFTER_INTERIM_SNAPSHOT',
                            'source_manifest_sha256': args.source_manifest_sha256,
                            'commands': commands.observations,
                            'unreturned_command_possible': command_return_pending}, 2097152)
        os.close(repo_fd)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--source-manifest', type=Path, required=True)
    parser.add_argument('--source-manifest-sha256', required=True)
    parser.add_argument('--run-root', type=Path, required=True)
    args = parser.parse_args()
    return run(args)


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError, IndexError, RecursionError) as error:
        print('Native task gate: INCOMPLETE; ' + str(error)[:1024], file=sys.stderr)
        sys.exit(1)
