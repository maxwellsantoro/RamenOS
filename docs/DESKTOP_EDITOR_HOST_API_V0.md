# Desktop editor host API v0: UI1.1a

**Status:** independently reviewed and coordinator-frozen API; implemented default-off host fixture with 13 passing assertions on macOS/Linux.

**Base:** `c1e7f8d0d60062ea21872480fcc1aeacec45e8fc`, with registered [wire v1](DESKTOP_EDITOR_WIRE_V1.md).
**Scope:** default-off `desktop_v0_dev`, in-process Rust consumer, volatile selected objects and offscreen composition.

The [reviewed proposal](plans/desktop-editor-v0.md) owns unchanged layouts, resource bounds, task and save-permit semantics. This packet resolves its host API/corpus choices; it changes no IDL. Protocol 336 and `dev::DevDesktop` remain unchanged. Types live in `desktop_service::editor_dev`; module registration, shared implementation paths and gates belong to the coordinator. This is registry enforcement in a trusted host fixture, with no actual editor process, Store IO, native IPC, OS containment, target, device or persistence evidence.

## Public types and codec boundary

Reuse `dev::Status` (0–11). All IDs below are `u64`, all hashes `[u8;32]`, all deadlines milliseconds. Fields of authority-bearing types are private; `Clone` duplicates an identity/lease, never authority. Diagnostic enum numbers are not caller authorization.

```rust
use kernel_api::{cap::Handle, ipc::Envelope};
use desktop_service::dev::Status;
#[repr(u32)]
pub enum EndpointClass { SelfStatus=1, FocusRead=2, Surface=3, Artifact=4,
    Chrome=5, InputProducer=6, Compositor=7, RecoveryReceipt=8 }
#[repr(u32)]
pub enum ObjectKind { Preview=1, Grants=2, SelectedText=3, DraftSource=4,
    SurfaceBuffer=5, Receipt=6, ComposedFrame=7 }
#[repr(u32)] pub enum PixelFormat { Bgra8888=1 }
#[repr(u32)] pub enum KeyPhase { Press=1, Release=2, Reset=3 }
#[repr(u32)] pub enum InstanceState {
    Starting=1, Running=2, Exited=3, Faulted=4, Revoked=5, Expired=6 }
#[repr(u32)] pub enum ReceiptOutcome { Committed=0, DefinitiveNoncommit=1 }
pub enum SaveState { Unsaved, Saving{operation_id:u64}, Saved{revision:u64},
    Conflict, Unknown{operation_id:u64} }
pub struct PeerContext { /* private registry, endpoint, owner, session/gen,
    instance/gen, class, resource/gen, service epoch, expiry */ }
#[derive(Clone)] pub struct Endpoint { pub peer: PeerContext, pub handle: Handle }
pub struct ReadLease { /* private identity/kind/generation/expiry */ }
#[derive(Clone)] pub struct WriteLease { /* private identity + mapping generation */ }
pub struct ObjectDescriptor { pub handle:Handle, pub kind:ObjectKind,
    pub object_generation:u64, pub byte_len:u32 }
pub trait EditorMessage: Sized {
    const PROTOCOL:u32; const KIND:u32; const LENGTH:usize;
    fn encode(&self, endpoint:Handle) -> Result<Envelope,Status>;
    fn decode(frame:&Envelope) -> Result<Self,Status>;
}
pub fn encode_envelope_wire(frame:&Envelope)->Result<[u8;88],Status>;
pub fn decode_envelope_wire(bytes:&[u8])->Result<Envelope,Status>;
```

