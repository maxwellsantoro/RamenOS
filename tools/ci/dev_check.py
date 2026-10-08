#!/usr/bin/env python3
"""Warm, bounded host development checks. Never a Foundry acceptance result."""
import argparse
import contextlib
import signal
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import selectors
import subprocess
import sys
import time
import tomllib

HERE = Path(__file__).absolute().parent
REPO = HERE.parents[1]
# Developer CLI executes reviewed checkout code, unlike pinned Foundry adapters.
spec = importlib.util.spec_from_file_location('development_build_cache', HERE / 'build_cache.py')
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)
HOST_EXCLUDED = {'kernel_uefi', 'kernel_aarch64'}


class DevelopmentDenied(ValueError):
    def __init__(self, reason, observation=None):
        super().__init__(reason); self.observation = observation


class DevelopmentInterrupted(DevelopmentDenied):
    def __init__(self, signum, observation=None):
        super().__init__('development interrupted by signal ' + str(signum), observation)
        self.signum = signum
        self.exit_code = 128 + signum


@contextlib.contextmanager
def interruption_handlers():
    def interrupted(signum, frame): raise DevelopmentInterrupted(signum)
    previous = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGTERM, signal.SIGINT)}
    try: yield
    finally:
        for sig, handler in previous.items(): signal.signal(sig, handler)


def require(value, reason):
    if not value: raise DevelopmentDenied(reason)


def identifier(value):
    require(type(value) is str and re.fullmatch('[A-Za-z0-9_-]{1,64}', value), 'bounded identifier')
    return value


def select_scope(repo, package, test, features):
    identifier(package); identifier(test)
    require(package not in HOST_EXCLUDED, 'package requires explicit architecture target, not host dev-check')
    require(type(features) is list and len(features) <= 16
            and all(type(f) is str for f in features) and len(features) == len(set(features)), 'feature inventory')
    for f in features: identifier(f)
    repo = Path(repo)
    workspace = tomllib.loads(cache.read_file(repo / 'Cargo.toml', required=True).decode('utf8'))
    members = workspace['workspace']['members']
    require(type(members) is list and 1 <= len(members) <= 64, 'bounded workspace membership')
    found = []
    for member in members:
        require(type(member) is str and re.fullmatch('[A-Za-z0-9_/-]{1,256}', member)
                and not member.startswith('/') and all(p not in ('', '.', '..') for p in member.split('/')), 'workspace member path')
        root = repo / member
        manifest = tomllib.loads(cache.read_file(root / 'Cargo.toml', required=True).decode('utf8'))
        if manifest['package']['name'] == package: found.append((root, manifest))
    require(len(found) == 1, 'unsupported or ambiguous workspace package')
    root, manifest = found[0]
    declared = manifest.get('features', {})
    require(set(features) <= set(declared), 'unknown development feature')
    activated = set(features)
    for _ in range(64):
        previous = len(activated)
        for f in tuple(activated):
            activated.update(dep for dep in declared.get(f, []) if dep in declared)
        require(len(activated) <= 64, 'feature closure bound')
        if len(activated) == previous: break
    if test == 'lib':
        path = root / manifest.get('lib', {}).get('path', 'src/lib.rs')
    else:
        explicit = [row for row in manifest.get('test', []) if row.get('name') == test]
        require(len(explicit) <= 1, 'ambiguous test scope')
        if explicit:
            row = explicit[0]; path = root / row.get('path', 'tests/' + test + '.rs')
            require(set(row.get('required-features', [])) <= activated, 'missing required feature')
        else:
            require(manifest.get('package', {}).get('autotests', True), 'autotests disabled')
            path = root / 'tests' / (test + '.rs')
    require(path.is_relative_to(root) and '..' not in path.parts, 'test path outside package')
    require(cache.file_digest(path, required=True) is not None, 'actual selected target')
    return {'package': package, 'test': test, 'features': sorted(features),
            'default_features': False, 'label': 'development_not_acceptance'}


def commands(scope, channel):
    require(re.fullmatch('nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}', channel), 'pinned toolchain')
    common = ['--locked', '--no-default-features', '-p', scope['package']]
    common += ['--lib'] if scope['test'] == 'lib' else ['--test', scope['test']]
    if scope['features']: common += ['--features', ','.join(scope['features'])]
    cargo = ['cargo', '+' + channel]
    return [cargo + ['test'] + common + ['--no-run'],
            cargo + ['clippy'] + common + ['--', '-D', 'warnings'],
            cargo + ['test'] + common + ['--', '--test-threads=1']]


