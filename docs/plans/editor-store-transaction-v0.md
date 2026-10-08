# UI1.1b — bounded Store-owned editor transactions

**Status:** prerequisite proposal for independent review and coordinator freeze;
no transaction handlers, schema registration or b gate are implemented by this
document. **Prepared:** 2026-10-05. **Dispatch base:** `a05b0c6`, with the reviewed
UI1.1a assembly. The accepted host source is
`e3dfe9f2d962a37e34ffbdf542cd9a2e5fce7007cb803955d190fde16a68218e`;
client source is
`8c9a0f1718ece83dbf74619a6e65797f66928118a0778a094770d91087412720`.
[Current status](../../CURRENT_STATUS.md) and [next tasks](../../NEXT_TASKS.md)
own landed evidence and execution order.

The outcome is one real host Store owner publishing immutable ASCII content and
atomically selecting its revision with the original save receipt, then reopening
that state after a supported writer has quiesced. It is the b prerequisite in the
[editor proposal](desktop-editor-v0.md), preserving its **exactly seven** b rows.
It does not establish the full keyboard-to-Store editor task. UI1.1c must first
freeze the bridge between desktop retirement/deadlines and Store's authoritative
live admission state; a copied authorization verdict is insufficient.

The b consumer invokes a Store-owned typed Artifact-368 adapter with opaque
fixture grants and bounded shared objects. Reused UI1.1a key/frame witnesses are
stamped `volatile/in-process`; the selected-object transaction is stamped
`real-host-cas/in-process-store-owner`. Neither implies native kernel IPC,
process containment, target execution, device flush durability, power-loss
qualification, or persistence across target boots. No model or credentials are
needed. The trusted host Store deputy has broader host IO authority; whole-host
observable-authority inclusion remains unknown.

## Existing boundaries and the smallest extension