Implement the message trait for all 43 generated types, with explicit initialized LE fields. Direction is checked by dispatch, not inferred from trait membership. Validate protocol/op/exact length/reserved/payload tail/outer pad/canonical raw handle before decoding or looking up data. Unsupported protocol/op returns Unsupported; malformed known frames return Invalid; authenticated wrong class/right/owner returns Denied; retired owned identity returns Stale. Authentication failure takes precedence over disclosing lifecycle state. Failure replies expose only request correlation (Input sequence) and status; every other field is zero. Authorized Conflict may expose its own current revision, never foreign data. Unknown exposes a known operation ID only in the authenticated original in-flight reply, authenticated LIVE Artifact/4 SaveStatus for its own original unsettled operation, or an owner-bound RecoveryReceipt lookup, with no result revision/receipt. A new request through a revoked endpoint returns Stale with zero data; foreign or unallocated operations return Denied with zero data. Local codec errors carry no reply/data.

## Fixture controller, consumer and lease signatures

```rust
pub struct HostConfig { pub preview_ttl_ms:u64, pub instance_ttl_ms:u64,
    pub font_rows:[[u8;16];96] } // glyphs ASCII 32..127; immutable fixture input
pub struct FixtureController { /* private owner of this registry; never a client */ }
pub struct HostDesktop { /* private shared registry + finite deadline supervisor */ }
pub struct SessionFixture { pub session_id:u64, pub session_generation:u64,
    pub chrome:Endpoint, pub input:Endpoint, pub compositor:Endpoint,
    pub device_generation:u64 }
pub struct EditorBindings { pub bootstrap:Envelope, pub self_status:Endpoint,
    pub focus_read:Endpoint, pub surface:Endpoint, pub artifact:Endpoint }
pub struct DraftSnapshot { pub bytes:Vec<u8>, pub cursor:usize,
    pub selection:Option<(usize,usize)>, pub first_visible_line:usize,
    pub confirmed_revision:u64, pub save:SaveState }
pub struct StepObservation { pub consumed_sequence:Option<u64>,
    pub draft_changed:bool, pub frame_sequence:Option<u64>, pub save:SaveState }
pub struct ComposedFrame { pub session_id:u64, pub instance_id:u64,
    pub focus_epoch:u64, pub sequence:u64, pub bgra:Vec<u8> }
pub enum PausePoint { BeforeKeyDelivery, BeforePresent, BeforeCommitPermit,
    AfterCommitPermit, BeforeReadReply }
pub struct PauseToken { /* private registry + bound request/operation */ }
pub struct BarrierSnapshot { pub request_id:u64, pub operation_id:u64,
    pub entered:bool, pub settled:bool, pub permit_issued:bool }
impl PauseToken {
    pub fn wait_until_entered(&self, timeout_ms:u64)->Result<BarrierSnapshot,Status>;
    pub fn wait_until_settled(&self, timeout_ms:u64)->Result<BarrierSnapshot,Status>;
}
pub enum ServiceKind { Focus, Compositor, Artifact }
pub struct ResourceLedger { pub sessions:u32, pub selected_objects:u32,
    pub instances:u32, pub endpoints:u32, pub shared_objects:u32,
    pub queued_keys:u32, pub operations_per_object:Vec<(u64,u32)> }
pub struct Evidence { pub exchanges:Vec<(Envelope,Envelope)>,
    pub ledger:ResourceLedger, pub permit_count:u32, pub transition_count:u32,
    pub mutation_dispatch_count:u32, pub volatile_backend:bool }
pub struct EditorClient { /* private local draft/cursor, endpoints and save ID */ }
impl HostDesktop {
    pub fn new(config:HostConfig)->Result<(Self,FixtureController),Status>;
    pub fn dispatch(&self, peer:&PeerContext, request:&Envelope, now_ms:u64)->Envelope;
    pub fn read_lease(&self, peer:&PeerContext, descriptor:&ObjectDescriptor,
        now_ms:u64)->Result<ReadLease,Status>;
    pub fn write_lease(&self, peer:&PeerContext, descriptor:&ObjectDescriptor,
        mapping_generation:u64, now_ms:u64)->Result<WriteLease,Status>;
    pub fn draft_source(&self, peer:&PeerContext, expected_revision:u64,
        bytes:&[u8], now_ms:u64)->Result<ObjectDescriptor,Status>;
    pub fn compose_next(&self, compositor:&Endpoint, now_ms:u64)
        ->Result<ComposedFrame,Status>;
    pub fn maintenance(&self, now_ms:u64)->Result<(),Status>;
}
impl ReadLease {
    pub fn copy_into(&self, offset:u32, output:&mut[u8], now_ms:u64)->Result<(),Status>;
}
impl WriteLease {
    pub fn copy_from(&self, offset:u32, input:&[u8], now_ms:u64)->Result<(),Status>;
}
impl FixtureController {
    pub fn register_session(&self, owner_id:u64, application_hash:[u8;32],
        manifest_hash:[u8;32], selected_bytes:&[u8], now_ms:u64)
        ->Result<SessionFixture,Status>;
    pub fn editor_bindings(&self, instance_id:u64)->Result<EditorBindings,Status>;
    pub fn set_policy_revision(&self, session_id:u64, revision:u64)->Result<(),Status>;
    pub fn set_application_identity(&self, session_id:u64, application_hash:[u8;32],
        manifest_hash:[u8;32])->Result<(),Status>;
    pub fn set_selected_identity(&self, session_id:u64, bytes:&[u8],
        revision:u64, now_ms:u64)->Result<(),Status>;
    pub fn detach_keyboard(&self, session_id:u64, now_ms:u64)->Result<(),Status>;
    pub fn reattach_keyboard(&self, session_id:u64, now_ms:u64)
        ->Result<(Endpoint,u64),Status>;
    pub fn pause_next(&self, session_id:u64, point:PausePoint)->Result<PauseToken,Status>;
    pub fn release(&self, token:&PauseToken)->Result<(),Status>;
    pub fn fault_editor(&self, instance_id:u64, now_ms:u64)->Result<(),Status>;
    pub fn restart_service(&self, service:ServiceKind, now_ms:u64)->Result<(),Status>;
    pub fn recovery_receipt(&self, session_id:u64, operation_id:u64)
        ->Result<Endpoint,Status>;
    pub fn evidence(&self)->Result<Evidence,Status>;
    pub fn seed_exhaustion(&self, counter:CounterKind)->Result<(),Status>;
}
pub enum CounterKind { Identity, FocusEpoch, QueueSequence, MappingGeneration,
    SelectedRevision, ServiceEpoch, TimeOrigin }
impl EditorClient {
    pub fn new(host:HostDesktop, bindings:EditorBindings, now_ms:u64)->Result<Self,Status>;
    pub fn step(&mut self, now_ms:u64)->Result<StepObservation,Status>; // <=1 key
    pub fn render(&mut self, now_ms:u64)->Result<u64,Status>; // owned frame sequence
    pub fn snapshot(&self)->DraftSnapshot; // local copy, no broker authority
}
```

