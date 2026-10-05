# UI1.1b executable prerequisite API

**Status:** coordinator-frozen contract, 2026-10-05. Independent review accepted
the exact schema, service and evidence signatures below. This contract prepares
pure-schema checks and exactly seven Store assertions before handlers; it does
not supply Store transaction execution or durability evidence.

The reviewed source proposal SHA256 is
`365dc1526279e7d70f0d44c610a580e1ed9881f8a66248e145608e285477bbe9`.
The governing [Store transaction prerequisite](plans/editor-store-transaction-v0.md)
has SHA256 `ea07ee5b797e209b19bf74554c2e03dc759ff26639b24f02e972a51a3c5331bb`.
[Current status](../CURRENT_STATUS.md) and [next tasks](../NEXT_TASKS.md) own
landed evidence and execution order. These Rust2021 signatures use existing
schema/Store dependencies. The coordinator owns shared exports, features,
registrations and later integration.

## Pure schema definitions

Module `artifact_store_schema::editor_save` has no filesystem, socket, clocks,
thread primitives, callbacks, `kernel_api` dependency or authority constructors.
Use its existing `Vec`, serde, serde_json, sha2 and alloc support. Constants:
`VERSION=1`, `TEXT_HEADER_LEN=64`, `RECEIPT_LEN=176`, `MAX_TEXT_LEN=4096`,
`MAX_OPERATIONS=16`, `MAX_JOURNAL_BYTES=131072`.

```rust
pub type Hash32 = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorSaveError {
    Version, Bounds, Identity, ContentId, Binding, State, Chain,
    Reserved, Ascii, Overflow, Json, NonCanonical,
}

// Derive Serialize + Deserialize + Clone + Debug + PartialEq + Eq for
// every following plain record; deny_unknown_fields on every record.
pub struct EditorActorV0 {
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}
pub struct SelectedArtifactV0 {
    pub schema_version: u32,
    pub owner_id: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub revision: u64,
    pub content_hash: Hash32,
    pub byte_len: u32,
    pub manifest_hash: Hash32,
}
pub struct EditorSaveAllocationV0 {
    pub schema_version: u32,
    pub actor: EditorActorV0,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub operation_id: u64,
    pub expected_revision: u64,
    pub expected_content_hash: Hash32,
}
pub struct EditorSourceBindingV0 {
    pub source_object_id: u64,       // canonical packed SHM reference
    pub source_generation: u64,      // checked widened32, never truncation
    pub byte_len: u32,              // ASCII body length, not header+body
    pub content_hash: Hash32,
}
pub struct EditorSaveBindingV0 {
    pub schema_version: u32,
    pub allocation: EditorSaveAllocationV0,
    pub source: EditorSourceBindingV0,
}
pub struct EditorCommitPermitV0 {
    pub schema_version: u32,
    pub binding: EditorSaveBindingV0,
    pub successor_revision: u64,
    pub service_epoch: u64,
    pub writer_id: u64,
    pub admission_epoch: u64,
}
#[repr(u32)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorReceiptOutcomeV0 { Committed = 0, DefinitiveNoncommit = 1 }
pub struct EditorSaveReceiptV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub outcome: EditorReceiptOutcomeV0,
    pub reserved: u32,
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub original_instance_id: u64,
    pub original_instance_generation: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub operation_id: u64,
    pub expected_revision: u64,
    pub result_revision: u64,
    pub backend_service_epoch: u64,
    pub committed_at_ms: u64,
    pub expected_content_hash: Hash32,
    pub source_content_hash: Hash32,
}
pub struct EditorTextHeaderV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub selected_object_id: u64,
    pub revision: u64,
    pub content_hash: Hash32,
    pub byte_len: u32,
    pub reserved: u32,
}
```

Exact constructors/method catalog (all associated `try_new` set version/size/
reserved fields; manually constructed/deserialized payloads still need validation):

| Receiver | Signature |
|---|---|
| `EditorActorV0` | `try_new(owner_id:u64, session_id:u64, session_generation:u64, instance_id:u64, instance_generation:u64) -> Result<Self,EditorSaveError>`; `validate(&self) -> Result<(),EditorSaveError>` |
| `SelectedArtifactV0` | `try_new(owner_id:u64, object_id:u64, generation:u64, revision:u64, content_hash:Hash32, byte_len:u32, manifest_hash:Hash32) -> Result<Self,EditorSaveError>`; `validate(&self)`; `content_id(&self) -> Result<crate::ContentId,EditorSaveError>` |
| `EditorSaveAllocationV0` | `try_new(actor:EditorActorV0, base:&SelectedArtifactV0, operation_id:u64) -> Result<Self,EditorSaveError>`; `validate(&self)` |
| `EditorSourceBindingV0` | `try_new(object_id:u64, generation:u64, byte_len:u32, content_hash:Hash32) -> Result<Self,EditorSaveError>`; `validate(&self)` |
| `EditorSaveBindingV0` | `try_new(allocation:EditorSaveAllocationV0, source:EditorSourceBindingV0) -> Result<Self,EditorSaveError>`; `validate(&self)` |
| `EditorCommitPermitV0` | `try_new(binding:EditorSaveBindingV0, service_epoch:u64, writer_id:u64, admission_epoch:u64) -> Result<Self,EditorSaveError>`; `validate(&self)`; successor derives checked expected+1, never supplied by caller |
| `EditorSaveReceiptV0` | `committed(permit:&EditorCommitPermitV0, committed_at_ms:u64) -> Result<Self,EditorSaveError>`; `noncommit(binding:&EditorSaveBindingV0, original_service_epoch:u64) -> Result<Self,EditorSaveError>`; `validate(&self)`; `validate_binding(&self,binding:&EditorSaveBindingV0) -> Result<(),EditorSaveError>`; `encode_le(&self) -> Result<[u8;176],EditorSaveError>`; `decode_le(bytes:&[u8]) -> Result<Self,EditorSaveError>` |
| `EditorTextHeaderV0` | `try_new(object_id:u64, revision:u64, bytes:&[u8]) -> Result<Self,EditorSaveError>`; `validate_bytes(&self,bytes:&[u8]) -> Result<(),EditorSaveError>`; `encode_le(&self) -> Result<[u8;64],EditorSaveError>`; `decode_le(bytes:&[u8]) -> Result<Self,EditorSaveError>` |

