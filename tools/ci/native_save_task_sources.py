#!/usr/bin/env python3
"""Root-selected compilation closure for the default-off native editor task.

Enumerates the six compiled workspace crates, their actual path dependencies,
workspace manifests, IDLs, and literal Rust includes. An exact reviewed registry
must match before a snapshot can be used. This is source admission, not a test.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import tomllib

ROOTS = ('apps/artifact_editor', 'services/desktop', 'services/store_service',
         'artifact_store_core', 'artifact_store_schema', 'kernel_api')
REGISTRY = 'tools/ci/native_save_task_sources_v0.json'
EXTRAS = ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'justfile',
          'tools/ci/ci_lanes.py', 'tools/ci/foundry_ci_extended.sh',
          'tools/ci/editor_store_runner.py', 'tools/ci/native_save_task_sources.py',
          REGISTRY, 'tools/ci/desktop_editor_task_result.py',
          'tools/ci/foundry_desktop_editor_task_ui1_1c.sh',
          'tools/ci/native_save_task_recording.py', 'tools/ci/owned_manifest_verifier.py',
          'docs/contracts/editor-native-save-task-v0.json',
          'docs/contracts/editor-native-save-recording-v0.json',
          'docs/contracts/editor-native-save-source-inventory-v0.json')
CAP = 8388608
COUNT = 4096


def require(value, reason):
    if not value:
        raise ValueError(reason)


def parts(name):
    require(type(name) is str and 0 < len(name.encode()) <= 4096, 'source path')
    value = name.split('/')
    require(len(value) <= 32 and all(re.fullmatch('[A-Za-z0-9_.-]{1,128}', p)
            and p not in ('.', '..') for p in value), 'source path components')
    return value


def directory(root_fd, name):
    fd = os.dup(root_fd)
    try:
        for part in parts(name):
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=fd)
            os.close(fd)
            fd = child
        return fd
    except BaseException:
        os.close(fd)
        raise


def read(root_fd, name):
    components = parts(name)
    parent = directory(root_fd, '/'.join(components[:-1])) if len(components) > 1 else os.dup(root_fd)
    try:
        fd = os.open(components[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                     dir_fd=parent)
        try:
            identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns,
                                  s.st_mode, s.st_nlink)
            before = os.fstat(fd)
            require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                    and 0 < before.st_size <= CAP, 'bounded regular source: ' + name)
            blocks = bytearray()
            while len(blocks) < before.st_size:
                block = os.read(fd, min(65536, before.st_size - len(blocks)))
                require(bool(block), 'truncated source')
                blocks.extend(block)
            require(not os.read(fd, 1) and identity(before) == identity(os.fstat(fd)), 'changed source')
            return bytes(blocks)
        finally:
            os.close(fd)
    finally:
        os.close(parent)


def source_names(root_fd):
    names = set(EXTRAS)
    visited_entries = 0
    def add(name):
        parts(name)
        require(name in names or len(names) < COUNT, 'source count before insertion')
        names.add(name)
    def walk(name, suffix=None, depth=0):
        nonlocal visited_entries
        require(depth <= 16, 'source enumeration depth')
        fd = directory(root_fd, name)
        try:
            entries = []
            with os.scandir(fd) as stream:
                for entry in stream:
                    require(len(entries) < COUNT and visited_entries < 16384,
                            'source enumeration bound before growth')
                    visited_entries += 1
                    entries.append(entry.name)
            for entry in sorted(entries):
                child = name + '/' + entry
                parts(child)
                info = os.stat(entry, dir_fd=fd, follow_symlinks=False)
                if stat.S_ISDIR(info.st_mode):
                    walk(child, suffix, depth + 1)
                else:
                    require(stat.S_ISREG(info.st_mode), 'nonregular compilation input: ' + child)
                    if suffix is None or child.endswith(suffix):
                        add(child)
        finally:
            os.close(fd)
    workspace = tomllib.loads(read(root_fd, 'Cargo.toml').decode('utf8', 'strict'))
    members = workspace['workspace']['members']
    require(type(members) is list and 1 <= len(members) <= 256, 'workspace members')
    for member in members:
        parts(member)
        add(member + '/Cargo.toml')
    pending, seen = set(ROOTS), set()
    while pending:
        crate = min(pending)
        pending.remove(crate)
        require(len(seen) < 256, 'path dependency bound')
        seen.add(crate)
        name = crate + '/Cargo.toml'
        add(name)
        package = tomllib.loads(read(root_fd, name).decode('utf8', 'strict'))
        def dependencies(table, base):
            for key, value in table.items():
                if key in ('dependencies', 'dev-dependencies', 'build-dependencies', 'patch', 'replace'):
                    def find_paths(node):
                        if type(node) is dict:
                            if 'path' in node:
                                require(type(node['path']) is str and not Path(node['path']).is_absolute(),
                                        'relative workspace path dependency')
                                target = os.path.normpath(base + '/' + node['path'])
                                parts(target)
                                if target not in seen:
                                    pending.add(target)
                            for child in node.values():
                                find_paths(child)
                    find_paths(value)
                elif type(value) is dict:
                    dependencies(value, base)
        dependencies(package, crate)
        dependencies(workspace, '.')
        for folder in ('src', 'tests', 'examples'):
            try:
                walk(crate + '/' + folder)
            except FileNotFoundError:
                # Only an absent optional directory is accepted; a broken leaf
                # or include is rejected when captured below.
                check = directory(root_fd, crate)
                try:
                    require(folder not in os.listdir(check), 'missing source inside existing directory')
                finally:
                    os.close(check)
        if package.get('package', {}).get('build') is False:
            pass
        else:
            build = package.get('package', {}).get('build', 'build.rs')
            require(type(build) is str, 'build script path')
            candidate = os.path.normpath(crate + '/' + build)
            try:
                read(root_fd, candidate)
            except FileNotFoundError:
                require(build == 'build.rs', 'declared build script absent')
            else:
                add(candidate)
    walk('idl', '.toml')
    scanned = set()
    while names - scanned:
        name = min(names - scanned)
        scanned.add(name)
        if not name.endswith('.rs'):
            continue
        source = read(root_fd, name).decode('utf8', 'strict')
        includes = list(re.finditer(r'\binclude(?:_bytes|_str)?!\s*\(\s*"([^"\n]+)"\s*\)', source))
        require(len(includes) == len(re.findall(r'\binclude(?:_bytes|_str)?!\s*\(', source)),
                'nonliteral include requires explicit closure review: ' + name)
        for include in includes:
            add(os.path.normpath(str(Path(name).parent / include.group(1))))
        for module in re.finditer(r'#\s*\[\s*path\s*=\s*"([^"\n]+)"\s*\]', source):
            add(os.path.normpath(str(Path(name).parent / module.group(1))))
    return sorted(names)


def capture(root_fd, expected):
    names = source_names(root_fd)
    require(names == expected, 'actual compilation closure differs from reviewed registry')
    raw_sources = {}
    source_bytes = 0
    for name in names:
        raw = read(root_fd, name)
        require(source_bytes + len(raw) <= 67108864, 'source snapshot total64MiB')
        source_bytes += len(raw)
        raw_sources[name] = raw
    rows = [dict(relative_path=name, byte_len=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            for name, raw in sorted(raw_sources.items())]
    require(source_names(root_fd) == names, 'compilation source set changed while captured')
    for name, raw in raw_sources.items():
        require(read(root_fd, name) == raw, 'source changed while captured')
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--review-registry', action='store_true')
    parser.add_argument('--registry-sha256')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    require(args.repo.is_absolute(), 'absolute repository')
    anchor = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        fd = directory(anchor, str(args.repo)[1:])
    finally:
        os.close(anchor)
    try:
        if args.review_registry:
            require(args.output is None and args.registry_sha256 is None, 'review only')
            print(json.dumps(dict(schema_version=1, paths=source_names(fd)), indent=2))
            return
        require(args.output is not None and args.output.is_absolute(), 'absolute new output')
        registry_raw = read(fd, REGISTRY)
        require(re.fullmatch('[0-9a-f]{64}', args.registry_sha256 or '')
                and hashlib.sha256(registry_raw).hexdigest() == args.registry_sha256, 'reviewed registry pin')
        registry = json.loads(registry_raw)
        require(type(registry) is dict and set(registry) == {'schema_version', 'paths'}
                and registry['schema_version'] == 1, 'registry shape')
        rows = capture(fd, registry['paths'])
        raw = json.dumps(rows, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()
        require(len(raw) <= 2097152, 'source manifest bound')
        anchor = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        try:
            parent = directory(anchor, str(args.output.parent)[1:])
        finally:
            os.close(anchor)
        try:
            out = os.open(args.output.name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                          0o600, dir_fd=parent)
            with os.fdopen(out, 'wb') as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
        finally:
            os.close(parent)
        print(hashlib.sha256(raw).hexdigest())
    finally:
        os.close(fd)


if __name__ == '__main__':
    main()