`HostDesktop` and `PeerContext` implement Clone; the controller is not cloneable or obtainable from a client. Lease reads/writes copy bounded bytes without callbacks, pointers, backing borrows or unrestricted create/map/close. `draft_source` requires Artifact REPLACE, binds the selected base, validates ASCII and hash, and replaces at most one unsubmitted draft-source allocation per instance; frozen/in-flight source cannot be replaced. Read leases cannot map DraftSource; write leases cannot map Preview/Grants/Text/Receipt/ComposedFrame. The controller is trusted test setup/instrumentation, never a payload endpoint. It cannot issue a confirmation event; approval must pass through Input→Focus→Chrome. Evidence APIs do not authorize effects.

Coordinator clarification: descriptor object_generation is the canonical SHM handle's checked32-bit generation widened to u64; ReadSelectedReply.object_generation must equal it. Slot/generation allocation rejects overflow before packing, never truncates. Selected/surface/mapping/service generations remain separate. Lease reads require Chrome/1 own live Preview, SelfStatus/1 own live Grants, Artifact/1 own SelectedText, Artifact/4 or RecoveryReceipt/4 own original Receipt, or Compositor/1 own frozen SurfaceBuffer/current ComposedFrame. Writes require Surface/2 current Acquire mapping for SurfaceBuffer or Artifact/2 bound base for DraftSource; all other combinations deny. Shared bytes/descriptors never grant authority.