def sanitized_environment(environment, target):
    e = dict(environment)
    exact = {'RUSTUP_TOOLCHAIN', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
             'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOC', 'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_TARGET_DIR'}
    for k in tuple(e):
        if k in exact or k.startswith(('RAMEN_DESKTOP_', 'CARGO_BUILD_', 'CARGO_PROFILE_', 'CARGO_TARGET_')): e.pop(k)
    e['CARGO_TARGET_DIR'] = str(target)
    return e


def run_command(argv, cwd, env, timeout=600, output_cap=1048576):
    require(type(argv) is list and 1 <= len(argv) <= 32
            and all(type(v) is str and 0 < len(v.encode()) <= 4096 and '\x00' not in v for v in argv), 'bounded argv')
    require(type(timeout) in (int, float) and 0 < timeout <= 600, 'finite command deadline')
    require(type(output_cap) is int and 1 <= output_cap <= 1048576, 'output byte cap')
    started = time.monotonic(); deadline = started + timeout
    reserve = min(2.0, timeout / 4)
    child = None; held = False; out = {'stdout': bytearray(), 'stderr': bytearray()}; fault = None
    poll = selectors.DefaultSelector(); interrupted = None
    try:
        child = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        held = True
        for name in out:
            pipe = getattr(child, name); os.set_blocking(pipe.fileno(), False); poll.register(pipe, selectors.EVENT_READ, name)
        while poll.get_map():
            require(time.monotonic() < deadline - reserve, 'development command deadline')
            for key, _ in poll.select(min(0.05, max(0, deadline - reserve - time.monotonic()))):
                chunk = os.read(key.fileobj.fileno(), min(65536, output_cap + 1))
                if not chunk: poll.unregister(key.fileobj); continue
                require(sum(len(x) for x in out.values()) + len(chunk) <= output_cap, 'development output cap')
                out[key.data].extend(chunk)
        status = child.wait(timeout=max(0.001, deadline - time.monotonic())); held = False
        require(time.monotonic() < deadline, 'development final deadline')
    except (DevelopmentDenied, OSError, subprocess.SubprocessError, KeyboardInterrupt) as e:
        fault = str(e)
        if isinstance(e, DevelopmentInterrupted): interrupted = e.signum
        elif isinstance(e, KeyboardInterrupt): interrupted = signal.SIGINT
    finally:
        if child is not None and held:
            # The session ID comes from our still-unreaped Popen, never an artifact.
            # Kill only this owned command group BEFORE clearing parent authority.
            try:
                os.killpg(child.pid, signal.SIGTERM)
                time.sleep(min(0.1, max(0, deadline - time.monotonic()) / 4))
                try: os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError: pass
            except ProcessLookupError: pass
            except OSError as e: fault = (fault or '') + '; group cleanup uncertain: ' + str(e)
            try: child.wait(timeout=max(0.001, deadline - time.monotonic())); held = False
            except subprocess.TimeoutExpired: fault = (fault or '') + '; owned reap uncertain'
        poll.close()
        if child is not None:
            for pipe in (child.stdout, child.stderr):
                if pipe is not None: pipe.close()
    record = {'argv': argv, 'actual_pid': child.pid if child else None,
              'exit_code': child.returncode if child else None, 'reaped': child is not None and not held,
              'elapsed_seconds': time.monotonic() - started, 'fault': fault,
              'stdout': bytes(out['stdout'][:16384]).decode('utf8', 'replace'),
              'stderr': bytes(out['stderr'][:16384]).decode('utf8', 'replace'),
              'stdout_bytes': len(out['stdout']), 'stderr_bytes': len(out['stderr']),
              'display_prefix_limit': 16384,
              'display_truncated': any(len(x) > 16384 for x in out.values()),
              'stdout_sha256': hashlib.sha256(out['stdout']).hexdigest(),
              'stderr_sha256': hashlib.sha256(out['stderr']).hexdigest()}
    if interrupted is not None: raise DevelopmentInterrupted(interrupted, record)
    if fault is not None: raise DevelopmentDenied(fault, record)
    return record