Bare `validate(&self)` above always returns `Result<(),EditorSaveError>`. IDs,
generations, revisions and epochs are positive. Length is0..4096; printable32..126,
LF=10 and TAB=9 are the only accepted body byte classes beyond printable.
Hash32 is exactly32 bytes; no arbitrary nonzero-hash assumption. Source binding
validates positive checked32 generation but schema cannot unpack a kernel handle
or infer source authority; Store performs that check. Pure receipt constructors
create payloads only, not evidence that a commit/noncommit happened. Committed
result is checked base+1; noncommit result/time zero. Original admitting/submitting
service epoch is retained, never replaced by recovery's new epoch. Disk receipt
outcomes serialize exactly as `"committed"` and `"definitive_noncommit"`; unknown
spellings and numbers are rejected. Wire outcomes remain explicit u32 values0/1.

Text header offsets0,4,8,16,24,56,60 exactly match the frozen64-byte schema.
`total_len=64+byte_len`, checked. Source uses selected object ID/base revision and edited-body hash in its header,
validated against the registered descriptor; its body hash is the edited source
hash. Thus **source validation is separate from SelectedText validation**: the
source header content_hash represents its actual source bytes, while expected
base hash comes from the retained allocation/registry. No byte hash supplies
authority. Receipt offsets are0/4/8/12, then twelve u64s16..104, expected hash112,
source hash144. Enum numbers are explicitly encoded/decoded, not `repr(C)` copied.

## Bounded journal and canonical bytes

Use this exact field order in canonical serialization. `BoundedVec<T,16>` has a
private Vec, no DerefMut, `try_from_vec(Vec<T>)`, `as_slice`, `len`, and validated
`try_push`; custom Deserialize visitor rejects a17th item without allocating an
unbounded vector. Each nested record remains strictly bounded. State enums are
serialized as the exact lowercase snake_case strings below, not numeric casts.

```rust
pub enum EditorOperationStateV0 {
    Allocated, Submitted, Permitted, Committed, Noncommit,
}
pub enum EditorClosureV0 { None, Deadline, Revoked, Expired, ServiceRetired }
pub struct EditorOperationRecordV0 {
    pub allocation: EditorSaveAllocationV0,
    pub allocated_service_epoch: u64,
    pub binding: Option<EditorSaveBindingV0>,
    pub submitted_service_epoch: Option<u64>,
    pub permit: Option<EditorCommitPermitV0>,
    pub closure: EditorClosureV0,
    pub state: EditorOperationStateV0,
    pub successor: Option<SelectedArtifactV0>,
    pub receipt: Option<EditorSaveReceiptV0>,
}
pub struct EditorSelectionJournalV0 {
    pub schema_version: u32,
    pub owner_id: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub signing_policy_hash: Hash32,
    pub initial: SelectedArtifactV0,
    pub current: SelectedArtifactV0,
    pub service_epoch: u64,
    pub operation_high_water: u64,
    pub writer_high_water: u64,
    pub transition_sequence: u64,
    pub prior_journal_hash: Hash32,
    pub operations: BoundedVec<EditorOperationRecordV0,16>,
}
```

`EditorSelectionJournalV0::try_new(initial:SelectedArtifactV0,
signing_policy_hash:Hash32) -> Result<Self,EditorSaveError>` creates genesis:
service epoch1, high-water0, writer0, transition1, prior digest zero, no operations.
It cannot serve clients until initial actual CAS validation and durable16-ID range
reservation complete. Initial revision is positive (normally1; MAX is the explicit
trusted exhaustion fixture). `validate(&self) -> Result<(),EditorSaveError>` checks
all field bounds, monotone operation IDs <= high-water, owner/object/gen equality,
complete state shape, receipt tuples and replayed revision chain. Active mutation
reservation is `(Submitted && closure == None) || unsettled Permitted`; total<=1
per object. Permitted remains active regardless of closure, because closure never
cancels its issued permit. Closed/unpermitted Submitted awaiting its bound Noncommit
has no active reservation and may coexist with a fresh operation. An Allocated-only record may close without inventing a source receipt;
Noncommit may have no binding/receipt only for that unsubmitted allocation. A
Submitted/Permitted bound Noncommit must retain the binding and original receipt;
issued permit evidence is never erased. Committed needs binding+permit+successor+
committed receipt all agreeing. Closed issued permits may still commit once.