DraftSource-only mapping_generation=0 is the non-surface sentinel: require Artifact/2, own live selected-base binding, descriptor generation and unfrozen/unsubmitted source. SurfaceBuffer always requires its nonzero active Acquire generation. Commit validates complete schema/hash/length/expected base against registry binding after authorized source edits; header bytes never authorize access.

Pause waits use a separate fixture condition variable and timeout 1..2000ms, never an enforcing registry/session/object lock. Entered means the bound dispatch reached that precise barrier; tests must observe entered before revoke/expire and settled after release. Operation ID is zero for non-save barriers. Release is one-shot; repeated release is Stale. Snapshot/entered/settled ordering is retained in evidence, including permit issuance. Neither a sleep nor merely arming `pause_next` proves entry.

Fixed finite registry: 2 sessions/selected objects, 16 instance records, 64 endpoint slots, 32 shared-object slots; checked handle indices/generations are never silently reused. Each instance has 4 issued endpoints, one reserved surface identity, 2 buffers, one source and one selected-read descriptor. Read descriptors reuse the current immutable revision object; superseded descriptor identity retires. Preview/grants allocate at most one each per session/instance; receipts live in 16 retained operation records per object, and materialize at most one current read descriptor per session. Reserve all admission slots before success, release ordinary objects on retirement, retain terminal instance/operation evidence. Admission at a full table returns Exhausted without partial grants/effects. Up to 256 exchanges and 16 composed frames per test evidence scope; evidence exhaustion fails the case, never truncates into PASS.

## Authority, lifecycle and surface ownership

The registry binds every endpoint/descriptor/lease to owner, session/gen, instance/gen, service epoch, resource/gen, rights and expiry. Peer fields have no public constructor/default/deserializer; the trusted controller issues registered Chrome/Input/Compositor contexts and issued bindings derive editor contexts. Payload class numbers, grants bytes and a copied handle cannot construct a peer. New UI1.1 allocation below is protocol/class scoped; it does not reinterpret UI1.0 Chrome mask127. Every operation also checks its bound resource/owner/lifetime. Unlisted class/operation combinations deny, regardless of equal numeric rights.

| Class / bit | Exact operations and additional scope |
|---|---|
| SelfStatus / 1 | Editor GetStatus, ObserveFocus; own instance only |
| FocusRead / 1 | Focus PollKeys; own currently focused epoch only |
| Surface / 1,2,4,8 | Surface Create, Acquire, Present, Destroy respectively; owned reserved surface only |
| Artifact / 1 | ReadSelected; own selected object |
| Artifact / 2 | AllocateSaveId, Commit, local draft_source/write lease; own selected base/source |
| Artifact / 4 | SaveStatus/read receipt; own original operation |
| Chrome / 1 | Editor PrepareLaunch; owned session/pinned application |
| Chrome / 2 | Editor ConfirmLaunch **and** Focus Assign; current trusted approval for Confirm, owned ready surface for Assign |
| Chrome / 4,8,16,32,64 | Editor CancelPreview, GetStatus, CloseInstance, RevokeInstance, PrepareRestart respectively; owned session/instance |
| Chrome / 32 | Surface Destroy for owned-session recovery; never foreign surface; ordinary editor Destroy requires Surface/8 |
| InputProducer / 1 | Input Attach, ProduceKey; owned current device/queue generation, no app destination |
| Compositor / 1 | Surface Consume and compose_next; owned frozen app frame or owned recovery output (no app Consume for recovery) |
| RecoveryReceipt / 4 | Artifact SaveStatus/read original receipt only; original owner/operation, no mutation or Surface Destroy |

