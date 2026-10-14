"""Pinned compilation-only adapter. Fresh evidence and assertions are mandatory."""
import hashlib
from pathlib import Path
import re
import types

CACHE_SOURCE = 'tools/ci/build_cache.py'
CACHE_SHA = '014ba0d352578b155565cd0d232fef30771e1aaf9e8ecfdf18b595820f6e37f2'


def require(value, reason):
    if not value: raise ValueError(reason)


def settings(environment):
    enabled = environment.get('RAMEN_FOUNDRY_BUILD_CACHE', '0')
    jobs = environment.get('CARGO_BUILD_JOBS', '2')
    require(enabled in ('0', '1'), 'compiler reuse opt-in must be 0 or 1')
    require(type(jobs) is str and re.fullmatch('[1-9][0-9]?', jobs) is not None
            and int(jobs) <= 32, 'build jobs must be canonical 1..32')
    return enabled == '1', jobs


def load_cache(h, repo_fd, rows):
    raw, _, sha = h.source_read(repo_fd, CACHE_SOURCE, 65536, True)
    require(sha == CACHE_SHA and hashlib.sha256(raw).hexdigest() == CACHE_SHA,
            'independently reviewed compiler cache helper pin')
    require(any(row['relative_path'] == CACHE_SOURCE and row['sha256'] == sha for row in rows),
            'compiler helper absent from frozen source closure')
    module = types.ModuleType('acceptance_compiler_cache')
    module.__file__ = CACHE_SOURCE
    exec(compile(raw, CACHE_SOURCE, 'exec'), module.__dict__)
    return module


class Compilation:
    def __init__(self, h, repo_fd, rows, repo, manifest_sha, commands, rustc, cargo, fresh_target, environment):
        enabled, jobs = settings(environment)
        commands.env['CARGO_BUILD_JOBS'] = jobs
        self.commands = commands
        self.fresh_target = Path(fresh_target)
        self.targets = {}; self.closed = False; self.finished = False
        self.cache = None
        if enabled:
            helper = load_cache(h, repo_fd, rows)
            self.cache = helper.GateCache(repo, manifest_sha, rustc, cargo, commands.env)

    def phase(self, name, features):
        require(not self.closed and not self.finished, 'compiler ownership already closed or completed')
        require(type(name) is str and re.fullmatch('[a-zA-Z0-9_-]{1,64}', name), 'bounded compiler phase')
        require(type(features) is list and len(features) <= 16
                and all(type(f) is str and re.fullmatch('[a-zA-Z0-9_-]{1,64}', f) for f in features)
                and len(features) == len(set(features)),
                'bounded unique compiler features')
        identity = tuple(sorted(features))
        if name in self.targets:
            old_features, target = self.targets[name]
            require(old_features == identity, 'compiler phase features changed')
        else:
            target = self.cache.target_for(name, list(identity)) if self.cache is not None else self.fresh_target
            self.targets[name] = (identity, target)
        self.commands.env['CARGO_TARGET_DIR'] = str(target)
        return target

    def metadata(self):
        return {'enabled': self.cache is not None, 'cached_acceptance': False,
                'phases': [] if self.cache is None else self.cache.records,
                'build_jobs': self.commands.env['CARGO_BUILD_JOBS']}

    def finish(self):
        require(not self.closed and not self.finished, 'compiler ownership already closed or completed')
        if self.cache is not None: self.cache.check_contexts()
        self.finished = True

    def close(self, accepted=False):
        if not self.closed:
            self.closed = True
            if self.cache is not None:
                self.cache.completed = bool(accepted and self.finished)
                self.cache.close()