`canonical_bytes(&self) -> Result<Vec<u8>,EditorSaveError>` validates then emits
compact typed `serde_json::to_vec(self)` in declaration field order, fixed explicit
null Option fields, operation ordering by increasing operation ID, no maps or
floating point. Replay committed records in that retained order and require each
base/successor to match the evolving selected state. Same-base allocations may
lose to a later ID, but an earlier loser cannot then commit against a new base.
Each result revision is unique; missing successor/duplicate/gap fails. Canonical bytes
must fit131072. `decode_canonical(bytes:&[u8]) -> Result<Self,EditorSaveError>` checks
input length before parse, uses bounded typed visitors (no serde_json::Value),
rejects unknown/duplicate fields, wrong enums, negative/fractional/out-of-range
integers, trailing tokens, then requires bytes==canonical encoding. Hashes use
JSON32-element integer arrays. The SHA256 of those exact bytes is
`digest(&self) -> Result<Hash32,EditorSaveError>`. `prior_journal_hash` hashes the
previous complete canonical journal; never exclude fields or self-hash this file.
The journal sequence/prior hash are transition witnesses, not signed anti-rollback
storage or proof of an unavailable chain of prior files.

Manifest hash is SHA256 of compact **typed full Manifest** serialization including
the exact existing signature strings; blob hash is SHA256 of body bytes. Manifest
file whitespace is not that digest. Signature verification uses existing
`manifest_signing_bytes`: it clears signatures, and Manifest's
`skip_serializing_if = "Vec::is_empty"` **omits the signatures field** from those
signing bytes. Reuse the actual helper; do not substitute an empty-array signing
payload. Profile restricts schema1,
kind `editor-selected-private`, one channel `dev-host-editor`, one Ed25519 signature,
fixture key ID1..64 ASCII bytes, signature text <=1024 bytes, manifest<=2048 bytes.
Use strict wrapper parsing before converting to existing Manifest; that existing
generic deserializer does not reject every profile extra field. Signature wrapper
has exactly algorithm/signature_data/key_id/timestamp/signer; timestamp/signer absent
for this fixture profile, including in the signed publisher's output. Metadata
owner<=512 and publication intent<=4096 use typed bounded wrappers too.

## Store fixture and adapter surface

Module `store_service::editor_store`, gated only by new default-off
`editor_store_v0_dev`. Ordinary Store default/main/agent_task behavior unchanged.
New `StoreStatus` is repr(u32) Ok0,Denied1,Invalid2,Unsupported3,Stale4,Exhausted5,
NotReady6,Conflict7,Timeout8,Disconnected9,Unknown10,Internal11. It does not depend
on desktop_service. `FixtureError { pub status:StoreStatus, pub reason:FixtureErrorKind }`
is privileged controller diagnostics; kinds InvalidProfile/Io/Corrupt/Unfenced/
Capacity/Exhausted/Timeout. Client errors never expose IO paths/reasons.

```rust
pub struct InitialObject { pub owner_id:u64, pub object_id:u64,
    pub generation:u64, pub revision:u64, pub bytes:Vec<u8> }
pub struct FixtureProfile { pub origin_ms:u64,
    pub initial_objects:Vec<InitialObject>, // checked1..2 before IO
    pub fixture_signing_seed:[u8;32], pub fixture_key_id:String }
pub struct GrantSpec { pub actor:EditorActorV0, pub selected_object_id:u64,
    pub selected_generation:u64, pub rights:u32, pub ttl_ms:u64 }
pub struct ReopenFailure { pub error:FixtureError, pub owner:QuiescedStoreOwner }
pub enum EndpointClass { Artifact, RecoveryReceipt }
pub enum SharedObjectKind { SelectedText, DraftSource, Receipt }
pub struct ObjectDescriptor { pub handle:kernel_api::cap::Handle,
    pub kind:SharedObjectKind, pub object_generation:u64, pub byte_len:u32 }
pub struct EditorEndpoint { pub peer:EditorPeer,
    pub handle:kernel_api::cap::Handle }
// Private-field types: StoreFixture (namespace entry), StoreHost (Clone),
// FixtureController (not Clone), EditorPeer (Clone), ReadLease (Clone),
// WriteLease (Clone), PauseToken, QuiescedStoreOwner (not Clone/serde).
```