Issued editor masks remain 1/1/15/7 (profile63); trusted Chrome=127, InputProducer=1, Compositor=1, RecoveryReceipt=4. ObserveFocus is SelfStatus only; Chrome reads focus through GetStatus. Registry-owned retirement may internally destroy surfaces without issuing a caller endpoint. No editor class permits Assign/Produce/Prepare/Confirm/Restart/Close/Revoke.

Confirm allocates concrete surface_id/generation and owner metadata; Surface Create consumes that single reservation and allocates buffers. A timed-out creation keeps its unresolved slot quarantined. Create descriptors alone confer no writable lease. Acquire grants one buffer generation; Present under its surface lock rechecks admission and retires every writer alias before acknowledging. Frozen pixels stay immutable while `compose_next` copies them; it issues generated Consume using the compositor context, releasing the frozen slot only after copying. No lock/borrow survives a pause. Reacquire after Consume uses a fresh generation. At most one frozen unconsumed frame/surface; either buffer may be acquired only when free, with one writer per buffer. Monotonic Present sequence starts at 1, strictly increases; duplicate/stale sequence, index>1 and wrong generation fail without replacing the last frame. Destroy retires all aliases/reservation. Compose clips app pixels to rows 48..527 and never maps its chrome/status output into the editor.

On terminal/recovery state change, compose_next may emit one chrome-only recovery frame: owned positive session/retired-instance IDs, blank white app crop, focus_epoch=0 and sequence=0 together as its reserved sentinel. It issues no app Consume/grant; unchanged repeated compose is NotReady. Chrome displays DRAFT LOST and VOLATILE / IN-PROCESS; unsettled permitted save additionally displays SAVE UNKNOWN, never guessed outcome/base. This actual recovery frame counts toward the16-frame evidence budget.

All effective time updates/comparisons and TTL origins clamp to the shared monotonic clock under the enforcing lock, including lease access, detached work and service maintenance. Preview TTL 1..30000 and instance TTL 1..600000; checked time addition fails closed. Fixed 1000 ms real absolute frame/artifact response deadlines use an independent supervisor, not accessor/maintenance polling. Before-permit delayed work rechecks current admission and cannot deliver keys/frames/selection after retirement. After-permit save timeout/revoke returns Unknown, retaining one original permitted transition and quarantining same-object mutation; it may settle after retirement. No mutation replay or guessed noncommit. Pause fixtures are finite owned barriers released by tests; arbitrary hung Rust threads cannot be killed/reaped and may quarantine slots until teardown. No global/session/object lock spans blocking work, rendering or deadline wait.

Focus/compositor restart increments checked service epochs, clears held keys/queues/approval, retires old input attachments, surfaces and editor grants, and faults affected instances. Fresh restart preview/approval is required; unsaved local draft is discarded. Artifact restart with an unsettled permit returns NotReady, preserving original state; it cannot replace that writer. After supported completion, recovery validates the retained original volatile receipt before reopening. RecoveryReceipt authorizes only that owner's original operation and no mutation/enumeration. Clock/counter exhaustion retires affected admission rather than wrapping. Ordinary descriptor retirement cannot erase copies already observed by a client.

## Exact logical keyboard and renderer fixture

This is a host event schema, not USB HID reports or controller evidence. Values below intentionally resemble the familiar usage numbering; only this table is accepted. Modifiers are CTRL=1, SHIFT=2, all other bits invalid. Modifier usage 224=Ctrl,225=Shift; phase field is Press=1/Release=2/Reset=3. Each attachment retains **producer pressed/modifier state** separately from focus delivery/approval state. `modifiers` equals producer held state **after** applying this transition. Duplicate press, unheld release and inconsistent modifiers are Invalid without changing either state/draft. Reset requires usage=0/modifiers=0 and clears both states. No repeat, CapsLock, Alt or locale inference.

