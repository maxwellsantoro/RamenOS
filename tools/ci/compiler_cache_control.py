"""Inspect or explicitly retire completed compiler entries. Never remove evidence."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('controlled_compiler_cache', HERE / 'build_cache.py')
cache = importlib.util.module_from_spec(spec); spec.loader.exec_module(cache)
TREE_CAP = 262144


def require(value, reason):
    if not value: raise ValueError(reason)


def root_path(repo, scope):
    require(scope in ('acceptance', 'development'), 'compiler cache scope')
    return Path(repo).absolute() / 'out/ci-optimization' / (
        'compiler-cache' if scope == 'acceptance' else 'development-cache')


def existing_lock(fd, name):
    # Inspection never creates a missing lock in a malformed entry.
    info = os.stat(name, dir_fd=fd, follow_symlinks=False)
    require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
            and stat.S_IMODE(info.st_mode) == 0o600 and info.st_nlink == 1, 'existing private lock')
    lock = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK, dir_fd=fd)
    try:
        held = os.fstat(lock)
        require((held.st_dev, held.st_ino, held.st_mode, held.st_uid, held.st_nlink)
                == (info.st_dev, info.st_ino, info.st_mode, info.st_uid, info.st_nlink), 'lock identity changed')
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return lock
    except BaseException:
        os.close(lock); raise


def bounded_names(fd, cap):
    names = []
    with os.scandir(fd) as entries:
        for entry in entries:
            require(len(names) < cap, 'directory inventory bound before growth')
            names.append(entry.name)
    return sorted(names)


def entry_context(fd, repo, scope, key):
    info = os.fstat(fd)
    require(info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700, 'private entry')
    raw = cache.regular_at(fd, 'context.json')
    data = json.loads(raw)
    require(cache.compact(data) == raw and cache.key_for(data) == key, 'entry canonical key/context')
    require(data['repo'] == str(Path(repo).absolute()) and data['scope'] == scope, 'entry repository/scope')
    return raw, data


def tree_inventory(fd, prefix=(), result=None):
    result = {} if result is None else result
    require(len(prefix) <= 32, 'compiler tree depth')
    names = bounded_names(fd, TREE_CAP - len(result))
    for name in names:
        info = os.stat(name, dir_fd=fd, follow_symlinks=False)
        require(info.st_uid == os.getuid() and not info.st_mode & 0o022, 'compiler tree owner/mode')
        require(stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode), 'compiler tree must not contain symlinks/special files')
        path = prefix + (name,)
        require(len(result) < TREE_CAP, 'compiler tree entry bound before insertion')
        result[path] = (info.st_dev, info.st_ino, info.st_mode, info.st_size,
                        info.st_mtime_ns if stat.S_ISREG(info.st_mode) else None)
        if stat.S_ISDIR(info.st_mode):
            child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
            try: tree_inventory(child, path, result)
            finally: os.close(child)
    return result


def remove_tree(fd, inventory, prefix=()):
    for name in bounded_names(fd, TREE_CAP):
        path = prefix + (name,); expected = inventory.get(path)
        require(expected is not None, 'compiler tree changed before retirement')
        info = os.stat(name, dir_fd=fd, follow_symlinks=False)
        identity = (info.st_dev, info.st_ino, info.st_mode, info.st_size,
                    info.st_mtime_ns if stat.S_ISREG(info.st_mode) else None)
        # Directory size changes during child removal; identity is held before recursion.
        require(identity == expected, 'compiler entry changed before retirement')
        if stat.S_ISDIR(info.st_mode):
            child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
            try: remove_tree(child, inventory, path)
            finally: os.close(child)
            os.rmdir(name, dir_fd=fd)
        else: os.unlink(name, dir_fd=fd)


def inspect_cache(repo, scope):
    root = root_path(repo, scope)
    if not root.exists(): return {'scope': scope, 'entries': [], 'available': False}
    fd = cache.directory(root, private=True); allocation = None
    try:
        allocation = existing_lock(fd, '.allocation.lock')
        keys = [name for name in bounded_names(fd, cache.MAX_ENTRIES + 1) if name != '.allocation.lock']
        require(len(keys) <= cache.MAX_ENTRIES and all(re.fullmatch('[a-f0-9]{64}', key) for key in keys), 'cache root inventory')
        rows = []
        for key in keys:
            child = os.open(key, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
            owner = None
            try:
                raw, data = entry_context(child, repo, scope, key)
                row = {'key': key, 'context_sha256': hashlib.sha256(raw).hexdigest(),
                       'phase': data['phase'], 'state': 'completed', 'logical_bytes': None}
                try: owner = existing_lock(child, 'owner.lock')
                except BlockingIOError: row['state'] = 'active'
                fields = set(bounded_names(child, 8))
                if '.inflight' in fields:
                    if row['state'] != 'active': row['state'] = 'quarantined'
                if row['state'] == 'completed':
                    target = cache.directory(root / key / 'target', private=True)
                    try: row['logical_bytes'] = sum(value[3] for value in tree_inventory(target).values() if stat.S_ISREG(value[2]))
                    finally: os.close(target)
                row['retirable'] = row['state'] == 'completed' and fields == {'context.json', 'owner.lock', 'target'}
                rows.append(row)
            finally:
                if owner is not None: os.close(owner)
                os.close(child)
        return {'scope': scope, 'available': True, 'entries': rows,
                'policy': 'only completed entries may be explicitly retired; evidence untouched'}
    finally:
        if allocation is not None: os.close(allocation)
        os.close(fd)


def retire(repo, scope, key, expected_context_sha):
    require(type(key) is str and re.fullmatch('[a-f0-9]{64}', key), 'exact compiler key')
    require(type(expected_context_sha) is str and re.fullmatch('[a-f0-9]{64}', expected_context_sha), 'exact expected context digest')
    root = root_path(repo, scope); fd = cache.directory(root, private=True)
    allocation = owner = child = target = None
    try:
        allocation = existing_lock(fd, '.allocation.lock')
        child = os.open(key, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
        owner = existing_lock(child, 'owner.lock')
        require(set(bounded_names(child, 8)) == {'context.json', 'owner.lock', 'target'}, 'inflight or unknown entry ownership')
        raw, _ = entry_context(child, repo, scope, key)
        require(hashlib.sha256(raw).hexdigest() == expected_context_sha, 'entry changed since inspection')
        target = cache.directory(root / key / 'target', private=True)
        inventory = tree_inventory(target)  # Complete bounded preflight before any deletion.
        marker = os.open('.inflight', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                         0o600, dir_fd=child)
        try:
            raw_marker = cache.compact({'action': 'explicit_compiler_retirement', 'key': key, 'pid': os.getpid()})
            require(os.write(marker, raw_marker) == len(raw_marker), 'retirement marker write')
            os.fsync(marker)
        finally: os.close(marker)
        os.fsync(child)  # An interrupted partial retirement is quarantined.
        remove_tree(target, inventory)
        os.close(target); target = None
        os.rmdir('target', dir_fd=child)
        os.unlink('context.json', dir_fd=child); os.unlink('owner.lock', dir_fd=child)
        os.unlink('.inflight', dir_fd=child)
        os.rmdir(key, dir_fd=fd); os.fsync(fd)
        return {'retired': key, 'scope': scope, 'evidence_removed': False}
    finally:
        for handle in (target, owner, child, allocation, fd):
            if handle is not None: os.close(handle)


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('action', choices=('inspect', 'retire'))
    parser.add_argument('--scope', choices=('acceptance', 'development'), default='acceptance')
    parser.add_argument('--key'); parser.add_argument('--expected-context-sha256')
    args = parser.parse_args()
    if args.action == 'inspect':
        require(args.key is None and args.expected_context_sha256 is None, 'inspection takes no deletion selection')
        result = inspect_cache(ROOT, args.scope)
    else: result = retire(ROOT, args.scope, args.key, args.expected_context_sha256)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__': main()