| Receiver / free function | Exact signature |
|---|---|
| `StoreFixture` | `create(root:&std::path::Path, profile:FixtureProfile) -> Result<(StoreHost,FixtureController),FixtureError>`; root must be new/private, bounded before side effects |
| `StoreFixture` | `reopen(token:QuiescedStoreOwner) -> Result<(StoreHost,FixtureController),ReopenFailure>`; consumes exact-root token including same immutable profile; failure returns its retained token, no substituted path/profile |
| `StoreHost` | `dispatch(&self, peer:&EditorPeer, request:&kernel_api::ipc::Envelope) -> Result<kernel_api::ipc::Envelope,StoreStatus>` |
| `StoreHost` | `draft_source(&self,peer:&EditorPeer,expected_revision:u64,bytes:&[u8]) -> Result<ObjectDescriptor,StoreStatus>` |
| `StoreHost` | `read_lease(&self,peer:&EditorPeer,descriptor:&ObjectDescriptor) -> Result<ReadLease,StoreStatus>`; `write_lease(&self,peer:&EditorPeer,descriptor:&ObjectDescriptor) -> Result<WriteLease,StoreStatus>` |
| `ReadLease` / `WriteLease` | `copy_into(&self,offset:u32,out:&mut[u8]) -> Result<(),StoreStatus>` / `copy_from(&self,offset:u32,input:&[u8]) -> Result<(),StoreStatus>`; checked bounds and live admission; no backing borrows/pointers/callbacks |
| `FixtureController` | `issue_editor(&self,spec:GrantSpec) -> Result<EditorEndpoint,FixtureError>`; rights subset1|2|4, ttl1..600000; existing session actor pair cannot be rebound to another owner |
| `FixtureController` | `issue_recovery(&self,actor:EditorActorV0,object_id:u64,operation_id:u64) -> Result<EditorEndpoint,FixtureError>`; retained original-owner scope only, rights4, immutable original tuple |
| `FixtureController` | `advance_time(&self,observed_ms:u64) -> Result<u64,FixtureError>`; shared clamped monotone fixture clock; backwards input cannot reduce it |
| `FixtureController` | `retire(&self,endpoint:&EditorEndpoint,reason:RetirementReason) -> Result<(),FixtureError>`; `RetirementReason::{Revoked,ServiceRetired}`; expiry derives controller clock+TTL, not caller assertion |
| `FixtureController` | `pause_next(&self,endpoint:&EditorEndpoint,point:PausePoint) -> Result<PauseToken,FixtureError>`; `release(&self,token:&PauseToken) -> Result<(),FixtureError>` |
| `PauseToken` | `wait_until_entered(&self,timeout_ms:u64) -> Result<BarrierSnapshot,FixtureError>`; `wait_until_settled(&self,timeout_ms:u64) -> Result<BarrierSnapshot,FixtureError>`; timeout1..2000 |
| `FixtureController` | `stop_at_phase(&self,token:&PauseToken) -> Result<(),FixtureError>`; supported one-shot worker stop without future selection IO; never arbitrary thread kill |
| `FixtureController` | `drop_next_reply(&self,endpoint:&EditorEndpoint) -> Result<(),FixtureError>`; original reply suppressed to client as Disconnected, actual internal outcome retained; no dispatch retry |
| `FixtureController` | `fail_next_io(&self,endpoint:&EditorEndpoint,point:IoPoint) -> Result<(),FixtureError>`; one named simulated error before that call, actual preceding IO retained |
| `FixtureController` | `quiesce(&self,timeout_ms:u64) -> Result<QuiescedStoreOwner,FixtureError>`; close admission, await all owned workers/supervisors, actually join outside locks; <=2000 supported cleanup |
| `FixtureController` | `evidence(&self) -> Result<StoreEvidence,FixtureError>`; `seed_counter(&self,object_id:u64,counter:CounterKind) -> Result<(),FixtureError>`; trusted genesis-only exhaustion setup, before any client grant/allocation; persists explicitly seeded coherent genesis metadata with its own instrumentation, never called by clients |
| `StoreFixture` | `corrupt_fenced(token:&mut QuiescedStoreOwner,object_id:u64,fault:StorageFault) -> Result<(),FixtureError>`; exact symbolic owned file targets, retains original witnesses; no arbitrary path/bytes authority |

`CounterKind::{OperationHighWater,WriterHighWater,ServiceEpoch,JournalSequence,
Identity,TimeOrigin}` seed MAX only in a grant-free, allocation-free trusted
genesis fixture. Counter seeding is a separately labeled test setup transition,
not an ordinary service transition or acceptance of a dishonest historical chain. Revision MAX comes from a
trusted initial object with revisionMAX, not rewriting a valid receipt chain.
A populated registry cannot be seeded: it returns NotReady without changing
any file or record. Revision MAX uses InitialObject; all other MAX seeds preserve
the initial blob/owner/signature profile and empty journal, then immediately fence
that issuance domain. The journal digest is read back and the setup event is retained.
There is no hidden counter reset or mutation of an acknowledged receipt history.
OperationHighWater seeding also invalidates the unused current ID range and marks
its cursor exhausted, so the first fresh ID attempts checked MAX+16 reservation
and fails rather than issuing a leftover pre-seed ID. Service-epoch/high-water
MAX failure on reopen retains the fenced token and grants no replacement owner. No production use
or claim that the ordinary lifetime actually reached MAX.
Identity and TimeOrigin are shared registry domains in this fixture. Their
genesis MAX seed may prevent *all* new grants in that registry; they do not claim
an authorized positive read there when no endpoint exists. Each such negative
leg runs beside an independent healthy signed Store fixture and explicitly labels
that positive control as a different registry, proving valid API consumption
rather than shared-registry isolation. Per-object counter MAX legs may use B only
if B's grants remain actually issuable under the unaffected domain; otherwise use
the same explicitly separate healthy control. Populated ordinary lineage tests
cover retained receipt/read positives after16-record capacity exhaustion and
fenced reopen without any MAX seeding. The actual paused-A/usable-B shared-registry
progress claim belongs to the stall case, not to a fresh MAX genesis fixture.