[artifact_store_schema](../../artifact_store_schema/src/lib.rs) already owns
content IDs, manifests and signature validation without filesystem/socket IO.
[artifact_store_core](../../artifact_store_core/src/lib.rs) owns CAS publication
and byte-identity verification. Its `publish_cas_artifact` requires caller writer
serialization and ownership validation; it preserves existing CAS metadata.
Its individual blob/manifest atomic-write helpers do not by themselves complete
a parent-directory synchronization protocol.
[DomainArtifactRegistry](../../services/store_service/src/domain_visibility.rs)
adds owner-bound publication intents, metadata recovery and directory sync.
Those are candidate-publication building blocks, not a selected revision/receipt
transaction. [projection CoW](../../services/store_service/src/projection_cow.rs)
uses a virtual path and lacks this operation/base/receipt contract.
[Agent Task's journal](../../services/store_service/src/agent_task/mod.rs)
provides relevant bounded validation patterns; its task semantics and locking
are not transplanted into editor saves.

Choose a new default-off `editor_store_v0_dev` module in `store_service`, using
existing dependencies. It owns the selected-object registry, IO worker, narrow
Artifact adapter and recovery. Pure schema types belong in
`artifact_store_schema::editor_save`. No desktop dependency on
`artifact_store_core`, kernel internal dependency, general Store listener change,
or new native protocol is needed for b. Protocol 368 and its four request/reply
pairs remain the canonical interface in
[the frozen wire contract](../DESKTOP_EDITOR_WIRE_V1.md).

Use a private Store-owned CAS namespace **per selected object** for this bounded
host profile. Two authorized sessions selecting the same object share that exact
namespace and admission state. Different objects have independent namespaces,
ownership registries and workers. This makes existing caller-serialized CAS
helpers usable without holding a global Store lock during IO. Roots are created
by the trusted fixture owner under its unique temporary directory, with owner-only
permissions; clients cannot provide a root, path, content ID, signing key or
ambient Store token. This is a named storage profile, not an implementation of
concurrent writers in the existing general Store CAS root. Deduplication across
these namespaces and general Store quota/GC integration remain future work.

The existing UIa `SelectedArtifactBackend` is service-owner-only, with private
volatile-state `CommitClaim`/`CommitPermit` constructors in
[host.rs](../../services/desktop/src/editor_dev/host.rs). It cannot simply accept
a caller-supplied Store callback. b independently freezes the Store boundary
below; c later owns a reviewed common admission/lease bridge and changes any
desktop integration centrally. b's shared cases exercise the actual UIa key/frame
path alongside the actual Store save path and correlate their actor/object
identities; they do not claim that UIa already forwards saves to Store.

## Typed data and authority

All schema records have `schema_version = 1`, explicit integer/enumeration
bounds, no unknown or duplicate fields, and checked validation after deserialization.
Hash fields are `[u8; 32]`; content IDs must also pass `ContentId::parse`, since
deserialization alone is not canonical-ID validation. These are validation data,
not grants. No schema function performs IO or mints a runtime permit.

| Proposed pure type | Exact logical fields and purpose |
|---|---|
| `SelectedArtifactV0` | version `u32`; owner, selected object ID, selected generation, revision `u64`; content hash `[u8;32]`; byte length `u32`; authenticated manifest hash `[u8;32]`. IDs/generation/revision are positive; bytes length is 0..4096. Content ID is derived from content hash, not independently caller-selected. |
| `EditorSaveAllocationV0` | version; owner/session/session generation/original instance/instance generation/object/object generation/operation/expected revision `u64`; expected content hash `[u8;32]`. Fully binds a durable allocation before Commit, without source bytes or mutation authority. |
| `EditorSaveBindingV0` | the allocation fields plus source object ID/generation `u64`, source length `u32`, source hash `[u8;32]`. The source descriptor and hash bind the exact submitted snapshot; service owner validates them against an actual registered source. |
| `EditorCommitPermitV0` | the complete binding, reserved successor revision, admitting service epoch, unique writer ID and admission epoch `u64`. This is an internal evidence/journal record. Deserializing it cannot issue authority. |
| `EditorSaveReceiptV0` | the existing **176-byte** receipt schema: version/total length/outcome/reserved `u32`; owner/session/session generation/original instance/instance generation/object/object generation/operation/expected revision/result revision/backend epoch/commit time `u64`; expected and source hashes `[u8;32]`. Committed outcome has checked successor revision; definitive noncommit has result revision and commit time zero. Reserved is zero. |

No added field changes the existing 64-byte text/source header, 176-byte receipt,
or <=64-byte Artifact messages. Schema serialization for disk is strict bounded
JSON, distinct from explicit little-endian shared-object encoding. A receipt's
source hash identifies the candidate content ID; the journal separately binds
its authenticated manifest. Data payload IDs, protocol numbers and right masks
are never authentication.

Opaque Store-issued `EditorEndpoint`/`EditorSourceLease`/`EditorReadLease` have
private fields and no public constructors or deserialization. `StoreCommitPermit`
is private, non-Clone and consumes one frozen source plus one object reservation.
It contains the validated `EditorCommitPermitV0` record, live owner-state reference,
private dispatch ticket and writer reservation. A copied journal permit record
cannot recreate it. Only the Store owner issues it under the admission mutex.

`Lang`: a selected-object endpoint can request ReadSelected (READ), AllocateSaveId
and Commit (REPLACE), or SaveStatus (RECEIPT), exactly according to its grant.
RecoveryReceipt is owner-bound original-operation lookup only; it cannot mutate,
enumerate operations, select another object, or open arbitrary content. The
trusted controller alone provisions sessions, revokes/expires them, advances
fixture time and installs bounded failure barriers. It is unavailable without
the explicit development feature.

`ObsContract`: authorized reads return that selected object's immutable snapshot;
own live conflicts may reveal its current revision. Original-operation lookup
returns only its bound receipt. Foreign, unallocated, changed-owner/generation/
base/source-tuple requests are Denied with zero data before lifecycle or outcome
observation. A newly invoked revoked endpoint returns Stale redacted. Copies
already read cannot be erased. Timings, host filesystem topology and the deputy's
broader authority are not claimed hidden.

## Proposed service API and response rules

Coordinator freezes final Rust signatures before assertions. The narrow surface
uses `kernel_api::generated::desktop_artifact_v1::{ReadSelected,
AllocateSaveId, Commit, SaveStatus}` and their generated reply types,
strict canonical envelope decoding, and Store-owned registered
shared-object descriptors. This sketch specifies ownership, not a new wire API:

```rust
// Default-off trusted fixture entry; paths never appear in client methods.
StoreFixture::create(profile, private_root) -> StoreOwner;
StoreOwner::issue_editor(grant_spec) -> EditorEndpoint;
StoreOwner::issue_recovery(original_owner_and_operation) -> RecoveryEndpoint;
EditorEndpoint::dispatch(canonical_envelope, now) -> ArtifactExchange;
EditorEndpoint::create_source() -> EditorSourceLease;
EditorSourceLease::write_ascii(bytes) -> Result<(), Status>;
EditorReadLease::copy_into(destination) -> Result<usize, Status>;
StoreOwner::retire(bound_actor, reason, now) -> Result<(), Status>;
StoreOwner::quiesce(deadline) -> Result<QuiescedStoreOwner, NotReady>;
StoreFixture::reopen(profile, QuiescedStoreOwner) -> Result<StoreOwner, Status>;
```

`GrantSpec` is controller-only and includes owner, session/generation, original
instance/generation, selected ID/generation, exact class/rights and expiry.
The positive owner ID is the existing Store domain ID; the private namespace has
no domain-zero/global bypass. Two same-object sessions in the positive race have
that same owner and independent original instances. Foreign owners are denied.
Requests must match the opaque endpoint's registered identity; a claimed identity
cannot replace the authenticated one. Source creation grants no selected-object
write by itself. Source headers/handles/class/generation/total size/ASCII are
validated before consumption; a submission freezes the registered source
atomically under the effect lock. Replacement while frozen is NotReady. Aliases
cannot mutate it; bytes are bounded and copied into the Store-owned immutable
snapshot before asynchronous IO. No foreign callback runs under an enforcing
lock. Store client control uses typed messages and the data uses registered
shared objects; Rust-owned snapshots are host implementation state, not a new
inline-byte native control interface.

| Request/result | Required rule |
|---|---|
| AllocateSaveId | Bind actor/object/base/hash under live authority; serialize a durable allocation, then acknowledge its ID. It changes no content. Lost reply retains any durable record. At 16 retained records return Exhausted without evicting one. |
| First Commit | Exact allocation and live source binding required. Freeze one source and private dispatch ticket; prepare durable source binding, then recheck live authority at permit issuance. Exactly one mutation dispatch and at most one permit/selection transition for that operation. |
| Same-object outstanding writer | New allocations/commits return NotReady without a second writer/permit. No fabricated Conflict against an unsettled reservation. |
| New exact duplicate Commit | Pending returns NotReady zero data, not Unknown. Settled/live exact duplicate retrieves the validated original result without publication. Changed tuple is Denied; revoked endpoint is Stale before outcome lookup. |
| Absolute deadline before permit | Close that original request under the authoritative admission mutex; original reply is definitive Timeout. Retire only its exact closed/unpermitted submitted source, preserving the record and sticky closure. A fresh explicit save may use a new operation; the old worker cannot later obtain a permit. |
| Original issued permit, then timeout/revoke or publication error | Original invocation may return Unknown with its bound operation. Preserve the immutable transition and same-object quarantine. No definitive Stale/Timeout implying no mutation once permitted. |
| SaveStatus / RecoveryReceipt | Authenticated original receipt lookup; pending original operation may be Unknown. Never replay Commit to discover the outcome. Missing/corrupt evidence remains Unknown/NotReady. |

All request deadlines are absolute **1000 ms from dispatch entry**, independent
of maintenance/accessor polling. A response timer never performs disk IO. Before
closing an operation it checks actor, retained original generation, expected base,
submitted source tuple and private dispatch ticket under that object's admission
lock. A different-ticket duplicate needs current live reauthentication before
any pending/settled observation. Only the original permitted dispatch may expose
its bound Unknown after retirement.

## One journal, finite identities and writer ownership

Each selected object owns `selection.json` and one `selection.next` staging name
in its private object directory, plus `cas/`. Journal version 1 contains:

- owner/object/generation, immutable initial selection, current selection;
- current service epoch, operation-ID high-water mark and writer-ID high-water
  mark (`u64`, checked), journal transition sequence and prior canonical journal
  digest;
- at most **16 ordered, non-evicted operation records**, including allocations,
  closed/noncommit, permitted/unknown and committed records;
- each record's allocation, optional complete submitted binding, optional permit
  record, finite state (`Allocated`, `Submitted`, `Permitted`, `Committed`,
  `Noncommit`), original allocated/submitted service epochs, and optional original
  receipt.

Committed records carry the manifest hash and selected-artifact successor. All
records retain original epochs/identities; reopening does not rewrite original
receipts to the new service epoch. Validation replays the committed records from
the initial selection in order: base revision/hash must match, successor must be
exactly base+1, receipt/binding/candidate must agree, and the computed final
selection must equal `current`. Noncommit records do not advance selection.
Permitted/submitted records never masquerade as committed. At most one unsettled
mutation reservation exists, and committed record order is publication order.
No separate receipt or revision file is authoritative.
The fixed profile also binds the trusted fixture signing-policy/key digest and
initial artifact identity. Journal digests detect inconsistent records; they do
not provide anti-rollback storage or authentication against a privileged host
rewriting a coherent journal. Fenced reopen retains the previous owner's known
durable allocation/receipt and acknowledged-sequence witnesses in its private
token and rejects a disk state that discards them. An independent cold-start
without that ownership/fencing evidence, or a coherently rolled-back storage
image, cannot satisfy this reopen contract.
The token also retains every observed durable Submitted/Permitted binding and
journal transition witness, all observed counter/high-water/service-epoch minima,
and issued runtime permits plus original request closure. An older coherent
Allocated-only journal cannot erase a known durable source/permit tuple. Validation
against these bounded retained owner witnesses is required before accepting prior
or successor state; it does not create general cold-start anti-rollback storage.

On initial open and every fenced reopen, persist a fresh checked **16-ID range**
by advancing the journal's operation high-water mark before exposing any ID from
that range. IDs are monotonic within `(selected object, generation)` and are
always authenticated with that object; a numeric ID alone is not a handle.
Reopen skips unused IDs from the prior range. It never reuses canceled, expired,
lost-reply or unknown allocated IDs and never deletes their retained records.
An ID tentatively taken from a durable range but not acknowledged or durably
recorded cannot be reused in the next range. Reserving a range does not create
16 allocations or reclaim record capacity: the actual retained record count
remains the admission bound across service epochs. A new allocation is not
acknowledged until its record has completed durable publication/readback.
Tentative ID reservation is distinguished from a durable allocation record;
once record publication may have happened its result is uncertain, the worker
slot stays reserved until settlement/fencing, and every validated durable record
is retained. Recovery cannot erase such a record to make the 17th allocation fit.
High-water, writer, journal-sequence, service-epoch or revision overflow fails
closed in the affected issuance/mutation domain; it never wraps or resets on
reopen. History rollover and generation replacement are outside b.

There is one worker reservation per object, including journal-only allocation,
source-submission and high-water updates. Only one worker owns that namespace's
mutable `DomainArtifactRegistry` and journal IO at a time. The registry is moved
into, or privately retained by, that writer; no blocking mutex acquisition or
IO is performed while holding global/session/object admission locks. A worker
returns an immutable result to the short state-update section. Pending metadata
work may make the same object NotReady but cannot acquire a mutation permit after
request closure. Different objects have independent admission and worker slots.

For Commit, atomically register/freeze the complete source binding. Publish a
`Submitted` metadata record before mutation admission; this only retains evidence,
like allocation, and cannot select content. After that bounded IO completes,
recheck live rights, object/actor generations, clamped expiry, absolute deadline,
sticky closure and current base under the same authoritative mutex updated by
retirement. Issue one runtime permit, reserve base+1 and quarantine the object.
Persist its `Permitted` record before candidate IO. The retained submitted binding
allows recovery to identify even a permit whose intent write failed. The worker
never reacquires permission through a new endpoint or treats epoch change as
retroactive cancellation of an issued permit.

The permitted writer alone performs its bound CAS and selection transition.
Future revocation/deadline closure prevents new admission but cannot recall that
one permission. Selected-revision exhaustion closes that object's future mutation
admission while keeping reads and original receipts usable. Other limits retire
only the affected domain; B is not reset to cover A's failure.

## Actual IO publication and authenticated reads

The private profile uses one filesystem per object, regular files, fixed owner
paths, bounded enumeration and no symlink-following reads. Opened metadata and
blob snapshots are validated as consumed: owner, manifest signature policy,
requested content ID, manifest digest, exact size and byte hash. Do not verify a
pathname and then return bytes from an unverified second open. Existing
`verify_blob_identity` is useful but a consumer must also authenticate its actual
snapshot. Signature policy is explicitly `RequireSignature` with one pinned
fixture public key. Only the feature-gated trusted fixture publisher has its
test signing key; no editor receives it, and no production signing authority is
claimed. The fixed manifest kind is `editor-selected-private`, its single channel
is `dev-host-editor`, and its signature has the pinned fixture key ID; all strings
are bounded before validation. Manifest signing uses the existing canonical signing bytes. Existing
foreign ownership, unsigned/invalid signature, corrupt manifest and CAS collision
denials remain enforced. Successful dedup retains the existing exact authenticated
metadata; the journal binds that retained manifest, not a replacement requested
manifest.
Before calling `DomainArtifactRegistry::new` or `publish_owned`, bounded
no-symlink preflight validates every pending publication intent, ownership record
and manifest against requested byte identity, `RequireSignature`, the pinned
key/kind/channel policy and this object's permitted operation binding (or its
trusted initial-artifact provisioning binding). Those helpers auto-recover
intents and can mutate metadata before returning, so a signature check performed
only after the call is too late. A malformed/unsigned/unbound intent denies before
helper mutation. If wrapper preflight cannot establish these prerequisites with
unchanged helpers, stop and request exact separately owned helper hardening.

