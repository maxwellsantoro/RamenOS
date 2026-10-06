#!/usr/bin/env python3
"""Opt-in, exclusive compiler targets. Cached bytes are never cached acceptance.

The gate supplies its independently frozen source digest and actual tool profiles.
Each feature phase owns a separate target and retains the lock through all child
execution and binary postchecks. Same-user malicious build processes and Cargo
child containment are outside this host development helper's claim.
"""
import argparse
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import stat
import secrets
import sys

INPUT_KEYS = {'schema_version', 'manifest_sha256', 'toolchain', 'platform', 'repo',
              'lock_sha256', 'config', 'environment', 'phase', 'features',
              'default_features', 'profile', 'scope'}
MAX_ENTRIES = 128
MAX_CONTEXT = 65536


class CacheDenied(ValueError): pass


def require(value, reason):
    if not value: raise CacheDenied(reason)


def compact(value):
    raw = json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True, allow_nan=False).encode()
    require(len(raw) <= MAX_CONTEXT, 'cache key/context byte bound')
    return raw


def key_for(inputs):
    require(type(inputs) is dict and set(inputs) == INPUT_KEYS, 'closed cache key fields')
    require(type(inputs['schema_version']) is int and inputs['schema_version'] == 1, 'cache key version')
    for name in ('manifest_sha256', 'lock_sha256'):
        require(type(inputs[name]) is str and re.fullmatch('[a-f0-9]{64}', inputs[name]), 'resolved ' + name)
    require(inputs['scope'] in ('acceptance', 'development'), 'cache scope')
    for name in ('phase', 'profile'):
        require(type(inputs[name]) is str and re.fullmatch('[a-zA-Z0-9_-]{1,64}', inputs[name]), 'cache phase/profile')
    require(type(inputs['repo']) is str and 0 < len(inputs['repo'].encode()) <= 4096
            and Path(inputs['repo']).is_absolute(), 'cache repository')
    require(type(inputs['default_features']) is bool, 'default features')
    fs = inputs['features']
    require(type(fs) is list and len(fs) <= 32
            and all(type(f) is str and re.fullmatch('[a-zA-Z0-9_-]{1,64}', f) for f in fs)
            and fs == sorted(set(fs)), 'closed feature list')
    require(type(inputs['toolchain']) is dict and set(inputs['toolchain']) == {'rustc', 'cargo'}
            and all(type(v) is str and 0 < len(v.encode()) <= 8192 for v in inputs['toolchain'].values()), 'actual tool profiles')
    require(type(inputs['platform']) is dict and set(inputs['platform']) == {'system', 'machine'}
            and all(type(v) is str and 0 < len(v) <= 256 for v in inputs['platform'].values()), 'platform identity')
    require(type(inputs['config']) is list and len(inputs['config']) <= 40
            and type(inputs['environment']) is list and len(inputs['environment']) <= 128, 'config/environment count')
    config_paths = []
    for row in inputs['config']:
        require(type(row) is dict and set(row) == {'path', 'sha256'}, 'compiler config row')
        path = row['path']; sha = row['sha256']
        require(type(path) is str and 0 < len(path.encode()) <= 4096 and '\x00' not in path
                and Path(path).is_absolute() and '..' not in Path(path).parts, 'compiler config path')
        require(sha is None or type(sha) is str and re.fullmatch('[a-f0-9]{64}', sha), 'compiler config pin')
        config_paths.append(path)
    require(config_paths == sorted(set(config_paths)), 'config order/duplicate')
    names = []
    for row in inputs['environment']:
        require(type(row) is list and len(row) == 2 and type(row[0]) is str
                and re.fullmatch('[A-Za-z_][A-Za-z0-9_]{0,127}', row[0])
                and type(row[1]) is str and re.fullmatch('[a-f0-9]{64}', row[1]), 'compiler environment row')
        names.append(row[0])
    require(names == sorted(set(names)), 'compiler environment order/duplicate')
    return hashlib.sha256(compact(inputs)).hexdigest()


def enabled(env):
    value = env.get('RAMEN_FOUNDRY_BUILD_CACHE', '0')
    require(value in ('0', '1'), 'RAMEN_FOUNDRY_BUILD_CACHE must be 0 or 1')
    return value == '1'


def check_process_umask():
    """Query and restore immediately; never change the persistent process mask."""
    previous = None
    try:
        previous = os.umask(0)
    finally:
        if previous is not None:
            os.umask(previous)
    require(previous & 0o022 == 0o022, 'compiler process umask must mask group/other write (022)')
    return previous