Client dispatch/lease methods have **no now argument**. Absolute response deadline
starts internally at dispatch entry using Instant; semantic expiry uses only the
Store-owned monotonic fixture clock. This refines the plan's schematic `now`
argument into controller-only authority. Clock advancement and retirement use the
same per-object admission mutex as permit issuance; no copied authorization bool
or caller-provided permit callback. A closed-unpermitted source retires exactly;
new explicit source/operation can succeed while an old **pre-permit dispatcher**
remains paused, provided its durable Submitted metadata IO has already completed
and relinquished namespace ownership as specified below. That released dispatcher
retains only its original private ticket/frozen-source snapshot; it cannot later
publish a stale journal image. An original issued permit may finish/quarantine
after retirement.

`dispatch` checks envelope canonical raw handle, protocol/op/direction, exact
payload length, zero outer pad/tail/reserved before data access. Known canonical
requests produce their typed reply, including semantic denials. Malformed known
frames return Err(Invalid) without a fabricated reply; unsupported protocol/op/
direction return Err(Unsupported). Authentication precedes state observation.
Denied/Stale/Timeout/NotReady/Exhausted replies redact all but correlation/status.
Authorized Conflict may expose only current owned revision; Unknown only original
in-flight Commit, own live SaveStatus or RecoveryReceipt, with known operation
and zero revision/receipt. A new pending duplicate Commit is NotReady zero.

Store owns `ArtifactMessage` trait with the same const PROTOCOL/KIND/LENGTH and
encode/decode signatures as UIa's EditorMessage, implemented for these8 generated
types only. Exports `encode_envelope_wire(&Envelope)->Result<[u8;88],StoreStatus>`
and `decode_envelope_wire(&[u8])->Result<Envelope,StoreStatus>`; factor/reuse a
coordinator-owned pure codec if desired, never depend on desktop for Store code.
Raw SHM handle must be Shmem, reserved bits zero, packed round-trip exact, slot
checked16 (1..65535) and generation checked32 (1..u32::MAX), descriptor.object_generation==handle.generation;
selected generation is separate. EditorPeer binds registry/endpoint/class/rights/
owner/session/instance/object/epoch/expiry and is not publicly constructible.
DraftSource write uses no surface mapping generation. Source full byte_len is
64+body len, allocation expected revision/hash from authoritative selected base.
Freeze only the registered same-source generation, under State→source lock order;
no lock spans barrier or IO. Revalidate source header/body/hash and allocation
before permit, not just descriptor numeric equality. Read leases deny DraftSource;
write leases deny Text/Receipt; header/body corruption through an owned unfrozen
writer is consumed and rejected before permit rather than trusted as authority.

## Barrier, IO and fencing evidence

```rust
pub enum PausePoint {
    BeforeAllocationJournal, BeforeSubmissionJournal, BeforeCommitPermit,
    AfterCommitPermit, BeforeCandidateWrite, AfterCandidatePublication,
    BeforeSelectionRename, AfterSelectionRename, AfterAcknowledgement,
    BeforeReadReply,
}
pub enum IoPoint {
    CandidateIntent, CandidateBlobWrite, CandidateBlobSync,
    CandidateManifest, CandidateOwner, CandidateDirectorySync, CandidateReadback,
    JournalWrite, JournalSync, JournalRename, JournalDirectorySync, JournalReadback,
}
pub struct BarrierSnapshot { pub request_id:u64, pub operation_id:u64,
    pub entered:bool, pub released:bool, pub settled:bool, pub permit_issued:bool }
pub enum StorageFault {
    MissingJournal, TruncatedJournal, UnknownJournalVersion, ExtraJournalField,
    DuplicateJournalField, WrongJournalBinding, BrokenJournalChain,
    EarlierAllocatedOnlyJournal, CorruptBlob, UnsignedManifest, WrongSigningKey,
    UnboundPublicationIntent, UnsignedPublicationIntent, SymlinkEntry,
}
```

`BeforeCommitPermit` is entered **after** durable Submitted metadata publication,
readback and release of the namespace IO reservation. The paused original service
dispatcher holds its private original request ticket and immutable source snapshot,
not the per-object namespace writer or a publishable stale journal snapshot.
Namespace IO release does not release the open Submitted admission reservation:
same-object new mutation remains NotReady until sticky closure or settlement. The
timer closes that ticket/source under admission with no IO. A fresh serialized
metadata owner persists the original bound Noncommit/closure (without erasing its
record/source binding), then a new allocation/submission. Its journal image is
built from the current authoritative state and exact serialized reservation.
The old dispatcher, once released, must recheck its sticky closed ticket under
admission, fail permission, and issue **no** metadata or selection IO from its
former view. Cleanup does not unfreeze or reuse the old descriptor. Source snapshot
retention alone cannot select or resurrect it.

This fresh-save-while-paused positive applies at that completed-metadata/prepermit
phase. Earlier metadata IO pauses still hold their namespace reservation and need
supported settlement before a fresh metadata writer proceeds; this is not an
arbitrary syscall cancellation guarantee. After permit, its sole namespace IO
reservation and same-object mutation quarantine remain with the original writer
through the one issued transition. Dispatcher/deadline response work and owned IO
workers are distinct producer roles; no enforcing lock spans any pause or IO.