def run_sequence(argvs, cwd, env, timeout=600):
    require(type(argvs) is list and 1 <= len(argvs) <= 5, 'development command count')
    result = {'schema_version': 1, 'label': 'development_not_acceptance', 'commands': [], 'exit_code': 0, 'acceptance': False}
    for argv in argvs:
        try: record = run_command(argv, cwd, env, timeout)
        except DevelopmentDenied as e:
            if e.observation is not None: result['commands'].append(e.observation)
            result['exit_code'] = getattr(e, 'exit_code', 1); result['fault'] = str(e); break
        result['commands'].append(record)
        if record['exit_code'] != 0:
            result['exit_code'] = record['exit_code'] if record['exit_code'] > 0 else 1
            break
    return result


def publish(directory, result):
    raw = (json.dumps(result, sort_keys=True, ensure_ascii=True) + '\n').encode()
    require(len(raw) <= 8388608, 'development result bound')
    fd = os.open(directory / 'result.json', os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        offset = 0
        while offset < len(raw):
            n = os.write(fd, raw[offset:]); require(n > 0, 'development record write'); offset += n
        os.fsync(fd)
    finally: os.close(fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', required=True)
    parser.add_argument('--test', default='lib')
    parser.add_argument('--features', default='')
    args = parser.parse_args()
    features = args.features.split(',') if args.features else []
    scope = select_scope(REPO, args.package, args.test, features)
    channel = tomllib.loads((REPO / 'rust-toolchain.toml').read_text())['toolchain']['channel']
    argvs = commands(scope, channel)
    parent = REPO / 'out/ci-optimization'
    # These private roots are separate from formal acceptance cache/evidence.
    parent.mkdir(parents=True, exist_ok=True)
    run = parent / ('dev-check-' + secrets.token_hex(16)); os.mkdir(run, 0o700)
    roots = parent / 'development-cache'
    env = sanitized_environment(os.environ, run / 'unused-profile-target')
    result = {'schema_version': 1, 'label': 'development_not_acceptance', 'acceptance': False, 'scope': scope}
    try:
        profiles = run_sequence([['rustc', '+' + channel, '-vV'], ['cargo', '+' + channel, '--version']], REPO, env)
        result['profile_commands'] = profiles['commands']
        if profiles['exit_code']:
            result['exit_code'] = profiles['exit_code']; return profiles['exit_code']
        # Development identity intentionally persists across edits. Cargo is
        # actually rerun on the live checkout; this digest is NOT a frozen source
        # manifest and is never admitted by a Foundry adapter.
        scope_sha = hashlib.sha256(cache.compact(scope)).hexdigest()
        inputs = cache.key_inputs(REPO, scope_sha, 'development-' + scope['package'], scope['features'],
                                  profiles['commands'][0]['stdout'], profiles['commands'][1]['stdout'], env, scope='development')
        with cache.CacheLease(roots, inputs) as lease:
            env = sanitized_environment(os.environ, lease.target)
            result.update(run_sequence(argvs, REPO, env)); result['scope'] = scope
            result['cache'] = lease.metadata
            # Known normal compiler/test failure is safe to retry after an edit.
            # Signals/faults/uncertain cleanup poison the target instead.
            require(bool(result['commands']) and all(c['reaped'] is True and c['fault'] is None
                    and type(c['exit_code']) is int and c['exit_code'] >= 0
                    for c in result['commands']), 'development cleanup incomplete or interrupted')
            final = cache.key_inputs(REPO, scope_sha, inputs['phase'], scope['features'], inputs['toolchain']['rustc'], inputs['toolchain']['cargo'], env, scope='development')
            require(cache.key_for(final) == cache.key_for(inputs), 'compiler config/lock changed during development run')
        return result['exit_code']
    except (DevelopmentDenied, cache.CacheDenied, OSError, ValueError) as e:
        result['fault'] = str(e)
        code = getattr(e, 'exit_code', result.get('exit_code') or 1)
        result['exit_code'] = code; return code
    finally:
        publish(run, result)
        print('DEVELOPMENT_NOT_ACCEPTANCE evidence=' + str(run))


if __name__ == '__main__':
    try:
        with interruption_handlers(): sys.exit(main())
    except (DevelopmentDenied, cache.CacheDenied, OSError, ValueError) as error:
        print('DEVELOPMENT_NOT_ACCEPTANCE: ' + str(error), file=sys.stderr)
        sys.exit(getattr(error, 'exit_code', 1))