def directory(path, create=False, private=False):
    """Hold each nonsymlink ancestor; private leaf is uid-owned mode0700."""
    path = Path(path).absolute()
    require('..' not in path.parts, 'cache path traversal')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        for index, part in enumerate(path.parts[1:]):
            last = index == len(path.parts) - 2
            if create and last:
                try: os.mkdir(part, 0o700, dir_fd=fd)
                except FileExistsError: pass
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
            os.close(fd); fd = child
            info = os.fstat(fd)
            require(info.st_uid in (0, os.getuid()), 'cache ancestor owner')
            sticky_root = info.st_uid == 0 and bool(info.st_mode & stat.S_ISVTX)
            require(not info.st_mode & 0o022 or sticky_root, 'writable cache ancestor')
            if last and private:
                require(info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700, 'private cache directory mode/owner')
        return fd
    except BaseException:
        os.close(fd); raise


def regular_at(fd, name, cap=MAX_CONTEXT, allow_root_owner=False):
    f = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=fd)
    try:
        before = os.fstat(f)
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= cap
                and before.st_uid in ((0, os.getuid()) if allow_root_owner else (os.getuid(),)) and before.st_nlink == 1
                and not before.st_mode & 0o022, 'cache/config regular shape')
        data = bytearray()
        while len(data) < before.st_size:
            block = os.read(f, min(65536, before.st_size - len(data)))
            require(block, 'truncated cache/config'); data.extend(block)
        after = os.fstat(f)
        identity = lambda i: (i.st_dev, i.st_ino, i.st_size, i.st_mtime_ns, i.st_ctime_ns)
        require(not os.read(f, 1) and identity(before) == identity(after), 'changed cache/config')
        return bytes(data)
    finally: os.close(f)


def read_file(path, required=False):
    path = Path(path).absolute()
    try: parent = directory(path.parent)
    except FileNotFoundError:
        if required: raise CacheDenied('required compiler input missing')
        return None
    try:
        try: raw = regular_at(parent, path.name, 8388608, allow_root_owner=True)
        except FileNotFoundError:
            if required: raise CacheDenied('required compiler input missing')
            return None
        return raw
    finally: os.close(parent)


def file_digest(path, required=False):
    raw = read_file(path, required)
    return hashlib.sha256(raw).hexdigest() if raw is not None else None


def configuration_snapshot(repo, env):
    repo = Path(repo).absolute()
    require(len(repo.parents) <= 16, 'bounded Cargo config ancestry')
    candidates = []
    for ancestor in (repo, *repo.parents):
        candidates.extend(ancestor / '.cargo' / n for n in ('config', 'config.toml'))
    cargo_home = Path(env.get('CARGO_HOME', str(Path.home() / '.cargo'))).absolute()
    candidates.extend(cargo_home / n for n in ('config', 'config.toml'))
    paths = sorted(set(candidates))
    require(len(paths) <= 40, 'config/environment count')
    for path in paths:
        require(0 < len(str(path).encode()) <= 4096 and '\x00' not in str(path)
                and '..' not in path.parts, 'compiler config path')
    return [{'path': str(p), 'sha256': file_digest(p)} for p in paths]


def key_inputs(repo, manifest_sha256, phase, features, rustc, cargo, env, scope='acceptance'):
    repo = Path(repo).absolute()
    config = configuration_snapshot(repo, env)
    # Hash effective compiler-related environment, not credentials or output dirs.
    environment = []
    for k in sorted(env):
        if k == 'CARGO_TARGET_DIR': continue
        if k == 'PATH' or k.startswith(('CARGO_', 'RUST', 'CC', 'CXX', 'AR', 'LD', 'PKG_CONFIG')):
            require(type(env[k]) is str and len(env[k].encode()) <= 8192, 'compiler environment value bound')
            environment.append([k, hashlib.sha256(env[k].encode()).hexdigest()])
    inputs = {'schema_version': 1, 'manifest_sha256': manifest_sha256,
              'toolchain': {'rustc': rustc, 'cargo': cargo},
              'platform': {'system': platform.system(), 'machine': platform.machine()},
              'repo': str(repo), 'lock_sha256': file_digest(repo / 'Cargo.lock', required=True),
              'config': config, 'environment': environment, 'phase': phase,
              'features': sorted(features), 'default_features': False,
              'profile': 'dev', 'scope': scope}
    key_for(inputs)
    return inputs


def locked_file(fd, name):
    lock = os.open(name, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK, 0o600, dir_fd=fd)
    try:
        i = os.fstat(lock)
        require(stat.S_ISREG(i.st_mode) and i.st_uid == os.getuid() and i.st_nlink == 1
                and stat.S_IMODE(i.st_mode) == 0o600, 'cache lock shape/owner')
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return lock
    except BaseException:
        os.close(lock); raise


