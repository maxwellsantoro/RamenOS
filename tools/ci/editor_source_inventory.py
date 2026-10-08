"""Conservative NativeRead/Preview dependency admission using the task collector.

Reuse the unchanged, bounded collector with its original roots and extras. This
includes the native app and Save prerequisites in addition to Read/Preview; it
is deliberately not a claim about Cargo's exact active-feature unit graph.
"""
import os
from pathlib import PurePosixPath
import tomllib
import types

COLLECTOR = 'tools/ci/native_save_task_sources.py'
COLLECTOR_SHA = '6323de423cd8476a6c0bca92119587742a5e7a010be3e669706643c2e7434b63'
SOURCE = 'tools/ci/editor_source_inventory.py'
TEST = 'tools/ci/test_editor_source_inventory.py'


def compiled_manifests(h, module, repo_fd):
    """Follow the collector's bounded path-dependency roots, excluding metadata-only members."""
    workspace = tomllib.loads(module.read(repo_fd, 'Cargo.toml').decode('utf8', 'strict'))
    pending, manifests = set(module.ROOTS), {}
    def dependencies(table, base):
        for key, value in table.items():
            if key in ('dependencies', 'dev-dependencies', 'build-dependencies', 'patch', 'replace'):
                def find_paths(node):
                    if type(node) is dict:
                        if 'path' in node:
                            path = node['path']
                            h.require(type(path) is str and not PurePosixPath(path).is_absolute(),
                                      'relative path dependency')
                            selected = os.path.normpath(base + '/' + path)
                            module.parts(selected)
                            if selected not in manifests:
                                h.require(len(pending) < 256, 'reachable manifest count')
                                pending.add(selected)
                        for child in node.values():
                            find_paths(child)
                find_paths(value)
            elif type(value) is dict:
                dependencies(value, base)
    while pending:
        crate = min(pending)
        pending.remove(crate)
        h.require(len(manifests) < 256, 'reachable manifest count')
        manifest = tomllib.loads(module.read(repo_fd, crate + '/Cargo.toml').decode('utf8', 'strict'))
        manifests[crate] = manifest
        dependencies(manifest, crate)
        dependencies(workspace, '.')
    return manifests


def validate(h, repo_fd, paths):
    """Reject an unreviewed dependency/include before compilation or cache lookup."""
    h.require({COLLECTOR, SOURCE, TEST} <= set(paths), 'source admission helpers omitted')
    raw, _, sha = h.source_read(repo_fd, COLLECTOR, 65536, True)
    h.require(sha == COLLECTOR_SHA, 'reviewed transitive source collector pin')
    module = types.ModuleType('native_gate_source_collector')
    module.__file__ = COLLECTOR
    exec(compile(raw, COLLECTOR, 'exec'), module.__dict__)
    names = module.source_names(repo_fd)
    collected = set(names)
    # The reused collector walks src/tests/examples and literal includes. Cargo
    # permits explicit targets elsewhere; reject those layouts unless the actual
    # target is already collected, rather than recording only its manifest.
    for crate, manifest in compiled_manifests(h, module, repo_fd).items():
        h.require(crate + '/Cargo.toml' in collected, 'reachable manifest absent from collector')
        for kind in ('lib', 'bin', 'test', 'bench', 'example'):
            targets = [manifest[kind]] if kind == 'lib' and kind in manifest else manifest.get(kind, [])
            h.require(type(targets) is list and len(targets) <= 256, 'bounded Cargo target table')
            for target in targets:
                h.require(type(target) is dict, 'Cargo target table')
                if 'path' in target:
                    path = target['path']
                    h.require(type(path) is str and 0 < len(path.encode()) <= 4096
                              and not PurePosixPath(path).is_absolute(), 'relative Cargo target path')
                    selected = os.path.normpath(str(PurePosixPath(crate) / path))
                    h.require(selected in collected,
                              'explicit Cargo target outside collected source layout: ' + selected)
    h.require(collected <= set(paths), 'transitive compilation input absent from reviewed registry')
    return names