Barriers are bound to one actual authenticated request/operation, not sleeps or
arbitrary callbacks. One armed/entered barrier per object. Pauses happen between
actual IO calls in the supported worker; they do not claim an OS syscall is
interrupted or that arbitrary filesystem hangs are cancellable. Stop-at-phase
releases that worker into a supported fail/return path and joins it before reopen;
after rename/ack it may only finish bookkeeping, never issue another mutation.

Evidence public structs contain data only: `StoreEvidence { exchanges:Vec<Exchange>,
objects:Vec<ObjectEvidence>, barriers:Vec<BarrierSnapshot>, io_events:Vec<IoEvent>,
ledger:StoreLedger }`. Hard limits256 exchanges,1024 IO events,64 barrier snapshots,
2 object evidence records,32 shared descriptors; no truncation. Each Exchange has
actual canonical request/reply bytes, authenticated actor/class/object and original
request/operation IDs, optional suppressed-reply marker, absolute elapsed micros
and observed Store clock. ObjectEvidence includes current selected record,
allocation/binding/permit/receipt records, dispatch/permit/transition counts,
latest journal digest+sequence, **separate** blob/manifest hashes, ownership digest,
quarantine state and pending writer identity. IoEvent has object/op/writer/sequence,
IoPoint, begin/completed/simulated-failure outcome and affected typed artifact hash;
no event fabricates syscall success. Ledger enforces all plan bounds and operation
counts across reopen. Evidence capacity exhaustion fails the case; diagnostics
remain bounded rather than dropping records to pass.

Exact public evidence field definitions (diagnostic data, not grants):

```rust
pub struct Exchange {
    pub actor: EditorActorV0,
    pub endpoint_class: EndpointClass,
    pub selected_object_id: u64,
    pub request_wire: [u8;88],
    pub actual_reply_wire: Option<[u8;88]>,
    pub reply_suppressed: bool,
    pub operation_id: u64,
    pub elapsed_us: u64,
    pub observed_ms: u64,
}
pub enum IoOutcome { Began, Completed, SimulatedFailure, ActualFailure }
pub struct IoEvent {
    pub selected_object_id:u64, pub operation_id:u64, pub writer_id:u64,
    pub sequence:u64, pub point:IoPoint, pub outcome:IoOutcome,
    pub artifact_hash:Option<Hash32>,
}
pub struct ObjectEvidence {
    pub selected:SelectedArtifactV0,
    pub operations:Vec<EditorOperationRecordV0>, // <=16
    pub mutation_dispatch_count:u32, pub permit_count:u32,
    pub transition_count:u32, pub journal_hash:Hash32, pub journal_sequence:u64,
    pub blob_hash:Hash32, pub manifest_hash:Hash32, pub ownership_hash:Hash32,
    pub mutation_quarantined:bool, pub pending_writer_id:Option<u64>,
    pub service_epoch:u64, pub reopen_witness:Option<ReopenWitness>,
}
pub struct StoreLedger {
    pub sessions:u32, pub selected_objects:u32, pub original_instances:u32,
    pub endpoints:u32, pub shared_objects:u32, pub active_io_workers:u32,
    pub active_supervisors:u32, pub held_service_producers:u32, // <=64 incl closed/paused
    pub operations_per_object:Vec<(u64,u32)>, // <=2
    pub cas_entries_per_object:Vec<(u64,u32)>, // <=2
    pub bytes_per_object:Vec<(u64,u64)>, // <=2
}
pub struct StoreEvidence {
    pub exchanges:Vec<Exchange>, pub objects:Vec<ObjectEvidence>,
    pub barriers:Vec<BarrierSnapshot>, pub io_events:Vec<IoEvent>,
    pub ledger:StoreLedger,
}
```

Privileged read-only accessor on the opaque token:
`QuiescedStoreOwner::fence_snapshot(&self) -> FenceSnapshot`. It copies bounded
producer diagnostic data without IO, clock mutation or new authority. The token
has no public constructor, Clone or serde; constructing/serializing snapshot data
cannot mint one. A test retains this snapshot before consuming the token in reopen.