| Usage | Unshifted / shifted text or action |
|---|---|
| 4..29 | `a`..`z` / `A`..`Z`, ascending |
| 30..38,39 | `1`..`9`,`0` / `!@#$%^&*()`, corresponding positions |
| 40,41,42,43,44 | Enter=LF, Escape, Backspace, Tab, Space (Shift unchanged) |
| 45,46,47,48,49 | `-/_`, `=/+`, left bracket/left brace, right bracket/right brace, backslash/vertical bar |
| 51,52,53,54,55,56 | `;/:`, apostrophe/quotation mark, grave/tilde, `,/<`, `./>`, `//?` |
| 74,77,79,80,81,82 | Home, End, Right, Left, Down, Up (Shift extends selection) |
| 224,225 | Ctrl, Shift; never insert text |

All other usages (including 50 and Delete=76) return Unsupported with no draft effect. Ctrl+A selects all; Ctrl+S saves; Ctrl+Q closes; Ctrl+R requests recovery preview; Ctrl+Esc routes to launcher. These are the only Ctrl combinations; unsupported combinations preserve draft. Releases insert nothing. Home/End are line bounds, arrows clamp at document/line bounds; vertical movement preserves preferred column. Printable bytes 32..126 plus LF/tab are accepted; length overflow preserves the entire prior draft/selection/cursor. Tab stores byte9 and renders to next 4-column stop. Glyph rows are MSB-left 8×16, only rows for 32..126 may render; newline advances a line. Viewport uses 80 columns×30 rows with visual wrapping at column80; cursor scrolls into view. The assertion packet pins one immutable readable font corpus/hash and expected composed pixel samples before handler implementation; font data is configuration, not authority.

Render colors are opaque BGRA: app background `[255,255,255,255]`, glyph/cursor `[0,0,0,255]`, selected-cell background `[255,192,128,255]`, chrome/status `[224,224,224,255]`. Paint selection before glyphs and a 2×16 cursor last at its cell's left edge; newline has no glyph. The reserved indicator is compositor-owned x8..23/y8..23, black border with green `[0,192,0,255]` interior. These fixture pixels and the visible `VOLATILE / IN-PROCESS` label are immutable app-excluded output. No blink/time-dependent raster; render is bounded independently of evidence retention.

Producer sequence starts 1 per attachment, must be contiguous on accepted ordinary events, and never wraps. Focus epoch changes flush queued events and clear **delivery** modifiers/approval, retaining producer pressed/modifier state and marking every currently pressed usage suppressed until its accepted Release. Those releases validate/update producer state but are not delivered into the new focus. A held Ctrl cannot affect new delivery until released and pressed again; delivery modifiers are recomputed from unsuppressed held modifiers, not blindly copied from the producer field. Thus EnterPress selecting the app can change focus, its legitimate Release40 remains accepted, and Press40 without that release is still a duplicate rather than fresh approval.

Queue capacity64: an accepted event that would overflow the ordinary queue advances producer sequence to that event's actual sequence, clears both held states and queued events, and reserves Reset with that same sequence/usage0/modifiers0. Ordinary production is NotReady until Reset consumed; no sequence/state advance on that rejection. The next accepted event uses its checked successor (65→66 is only the initial-fill example). Reset never counts as a release→press approval witness. Delivery sequences increase with gaps after flush. Detach retires producer/queue and clears both states; reattach gets fresh generations. Chrome routes reserved Ctrl actions and Enter/Escape while launcher/preview/recovery has focus; editor polls only its current epoch. Selection Press40 creates preview but cannot approve it. Approval requires a later accepted Release40 of that selecting press and then a new unmodified Press40 in the current preview epoch, with no intervening reset/cancel/identity change. Both events must follow preview creation; queued/same-press Enter cannot approve. It records one revision/epoch/generation/expiry-bound event and dispatches Confirm. If reset invalidates that cycle, return to launcher and require a fresh selection/approval cycle. No public injection helper substitutes for this route.

