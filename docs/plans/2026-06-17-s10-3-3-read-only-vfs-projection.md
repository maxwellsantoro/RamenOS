# S10.3.3: Read-Only VFS Projection

**Last Updated:** 2026-06-17
**Status:** Complete (2026-06-17)
**Related:** `docs/plans/2026-02-20-s10-3-projection-storage.md`, `docs/COMPAT_CAPSULE_V0.md`

## Executive Summary

S10.3.3 exposes projection index paths as a read-only POSIX directory tree for compat domains. The host materializes `ProjectionIndexV0.path_projections` into a directory of symlinks to CAS blobs; compat capsules mount that tree via **QEMU virtio-9p** (`-virtfs`).

virtio-fs is deferred until measured I/O latency or multi-VM sharing requirements justify adding `virtiofsd` to the supervisor TCB.

## Decision

Choose **QEMU virtio-9p (`-virtfs local,readonly=on`)** for S10.3.3 compat VFS transport.

Rejected for v0: **virtio-fs + virtiofsd**.

## Why This Choice

| Criterion | virtio-9p (`-virtfs`) | virtio-fs |
|-----------|----------------------|-----------|
| QEMU integration | Single `-virtfs` flag on existing compat runner | Requires `virtiofsd` daemon + socket setup |
| Read-only enforcement | `readonly=on` at export | Possible but more moving parts |
| Guest support | `9p2000.L` in standard compat kernels | Needs `virtiofs` driver + daemon version match |
| Foundry gateability | Host materialization testable without daemon lifecycle | Extra process management in gates |
| CoW path (S10.3.4) | Host rewrite + remount/rematerialize is straightforward | Same, but heavier bootstrap |

## Scope

### In Scope

- `projection_vfs::materialize_read_only(store_root, mount_root)` — build symlink tree from `projection_index.json`.
- Path safety: reject `..` and mount-root escapes when mapping `virtual_path` → host path.
- `compat_runner` optional `projection_vfs` mount: `-virtfs local,path=...,mount_tag=ramen_store,readonly=on`.
- Foundry gate: materialize after ingest → read projected path bytes on host.

### Out of Scope

- virtio-fs / virtiofsd daemon.
- CoW writes (S10.3.4).
- Full QEMU compat-domain read gate (requires 9p-enabled initrd/kernel; tracked as follow-up).
- Custom in-process 9p server (QEMU exports materialized directory).

## Mount Boundary

```text
CAS blobs          projection_index.json
     \                    /
      \                  /
       v                v
   materialize_read_only → {mount_root}/store/...  (symlinks → *.blob)
                |
                v
   QEMU -virtfs local,readonly=on,mount_tag=ramen_store
                |
                v
   Guest: mount -t 9p -o trans=virtio,version=9p2000.L ramen_store /store
```

Virtual paths from S10.3.2 ingest (`/store/{kind}/{channel}/{filename}`) map 1:1 under the guest `/store` mount when `mount_root` is materialized with the leading `/store` segment preserved.

## Foundry Gate

Extend `foundry_projection_storage_s10_3.sh`:

```text
FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=read_only_vfs_projection
```

Assertions (`cargo test -p store_service read_only_vfs_projection`):

- Ingest artifact → projection index updated (S10.3.2).
- `materialize_read_only` creates symlink at projected path.
- Reading through the symlink returns the original blob bytes.
- Write to materialized path fails (read-only host tree; optional negative).

## Success Criteria

- Gate step passes.
- `compat_runner` accepts optional `projection_vfs` in capsule config and emits `-virtfs` when set.
- No virtio-fs dependency.

## Revisit Criteria

Reopen virtio-fs when:

- 9p export latency fails the <10 ms path-open gate at 10k entries in QEMU, or
- multiple concurrent compat capsules need shared cache-coherent host export without rematerializing per VM.