```rust
pub enum ProducerKind { Dispatcher, Supervisor, ObjectIoWorker }
pub enum ProducerJoinOutcome { Returned, Panicked }
pub struct ProducerFence {
    pub producer_id:u64, pub producer_generation:u64,
    pub kind:ProducerKind, pub native_pid:u32,
    pub rust_thread_id:String, // <=64 bytes from actual held JoinHandle.thread().id()
    pub selected_object_id:Option<u64>,
    pub request_id:Option<u64>, pub operation_id:Option<u64>,
    pub completed_join:bool, pub join_outcome:ProducerJoinOutcome,
    pub settled_barrier:Option<BarrierSnapshot>,
    pub live_ownership_after_join:bool,
}
pub struct OperationClosureWitness {
    pub allocation:EditorSaveAllocationV0,
    pub submitted_binding:Option<EditorSaveBindingV0>,
    pub original_request_id:u64,
    pub original_producer_id:u64, pub original_producer_generation:u64,
    pub closure:EditorClosureV0,
}
pub struct ObjectFenceWitness {
    pub owner_id:u64, pub selected_object_id:u64, pub selected_generation:u64,
    pub service_epoch_at_fence:u64, pub reserved_successor_epoch:Option<u64>,
    pub durable_journal_sequence:u64, pub durable_journal_hash:Hash32,
    pub operation_high_water_min:u64, pub writer_high_water_min:u64,
    pub admission_epoch_min:u64,
    pub known_durable_operations:Vec<EditorOperationRecordV0>, // <=16
    pub issued_runtime_permits:Vec<EditorCommitPermitV0>, // <=16
    pub original_closures:Vec<OperationClosureWitness>, // <=16
}
pub struct FenceSnapshot {
    pub schema_version:u32, pub namespace_id:u64, pub fence_id:u64,
    pub profile_hash:Hash32,
    pub semantic_clock_min:u64, pub identity_counter_min:u64,
    pub service_producers:Vec<ProducerFence>, // Dispatcher+Supervisor combined<=64
    pub object_workers:Vec<ProducerFence>, // ObjectIoWorker<=2
    pub objects:Vec<ObjectFenceWitness>, // <=2
    pub live_ownership_after_join:u32, // exactly0
}
pub struct ReopenWitness {
    pub fence_id:u64, pub fence_snapshot_hash:Hash32,
    pub selected_object_id:u64, pub selected_generation:u64,
    pub prior_service_epoch:u64, pub actual_service_epoch:u64,
    pub reopened_journal_sequence:u64, pub reopened_journal_hash:Hash32,
}
```

Producer IDs/generations are checked positive registered identities bound to
actual handles held at the start of this quiescence, and the Rust thread ID is
captured from that exact JoinHandle before consuming it in join. It is not a
claimed native TID or process-containment proof. Every returned ProducerFence has
completed_join=true, an actual terminal Join result (Returned or Panicked),
live_ownership_after_join=false, and any associated entered barrier is settled
with matching request/operation identity. Panicked reports the real join error;
it does not infer successful IO or a selected outcome. Object worker identity
matches its held reservation; service producer identities match their registered
request tickets. Snapshot counts/uniqueness/kind partition are checked and the
total held-handle snapshot is<=66. It contains exactly handles held for this
quiescence, not an accumulated all-time join history; earlier joined handles need
not be relisted. The private owner still retains bounded original operation
witnesses separately. No unfinished/live handle can be omitted to manufacture a
token. A timeout creates no FenceSnapshot/token and retains charged handles.

The diagnostic snapshot digest is SHA256 of its compact typed serialization in
the declaration field order, producer vectors sorted by registered identity,
objects by selected ID and operation witnesses by original operation ID. Version
is1; all declared bounds, tuple/terminal-join checks and zero live ownership must
pass before encoding. This is an evidence digest, not an authority credential.
`reserved_successor_epoch` is checked service_epoch_at_fence+1 when representable,
or None at MAX; it is **proposed only**, never an actual reopened epoch. Successful
reopen supplies ObjectEvidence.service_epoch and its ReopenWitness after real
journal publication/sync/readback, binding fence_id+snapshot hash, unchanged object
identity, prior epoch and actual epoch equal to the checked successor. The actual
reopened journal sequence/digest must agree with object evidence. Failed reopen
returns its retained token preserving known witnesses and emits no actual-success
epoch witness.
This is one most-recent reopen witness per object, not a growing epoch history.

Pending duplicate/changed tuple/lifecycle authentication happens before the
active-invocation reservation can emit NotReady, so a capacity guard cannot hide
foreign Denied or revoked Stale. An original deadline response releases **only**
the per-endpoint active-invocation guard. Every still-held Store-owned dispatcher
or supervisor handle remains charged to the shared64-held-producer cap until its
actual join, even after closure or deadline response. Fresh save uses a remaining
producer slot; it cannot evade this cap by leaving paused originals behind. The
separate per-object IO worker reservation persists while its permit is unsettled.
SaveStatus can observe the original Unknown without allowing another writer.
Request producers are held/joined;
public response completion is not evidence of worker quiescence. Attempted reopen
is demonstrated by `quiesce(1)` returning privileged `status=NotReady` while the
entered supported writer remains live, with no token created and no epoch change.
Its controller can still release/stop that exact barrier and retry quiescence.

`QuiescedStoreOwner` privately retains exact root/profile and bound object identities,
all actual joined IO-worker/dispatcher/supervisor proofs, last complete validated durable journal
snapshot per object, all observed durable Submitted/Permitted bindings, current
issued immutable permits/closures, per-counter/epoch/high-water minima and known
acknowledged outcomes. Those collections are bounded by16 operations/object and
the one-worker slot, not an ever-growing list of every journal file. The latest
durable sequence+digest and complete record snapshot subsume prior durable transition
witnesses: same sequence needs same digest; later sequence must monotonically
preserve every known binding/permit/receipt, capacity count and counter minimum.
An earlier coherent Allocated-only image cannot erase a known Submitted/Permitted
tuple. Permit state may reconcile to Noncommit only after fencing with the immutable
permit evidence retained. A possible later unacknowledged transition may be admitted
only if complete, valid, bound to the exact original writer and compatible with
all retained witnesses. The token is not serializable, caller minted, or proof of
an independently reopened/rolled-back storage image without these witnesses.