The write sequence for one permitted operation is:

1. Persist the owner-bound candidate publication intent using existing Store
   ownership checks. Publish and synchronize the immutable blob and manifest;
   finalize and synchronize ownership. Read back the bounded candidate and verify
   the same bytes/manifest/owner to be selected. Old blobs are never overwritten.
2. Build the complete successor journal in bounded memory: same operation binding,
   exactly one committed receipt and the matching selected revision/content. Write
   `selection.next`, check all writes, synchronize that file, and explicitly handle
   close/error reporting where the platform exposes it.
3. Rename within the same directory to `selection.json`, synchronize the parent
   directory, reopen/validate/hash the actual authoritative file and its selected
   candidate, then install the known result in memory and acknowledge it. The
   rename is the possible selection publication point. An acknowledged commit
   requires the full synchronization and readback sequence.

Metadata-only updates use the same whole-journal publication path without changing
selection. Staging publication never uses two independent selected/receipt files.
Once rename may have happened, any IO/readback/response error yields Unknown and
poisons that object's mutation/restart-base admission. It cannot be converted to
definitive failure by reading cached prior state. No retry of rename/publication
occurs as an automatic mutation replay. Original receipt reconciliation follows
writer quiescence and disk validation. Even candidate-only errors after permit
retain uncertainty until the supported recovery rule applies.