Recovery route distinction: Ctrl+R already selects the owned app and creates its recovery preview. After accepted releases of that selector chord, one fresh unmodified Enter press after the preview epoch/origin confirms. No never-pressed Enter release or extra selection-only Enter is required. Any Enter held/queued at preview creation is suppressed until its actual release and later press. Both routes require the same single-use revision/epoch/generation/expiry binding.

## Reserved artifact backend boundary (no Store implementation)

```rust
pub struct SelectedObjectId { /* private selected object + generation */ }
pub struct ObjectAdmissionState { /* private authoritative per-object Mutex state */ }
pub struct CommitClaim { /* private Arc<ObjectAdmissionState> live reference,
    shared registry clock reference, bound peer/source/operation/base/expiry;
    no cached permission boolean or epoch substitutes for the live state */ }
pub struct CommitPermit { /* private immutable unique writer and full bound transition */ }
pub struct ArtifactSnapshot { pub revision:u64, pub content_hash:[u8;32], pub bytes:Vec<u8> }
pub struct ReceiptSnapshot { pub schema_version:u32, pub total_len:u32,
    pub outcome:ReceiptOutcome, pub owner_id:u64, pub session_id:u64,
    pub session_generation:u64, pub original_instance_id:u64,
    pub original_instance_generation:u64, pub selected_object_id:u64,
    pub selected_generation:u64, pub operation_id:u64, pub expected_revision:u64,
    pub result_revision:u64, pub backend_service_epoch:u64, pub committed_at_ms:u64,
    pub expected_content_hash:[u8;32], pub source_content_hash:[u8;32] }
pub trait SelectedArtifactBackend: Send + Sync {
    fn read(&self, object:&SelectedObjectId)->Result<ArtifactSnapshot,Status>;
    fn admit(&self, claim:CommitClaim)->Result<CommitPermit,Status>;
    fn finish(&self, permit:CommitPermit)->Result<ReceiptSnapshot,Status>;
    fn original_receipt(&self, object:&SelectedObjectId, operation_id:u64)
        ->Result<ReceiptSnapshot,Status>;
}
```

This trait is service-owner-only, never exposed to editor/context constructors. CommitClaim retains the live `Arc<ObjectAdmissionState>` from authenticated dispatch, not a copied authorization verdict. Its **admission mutex** is authoritative for bound active rights/owner/generations/service epoch/expiry, operation reservation and selected revision; revocation, deadline closure, policy retirement and service retirement update that same state under that mutex. `admit` locks it, reads current shared monotonic time, rechecks all bindings/live rights/epoch/expiry and expected revision/hash, reserves the checked successor and issues one permit. A claim paused before admit cannot retain permission after retirement. No blocking wait/IO or global/session lock is held there. `finish` consumes the non-Clone permit for one transition; definitive noncommit requires fencing evidence, otherwise Status::Unknown. Receipt publication and revision selection are one transition. AllocateSaveId reserves one of16 non-evicted records before Commit; an authenticated live exact duplicate retrieves the original result without another permit, changed binding denies. UI1.1a uses only trusted volatile implementation; no caller-supplied backend/IO callback. UI1.1b must freeze the actual Store owner/locking adapter before implementation; the trait alone establishes no durable atomicity.

## Frozen UI1.1a case inventory and observations

Every case requires a valid producer/consumer witness and zero-data denied replies plus prior-state/resource comparison. All 13 names run exactly once, no ignored/extra cases. Commands remain **proposed until coordinator registration**: `just foundry-desktop-editor-host-ui1-1a`, output `out/desktop/ui1-1a/`.

