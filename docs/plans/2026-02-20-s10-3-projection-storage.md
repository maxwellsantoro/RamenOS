# S10.3: Projection Storage

**Last Updated:** 2026-10-03
**Status:** Maintained reference; S10.3.0–S10.3.4 host phases landed

Projection Storage gives human-readable paths and typed path/tag queries over
immutable content-addressed blobs. The same content can support different views
without defining the native API around POSIX. Vector search, embeddings,
GraphQL/Cypher, and a general guest filesystem are deferred.

## Ownership

| Component | Landed responsibility |
|-----------|-----------------------|
| [Artifact Store](../../artifact_store_core/) | Blob/manifest identity and verified content |
| [Projection schema](../../artifact_store_schema/src/projection_storage.rs) | `ProjectionIndexV0`, entries, path projections |
| [Store index](../../services/store_service/src/projection_index.rs) | Durable CAS snapshot plus atomic working copy |
| [Read-only projector](../../services/store_service/src/projection_vfs.rs) | Host materialized path view; optional compat 9p export |
| [CoW commit](../../services/store_service/src/projection_cow.rs) | Typed replacement-byte ingestion and path repointing |
| [Semantic Store IDL](../../idl/harness/semantic_store_v1.toml) | Typed path/tag query contract |

The Store service owns ingestion and index mutation. Caller authority, content
identity, source descriptors, labels, and path segments are checked at their
consumption boundaries. A path alias or tag is metadata, not proof of access.

## Landed phases

| Phase | Behavior |
|-------|----------|
| S10.3.0 | Schema/IDL, read-only queries, CLI index validation |
| S10.3.1 | CAS-backed durable index; corrupt working copies fail closed |
| S10.3.2 | Successful ingestion updates path/tag mappings and durable reload |
| S10.3.3 | Read-only host projection; QEMU compat export uses `readonly=on` |
| S10.3.4 | Typed host CoW commit preserves the previous blob and repoints the path |

Default working-copy location is `{store_root}/projection_index.json`;
`RAMEN_STORE_PROJECTION_INDEX` overrides it. Ingested aliases use sanitized
`/store/{kind}/{channel}/{source_filename}` segments, with kind/channel tags.
These are current conventions, not unrestricted host mounts.

## Evidence boundary

`just foundry-projection-storage-s10-3` checks host queries, durable roundtrip,
corrupt-index rejection, materialized reads, and named CoW behavior. The 9p
export's guest read path still needs a suitable initrd and its own end-to-end
assertion. Typed CoW commits do not make the 9p export writable or intercept
arbitrary POSIX writes.

Immutable hashes preserve content versions; they do not supply tamper-proof
metadata, instant durable rollback, or S13 power-loss guarantees. The proposed
path/tag latency target of <10ms at 10,000 entries needs measurement; the gate is
not that benchmark. SQLite remains deferred until an evidenced need justifies it.

See the historical [index decision](../archive/plans/2026-06-17-s10-3-1-projection-index-backend.md),
[read-only export](../archive/plans/2026-06-17-s10-3-3-read-only-vfs-projection.md),
and [CoW decision](../archive/plans/2026-06-17-s10-3-4-cow-projection-writes.md).
[Current Status](../../CURRENT_STATUS.md) and [Next Tasks](../../NEXT_TASKS.md)
own operational state and remaining work.