The host primitives are qualified for the named gate filesystem. Rust documents
that `rename` replaces the destination and can fail across mount points, and that
file close errors may be ignored by `Drop`; explicit error-checked synchronization
is required here. Those API properties do not prove hardware durability. Record
the actual filesystem/mount/toolchain profile and gate the directory-sync calls
and reopen behavior on Linux; unsupported directory synchronization is INCOMPLETE,
not a silently weaker successful save.
([Rust rename](https://doc.rust-lang.org/std/fs/fn.rename.html),
[Rust File](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all).)

## Fenced reopen, failures and orphan bounds

`QuiescedStoreOwner` is a private, non-serializable ownership token. It is produced
only after closing new admission and observing completion plus an actual join of
**all** owned metadata/CAS/journal writers and request supervisors for that root.
A thread flag, retired generation, dropped `JoinHandle`, epoch increment, timeout
reply or barrier-release request is insufficient. Join is observed outside every
enforcing lock. Supported pause fixtures use owned finite barriers, entry/release/
settlement acknowledgements and <=2000 ms cleanup deadlines. If a writer fails to
settle, retain Unknown/quarantine and report INCOMPLETE; do not detach it and reopen
the same root. A future process-backed fixture would need an actual held owned
child, stop and wait evidence before supplying this token; b does not require or
claim arbitrary thread cancellation.

Failure fixtures stop the supported worker at named phases without replaying its
mutation: before candidate IO; after candidate/ownership publication but before
successor journal; after journal rename before acknowledgement; after acknowledged
commit. Each fixture reaches a bound barrier, returns/stops through the supported
owner path and is joined. The tests actually close/reopen the Store owner and
files; editing a journal to pretend a worker died is only a separate corruption
negative leg. Explicit syscall-failure injection may cause a named IO call to
return an error, but real preceding IO must remain visible and labeled. It is not
evidence of a real disk failing or power interruption.

After fencing, validate the authoritative `selection.json` and every referenced
selected candidate/receipt/chain. A complete prior selection with a known submitted
binding and no future writer can publish is definitive noncommit for that bound
operation, even if a candidate orphan exists. Persist its Noncommit reconciliation
without changing selection. A complete new selection with the matching original
receipt is committed. An allocation without durable source binding is retained;
no source-bound receipt is invented for it. Missing, truncated, unknown-version,
extra-field, duplicate-field, mismatched-binding, broken-chain or unauthenticated
authoritative state remains Unknown/NotReady. Never choose `selection.next`, an
orphan blob, cached state or a new grant as a guessed restart base. Readback errors
during reconciliation keep quarantine. Original receipts may be read through fresh
owner-bound recovery authority after old endpoints retire.

The selected-object storage limits are part of the fixture contract:

| Resource | Hard bound and exhaustion rule |
|---|---|
| Sessions / selected objects / original instances / endpoints | 2 / 2 / 16 / 64 for the host registry; no hidden growing maps or unbounded retired-handle history. Preserve original identities in the bounded operation records. |
| Submitted/shared objects | 32 live registered objects; each source/text header64 + ASCII0..4096 bytes, receipt176; retired/frozen aliases deny. |
| Operation records | 16 per selected object for its entire tested lineage, including allocation-only, closed, unknown and committed. No eviction on reopen. |
| Candidate blobs | Initial blob plus at most one distinct candidate per operation: <=17 regular blobs per object, each <=4096 bytes. Matching dedup creates no extra candidate. |
| Metadata | Each manifest <=2048 bytes, ownership sidecar <=512, publication intent <=4096; bounded signature/key/channel strings and at most one signature, one channel. Journal <=131072 bytes and 16 records, checked before deserialization growth. |
| Files / bytes | <=64 CAS entries, fixed journal/staging names outside CAS, and <=2 MiB total per object including allowed temporary files. Preflight bounds occur before existing registry scans; unexpected entries/types/overflow fail closed. |
| Workers / response / evidence | One IO worker per object; bounded active request supervisors; absolute1000 ms response deadline; <=256 actual exchanges and <=16 actual composed frames per case. Cleanup <=2000 ms after release for the supported fixture; failure is INCOMPLETE. |

Use bounded deserialize visitors, not unlimited vectors followed by a count check.
Journal high-water reservation leaves one fixed staging file; owner/CAS publication
has only the named helper staging/intents for that single writer. Complete candidate
orphans remain immutable and charged to the 17-blob bound. After fencing, remove
only attributable, validated staging files for this exact private namespace,
synchronize the parent and verify absence; do not broadly sweep a shared Store or
delete a blob referenced by current/prior receipts. Orphan GC beyond this finite
profile and total quotas for the general Store remain open. Partial publications
whose identity/owner cannot be validated poison the object rather than being
silently deleted to regain capacity.

## Exact seven b assertions and retained consumer evidence

Coordinator registers this exact inventory before handlers. Test assertions and
consumer validators get independent review and retained RED against absent
transaction behavior. No zero-test/ignored/missing-case pass; each required name
has one actual case outcome. The allowed UIa leg cannot satisfy a Store IO leg.
Every denial/failure compares prior state and includes an authorized positive
producer/consumer witness.

| Exact name | Required executable legs |
|---|---|
| `ui1_1_revoke_paused_key_frame_and_save` | Retain actual UIa before-key/frame barriers and denials labeled volatile. Real Store source-submit/before-permit barrier then revoke and expiry independently: no permit, no selected revision change, stale aliases; fresh authorized explicit save succeeds, closed worker released later cannot publish. After-permit retirement is a separate Unknown leg with at most its original transition. |
| `ui1_1_stalled_session_leaves_other_session_usable` | A's actual Store IO worker pauses after permit and reaches original absolute1000 ms Unknown; same-object new mutations NotReady, no new permit. B's different selected object performs real authorized read/save/read within its registered1000 ms deadline while actual UIa input/frame remains usable, labeled separately. Release/join A and reconcile only its original receipt. Timings exclude setup/hashing/evidence IO and prove this finite fixture, not scheduler guarantees. |
| `ui1_1_clock_capacity_and_counter_limits_fail_closed` | Clamped clock/expiry and checked time/counter arithmetic; 16 allocations including canceled/lost-reply/unknown retained across actual reopen, 17th Exhausted with unchanged selection. Durable range gaps never reuse IDs, high-water/epoch/revision/writer/transition MAX cases fail closed; bounded source/file/journal limits and affected-domain retirement, B/read/original-receipt positive witnesses. Keep actual UIa queue/frame bounds labeled volatile. |
| `ui1_1_store_commit_reopen_preserves_prior_blob` | Actual owner-authenticated signed initial read, immutable source, candidate/manifest/ownership publication, one atomic selected+receipt journal, file and parent sync/readback, join/close/reopen, fresh typed read of new bytes. Hash/open old blob unchanged. Wrong rights/foreign owner/object, source alias/malformed ASCII, wrong-key/unsigned/corrupt manifest and CAS metadata collision deny; subsequent valid allowed publication works where recovery is safe. |
| `ui1_1_store_same_base_conflict_and_operation_reuse` | Two separately authorized sessions bind the same object/base; entered barriers synchronize real attempts. One permit/winner; other is NotReady while pending and Conflict after settlement. Changed owner/session/instance/generation/base/source bytes or reused tuple is Denied without new publication. Pending exact duplicate NotReady zero; settled live exact duplicate returns original receipt, count unchanged; revoked duplicate Stale. Explicit new-base save requires new user operation. |
| `ui1_1_store_lost_reply_reconciles_without_mutation_replay` | Drop original reply before permit and after permit/selection in separate actual legs. Known allocated ID survives; exactly one Commit dispatch, at most one permit/selection. After-permit revoke/deadline while actual candidate/journal IO is paused returns original Unknown, same-object quarantine, B progress, no replacement permit. Supported release/completion/join then original SaveStatus/RecoveryReceipt yields validated result without recommitting. Pre-permit definitive Timeout retires exact source and permits fresh explicit operation without reviving old one. |
| `ui1_1_store_recovery_validates_atomic_selection_and_receipt` | Reject reopen/new epoch while writer is entered and unquiesced. At each of four phases above, supported stop/completion and actual join precede close/reopen; accept only complete prior/noncommit or complete new/original-receipt state. Separate missing/truncated/corrupt/extra/duplicate/unknown-schema/binding-chain/candidate negatives remain Unknown/NotReady, no inferred rollback or replay. Validate attributable orphan staging cleanup and finite retained capacity. Positive signed prior and committed recovery witnesses remain. |

Proposed gate: `just foundry-desktop-editor-store-ui1-1b`, coordinator-owned fixed
output `out/desktop/ui1-1b/`. This document does not register or run it. Evidence
must include a strict versioned summary with exact names/counts/outcomes, claim
flags and backend labels; canonical actual Artifact368 request/reply bytes and
descriptor objects; allocation/binding/permit/receipt hashes and dispatch counts;
actual selected journal bytes/digests before/after/reopen; consumed manifest,
ownership and content digests; named IO phase completion/error events; barrier
entry/release/settlement ordering; join/fence proof; prior/new revision and old-blob
digests; finite resource ledgers and B timing witnesses. Record the process/PID,
toolchain, source and Cargo-selected binary hashes plus filesystem profile.
An IO-event log is instrumentation corroborated by real files/readback, not proof
of kernel/device flush internals. Validators reject redaction violations, missing
sync/readback/fence witnesses, guessed Unknown→failure transitions, extra dispatches,
missing/extra cases and manufactured exchanges. Optimized Python must retain all
checks. Linux acceptance remains INCOMPLETE until the final assembled required
gate runs; worker-only fixture success is not integrated acceptance.

## Proposed subsequent file ownership and landing sequence

No path below is assigned or implemented by this document. Coordinator narrows
writers after independent contract review, then assertion RED, then handlers.

| Proposed path | Responsibility |
|---|---|
| `artifact_store_schema/src/editor_save.rs` | Pure versioned record/receipt/shared-data validation, bounded journal validation/replay; no IO or runtime authority constructors. |
| `artifact_store_schema/tests/editor_save.rs` | Pure schema/layout/binding/bounds/chain positives and negatives. |
| `services/store_service/src/editor_store/mod.rs` | Default-off opaque endpoint/lease registry, strict Artifact adapter, clock/response/authority admission, private runtime permit and fixture barriers. |
| `services/store_service/src/editor_store/journal.rs` | Store-owned bounded file/CAS/owner/signature consumption, actual atomic journal publication/readback, supported IO phase injection, fenced recovery and orphan accounting. |
| `services/store_service/tests/editor_store.rs` | Exactly seven reviewed b cases, including typed Store consumer and shared UIa witnesses through test-only support. |
| `services/store_service/tests/support/editor_store.rs` | Finite actor/root/source/barrier/evidence fixtures; no selectable production authority. |
| `tools/ci/editor_store_result.py` | Strict actual evidence consumer and exact semantic inventories. |
| `tools/ci/foundry_desktop_editor_store_ui1_1b.sh` | Default-off exclusion, actual7-case execution, Cargo/source/artifact provenance, owned cleanup and result validation. |

Coordinator alone initially owns schema/module exports, manifests/features and
any test-only desktop dependency, shared code extraction/Artifact codec reuse,
`justfile`, CI registration, maintained contracts, decisions and status/history.
Reuse existing core/ownership helpers unchanged if the frozen private-namespace
profile suffices; any needed helper hardening or generalized shared-root locking
returns for a separately assigned exact scope. No generated-file edit or codegen
is expected unless the coordinator finds a contract change requiring it.

The next executable prerequisite is the pure schema and seven-case service API
freeze: exact constructors, descriptor leases, status mappings, journal canonical
serialization/digests, high-water ranges and all supported IO failure checkpoints.
Then register RED assertions, independently review, implement one Store writer and
run affected ownership/signature/CoW/agent-task consumers plus the assembled b gate.
General Store RPC deployment, the c live-admission bridge, real editor process,
target storage/runtime and physical restart remain separate dependencies. There
is no approval, merge, release, paid execution or hardware actuation authority in
this proposal.