| Exact assertion | Required observable result |
|---|---|
| `ui1_1_keyboard_edits_ascii_and_renders_owned_frame` | Actual generated press/releases drive `note=old\n`→`note=new\n`; cursor, selection, scroll and independently pinned composed pixels/sequence agree. |
| `ui1_1_ascii_bounds_preserve_prior_draft` | Printable/LF/tab and 4096 edge work; bad usage/modifiers/byte schema and overflow preserve prior draft/cursor/selection. |
| `ui1_1_focus_routes_reserved_keys_to_trusted_chrome` | Input→Focus→Chrome fresh Enter succeeds; endpoint-only confirm, spoofed/foreign/nonproducer/editor privileged requests deny; editor never receives reserved approval keys. |
| `ui1_1_focus_epoch_reset_overflow_and_detach` | Queued Enter/modifiers/releases cannot cross focus; event65 yields prioritized reset, no approval; detach/reattach stale contexts deny and fresh input works. |
| `ui1_1_surface_freeze_alias_quota_and_chrome_clip` | Retained writer clones deny after Present; Consume then fresh Acquire works; invalid seq/index/quota preserves frame; adversarial app pixels cannot overwrite chrome/indicator. |
| `ui1_1_wire_schema_descriptor_and_identity_fail_closed` | All five protocols' malformed direction/length/reserved/tail/pad/raw handle and bad schema/hash/range/stride/owner/gen fail before data use; valid neighboring request works. |
| `ui1_1_editor_grants_are_exact_and_preview_single_use` | Profile63 and four masks1/1/15/7 match actual registry grants; missing/replayed/stale/policy/manifest/artifact/expired approval cannot launch; fresh approval works. |
| `ui1_1_volatile_save_and_reopen_are_explicitly_labeled` | One volatile operation/receipt advances revision and fresh reopen reads new text; trusted chrome/output/evidence says volatile. After-permit lost reply yields Unknown, then original receipt settles without replay. |
| `ui1_1_revoke_paused_key_frame_and_save` | Before-delivery/present/permit barriers then revoke/expire: no late key/frame/selection; retained handles deny. Separate after-permit revoke permits at most original one transition, Unknown until original receipt, no new save. |
| `ui1_1_fault_restart_discards_unsaved_draft_and_old_grants` | Unsaved draft lost visibly; no silent restart; fresh preview/instance/gens opens confirmed base, all retained old grants fail. |
| `ui1_1_focus_compositor_restart_retires_old_epochs` | Each service restart retires old focus/input/surface grants and approval; fresh attach/preview/first frame restores authenticated routing. |
| `ui1_1_stalled_session_leaves_other_session_usable` | Independent 1000ms deadline returns Timeout/read or Unknown/admitted-save without polling; different-object B keys/frame/read work within fixture deadline; same-object quarantine and finite released-fixture cleanup proven. |
| `ui1_1_clock_capacity_and_counter_limits_fail_closed` | B advances time then backward A cannot revive anything; session/object/instance/endpoint/shared/operation/trace quotas and checked counter/time overflow fail before effect, release does not reuse valid identities. |

Evidence retains canonical actual exchanges, corpus/font hashes, issued grants, composed images, old/new text hashes/revisions, operation binding/receipt, barrier ordering, deadline and resource ledgers; backend/execution labels are volatile/in-process. Setup/font hashing and evidence IO stay outside timing intervals; durations are fixture results, not scheduler guarantees. Initial RED must show absent behavior, not absent metadata. The registered `just foundry-desktop-editor-host-ui1-1a` gate executes all 13 cases, validates their retained witnesses and checks default API exclusion. b/c/d, RUN0, IN0 and physical persistence remain separate.

Grounding: [wire allocation](DESKTOP_EDITOR_WIRE_V1.md), [reviewed editor proposal](plans/desktop-editor-v0.md), [UI1.0 contract](DESKTOP_SESSION_V1.md), `services/desktop/src/dev.rs` opaque contexts/strict codec/monotonic lifecycle, [Constitution](../CONSTITUTION.md) and [Agentic Workflow](AGENTIC_WORKFLOW.md). Request authority is class/right/resource/lifetime enforcement (Lang); observations are only owned keys/draft/frames/selected bytes/original receipt plus previously copied bytes (ObsContract). Trusted fixture/controller visibility is separately labeled and does not prove client isolation.