Before DomainArtifactRegistry::new or publish_owned (both auto-recover), Store
preflight enumerates<=64 regular no-symlink CAS entries and bounds file lengths,
strictly parses every pending intent/owner/manifest and checks RequireSignature,
pinned key/kind/channel, canonical content ID+size, foreign/global ownership and
permitted-operation source binding. Missing candidate bytes are allowed only in
the exact valid prepublication stage; existing bytes must match before helper
mutation. Duplicate/corrupt/unattributed metadata fails before auto-recovery.
Initial publication has an explicit private `ProvisioningBinding { owner,object,
generation,initial revision,blob hash,len,manifest hash,profile hash }` created
by StoreFixture from validated initial config, never from a client. Later intents
must match a retained permitted operation (disk or fenced original runtime permit
evidence), not merely any signature-valid blob. After successful helper recovery,
bounded authenticated consumed-snapshot readback is still required. If these
preconditions cannot be proved with unchanged helpers, stop for exact owner-scoped
hardening rather than mutating first and validating afterward.

## Exact assertion affordances and UIa reuse

Exactly7 test names remain those in the plan, no additional b test inventory:

| Name suffix (`ui1_1_` prefix) | Actual API needed |
|---|---|
| `revoke_paused_key_frame_and_save` | UIa HostDesktop/FixtureController before-key/present barriers; Store draft/source lease, allocation/Commit, before-permit barrier, retire/clock expiry, source stale and fresh explicit operation; separate after-permit original Unknown |
| `stalled_session_leaves_other_session_usable` | Store after-permit/candidate barrier, real1000ms supervisor reply, B independent object typed read/save/read plus actual UIa B input/frame, release/join and original status lookup |
| `clock_capacity_and_counter_limits_fail_closed` | Controller-only clock, bounded grants/sources16 retained allocation records, dropped allocation reply, fenced reopen range gaps/no eviction, trusted counter/genesis MAX fixtures, UIa finite queues/frames |
| `store_commit_reopen_preserves_prior_blob` | Signed genesis provisioning, actual typed8-message codec, immutable data leases, real files/owner/publication/journal/readback, quiesce token+reopen and fresh read; rights/foreign/collision/signature negatives |
| `store_same_base_conflict_and_operation_reuse` | Two issued independent sessions same owner/object, before-permit barrier, source tuples and Allocate/Commit, pending duplicate NotReady/settled receipt/revoked duplicate Stale, original counters |
| `store_lost_reply_reconciles_without_mutation_replay` | Drop-next reply, before/after-permit/rename pauses, original Known-op Unknown, same-object quarantine, SaveStatus/recovery-only lookup, prepermit source cleanup and distinct explicit new save |
| `store_recovery_validates_atomic_selection_and_receipt` | All four supported phases, entered+settled/join proof, unquiesced reopen refusal, actual close/reopen, corruption symbolic fixture and retained-token downgrade checks, helper auto-recovery intent negatives before metadata mutation |

UIa usable public types are `HostDesktop`, `FixtureController`, `HostConfig`,
`SessionFixture`, `EditorBindings`, `EditorClient`, `DraftSnapshot`, `ComposedFrame`,
`PausePoint`, `PauseToken`, `Evidence`, `ObjectDescriptor` and `dev::Status` under
feature `desktop_v0_dev`. Existing test helpers are in
`services/desktop/tests/support/editor_host_assertions.rs`, not a public crate
module: Fixture::new/ttl/launch, Driver::register/key/text, render, revoke,
assert_volatile_label/assert_caps and Record. A Store integration test may use
`#[path="../../desktop/tests/support/editor_host_assertions.rs"] mod ui_a;`
from its `services/store_service/tests/editor_store.rs` file, retaining original
font includes and hashing this exact test-only source. No test functions are
included by that module, so the b case count remains7. Store dev-dependency on
desktop_service with `desktop_v0_dev` is coordinator-owned and test-only; no Store
production dependency or source/API edit in UIa. Helpers Record use
`RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE`; the coordinator must route these outputs to
a dedicated labeled b subdirectory and use a b consumer, not forge UIa output or
run two gates against shared fixed paths. UIa time parameters remain trusted
fixture inputs and do not become Store client clock authority. Host/UIa registry
identities are explicitly correlated by controller GrantSpec, not treated as
cross-registry authentication. No integrated forwarding task is claimed.

Required coordinator exports/features: schema editor_save module; Store cfg module
and default-off feature; Store integration-test dev-dependency; any pure codec
extraction; gate/recipe/evidence schema registration. No new dependency, kernel
change, IDL change or codegen expected. The frozen exact **held** service-producer
bound is64 for dispatchers and supervisors combined, reserved before spawn and
released only on actual join, plus<=2 per-object IO workers. One current invocation
per endpoint is a separate guard and there is no automatic retry. Use an internal
bounded service executor/producer with the caller performing recv_timeout; the
caller thread is not Store-owned and confers no clock/permit authority. MAX seeding is
genesis-only setup as specified above, not a generalized live-history setter.
These prerequisite choices are independently reviewed and coordinator-frozen. A withheld
response/blocked worker cannot create unbounded detached work; full reservation
returns Exhausted or same-endpoint NotReady. These details need reviewed executable
RED before handlers. This contract claims no executable or runtime acceptance.