class CacheLease:
    def __init__(self, root, inputs, max_entries=MAX_ENTRIES):
        self.root = Path(root).absolute()
        key_for(inputs)  # Validate shape/bounds before snapshot allocation.
        self.inputs = json.loads(compact(inputs)); self.key = key_for(self.inputs)
        self.max_entries = max_entries
        require(type(max_entries) is int and 1 <= max_entries <= MAX_ENTRIES, 'finite cache entries')
        self.lock = None; self.metadata = None; self.target = None
        self.entry_fd = None; self.marker = None

    def __enter__(self):
        require(self.lock is None, 'cache lease already held')
        root = allocation = entry = None
        try:
            root = directory(self.root, create=True, private=True)
            allocation = locked_file(root, '.allocation.lock')
            count = 0
            with os.scandir(root) as entries:
                for row in entries:
                    if row.name == '.allocation.lock': continue
                    require(re.fullmatch('[a-f0-9]{64}', row.name) and row.is_dir(follow_symlinks=False), 'cache root inventory')
                    count += 1; require(count <= self.max_entries, 'cache entry bound')
            hit = True
            try: entry = os.open(self.key, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=root)
            except FileNotFoundError:
                require(count < self.max_entries, 'cache capacity exhausted')
                os.mkdir(self.key, 0o700, dir_fd=root); hit = False
                entry = os.open(self.key, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=root)
            info = os.fstat(entry)
            require(info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700, 'cache entry private')
            self.lock = locked_file(entry, 'owner.lock')
            try:
                os.stat('.inflight', dir_fd=entry, follow_symlinks=False)
            except FileNotFoundError: pass
            else: raise CacheDenied('abandoned or active compiler target; operator quiescence/reset required')
            expected = compact(self.inputs)
            if hit:
                require(regular_at(entry, 'context.json') == expected, 'cache context mismatch')
            else:
                out = os.open('context.json', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=entry)
                try:
                    offset = 0
                    while offset < len(expected):
                        n = os.write(out, expected[offset:]); require(n > 0, 'context write'); offset += n
                    os.fsync(out)
                finally: os.close(out)
            self.marker = compact({'pid': os.getpid(), 'nonce': secrets.token_hex(16), 'key': self.key})
            marker = os.open('.inflight', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=entry)
            try:
                require(os.write(marker, self.marker) == len(self.marker), 'inflight marker write')
                os.fsync(marker)
            finally: os.close(marker)
            self.entry_fd = os.dup(entry)
            self.target = self.root / self.key / 'target'
            target_fd = directory(self.target, create=not hit, private=True)
            os.close(target_fd)
            self.metadata = {'schema_version': 1, 'enabled': True, 'admitted': True,
                             'key_sha256': self.key, 'hit': hit, 'phase': self.inputs['phase'],
                             'features': self.inputs['features'], 'target': str(self.target),
                             'exclusive_lock_held': True, 'scope': self.inputs['scope'],
                             'cached_acceptance': False, 'inflight_marker_admitted': True}
            return self
        except (OSError, CacheDenied) as e:
            self.close(); raise CacheDenied('compiler cache admission failed: ' + str(e)) from e
        finally:
            for f in (entry, allocation, root):
                if f is not None: os.close(f)

    def close(self, completed=False):
        try:
            if completed and self.entry_fd is not None:
                require(regular_at(self.entry_fd, '.inflight') == self.marker, 'inflight marker replaced')
                os.unlink('.inflight', dir_fd=self.entry_fd)
                os.fsync(self.entry_fd)
        finally:
            if self.entry_fd is not None:
                os.close(self.entry_fd); self.entry_fd = None
            if self.lock is not None:
                os.close(self.lock); self.lock = None

    def __exit__(self, exc_type, exc, tb): self.close(completed=exc_type is None)


