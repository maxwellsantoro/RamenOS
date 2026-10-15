# Comprehensive review repair successor

**Date:** 2026-10-14  
**Scope:** confirmed A–D/F/G behavior defects and their consumers; explicit E/H limits  
**Starting revision:** `23f37445e35ba8f5f982f7d252831a5c4e298ce4` (PR #39)

This successor follows the requested order: D/C, A/F, B, then G. The source
changes do not graduate RamenOS beyond public pre-alpha. Local tests, QEMU table
fixtures, hosted checks, active userspace and physical evidence have distinct
scopes. The authoritative implementation and queue remain in
[Current Status](../../CURRENT_STATUS.md) and [Next Tasks](../../NEXT_TASKS.md).

## Repair contracts

| Finding | Enforcing boundary and consumer | Regression obligation |
|---------|----------------------------------|-----------------------|
| D — accepted unsigned publication under strict reads | Ordinary host Store signed descriptor-ingest extension; common publisher requires signature policy; exact signed metadata checked after intent recovery | Actual strict service, separate artifact/capability signing keys, valid signed control; unsigned/invalid publication denial without new CAS/ownership/projection; next valid publication; kill/wait/restart/read |
| C — generation wider than wire | IPC/Shmem/Trace producers and validators use the existing canonical `1..=u32::MAX` field; maximum release permanently retires the slot | Last legal handle round-trips; stale/invalid forms deny; allocator skips retired slots; exhaustion remains bounded; another slot/domain/resource continues |
| A — numeric handle selects caller identity | Both kernel dispatchers require an independent kernel-private caller context and matching live IPC namespace; Trace payload and Oracle data-plane capabilities match that namespace | Copy A's complete handle under B's known context; copy A's Trace/Shmem cap under B's own IPC; deny before state/bytes/cursor changes; A and B next operations succeed |
| F — foreign mapping and PTE replacement | Legacy map is self-only; versioned owner offer/recipient acceptance; both real MMUs preflight the whole range and reserve every table frame before PTE publication | Foreign direct map/acceptance deny; cancellation, capacity and stale offers recover; leaf and huge/block collisions preserve old mappings; allocation failure returns only new frames; subsequent map succeeds; Shmem must not unmap the rejected range |
| B — metadata-only trace consumption | Versioned native Shmem delivery plus explicit cursor Commit; legacy nonzero reads deny without consumption | Inspect actual event bytes/zero padding/guards before Commit; interrupt/retry; destination/source generation checks; overflow resync; exact committed outcome survives destination close; next read succeeds |
| G — no live ordinary Store grant/revoke state | Durable issuer-authenticated registry and guarded common operation admission/publication | Signed but unknown IDs deny; authenticated grant/revoke, role separation, exact identity, tombstones and restart; revoke during preparation prevents later commit; unrelated live authority still progresses |

The D extension is host socket opcode 8/version 2, carrying an externally signed
manifest and a separately transferred source descriptor. It does not make the
host bincode protocol a target IDL implementation. The artifact publisher does
not receive an ambient Store signing key. Existing explicit development ingestion
retains its unsigned policy. Exact deduplication follows recovery of older
publication intents, so the durable manifest and projection cannot disagree.
A denied or malformed descriptor-ingest reply retires the client transport
immediately; the next explicit request reconnects without replaying mutation.

The native consent contract is
[`shmem_consent_v2.toml`](../../idl/harness/shmem_consent_v2.toml), protocol 531.
Offers are bounded and never reused during one table lifetime. A failed mapping
retains its offer; successful acceptance consumes it. An interrupted acceptance
reply does not grant permission to replay another map. The recipient can release
its own accepted mapping. The existing common-VA layout remains bounded: a
collision denies rather than replacing a pre-existing mapping.

The native trace contract is
[`trace_delivery_v3.toml`](../../idl/harness/trace_delivery_v3.toml), protocol 532.
A pending receipt binds exact source/destination generations, destination range,
source cursor and actual whole-event length. Cursor advancement requires Commit
while the destination remains live. A completed receipt instead reconciles its
original outcome without requiring the retired destination or consuming newer
events. Issuance retires at exhaustion. Producer overflow may invalidate a
pending cursor and requires an explicit metadata resync.

## Ordinary host Store authority operation

Strict startup requires an explicit, nonempty, entirely valid
`RAMEN_STORE_CAP_TRUSTED_KEYS` list. Artifact publisher keys do not substitute for
capability-authority keys. An external issuer signs both a `StoreCapability` and
its domain-separated `AuthorityCommand`; `StoreClient::apply_authority_update`
submits the bounded command on host opcode 9/version 1. Connected peer write
policy and issuer signature are both required. Grant binds the exact signed token;
revoke permanently retires its ID. Exact historical command retry is idempotent
and does not extend expiry. Reissuing authority requires a new ID.

The root holds `.live-authority.lock.v1` for its lifetime. Durable records in
`.live-authority.v1` and `.live-authority.initialized.v1` must agree; malformed,
missing, noncanonical or symlink state denies startup. Existing strict roots with
no registry start with no grants and need explicit issuer provisioning. Never
resolve a validation failure by deleting authority state or silently enabling
`RAMEN_STORE_DEV_MODE`. Uncertain write/sync failure disables all live-authority
operations until verified restart; failed restart requires operator recovery
that preserves revocation history. Limits are 1024 retained IDs, 512-byte tokens,
1024-byte management requests and a 2 MiB database. Capacity denial retains room
to revoke an existing ID. The registry does not confer host filesystem isolation.

## Gate and evidence boundaries

`just foundry-review-boundaries` retains old families and adds nonempty selected
inventories for wire-generation retirement, caller namespaces, target consent,
strict Store publication/recovery, descriptor denial/reconnect, live authority
restart/revocation, and Shmem trace delivery. A missing or renamed test family cannot produce an empty green.

`just foundry-shmem-dataplane-s8-phase4-integration` now runs both x86_64 and
aarch64. Each architecture must emit all seventeen real MMU assertion markers,
the Shmem collision/retry marker and the trace bytes/Commit marker. It waits for
the final `shmem_test: COMPLETE` marker before stopping QEMU. The roots are
allocated and programmed but never activated as a user process's CR3/TTBR.
The required QEMU umbrella invokes this gate after its existing S8 contract
checks, so both architectures are required in hosted CI without changing the
canonical stage inventory. Per-call allocation fault injection uses the same mapping core and real allocator;
it does not replace the allocator or introduce an ambient failure switch.

The original REDs include strict unsigned acceptance, all twelve generation
assertions, foreign Oracle Shmem acceptance, four real MMU collision failures on
each architecture, Shmem rollback deleting the rejected PTE, legacy trace
consumption, and unknown signed Store capability acceptance. Additional failure
assertions exposed recovery-order metadata conflict, a client close/next-request
race, completed trace receipt reconciliation after destination retirement, and a
rejected second Store startup removing the active publisher's durable intent.
The root ownership lock now precedes publication recovery and audit initialization.
Each was reproduced separately before its repair. Missing new APIs were recorded
as compiler RED, separately from behavioral RED.

Detailed logs and immutable review notes are retained in the tool-managed private
`comprehensive-repairs-2026-10-14` artifact collection. Do not place tokens, signing
secrets, raw sensitive traces or private scan material in the public repository.
Fresh affected consumer checks and matching hosted qualification must identify
the assembled revision; preceding PR #39's all-green run is historical evidence.

## Boundaries that remain unfinished

- **A / RUN0:** the caller context currently comes only from trusted boot/test
  fixtures. No authenticated untrusted syscall, scheduler or process transport
  supplies execution identity yet. Numeric namespace checks do not establish
  complete process isolation or protocol-specific IPC rights.
- **F:** both MMUs require continuous exclusive access to their page-table
  hierarchy. Allocator and root-registry locks do not provide that exclusion.
  These repairs do not implement SMP page-table synchronization, active userspace
  translation, firmware exit or a general target physical-access adapter.
- **B:** the existing `trace_client` constructor still defaults to `MockTransport`.
  It has no authenticated kernel carrier for v3. Kernel/QEMU fixture delivery is
  not a claim that this prototype client reads a running target's trace stream.
- **E / G:** the existing Domain Manager default policy and simulated backend do
  not become a production grant backend through this host Store registry. Target
  grants, authenticated transport and native Store IDL integration remain work.
- **G:** ordinary Store GetBlob returns a host path. Revocation controls subsequent
  service operations; it cannot retract already delivered paths, bytes or file
  access. A shared host UID/filesystem is not a domain isolation boundary.
- **H:** the default POSIX compatibility path remains an explicitly acknowledged,
  trusted host development path with best-effort rlimits. Helper seccomp/chroot/
  namespace controls remain separate from default containment.
- The real editor child, the full P0 assertion families, device input, target
  storage, physical capture and everyday readiness remain unfinished.
