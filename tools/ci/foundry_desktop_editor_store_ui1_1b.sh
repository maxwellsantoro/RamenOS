#!/usr/bin/env bash
# Default-off host CAS and evidence gate; hardware and editor processes are separate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
if [[ $# != 0 && $# != 2 ]]; then
  echo 'usage: foundry_desktop_editor_store_ui1_1b.sh [source-manifest source-manifest-sha256]' >&2
  exit 2
fi
mkdir -p out/desktop
STORE_RUN="$(mktemp -d "$PWD/out/desktop/ui1-1b.XXXXXXXX")"
if [[ $# == 2 ]]; then
  STORE_MANIFEST="$1"
  STORE_MANIFEST_SHA="$2"
else
  STORE_MANIFEST="$STORE_RUN/source-manifest.json"
  # This reviewed exact path registry selects the source closure independently
  # of test artifacts. Freeze actual bytes before any build or test execution.
  STORE_MANIFEST_SHA="$(python3 - "$PWD" "$STORE_MANIFEST" <<'PY'
import hashlib, json, os, stat, sys
repo, destination = sys.argv[1:]
root = os.open(repo, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
try:
    registry_parent = os.dup(root)
    try:
        for part in ('tools', 'ci'):
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=registry_parent)
            os.close(registry_parent)
            registry_parent = child
        fd = os.open('editor_store_sources_v0.json', os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=registry_parent)
        try:
            before = os.fstat(fd)
            if not stat.S_ISREG(before.st_mode) or not 1 <= before.st_size <= 65536:
                raise ValueError('source registry file shape or bound')
            registry_raw = bytearray()
            while True:
                block = os.read(fd, min(4096, 65537 - len(registry_raw)))
                if not block:
                    break
                if len(registry_raw) + len(block) > 65536:
                    raise ValueError('source registry grew beyond bound')
                registry_raw.extend(block)
            after = os.fstat(fd)
            identity = lambda info: (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
            if identity(before) != identity(after) or len(registry_raw) != before.st_size:
                raise ValueError('source registry changed while reading')
            if hashlib.sha256(registry_raw).hexdigest() != '274599ee5b6eb2c5c86bbe2d315e4bef59cdaca2dc41379d2e109c73149ad2bc':
                raise ValueError('reviewed exact source registry hash mismatch')
        finally:
            os.close(fd)
    finally:
        os.close(registry_parent)
    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate source registry key')
            result[key] = value
        return result
    registry = json.loads(bytes(registry_raw).decode('utf8'), object_pairs_hook=unique_pairs)
    if set(registry) != {'schema_version', 'paths'} or type(registry['schema_version']) is not int or registry['schema_version'] != 1:
        raise ValueError('source registry shape')
    paths = registry['paths']
    if type(paths) is not list or not 1 <= len(paths) <= 1024 or paths != sorted(set(paths)):
        raise ValueError('source registry inventory')
    records = []
    for path in paths:
        if type(path) is not str or not path or len(path) > 256 or path.startswith('/'):
            raise ValueError('source relative path')
        parts = path.split('/')
        if any(part in ('', '.', '..') for part in parts):
            raise ValueError('source path component')
        parent = os.dup(root)
        try:
            for part in parts[:-1]:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
                os.close(parent)
                parent = child
            fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
            try:
                before = os.fstat(fd)
                if not stat.S_ISREG(before.st_mode) or not 1 <= before.st_size <= 8388608:
                    raise ValueError('source file shape or bound')
                digest = hashlib.sha256()
                consumed = 0
                while True:
                    block = os.read(fd, min(65536, before.st_size + 1 - consumed))
                    if not block:
                        break
                    consumed += len(block)
                    if consumed > before.st_size:
                        raise ValueError('source grew while freezing')
                    digest.update(block)
                after = os.fstat(fd)
                identity = lambda info: (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
                if identity(before) != identity(after) or consumed != before.st_size:
                    raise ValueError('source changed while freezing')
                record = {'relative_path': path, 'byte_len': consumed, 'sha256': digest.hexdigest()}
                if len(json.dumps(record, separators=(',', ':')).encode('utf8')) > 512:
                    raise ValueError('source record bound')
                records.append(record)
            finally:
                os.close(fd)
        finally:
            os.close(parent)
    raw = json.dumps(records, ensure_ascii=False, separators=(',', ':')).encode('utf8')
    if len(raw) > 524288:
        raise ValueError('source manifest bound')
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, 'wb', closefd=False) as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(fd)
    finally:
        os.close(fd)
    print(hashlib.sha256(raw).hexdigest())
finally:
    os.close(root)
PY
)"
fi
python3 tools/ci/editor_store_runner.py \
  --repo "$PWD" --source-manifest "$STORE_MANIFEST" \
  --source-manifest-sha256 "$STORE_MANIFEST_SHA" \
  --target-dir "$PWD/out/desktop/ui1-1b-target" --evidence "$STORE_RUN/evidence"
echo "FOUNDRY_DESKTOP_EDITOR_STORE_UI1_1B: PASS scope=host-cas evidence=$STORE_RUN/evidence target=false device=false containment=false"
