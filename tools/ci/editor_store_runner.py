#!/usr/bin/env python3
"""Root-owned host CAS gate runner; execution requires separate coordinator authority.

The independently pinned CLI manifest selects reviewed repository inputs, never
an evidence artifact. Fixture root paths and PIDs are diagnostic data only. This
runner does not establish integrated editor, target, device, power-loss or process
containment. Only its actually held Popen children may be signalled. A timeout or
uncertain reap fails the gate; Cargo descendants are not claimed contained.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import secrets
import select
import stat
import struct
import subprocess
import sys
import time
import tomllib

CONTRACT_SHA = 'aa3cc795374a9bfda490adec57750d8258d4d3589155f8fba50a1010ce3848be'
DECODER_SOURCE_SHA = '510782aa1f993c76162ccb6a98677ecafea47ba9425bf6af909c08f635b8f0de'
COMPOSITE_SHA = 'b15b668f9bc8e55298ceb344144d491e370d2d314102020ad0058a501cce064c'
CASES = (
    'ui1_1_revoke_paused_key_frame_and_save',
    'ui1_1_stalled_session_leaves_other_session_usable',
    'ui1_1_clock_capacity_and_counter_limits_fail_closed',
    'ui1_1_store_commit_reopen_preserves_prior_blob',
    'ui1_1_store_same_base_conflict_and_operation_reuse',
    'ui1_1_store_lost_reply_reconciles_without_mutation_replay',
    'ui1_1_store_recovery_validates_atomic_selection_and_receipt',
)
LEGS = (3, 1, 16, 5, 1, 4, 18)
EXPORTS = ('RAMEN_DESKTOP_EDITOR_STORE_GATE_EVIDENCE',
           'RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE',
           'RAMEN_DESKTOP_EDITOR_STORE_GATE_RUN_NONCE')
OTHER_CAP = 4194304
RESULT_RESERVE = 65536
LOG_CAP = 8388608  #Two test/list logs together <=16MiB in the semantic reader.
SOURCE_CAP = 8388608

class Incomplete(ValueError):
    pass


def require(condition, reason):
    if not condition:
        raise Incomplete(reason)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode('utf8')


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    def bad(_):
        raise Incomplete('noninteger JSON number')
    value = json.loads(raw.decode('utf8', 'strict'), object_pairs_hook=pairs,
                       parse_float=bad, parse_constant=bad)
    def walk(item, depth=0):
        require(depth <= 32, 'JSON depth')
        if type(item) is dict:
            for key, child in item.items():
                require(type(key) is str, 'JSON key type')
                walk(child, depth + 1)
        elif type(item) is list:
            for child in item:
                walk(child, depth + 1)
        else:
            require(item is None or type(item) in (str, int, bool), 'JSON type')
    walk(value)
    return value


def hexhash(value):
    require(type(value) is str and re.fullmatch('[0-9a-f]{64}', value) is not None, 'SHA256 input')
    return value


def identity(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns, info.st_mode)


def regular(path, cap, keep=True):
    """Explicit trusted path only; bounded nofollow held-file read/hash."""
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= cap, 'regular file admission')
        sha = hashlib.sha256()
        raw = bytearray()
        count = 0
        while count < before.st_size:
            block = os.read(fd, min(65536, before.st_size - count))
            require(bool(block), 'truncated regular file')
            sha.update(block)
            count += len(block)
            if keep:
                raw.extend(block)
        require(not os.read(fd, 1) and identity(os.fstat(fd)) == identity(before), 'regular file changed')
        return bytes(raw), before.st_size, sha.hexdigest(), identity(before)
    finally:
        os.close(fd)


def relative_parts(path):
    require(type(path) is str and 0 < len(path.encode()) <= 4096, 'source relative path bound')
    parts = path.split('/')
    require(all(re.fullmatch('[A-Za-z0-9_.-]{1,128}', p) and p not in ('.', '..') for p in parts), 'source path components')
    require(len(parts) <= 32, 'source path depth')
    return parts


def source_read(repo_fd, name, cap=SOURCE_CAP, keep=False):
    parts = relative_parts(name)
    parent = os.dup(repo_fd)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
            os.close(parent)
            parent = child
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK, dir_fd=parent)
        try:
            before = os.fstat(fd)
            require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= cap, 'source file admission')
            sha = hashlib.sha256()
            raw = bytearray()
            count = 0
            while count < before.st_size:
                block = os.read(fd, min(65536, before.st_size - count))
                require(bool(block), 'truncated source')
                count += len(block)
                sha.update(block)
                if keep:
                    raw.extend(block)
            require(not os.read(fd, 1) and identity(os.fstat(fd)) == identity(before), 'source changed during read')
            return bytes(raw), before.st_size, sha.hexdigest()
        finally:
            os.close(fd)
    finally:
        os.close(parent)


def read_manifest(path, pin):
    raw, _, sha, _ = regular(path, 524288)
    require(sha == hexhash(pin), 'independently frozen manifest mismatch')
    rows = strict_json(raw)
    require(type(rows) is list and 1 <= len(rows) <= 1024, 'source row count')
    ordered = []
    for row in rows:
        require(type(row) is dict and set(row) == {'relative_path', 'byte_len', 'sha256'}, 'source row keys')
        relative_parts(row['relative_path'])
        require(type(row['byte_len']) is int and 1 <= row['byte_len'] <= SOURCE_CAP, 'source size')
        hexhash(row['sha256'])
        item = {k: row[k] for k in ('relative_path', 'byte_len', 'sha256')}
        require(len(compact(item)) <= 512, 'source row serialized bound')
        ordered.append(item)
    names = [r['relative_path'] for r in ordered]
    require(names == sorted(set(names)) and compact(ordered) == raw, 'canonical sorted exact source manifest')
    mandatory = {'rust-toolchain.toml', 'Cargo.toml', 'Cargo.lock',
                 'tools/ci/editor_store_runner.py', 'tools/ci/editor_store_result.py',
                 'docs/contracts/editor-store-recording-v0.json',
                 'services/store_service/examples/check_editor_store_evidence.rs',
                 'services/store_service/tests/editor_store.rs', 'services/store_service/tests/support/editor_store.rs'}
    require(mandatory <= set(names), 'trusted closure lacks runner dependencies')
    return ordered


def check_sources(repo_fd, rows):
    for row in rows:
        _, size, sha = source_read(repo_fd, row['relative_path'])
        require((size, sha) == (row['byte_len'], row['sha256']), 'actual accepted source mismatch: ' + row['relative_path'])


def validate_shape(value, spec, definitions):
    if '$ref' in spec:
        return validate_shape(value, definitions[spec['$ref'][8:]], definitions)
    if 'oneOf' in spec:
        passed = 0
        for branch in spec['oneOf']:
            try:
                validate_shape(value, branch, definitions)
                passed += 1
            except Incomplete:
                pass
        require(passed == 1, 'shape oneOf')
        return
    if 'const' in spec:
        require(type(value) is type(spec['const']) and value == spec['const'], 'shape const')
    if 'enum' in spec:
        require(any(type(value) is type(x) and value == x for x in spec['enum']), 'shape enum')
    kind = spec.get('type')
    if kind == 'object':
        require(type(value) is dict and set(value) == set(spec['properties']), 'shape closed keys')
        for key, subspec in spec['properties'].items():
            validate_shape(value[key], subspec, definitions)
    elif kind == 'array':
        require(type(value) is list and spec.get('minItems', 0) <= len(value) <= spec['maxItems'], 'shape array')
        for child in value:
            validate_shape(child, spec['items'], definitions)
    elif kind == 'string':
        require(type(value) is str and spec.get('minLength', 0) <= len(value) <= spec.get('maxLength', 4096), 'shape string')
        if 'pattern' in spec:
            require(re.fullmatch(spec['pattern'], value) is not None, 'shape pattern')
    elif kind == 'integer':
        require(type(value) is int and spec['minimum'] <= value <= spec['maximum'], 'shape integer')
    elif kind == 'boolean':
        require(type(value) is bool, 'shape boolean')
    elif kind == 'null':
        require(value is None, 'shape null')
    else:
        require(kind is None and ('const' in spec or 'enum' in spec), 'unsupported shape')


class Budget:
    def __init__(self, root):
        self.root = root
        self.other = 0
        self.test = 0
        self.files = 0
    def open(self, name):
        require(re.fullmatch('[a-z0-9_.-]{1,128}', name) is not None and self.files < 35, 'runner filename/count')
        fd = os.open(self.root / name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
        self.files += 1
        return fd
    def charge(self, count, test=False):
        if test:
            require(self.test + count <= 16777216, 'combined test/list log cap before growth')
            self.test += count
        else:
            require(self.other + count <= OTHER_CAP - RESULT_RESERVE, 'runner other bytes before growth')
            self.other += count
    def publish(self, name, value, cap):
        raw = bytearray()
        for piece in json.JSONEncoder(ensure_ascii=False, separators=(',', ':'), allow_nan=False).iterencode(value):
            block = piece.encode('utf8')
            require(len(raw) + len(block) <= cap, 'publication bytes before growth')
            raw.extend(block)
        self.charge(len(raw))
        fd = self.open(name)
        try:
            write_all(fd, raw)
            os.fsync(fd)
        finally:
            os.close(fd)


def write_all(fd, raw):
    offset = 0
    while offset < len(raw):
        wrote = os.write(fd, raw[offset:])
        require(wrote > 0, 'write status')
        offset += wrote


def native_birth(process):
    """Read only our still-held native child identity; never an artifact PID."""
    require(process.poll() is None, 'owned child exited before birth observation')
    pid = process.pid
    require(type(pid) is int and 0 < pid <= 0xffffffff, 'actual native child PID')
    if sys.platform == 'linux':
        raw = proc_read('/proc/' + str(pid) + '/stat', 8192)
        text = raw.decode('ascii', 'strict')
        require(int(text.split(' ', 1)[0]) == pid, 'proc child PID mismatch')
        fields = text.rsplit(') ', 1)[1].split()
        require(20 <= len(fields) < 128 and fields[19].isdigit(), 'proc field22')
        boot = proc_read('/proc/sys/kernel/random/boot_id', 64).decode('ascii').strip()
        require(re.fullmatch('[0-9a-f-]{36}', boot) is not None, 'boot identity')
        birth = 'linux:' + boot + ':' + str(int(fields[19]))
        method = 'linux_proc_starttime_boot_id'
    elif sys.platform == 'darwin':
        lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
        fn = lib.proc_pidinfo
        fn.argtypes = (ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int)
        fn.restype = ctypes.c_int
        info = ctypes.create_string_buffer(136)
        require(fn(pid, 3, 0, info, 136) == 136, 'native libproc136B')
        native_pid = struct.unpack_from('=I', info.raw, 12)[0]
        seconds, micros = struct.unpack_from('=QQ', info.raw, 120)
        require(native_pid == pid and micros < 1000000, 'native libproc child identity')
        birth = f'macos:{native_pid}:{seconds}:{micros}'
        method = 'macos_proc_starttime'
    else:
        raise Incomplete('unsupported native process platform')
    require(process.poll() is None, 'child not held across birth observation')
    return birth, method


def proc_read(path, cap):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        raw = bytearray()
        while len(raw) <= cap:
            block = os.read(fd, min(4096, cap + 1 - len(raw)))
            if not block:
                break
            require(len(raw) + len(block) <= cap, 'proc observation bound before growth')
            raw.extend(block)
        require(bool(raw), 'empty native observation')
        return bytes(raw)
    finally:
        os.close(fd)


class OwnedCommand:
    """Finite streamed command; held Popen authority cleared at actual reap."""
    def __init__(self, budget, cwd, env):
        self.budget, self.cwd, self.env = budget, cwd, env
        self.ordinal = 0
        self.observations = []
    def run(self, argv, name, timeout=600, test=False, observe_birth=False):
        require(self.ordinal < 16 and 1 <= len(argv) <= 32, 'owned command inventory')
        require(all(type(s) is str and 0 < len(s.encode()) <= 4096 for s in argv), 'command argv bound')
        self.ordinal += 1
        cap = LOG_CAP if test else 1048576
        #One combined logfile preserves pipe bytes without synthesized test lines.
        fd = self.budget.open(name)
        retained = 0
        sha = hashlib.sha256()
        stdout = bytearray()
        stderr = bytearray()
        process = None
        held = False
        status = None
        birth = method = None
        started = time.monotonic_ns()
        deadline = time.monotonic() + timeout
        def reap():
            nonlocal held, status
            if held:
                observed = process.poll()
                if observed is not None:
                    held = False  #Retire signal authority before any diagnostic work.
                    status = observed
        try:
            process = subprocess.Popen(argv, cwd=self.cwd, env=self.env, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0, close_fds=True)
            held = True
            if observe_birth:
                birth, method = native_birth(process)
            streams = {process.stdout.fileno(): stdout, process.stderr.fileno(): stderr}
            for pipe in streams:
                os.set_blocking(pipe, False)
            while held or streams:
                require(time.monotonic() < deadline - 2, 'owned command work deadline')
                ready, _, _ = select.select(list(streams), [], [], min(0.02, max(0, deadline - 2 - time.monotonic())))
                for pipe in ready:
                    block = os.read(pipe, min(65536, cap + 1 - retained))
                    if not block:
                        del streams[pipe]
                        continue
                    require(retained + len(block) <= cap, 'subprocess log cap before growth')
                    self.budget.charge(len(block), test)
                    write_all(fd, block)
                    sha.update(block)
                    retained += len(block)
                    #Logs and in-memory parse inputs share the same admitted cap.
                    (stdout if pipe == process.stdout.fileno() else stderr).extend(block)
                reap()
            require(status is not None and not held, 'actual command reap')
            exited = time.monotonic_ns()
            require(time.monotonic() < deadline, 'owned command final deadline')
            os.fsync(fd)
            require(time.monotonic() < deadline, 'owned command log publication deadline')
            result = {'stdout': bytes(stdout), 'stderr': bytes(stderr), 'status': status,
                      'log': {'relative_file': name, 'byte_len': retained, 'sha256': sha.hexdigest()},
                      'process': {'native_pid': process.pid, 'start_identity': birth,
                                  'start_identity_method': method, 'spawn_observed': True,
                                  'start_monotonic_ns': started, 'exit_monotonic_ns': exited,
                                  'exit_code_or_null': status if status >= 0 else None,
                                  'term_signal_or_null': -status if status < 0 else None,
                                  'wait_reaped': True, 'wait_method': 'owned_subprocess_wait', 'argv': argv}}
            self.observations.append({'ordinal': len(self.observations), 'argv': argv,
                                      'actual_process': result['process'], 'log': result['log'],
                                      'stdout_bytes': len(stdout), 'stdout_sha256': digest(stdout),
                                      'stderr_bytes': len(stderr), 'stderr_sha256': digest(stderr),
                                      'deadline_seconds': timeout})
            return result
        finally:
            try:
                if held:
                    reap()
                if held:
                    process.kill()
                    while held and time.monotonic() < deadline:
                        reap()
                        if held:
                            time.sleep(min(0.01, max(0, deadline - time.monotonic())))
                    require(not held, 'owned command reap uncertain')
            finally:
                os.close(fd)
                if process is not None:
                    for stream in (process.stdout, process.stderr):
                        if stream is not None:
                            stream.close()


def successful(record):
    require(record['status'] == 0, 'actual owned command failed')
    return record['stdout']


def artifact(raw, target_name, kind, is_test, target_dir, filename_suffix=None):
    require(type(raw) is bytes and 0 < len(raw) <= 1048576, 'Cargo JSON admission')
    found = []
    finished = False
    for line in raw.splitlines():
        require(len(line) <= 262144 and bool(line), 'Cargo JSON line bound')
        row = strict_json(line)
        require(type(row) is dict and type(row.get('reason')) is str and not finished, 'Cargo message completion/order')
        if row['reason'] == 'build-finished':
            require(row.get('success') is True, 'Cargo unsuccessful build-finished')
            finished = True
        elif row['reason'] == 'compiler-artifact':
            target = row.get('target')
            require(type(target) is dict and type(target.get('name')) is str and type(target.get('kind')) is list, 'Cargo target')
            if target['name'] != target_name or target['kind'] != [kind]:
                continue
            require(type(row.get('profile')) is dict and row['profile'].get('test') is is_test, 'Cargo actual profile')
            if filename_suffix is None:
                path = row.get('executable')
                require(type(path) is str and bool(path), 'Cargo actual executable')
                found.append(path)
            else:
                filenames = row.get('filenames')
                require(type(filenames) is list and len(filenames) <= 16, 'Cargo library filenames')
                selected = [p for p in filenames if type(p) is str and p.endswith(filename_suffix)]
                require(len(selected) == 1, 'actual rlib inventory')
                found.extend(selected)
    require(finished and len(found) == 1, 'one exact Cargo artifact required')
    chosen = Path(found[0])
    require(chosen.is_absolute() and target_dir in chosen.parents and not any(ord(c) < 32 for c in str(chosen)), 'Cargo artifact trusted target path')
    _, size, sha, ident = regular(chosen, 1073741824, False)
    if filename_suffix is None:
        require(ident[-1] & 0o111, 'selected binary executable mode')
    return chosen, {'path': str(chosen), 'byte_len': size, 'sha256': sha}


def unchanged_binary(path, before):
    _, size, sha, _ = regular(path, 1073741824, False)
    require((size, sha) == (before['byte_len'], before['sha256']), 'actual executable changed')


def absolute(value):
    require(type(value) is str and 0 < len(value.encode()) <= 4096, 'absolute CLI path bound')
    path = Path(value)
    require(path.is_absolute() and '..' not in path.parts, 'explicit absolute path required')
    return path


def private_directory(path, create=False):
    #Validate every parent without following a symlink. Shared ancestors such as
    #/tmp are allowed; the directly selected owned directory must be private.
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        if current == path and create:
            os.mkdir(current, 0o700)
        info = os.stat(current, follow_symlinks=False)
        require(stat.S_ISDIR(info.st_mode), 'directory or parent symlink')
    info = os.stat(path, follow_symlinks=False)
    require(info.st_uid == os.geteuid() and info.st_mode & 0o077 == 0, 'private owned directory required')


def run_gate(args):
    repo, target, evidence = map(absolute, (args.repo, args.target_dir, args.evidence))
    manifest_path = absolute(args.source_manifest)
    pin = hexhash(args.source_manifest_sha256)
    require(target != evidence and target not in evidence.parents and evidence not in target.parents, 'target/evidence overlap')
    require(not evidence.exists() and not evidence.is_symlink(), 'evidence must be new')
    private_directory(evidence.parent)
    private_directory(evidence, create=True)
    if not target.exists():
        #Build target is root-selected and outside evidence; it may persist.
        private_directory(target, create=True)
    else:
        private_directory(target)
    repo_fd = os.open(repo, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        repo_identity = (os.fstat(repo_fd).st_dev, os.fstat(repo_fd).st_ino)
        rows = read_manifest(manifest_path, pin)
        check_sources(repo_fd, rows)
        own_raw, _, _ = source_read(repo_fd, 'tools/ci/editor_store_runner.py', SOURCE_CAP, True)
        _, _, own_sha, _ = regular(Path(__file__), SOURCE_CAP)
        require(digest(own_raw) == own_sha, 'executing runner must equal accepted repository runner')
        _, _, contract_sha = source_read(repo_fd, 'docs/contracts/editor-store-recording-v0.json')
        require(contract_sha == CONTRACT_SHA, 'fixed runtime contract pin')
        _, _, decoder_sha = source_read(repo_fd, 'services/store_service/examples/check_editor_store_evidence.rs')
        require(decoder_sha == DECODER_SOURCE_SHA, 'fixed native decoder source pin')
        contract_raw, _, _ = source_read(repo_fd, 'docs/contracts/editor-store-recording-v0.json', 131072, True)
        contract = strict_json(contract_raw)
        definitions = contract['shapes']['$defs']
        toolchain_raw, _, _ = source_read(repo_fd, 'rust-toolchain.toml', 8192, True)
        channel = tomllib.loads(toolchain_raw.decode('utf8'))['toolchain']['channel']
        require(type(channel) is str and re.fullmatch('nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}', channel) is not None, 'actual pinned toolchain')
        env = dict(os.environ)
        for key in (*EXPORTS, 'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
                    'CARGO_BUILD_RUSTC', 'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER',
                    'CARGO_BUILD_RUSTFLAGS', 'RUSTDOC', 'RUSTDOCFLAGS', 'CARGO_BUILD_RUSTDOC'):
            env.pop(key, None)
        env['CARGO_TARGET_DIR'] = str(target)
        budget = Budget(evidence)
        commands = OwnedCommand(budget, str(repo), env)
        rustc = commands.run(['rustc', '+' + channel, '-vV'], 'rustc.log')
        cargo_version = commands.run(['cargo', '+' + channel, '--version'], 'cargo-version.log')
        rust_stdout = successful(rustc).decode('utf8', 'strict')
        cargo_stdout = successful(cargo_version).decode('utf8', 'strict')
        lines = dict(line.split(': ', 1) for line in rust_stdout.splitlines() if ': ' in line)
        rust_profile = {'stdout': rust_stdout, 'stdout_sha256': digest(rustc['stdout']), 'exit_code': rustc['status'],
                        'release': lines['release'], 'commit_hash': lines['commit-hash'], 'commit_date': lines['commit-date'],
                        'host': lines['host'], 'llvm_version': lines['LLVM version']}
        cargo_profile = {'stdout': cargo_stdout, 'stdout_sha256': digest(cargo_version['stdout']), 'exit_code': cargo_version['status']}
        cargo = ['cargo', '+' + channel]
        default_build = commands.run(cargo + ['build', '--locked', '--no-default-features', '-p', 'store_service', '--lib', '--message-format=json'], 'default-build.log')
        rlib, _ = artifact(successful(default_build), 'store_service', 'lib', False, target, '.rlib')
        dependency_dir = rlib.parent if rlib.parent.name == 'deps' else rlib.parent / 'deps'
        require(dependency_dir.is_dir() and not dependency_dir.is_symlink(), 'actual Cargo dependency directory')
        #A meaningful default-off negative uses this actual default library.
        probe_dir = target / ('editor-store-default-off-' + secrets.token_hex(16))
        os.mkdir(probe_dir, 0o700)
        probe = probe_dir / 'probe.rs'
        probe_fd = os.open(probe, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        try:
            write_all(probe_fd, b'use store_service::editor_store::StoreFixture;\nfn main() {}\n')
        finally:
            os.close(probe_fd)
        denial = commands.run(['rustc', '+' + channel, '--edition=2021', '--error-format=json', str(probe),
                               '--extern', 'store_service=' + str(rlib), '-L', 'dependency=' + str(dependency_dir),
                               '-o', str(probe_dir / 'probe')], 'default-off.log')
        require(denial['status'] != 0, 'default-off namespace unexpectedly available')
        diagnostics = [strict_json(line) for line in denial['stderr'].splitlines()]
        coded_errors = [d for d in diagnostics if d.get('level') == 'error' and d.get('code') is not None]
        require(len(coded_errors) == 1 and coded_errors[0]['code']['code'] == 'E0432'
                and 'store_service::editor_store' in coded_errors[0]['message'], 'default-off must fail actual missing editor_store E0432')
        require(all(d.get('level') != 'error' or d in coded_errors or d.get('message') == 'aborting due to 1 previous error' for d in diagnostics), 'unrelated default-off compiler failure')
        os.unlink(probe)
        os.rmdir(probe_dir)
        decoder_build = commands.run(cargo + ['build', '--locked', '--no-default-features', '-p', 'store_service', '--example',
                                             'check_editor_store_evidence', '--message-format=json'], 'decoder-build.log')
        decoder, decoder_info = artifact(successful(decoder_build), 'check_editor_store_evidence', 'example', False, target)
        test_build = commands.run(cargo + ['test', '--locked', '-p', 'store_service', '--features', 'editor_store_v0_dev',
                                          '--test', 'editor_store', '--no-run', '--message-format=json'], 'test-build.log')
        binary, binary_info = artifact(successful(test_build), 'editor_store', 'test', True, target)
        clippy = commands.run(cargo + ['clippy', '--locked', '-p', 'store_service', '--features', 'editor_store_v0_dev',
                                      '--test', 'editor_store', '--', '-D', 'warnings'], 'clippy.log')
        successful(clippy)
        check_sources(repo_fd, rows)
        listing = commands.run([str(binary), '--list'], 'test-list.log', test=True)
        listed = successful(listing).decode('utf8', 'strict')
        require(sorted(re.findall(r'^(\w+): test$', listed, re.MULTILINE)) == sorted(CASES)
                and '7 tests, 0 benchmarks' in listed, 'actual exact test inventory')
        require(not re.search(r': benchmark$', listed, re.MULTILINE), 'unexpected benchmark')
        unchanged_binary(binary, binary_info)
        nonce = secrets.token_hex(16)
        store_root, ui_root = evidence / 'cases', evidence / 'ui-a'
        os.mkdir(store_root, 0o700)
        os.mkdir(ui_root, 0o700)
        commands.env = dict(env, **{EXPORTS[0]: str(store_root), EXPORTS[1]: str(ui_root), EXPORTS[2]: nonce})
        #OwnedCommand calls native_birth on the actual held Popen, not fixture data.
        tested = commands.run([str(binary), '--test-threads=1'], 'test.log', test=True, observe_birth=True)
        successful(tested)
        tested_text = tested['stdout'].decode('utf8', 'strict')
        require(sorted(re.findall(r'^test (\w+) \.\.\. (\w+)$', tested_text, re.MULTILINE)) == sorted((c, 'ok') for c in CASES), 'actual seven test outcomes')
        require(re.search(r'test result: ok\. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', tested_text), 'actual test completion summary')
        unchanged_binary(binary, binary_info)
        unchanged_binary(decoder, decoder_info)
        check_sources(repo_fd, rows)
        #Only fixed inventory paths are opened; diagnostic carrier paths/PIDs
        #are copied and validated, never used for source or lifecycle actions.
        filesystems = []
        directory_sync_observed = False
        for case, count in zip(CASES, LEGS):
            for leg in range(count):
                cleanup_raw, _, _, _ = regular(store_root / case / f'store-{leg}' / 'cleanup.json', 1048576)
                carrier = strict_json(cleanup_raw)
                require(carrier['run_nonce'] == nonce and carrier['native_test_pid'] == tested['process']['native_pid']
                        and carrier['test_process_start_identity'] == tested['process']['start_identity']
                        and carrier['case'] == case and type(carrier['leg']) is int and carrier['leg'] == leg, 'actual cleanup carrier identity')
                filesystem = carrier['fixture_filesystem']
                validate_shape(filesystem, definitions['fixture_filesystem'], definitions)
                require(filesystem['native_test_pid'] == tested['process']['native_pid'] and filesystem['observed_while_root_live'] is True, 'live trusted fixture observation')
                require(filesystem['fixture_id'] == carrier['fixture_id'], 'fixture filesystem identity')
                filesystems.append(filesystem)
                snapshot_raw, _, _, _ = regular(store_root / case / f'store-{leg}' / 'epoch-0-after-join.json', 2097152)
                snapshot = strict_json(snapshot_raw)
                require(snapshot['run_nonce'] == nonce and snapshot['native_test_pid'] == tested['process']['native_pid']
                        and snapshot['test_process_start_identity'] == tested['process']['start_identity'], 'fixed snapshot run identity')
                events = snapshot['public_evidence']['io_events']
                require(type(events) is list and len(events) <= 1024, 'actual IO observation bound')
                directory_sync_observed |= any(type(e) is dict and e.get('point') in ('candidate_directory_sync', 'journal_directory_sync')
                                               and e.get('outcome') == 'completed' for e in events)
        require(len(filesystems) == 48 and len({f['fixture_id'] for f in filesystems}) == 48, 'actual filesystem inventory')
        require(directory_sync_observed, 'actual named directory sync completion missing')
        test_process = dict(tested['process'], test_binary=binary_info)
        uname = platform.uname()
        os_profile = {'system': uname.system, 'release': uname.release, 'version': uname.version,
                      'machine': uname.machine, 'observation_monotonic_ns': time.monotonic_ns()}
        provenance = {'schema_version': 1, 'run_nonce': nonce, 'evidence_contract_sha256': COMPOSITE_SHA,
                      'accepted_source_manifest_sha256': pin, 'sources': rows, 'test_process': test_process,
                      'os_profile': os_profile, 'rustc_profile': rust_profile, 'cargo_profile': cargo_profile,
                      'fixture_filesystems': filesystems, 'test_inventory': list(CASES),
                      'test_log': tested['log'], 'test_list_log': listing['log'],
                      'store_evidence_root': str(store_root), 'ui_evidence_root': str(ui_root),
                      'claims': {k: k in ('host_cas', 'directory_sync_observed') for k in
                                 ('host_cas', 'directory_sync_observed', 'integrated_editor_save', 'target', 'device_flush', 'power_loss', 'containment')}}
        validate_shape(provenance, definitions['runner_provenance'], definitions)
        require(len(compact(filesystems)) <= 786432, 'filesystem rows aggregate')
        budget.publish('provenance.json', provenance, 2097152)
        commands.env = env
        consumer_path = repo / 'tools/ci/editor_store_result.py'
        consumer = commands.run([sys.executable, str(consumer_path), '--evidence', str(evidence),
                                 '--source-manifest', str(manifest_path), '--source-manifest-sha256', pin,
                                 '--codec-binary', str(decoder), '--codec-binary-sha256', decoder_info['sha256']], 'consumer.log')
        successful(consumer)  #Actual owned exit0 is mandatory; JSON alone never passes.
        check_sources(repo_fd, rows)
        live_repo = os.stat(repo, follow_symlinks=False)
        require((live_repo.st_dev, live_repo.st_ino) == repo_identity, 'actual build/run repository identity changed')
        unchanged_binary(binary, binary_info)
        unchanged_binary(decoder, decoder_info)
        result_raw, result_size, _, _ = regular(evidence / 'result.json', RESULT_RESERVE)
        result = strict_json(result_raw)
        require(type(result) is dict and result['status'] == 'PASS' and type(result['schema_version']) is int and result['schema_version'] == 1,
                'actual consumer result missing PASS')
        require(set(result) == {'schema_version','gate','status','behavior_cases','behavior_case_count','semantic_cases','semantic_case_count',
                                'runtime_contract_sha256','evidence_contract_sha256','source_manifest_sha256','source_count','run_nonce',
                                'actual_test_pid','test_process_start_identity','codec','scope','claims'}, 'closed consumer result keys')
        require(result['gate'] == 'foundry-desktop-editor-store-ui1-1b' and result['behavior_cases'] == list(CASES)
                and type(result['behavior_case_count']) is int and result['behavior_case_count'] == 7
                and type(result['semantic_case_count']) is int and result['semantic_case_count'] == 16,
                'actual consumer inventory')
        require(result['source_manifest_sha256'] == pin and result['runtime_contract_sha256'] == CONTRACT_SHA
                and result['run_nonce'] == nonce and result['actual_test_pid'] == test_process['native_pid']
                and result['test_process_start_identity'] == test_process['start_identity'], 'consumer actual run binding')
        require(result['evidence_contract_sha256'] == COMPOSITE_SHA
                and type(result['source_count']) is int and result['source_count'] == len(rows)
                and result['codec']['binary_sha256'] == decoder_info['sha256'], 'consumer accepted closure/decoder binding')
        require(result['claims'] == {'host_cas': True, 'pure_codec': True, 'integrated_editor_save': False,
                                    'target': False, 'device_flush': False, 'power_loss': False,
                                    'containment': False, 'runtime_isolation': False}
                and all(type(flag) is bool for flag in result['claims'].values()), 'consumer exact claim scope')
        #This supplemental diagnostic ledger is actual command evidence, never
        #a process authority certificate or substitute for the consumer result.
        budget.publish('execution.json', {'schema_version': 1, 'run_nonce': nonce,
                                         'source_manifest_sha256': pin, 'runner_sha256': own_sha,
                                         'commands': commands.observations, 'test_binary': binary_info,
                                         'codec_binary': decoder_info, 'consumer_actual_exit_code': consumer['status'],
                                         'result_raw_sha256': digest(result_raw)}, 65536)
        require(budget.other + result_size <= OTHER_CAP and budget.files + 1 <= 36, 'final runner byte/file bounds')
        return 0
    finally:
        os.close(repo_fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True)
    parser.add_argument('--source-manifest', required=True)
    parser.add_argument('--source-manifest-sha256', required=True)
    parser.add_argument('--target-dir', required=True)
    parser.add_argument('--evidence', required=True)
    args = parser.parse_args()
    return run_gate(args)


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (Incomplete, OSError, ValueError, KeyError, IndexError, subprocess.SubprocessError, RecursionError):
        print('editor Store runner: INCOMPLETE or failed validation', file=sys.stderr)
        sys.exit(1)
