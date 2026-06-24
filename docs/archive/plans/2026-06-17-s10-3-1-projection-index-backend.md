# S10.3.1: Projection Index Backend

**Last Updated:** 2026-06-17
**Status:** Complete (2026-06-17)
**Related:** `docs/plans/2026-02-20-s10-3-projection-storage.md`, `STORE_SPEC.md`, `CONSTITUTION.md`

## Executive Summary

S10.3.1 keeps the durable projection index as a versioned Store artifact, not a SQLite database inside `store_service`.

The chosen v0 backend is:

```text
CAS blobs remain the source of truth.
ProjectionIndexV0 remains the canonical index artifact.
store_service maintains an atomic working copy at {store_root}/projection_index.json.
Each successful index mutation also snapshots that JSON as a CAS-backed artifact.
```

SQLite is deferred until a later search/indexing slice proves that JSON scan + in-memory lookup cannot meet the S10.3 path/tag workload.

## Decision

Choose **Option B: CAS-backed `ProjectionIndexV0` artifact with an atomic working copy** for S10.3.1.

Rejected for v0: **SQLite inside `store_service`**.

## Why This Choice

| Criterion | CAS-backed `ProjectionIndexV0` | SQLite in `store_service` |
|-----------|--------------------------------|----------------------------|
| TCB surface | Reuses JSON schema validation and existing Store blob/manifest path | Adds database engine, schema migrations, and query runtime to service TCB |
| Backup/restore | Ordinary CAS artifact history; snapshot is auditable | Needs DB file backup semantics and migration metadata |
| Migration | Schema-versioned JSON can be transformed by Store tools | Requires SQL migrations plus artifact schema compatibility |
| Gateability | Easy to assert with content IDs, manifests, and path/tag queries | Needs DB setup/teardown and corruption cases |
| Query latency | Good enough for path/tag v0 with startup load into memory | Better for large indexes and richer predicates |

This keeps S10.3.1 inside the current vertical slice: typed schema, durable Store artifact, service consumer, and a Foundry gate.

## Scope

### In Scope

- Add a mutable `ProjectionIndexStore` path that can atomically persist `ProjectionIndexV0` JSON.
- On startup, load `{store_root}/projection_index.json` if present, else start with an empty valid index.
- After index mutation, write a temp file, validate it, rename it over the working copy, and snapshot the same bytes into the CAS with a manifest kind such as `projection_index_v0`.
- Preserve the existing `RAMEN_STORE_PROJECTION_INDEX` override for read-only/query tests. If the override points outside `store_root`, S10.3.1 should treat it as query-only unless the gate explicitly opts into mutation.
- Keep query APIs unchanged: `query_by_path`, `query_by_tag`.

### Out of Scope

- SQLite or any embedded SQL dependency.
- Vector embeddings, graph traversal, GraphQL, or Cypher.
- Read-only VFS projection and CoW POSIX writes.
- Capability-filtered snapshot semantics beyond the existing store read/write capability checks.

## Implementation Sketch

1. Extend `services/store_service/src/projection_index.rs` with a durable writer:
   - `ProjectionIndexStore::load_or_empty(path)`
   - `upsert_entry(entry)`
   - `upsert_path_projection(projection)`
   - `persist_atomic()`
2. Keep the in-memory index behind the existing service state, replacing the current read-only `Arc<ProjectionIndexStore>` with a synchronized store only where mutation is required.
3. Reuse `artifact_store_schema::projection_storage::validate_projection_index` before every persisted write.
4. Snapshot the persisted JSON through the existing Store write path as a `projection_index_v0` artifact.
5. Add metadata helpers only if required to derive stable path aliases from `kind`, `channel`, and manifest content.

## Foundry Gate

Extend `tools/ci/foundry_projection_storage_s10_3.sh` with a new S10.3.1 step:

```text
FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=durable_index_roundtrip
```

Required assertions:

- Start with an empty store root and no `projection_index.json`.
- Ingest or otherwise apply one deterministic projection entry.
- Assert `{store_root}/projection_index.json` exists and validates with `store_cli validate-projection-index`.
- Restart/reload `store_service` or the projection index store.
- Assert `query_by_path("/...")` returns the expected `content_id`.
- Assert `query_by_tag("...")` returns the expected `content_id`.
- Assert a `projection_index_v0` manifest/blob pair exists in the CAS.

Negative assertion:

- Corrupt `projection_index.json` and verify startup/query initialization fails closed with `STATUS_INVALID_INDEX` or a service startup failure, not an empty silently accepted index.

## Success Criteria

- S10.3.1 passes the extended Foundry gate.
- Existing S10.3.0 schema, IDL, CLI validation, and read-only query tests still pass.
- No SQLite dependency appears in `services/store_service/Cargo.toml`.
- `ProjectionIndexV0` remains the only canonical durable index format.

## Revisit Criteria

Reopen the SQLite decision only after a measured gate shows one of:

- path/tag query latency exceeds 10 ms at 10,000 entries on the host gate,
- index mutation time is dominated by JSON rewrite cost,
- S10.6 vector/graph search requires query shapes that do not fit `ProjectionIndexV0`.

Until then, SQLite remains a post-VFS optimization, not an S10.3 dependency.
