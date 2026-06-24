# S10.3.4: Copy-on-Write Projection Writes

**Last Updated:** 2026-06-17
**Status:** Complete (2026-06-17)
**Related:** `docs/plans/2026-02-20-s10-3-projection-storage.md`, `docs/plans/2026-06-17-s10-3-3-read-only-vfs-projection.md`

## Executive Summary

S10.3.4 adds **typed Store/Projection copy-on-write** for virtual paths. Writers buffer replacement bytes in a scratch/write-intent path on the host; on `commit`, `store_service` ingests a fresh CAS blob and repoints the projection index. Prior blobs are never mutated.

The S10.3.3 virtio-9p export stays **`readonly=on`**. Compat guest writes do not target a mutable 9p tree in v0.

## Decision

v0 CoW is a **host-side `commit_projection_write` operation** on `store_service`, not a writable 9p projection.

Rejected for v0: **mutable virtio-9p export** as the first write path.

## Why This Choice

| Criterion | Typed commit (scratch → CAS → index) | Writable 9p export |
|-----------|--------------------------------------|--------------------|
| TCB / semantics | Reuses ingest + projection persist paths | Guest write errors, partial writes, cache coherency |
| Fail-closed | Capability + validation at commit boundary | Harder to enforce at POSIX layer |
| Gateability | Deterministic host integration test | Needs guest kernel + 9p write semantics |
| S10.3.3 boundary | Read-only VFS unchanged | Collapses 10.3.3 and 10.3.4 concerns |

Compat domains reach CoW later via a broker/supervisor that holds scratch state and calls commit — not by flipping `readonly=on`.

## Scope

### In Scope

- `projection_cow::commit_projection_write` — single-shot commit API for v0 (scratch bytes in memory).
- On commit:
  1. Resolve `virtual_path` → `prior_content_id` (fail if not projected).
  2. Ingest `replacement_bytes` as new blob + manifest (`kind`/`channel` from commit or prior entry).
  3. Upsert semantic index entry for **new** `content_id` (keep prior entry).
  4. Repoint `path_projections[virtual_path]` to new `content_id`.
  5. `persist_atomic` projection index.
- Prior CAS blob/manifest untouched; `get_blob(prior_content_id)` still works.
- Foundry gate: `projection_cow_commit_repoints_path_preserves_prior_blob`.

### Out of Scope

- Writable 9p / virtio-fs guest writes.
- Multi-step scratch lease API (open/write/close) — design hook only; v0 uses one commit call.
- `semantic_store_v1` IDL message for commit (optional follow-up after host gate passes).
- CoW via compat `write()` syscall interception.

## v0 API Sketch

```rust
pub struct ProjectionWriteCommit<'a> {
    pub virtual_path: &'a str,
    pub replacement_bytes: &'a [u8],
    pub kind: &'a str,
    pub channel: &'a str,
    pub domain_id: u64,
}

pub struct ProjectionWriteCommitResult {
    pub virtual_path: String,
    pub prior_content_id: String,
    pub new_content_id: String,
}

pub fn commit_projection_write(
    store_root: &Path,
    projection_index: &mut ProjectionIndexStore,
    commit: &ProjectionWriteCommit<'_>,
) -> Result<ProjectionWriteCommitResult, ProjectionCowError>;
```

Future (post-v0): `open_write_intent(virtual_path) → WriteIntentHandle`, buffer in `{store_root}/.projection_scratch/{handle}`, `commit_write_intent(handle)` calls the same commit core.

## Commit Flow

```text
query_by_path(virtual_path) → prior_content_id
        |
        v
replacement_bytes ──► hash ──► new blob + manifest (CAS)
        |
        v
upsert_entry(new) + upsert_path_projection(repoint)
        |
        v
persist_atomic(projection_index.json + CAS snapshot)
        |
        v
query_by_path(virtual_path) → new_content_id
get_blob(prior_content_id)  → unchanged bytes
```

## Foundry Gate

Extend `tools/ci/foundry_projection_storage_s10_3.sh`:

```text
FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=projection_cow_commit
```

Test `projection_cow_commit_repoints_path_preserves_prior_blob`:

1. Seed store via ingest path → `virtual_path` maps to `prior_content_id`; read returns `OLD_BYTES`.
2. `commit_projection_write` with `NEW_BYTES` for same `virtual_path`.
3. `query_by_path(virtual_path)` → `new_content_id` ≠ `prior_content_id`.
4. Materialized/read path returns `NEW_BYTES`.
5. `{store_root}/{prior_hash}.blob` still contains `OLD_BYTES`.
6. `{store_root}/{new_hash}.blob` contains `NEW_BYTES`.

Gate passes with `commit_projection_write` implemented.

## Success Criteria

- Gate step passes after implementation.
- S10.3.0–10.3.3 gates remain green.
- No change to `compat_runner` `readonly=on` virtfs export.

## Revisit Criteria

Promote compat write path when:

- host commit gate is stable, and
- a short design exists for supervisor scratch → commit IPC without mutable 9p.