class GateCache:
    """Adapter retaining every phase lock until final source/binary postchecks."""
    def __init__(self, repo, manifest_sha256, rustc, cargo, environment):
        check_process_umask()
        self.repo = Path(repo).absolute(); self.manifest_sha256 = manifest_sha256
        self.rustc = rustc; self.cargo = cargo; self.environment = dict(environment)
        self.leases = []; self.records = []; self.completed = False
        self.root = self.repo / 'out/ci-optimization/compiler-cache'
        for p in (self.repo / 'out', self.root.parent):
            fd = directory(p, create=True); os.close(fd)

    def target_for(self, phase, features):
        inputs = key_inputs(self.repo, self.manifest_sha256, phase, features,
                            self.rustc, self.cargo, self.environment)
        lease = CacheLease(self.root, inputs)
        lease.__enter__(); self.leases.append(lease); self.records.append(lease.metadata)
        return lease.target

    def check_contexts(self):
        for lease in self.leases:
            current = key_inputs(self.repo, self.manifest_sha256, lease.inputs['phase'], lease.inputs['features'],
                                 self.rustc, self.cargo, self.environment)
            require(key_for(current) == lease.key, 'compiler cache lock/config context changed')
        self.completed = True

    def close(self):
        failure = None
        try:
            for lease in reversed(self.leases):
                try: lease.close(completed=self.completed)
                except (OSError, CacheDenied) as e:
                    if failure is None: failure = e
        finally: self.leases.clear()
        if failure is not None: raise failure


def retain_binary(source, expected, destination):
    """Retain actual selected bytes in a fresh run, independent of cache reset."""
    source = Path(source).absolute(); destination = Path(destination).absolute()
    require(type(expected) is dict and set(expected) == {'path', 'byte_len', 'sha256'}
            and expected['path'] == str(source) and type(expected['byte_len']) is int
            and 0 < expected['byte_len'] <= 1073741824
            and type(expected['sha256']) is str and re.fullmatch('[a-f0-9]{64}', expected['sha256']), 'selected binary pin')
    parent = out_parent = read_fd = write_fd = None
    created = False; success = False; output_identity = None
    identity = lambda i: (i.st_dev, i.st_ino, i.st_size, i.st_mtime_ns, i.st_ctime_ns, i.st_mode)
    try:
        parent = directory(source.parent)
        read_fd = os.open(source.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=parent)
        before = os.fstat(read_fd)
        require(stat.S_ISREG(before.st_mode) and before.st_uid == os.getuid()
                and before.st_size == expected['byte_len'] and before.st_mode & 0o111
                and not before.st_mode & 0o022, 'selected binary shape/size')
        out_parent = directory(destination.parent, private=True)
        write_fd = os.open(destination.name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=out_parent)
        created = True; info = os.fstat(write_fd); output_identity = (info.st_dev, info.st_ino)
        sha = hashlib.sha256(); size = 0
        while size < before.st_size:
            chunk = os.read(read_fd, min(1048576, before.st_size - size))
            require(chunk, 'binary retention truncated')
            sha.update(chunk); size += len(chunk)
            offset = 0
            while offset < len(chunk):
                n = os.write(write_fd, chunk[offset:]); require(n > 0, 'binary retention write'); offset += n
        require(not os.read(read_fd, 1) and identity(os.fstat(read_fd)) == identity(before), 'selected binary changed during retention')
        require(sha.hexdigest() == expected['sha256'] and size == expected['byte_len'], 'selected binary retention hash')
        os.fchmod(write_fd, 0o500); os.fsync(write_fd)
        after = os.fstat(write_fd); named = os.stat(destination.name, dir_fd=out_parent, follow_symlinks=False)
        require(identity(named) == identity(after) and after.st_size == size, 'retained file replaced')
        os.fsync(out_parent); success = True
        return {'path': str(destination), 'byte_len': size, 'sha256': sha.hexdigest()}
    except OSError as e: raise CacheDenied('binary retention failed: ' + str(e)) from e
    finally:
        try:
            if created and not success:
                try:
                    current = os.stat(destination.name, dir_fd=out_parent, follow_symlinks=False)
                    if (current.st_dev, current.st_ino) == output_identity: os.unlink(destination.name, dir_fd=out_parent)
                except FileNotFoundError: pass
        finally:
            for fd in (write_fd, read_fd, out_parent, parent):
                if fd is not None: os.close(fd)


def main():
    parser = argparse.ArgumentParser(description='Read-only compiler cache input prerequisites; no cache or compiler execution.',
                                     allow_abbrev=False, add_help=False)
    parser.add_argument('--check-inputs', action='store_true', required=True)
    parser.parse_args()
    if sys.argv[1:] != ['--check-inputs']:
        parser.error('exact --check-inputs argument required')
    try:
        if not enabled(os.environ):
            print('BUILD_CACHE_INPUTS: INFO disabled')
            return 0
        check_process_umask()
        repo = Path(__file__).absolute().parents[2]
        lock = file_digest(repo / 'Cargo.lock', required=True)
        config = configuration_snapshot(repo, os.environ)
        print(f'BUILD_CACHE_INPUTS: PASS lock_sha256={lock} config_paths={len(config)}')
        return 0
    except (CacheDenied, OSError) as error:
        print('BUILD_CACHE_INPUTS: FAIL ' + str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
