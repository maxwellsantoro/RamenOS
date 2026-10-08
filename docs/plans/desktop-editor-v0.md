# Desktop editor v0: UI1.1 host contract proposal

**Date:** 2026-10-04  
**Status:** independently reviewed proposal; coordinator allocation and assertions pending  
**Source snapshot:** `a44993e5880fee83d9ca4400f8a291f309ef813c`  
**Consumer:** keyboard-only Rust ASCII artifact editor; no model required

This packet narrows [desktop v0](desktop-v0.md) to UI1.1 preparation in
[Next Tasks](../../NEXT_TASKS.md). It specifies executable successors, not
implemented interfaces or new evidence. Only this plan is authored by this
packet. Protocol IDs, message IDs, public APIs, dependency registration and gate
paths below are proposals until the coordinator freezes them.

## Existing boundary and smallest successor

[Desktop session v1](../DESKTOP_SESSION_V1.md) is already a default-off Unix host
launch/lifetime proof with a pinned real child, synthetic trusted confirmation,
monotonic expiry, independent kill/reap and 17 named cases. Protocol 336 grants
that witness only self-observation. Preserve its layouts, grant allowlist,
bootstrap and gates. Do not turn that witness into an editor by interpreting
unused rights or bytes differently. An editor session uses a newly allocated
protocol and versioned grant/preview objects; launch mechanics can be extracted
or reused through a separately reviewed coordinator-owned integration change.
The existing real-child evidence does not prove execution of the new editor.

The first executable successor, **UI1.1a**, is an in-process Rust editor consumer
of generated, strictly encoded control messages, opaque broker contexts and
broker-owned shared data objects. It includes authenticated fixture keyboard
production, focus routing, an offscreen compositor and an explicitly volatile
artifact adapter. The client gets neither backing `Vec` references nor raw
pointers: scoped mapping leases check owner, rights, generation and freeze state
at each access. This proves the host registry's typed boundary and host editor
behavior. It does not prove independent application execution, OS mapping
revocation, hostile same-UID process isolation or native IPC. Keep the
`desktop_v0_dev` feature default-off; no device or target prerequisite blocks
these executable assertions.

**UI1.1b** is a Store-owned selected-artifact transaction adapter and its real
host CAS consumer. It must pass publication, receipt, conflict and service-reopen
cases before any integrated real Store claim. **UI1.1c** joins the same editor
scenario to that adapter and the editor permission/launch/recovery flow. First
integration may use the explicitly named in-process editor; a separate
**UI1.1d** packet, if admitted, executes the pinned editor as a real host process
and supplies actual mapped-data/transport and process evidence. UI1.1d cannot
reuse the non-rendering witness's PID as editor execution evidence. These splits
let UI1.1a start before the transaction adapter or target runtime exists.

## Human task and deterministic output

Start with the private selected artifact `note=old\n`, revision 1. `Ctrl+Esc`
opens the trusted launcher. Enter selects `artifact-editor-v0` and shows the
pinned application identity, selected object/revision, the exact permissions,
limits and evidence label. Escape cancels with no instance or grants. A fresh
Enter press on the currently focused preview approves exactly that revision.
Focus goes to the editor only after its first valid surface publication.

`Ctrl+A` selects all; typed US-keyboard events replace the draft with
`note=new\n`. Arrow, Home, End, Backspace, Tab and Enter work locally; there is no
synthetic repeat. Printable ASCII, LF and tab are the complete text alphabet.
The draft is at most 4096 bytes. Reject unsupported bytes, unsupported key
translation and overflow without replacing the previous draft. Cursor movement
and selection use checked byte indices. A 4096-byte draft may scroll in the
fixed viewport; clipping is deterministic. Unicode, pointer, clipboard,
network, arbitrary files and compatibility application interfaces are deferred.

`Ctrl+S` freezes one draft snapshot and dispatches one save operation. Trusted
status chrome obtains save results from the enforcing artifact service and
shows Unsaved, Saving, Saved, Conflict or Save unknown. Saved requires a verified
receipt; an application-rendered label or hash is insufficient. An older save's
success while the user has edited again records the committed revision but keeps
the current draft Unsaved. Permit further local editing while one save is in
flight, but no second commit until it finishes or is reconciled. Repeated
`Ctrl+S` during Saving/Unknown cannot send another mutation.

After confirmed save, `Ctrl+Q` closes, and a fresh preview/approval reopens the
selected revision as `note=new\n`. A crash shows that unsaved draft contents are
lost; v0 promises no autosave. `Ctrl+R` opens a fresh restart preview. An unknown
save must be reconciled before choosing a restart base. If it stays unknown,
recovery can return to the launcher with an explicit unresolved-save message;
it cannot silently pick old/new bytes and count reopen as success.

Render a 640×480 opaque BGRA8888 app surface, stride 2560, two buffers of
1,228,800 bytes. The compositor's 640×568 output contains top chrome rows 0–47,
the app at 48–527, and status/recovery rows 528–567. A small fixed bitmap font
and deterministic cursor/selection regions have fixture digests. The compositor
owns a reserved system indicator and the whole chrome region, clips all app
pixels, and never provides a chrome or scanout mapping to the editor. Apps can
imitate chrome within their region; clipping and route authentication are the
claimed properties, not universal spoof resistance. Pixel assertions examine
composed output, not merely a client-supplied frame digest. Host output is a
retained offscreen image; real window presentation and human usability remain
separate measurements.

## Proposed authority and bounds

Every grant binds owner/opaque peer, session and generation, instance and
generation, resource identity and generation, service epoch, rights and expiry.
IDs in messages are checked references, never caller authentication. Host broker
contexts are opaque, issued for registered endpoints and bound to the actual
fixture consumer. A driver fixture's production context is separate from an
editor or chrome context. A future process context comes from its owned transport
and actual `Child`, never a payload role/PID. Target caller binding and kernel
fast-path validation remain RUN0/UI2 requirements.

| Holder / endpoint class | Proposed rights and resources | Allowed observations |
|---|---|---|
| Editor session | profile bits SELF_STATUS=1, FOCUSED_KEYS=2, ONE_SURFACE=4, SELECTED_READ=8, SELECTED_REPLACE=16, OWN_RECEIPT=32; exact mask 63 | Own delivered keys, draft, frames, selected bytes/revision and own receipt/state |
| Input producer | PRODUCE=1 on one attached keyboard/queue generation | Own attachment/queue status; no artifact, preview or app pixels |
| Focus service / chrome | ASSIGN=1 on owned session; preview/confirm/cancel/close/recover/status endpoints use separate chrome class and reviewed launch rights | Owned preview, focus and recovery state; no unrelated-owner content |
| Surface instance | CREATE=1, ACQUIRE=2, PRESENT=4, DESTROY=8 on one owned surface | Own descriptors/publication result; no screen readback or other surface |
| Compositor/adapter | CONSUME=1 on the selected frozen frame | Only app frames it must compose, plus service-owned chrome |
| Artifact instance | READ=1, REPLACE=2, RECEIPT=4 on one selected object | Selected text and own operation results; no path/tag/content-ID search |
| Recovery chrome | RECEIPT=4 on the owner's original operation only | Frozen binding and original outcome; no mutation or general receipt enumeration |

Equal bit values in different classes grant no other class's operation. The
editor profile produces distinct endpoint/resource grants, not an unrestricted
bitmask dispatcher. No editor focus assignment, preview/confirm/restart, input
production, physical framebuffer, arbitrary shared-memory create/map/close,
Store ingestion/search or filesystem grant exists. Every denied reply has zero
data handles, lengths, hashes, foreign IDs, state and PID; its status and echoed
request ID are the only useful fields. Denial preserves the selected artifact,
draft, focus, frame and resource counts and includes a working authorized witness.

Use at most two sessions, one preview/instance/surface and one in-flight save per
session. Preview TTL 1–30000 ms and instance TTL 1–600000 ms retain UI1.0's
monotonic registry rule. All origins/comparisons use effective time under the
enforcing lock; an earlier timestamp from another session or captured reader
cannot revive a plan/lease. Bounded frame/artifact requests have an absolute
1000 ms host deadline, enforced independently of request polling. The real host
process packet retains UI1.0's 1–5000 ms absolute child deadline until a separately
reviewed dispatch/yield liveness contract exists.

For in-process work, the independent deadline retires response delivery and
new-effect admission, plus ordinary shared-object leases. Reads return Timeout;
an admitted save whose outcome is unproved returns Unknown. Late key/frame work
rechecks generation and expiry before effects. Save authority is linearized at
the per-object commit permit specified below: work with no permit cannot later
publish selection; work already holding its one immutable permit may finish
that bound transition after an observed deadline/revocation. The deadline cannot
cancel an already-started filesystem rename/fsync or establish noncommit.
Rust cannot forcibly kill/reap an arbitrary stalled thread. Gates use bounded
supported pause/delay fixtures that eventually release; they verify no late
unpermitted effects and finite tracked cleanup for those fixtures. They do not
promise join, memory reclamation or containment of an arbitrary hung in-process worker. Such
a worker can leave a slot quarantined/Exhausted until host teardown. A real
process adapter needs its separate independent kill/reap evidence. Lease methods
are bounded reads/writes without caller callbacks or escaped backing references;
delayed work holds no registry lock or raw mapping borrow across its pause.

Queues hold 64 ordinary key events. The 65th causes a reserved reset record,
discards queued keys, clears held modifiers, and never creates Enter confirmation.
Reset delivery cannot be displaced by ordinary events. Sequence, focus epoch,
mapping generation, revision and identity counter additions are checked; exhaustion
fails closed and retires rather than wraps. Keep at most 16 instance records
(including live instances), 16 non-evicted operation records per selected object,
and two selected objects. Reserve operation capacity before dispatch; Exhausted
before the 17th operation causes no publication. Audit/frame trace budgets must
also be frozen by the coordinator (proposed 256 control exchanges per case and
16 retained frame images); full logs cannot be an unbounded authority bypass.
Do not hold a global/session registry lock during blocking Store IO or rendering.

## Proposed typed control and data contracts

Proposed files are `idl/harness/input_v1.toml`,
`idl/services/desktop_focus_v1.toml`,
`idl/services/desktop_surface_v1.toml`,
`idl/portals/desktop_editor_session_v1.toml` and
`idl/portals/desktop_artifact_v1.toml`. All protocol/message IDs are unallocated.
The new editor session protocol replaces none of protocol 336's meanings.
Coordinator freezes namespaces, message numbers, generated inclusion points and
shared schemas together. Requests/replies use generated `Envelope` types and
explicit per-field LE serialization. Reject unsupported protocol/operation,
wrong direction, wrong kind, exact-length mismatch, nonzero reserved/payload tail,
noncanonical raw handle bits and bridge pad before normalization or data access.
The Envelope has no version field: a distinct protocol identifies each
incompatible IDL version. Every shared object has `schema_version`, exact total
length and reserved fields. Never serialize `repr(C)` padding as a codec.

In this table, every unnamed scalar is `u64`; `hash` is `bytes32` and explicitly
marked small fields are `u32`. Fields occur in the listed order. `req` means
request_id; `sg`, `ig`, `sf_g` and `mg` mean session, instance, surface and mapping
generation. The listed lengths include aligned groupings and fit 64-byte payloads.
The endpoint in `Envelope.handle` supplies the authority context, outside payload.

| Proposed message | Ordered fields | Bytes / enforcement |
|---|---|---|
| Input Attach | req, session, sg, device_generation | 32; focus attachment endpoint, no app recipient chosen by driver |
| AttachReply | req, queue_generation, status:u32, reserved:u32 | 24 |
| Input ProduceKey | session, sg, queue_generation, sequence, usage:u32, phase:u32, modifiers:u32, reserved:u32 | 48; attached producer only; phase press/release/reset |
| ProduceKeyReply | sequence, status:u32, reserved:u32 | 16 |
| Focus Assign | req, session, sg, instance, ig, surface, sf_g | 56; registered chrome/focus only, ready owned surface |
| AssignReply | req, focus_epoch, service_epoch, status:u32, reserved:u32 | 32 |
| Focus PollKeys | req, session, sg, instance, ig, focus_epoch | 48; focused instance only |
| PollKeysReply | req, sequence, focus_epoch, usage:u32, phase:u32, modifiers:u32, status:u32 | 40; NotReady means no event with all event fields zero |
| Surface Create | req, session, sg, instance, ig, width:u32, height:u32, format:u32, reserved:u32 | 56; fixed profile, quota one |
| CreateReply | req, surface, sf_g, buffer0_shm, buffer1_shm, status:u32, stride:u32, format:u32, reserved:u32 | 56; descriptors alone give no writable lease |
| Surface Acquire | req, surface, sf_g, buffer_index:u32, reserved:u32 | 32; service-bound owning instance |
| AcquireReply | req, mg, buffer_shm, status:u32, byte_len:u32 | 32; one fresh writable lease |
| Surface Present | req, surface, sf_g, mg, sequence, buffer_index:u32, reserved:u32 | 48; freeze before successful acknowledgement |
| PresentReply / ConsumeReply | req, sequence, status:u32, reserved:u32 | 24 |
| Surface Consume | req, surface, sf_g, sequence | 32; compositor owns frame consumption |
| Surface Destroy | req, surface, sf_g | 24; instance/recovery ownership |
| DestroyReply / short replies | req, status:u32, reserved:u32 | 16 |
| Editor Prepare | req, session, sg, application_hash, requested_rights:u32, reserved:u32 | 64; chrome only, mask 63 and pinned manifest |
| PrepareReply | req, plan, preview_revision, preview_shm, preview_len:u32, status:u32, granted_rights:u32, reserved:u32 | 48 |
| Editor Confirm | req, session, sg, plan, preview_revision | 40; one current trusted Enter event required |
| ConfirmReply | req, sg, instance, ig, grants_shm, grants_len:u32, status:u32 | 48; authenticated context already binds session |
| EditorBootstrap | session, sg, instance, ig, grants_shm, grants_len:u32, status:u32 | 48; service-to-client only, status must be Ok |
| Editor Status / Close / PrepareRestart | req, session, sg, instance, ig | 40; status permitted only for own instance, other operations chrome only |
| StatusReply | req, instance, ig, focus_epoch, service_epoch, state:u32, status:u32 | 48; save detail stays in receipt object |
| Editor ObserveFocus | req, session, sg, instance, ig | 40; own epoch read, never focus assignment |
| ObserveFocusReply | req, focus_epoch, service_epoch, status:u32, reserved:u32 | 32 |
| CancelPreview | req, session, sg, plan | 32; chrome only |
| Artifact ReadSelected | req, session, sg, instance, ig | 40; object resolved from grant |
| ReadSelectedReply | req, revision, data_shm, object_generation, len:u32, status:u32 | 40; read-only validated descriptor |
| Artifact AllocateSaveId | req, session, sg, expected_revision | 32; REPLACE right, reserve one owned operation slot before mutation |
| AllocateSaveIdReply | req, operation, status:u32, reserved:u32 | 24; fresh broker-issued nonzero ID, no publication |
| Artifact Commit | req, session, sg, expected_revision, operation, src_shm, len:u32, reserved:u32 | 56; peer binds instance/ig; no selectable path/content ID |
| CommitReply / SaveStatusReply | req, operation, revision, receipt_shm, status:u32, receipt_len:u32 | 40; Ok only with verifiable receipt |
| Artifact SaveStatus | req, session, sg, operation | 32; scoped receipt read, never redispatch |

Allocate reply IDs for every operation and reuse PrepareReply shape for restart.
ObserveFocus lets the editor learn its epoch before PollKeys without guessing
an epoch or parsing app pixels. A not-focused authorized ObserveFocus returns
NotReady and zero epoch fields; unauthorized/foreign callers receive Denied.

Preview/grant objects are read-only, schema 1. Proposed exact LE header (240
bytes): schema_version/total_len/profile_rights/grant_count:u32; then session,
sg, instance, ig, selected_object, selected_generation, selected_revision,
policy_revision, plan, preview_revision, expires_at_ms, issuer_service_epoch:u64;
then application_hash/manifest_hash/selected_content_hash:bytes32; then
text_limit/width/height/stride/format/key_queue_limit/surface_limit/reserved:u32.
Each of four grant records is 56 bytes: endpoint/resource/resource_generation/
service_epoch/expires_at_ms:u64 followed by class/rights/schema_version/reserved:u32.
The complete object is exactly 464 bytes (below UI0's 2048-byte bound). The
records are sorted SELF, FOCUSED_KEYS, SURFACE, ARTIFACT, with endpoint masks
1, 1, 15, 7 respectively; profile mask is 63. Preview endpoints and future
instance/surface IDs are zero descriptions, never grants. Issued objects contain
valid concrete endpoints/resources bound to the new instance; object kind in
the descriptor registry distinguishes preview from issued grants. Confirm
revalidates these identities and stores grants internally; shared bytes are
never the grant source. The editor manifest is derived from named edit/save/
failure scenarios and checked against mask 63; these observations do not cover
every possible authority requirement. Cancellation, policy/artifact change,
expiry or focus/service reset invalidates the preview event. An earlier queued
Enter or synthetic reset cannot approve a newly shown preview.

Text data uses a fixed 64-byte header: schema_version:u32, total_len:u32,
selected_object:u64, revision:u64, content_hash:bytes32, byte_len:u32,
reserved:u32, followed by at most 4096 raw text bytes. Owner/session/instance/
object generation and read/write/freeze permissions live in the authoritative
descriptor registry. A commit source names a separately owned draft object;
its header's selected object/revision must match the grant and expected base.
Check actual allocation, declared lengths, format, hash, offset arithmetic and
descriptor generation before consuming. Proposed exact read-only receipt is
176 bytes: schema_version/total_len/outcome/reserved:u32; owner/session/sg/
original_instance/ig/selected_object/selected_generation/operation/
expected_revision/result_revision/backend_service_epoch/committed_at_ms:u64;
then expected_content_hash/source_content_hash:bytes32. A committed result hash
equals the frozen source hash. Definitive noncommit has result_revision and
committed_at_ms zero; Unknown returns no receipt descriptor and no result
revision. The trusted registry/Store journal retains the complete operation
binding. Receipt hashes are raw SHA-256 values, not caller-supplied strings.
This is below 512 bytes.
Receipt outcome allocation is proposed Committed=0, DefinitiveNoncommit=1;
reject any other value. Unknown is represented by reply status Unknown=10
without a receipt, never by a guessed noncommit record. Commit Ok means a
Committed receipt; SaveStatus Ok means a validated retained receipt whose
outcome must still be inspected. This distinguishes successful read from
successful mutation. Lifecycle/status numeric values stay those frozen in
UI1.0; input phases, usages and format/class values await coordinator allocation.
Retired mapping handles cannot read receipt/text bytes. Previously observed
bytes cannot be erased; retain that distinction in ObsContract reports.

Bulk text and pixels stay in bounded shared-object storage; control transfers
descriptors only. In UI1.1a, every alias is a registry-checked lease; after
Present the writer lease is retired before composition, even for retained old
handles. Consume releases the frozen buffer and only then permits a fresh
Acquire/mapping generation. Check format, stride×height, byte length and all
indexes before composition. CPU composition copies pixels into its output;
this is not end-to-end copy-free or target MMU evidence. A later raw mapped
host process adapter must prove every writable alias is revoked/sealed before
readers run, or retain an explicitly trusted producer/copy scope and cannot
claim UI0's adversarial mapping assertion. Target pages require actual MMU proof.

## Input, focus and recovery state machine

The trusted producer fixture submits usage/phase/modifiers, never arbitrary
text or a destination app. This is a logical host event corpus, not evidence
for a USB/HID report decoder or hardware keyboard. Coordinator freezes exact
phase/usage/modifier numeric values and the checked-in fixed US translation
table/corpus source and digest before assertions; resemblance to HID usage
values does not qualify a native controller. Focus validates device/queue/session generations and
strict sequence. A fixed US translation maps accepted usages to ASCII; reject
invalid phases, usages and contradictory modifier state without fabricating
keys. Ctrl+Esc, preview/recovery Enter/Escape, Ctrl+Q and recovery Ctrl+R route
through authenticated chrome/focus, with their releases consumed there. The
editor receives Ctrl+S and local editing combinations only while focused.
The human-task gate must obtain confirmation through this typed producer →
focus/chrome route; directly calling UI1.0's synthetic-event helper cannot
replace that evidence. The helper remains valid only for its existing launch
fixture gate. Route events bind current service/focus/session/preview epochs
and are single-use, including when key releases arrive after a focus transfer.

Every focus transfer increments focus_epoch, discards old queued events and
clears old pressed-key/selection transients through a reset. Delivery rechecks
focus epoch and grant lifetime at dequeue. Focus-service restart increments its
service epoch and resets attachment/held state; compositor restart retires its
surfaces and pending frames. No stale events, surfaces or lease can attach to
the new epoch. Fresh surface/grants and a ready frame precede reassignment. The
host initial scope can stop the old editor consumer and ask for fresh approval;
it must not silently recreate an instance or grant while recovering services.

Editor fault, expiry or revocation performs stop input/save admission → cancel
or finish bounded nonpublishing work → freeze known/unknown operation state →
quiesce bounded lease operations → retire surface/mappings/endpoints → show recovery.
An in-process delayed worker is fenced from new unpermitted effects rather than
asserted dead; its unresolved operation/slot is retained until the supported
fixture releases or actual process termination is established. Arbitrary thread
cleanup is outside this gate. Authority is rechecked at frame acceptance, event
delivery and issuance of the Store commit permit. After retirement, no key/frame
delivery or selection transition lacking a pre-retirement permit may occur.
A pre-retirement permit authorizes exactly its frozen transition even if blocking
IO completes after retirement. Keep that operation Unknown until its receipt
settles; revocation does not undo the permit or a completed committed selection.
Fresh recovery receipt authority is bound to the original owner's operation,
not to the old instance's retired handle. An unrelated session retains its own
focus/input/frame/Store authority throughout another session's stall/restart.

## Store-owned save and uncertainty protocol

`artifact_store_schema` owns proposed `SelectedArtifactV0`,
`EditorSaveBindingV0`, internal `EditorCommitPermitV0` and `EditorSaveReceiptV0`
validation/state types. They own
no filesystem/socket IO. The artifact portal is a narrow deputy whose grant
resolves the selected object; it never accepts a caller-selected path, arbitrary
content ID or ambient Store token. `store_service`/`artifact_store_core` own
authenticated read, publication, revision selection, receipt lookup and recovery.
The UI client consumes these schema types and typed messages without opening
CAS files. The Store adapter itself is a trusted host deputy with named broader
host IO authority, not proof of whole-host client containment.

AllocateSaveId reserves a fresh monotonic operation ID from the selected-object
registry before the UI dispatches Commit, so a lost Commit reply still leaves
the UI with a known ID. The allocation binds owner/session/instance/base, carries
no mutation, and requires REPLACE. IDs are never reused, including canceled or
expired allocations. The bounded non-evicted 16-record history counts allocations
as well as committed/aborted operations; restart does not recover admission
capacity by discarding unknown evidence. A lost allocation reply can leave a
reserved slot but cannot have committed content. Report this finite-session
limit and require a separately reviewed history rollover for longer use.

Bind operation ID to owner, session generation, original instance/generation,
selected object, expected revision/hash and immutable source bytes/hash. Reserve
bounded operation capacity and freeze the source at admission. Reject reuse
with a different binding; exact duplicate dispatch may return the retained
original result without executing publication again. The UI never automatically
redispatches a mutation, even if the service supports exact result lookup.

Store serializes writers per selected object. Before starting irreversible
publication/selection journal IO, it holds a short object state lock, checks
current authority/generations/effective expiry and the complete operation binding,
compares the expected revision/hash, and issues one immutable commit permit.
Issuance is the authority linearization point: reserve that expected revision
and its checked successor for that writer only. The permit binds the original
owner/session/instance, operation, selected object/generation, expected/new
revision, immutable source hash/bytes, service epoch and unique writer identity.
It is internal Store state, never a caller-issued ID or editor endpoint right.
Coordinator freezes its exact schema/API and the writer-fencing/locking contract
before executable assertions. No blocking IO occurs under global/session or
object state locks; a bounded object state reservation blocks other same-object
mutations while unrelated objects remain usable.
Revocation and deadline closure update the object's admission epoch under that
same short lock, so an earlier cached authority check cannot race permit issuance.
The admitted writer later validates its exact retained permit/reservation,
not a newly issued grant and not an epoch change interpreted as retroactive
permit cancellation. A permit is consumed once for its one transition; it
cannot authorize another candidate, revision or operation.

Revocation/deadline/service retirement taking that lock before permit issuance
forbids later publication by the old operation. A definitive noncommit result
requires proof that no permit was issued and no selection writer remains able
to publish. If permit issuance wins first, later retirement cannot cancel the
writer's single bound transition, including a journal rename/fsync already in
progress. It may logically complete its admitted save after an observed timeout
or revoked handle. Expose Unknown while unsettled, quarantine that object's
mutation slot/reserved revision, and reconcile only the original receipt. Do
not claim absence of late OS selection effects in this after-permit case. Never
issue a replacement permit or reinterpret Unknown as definitive failure/retry.
New key/frame/save admission is still denied for the retired instance.

The permitted writer verifies/publishes the immutable candidate and atomically
records new selection plus its receipt in one bounded journal/state transition.
Do not compose a separate revision write and receipt write and call the pair
atomic. Real host acceptance requires fsynced candidate/ownership and bounded
authoritative selection/receipt state, atomic rename/readback, parent directory
synchronization and validated recovery. A storage error after possible publication
poisons that object's mutations until recovery; it does not fabricate failure.
Blob creation alone is not successful selected-revision publication. Orphan
candidate cleanup and its limits must be stated by the Store owner.

A new service epoch or recovery adapter cannot replace/reopen authoritative
object state while an old writer is unquiesced. First obtain supported fixture
completion plus joined writer evidence, or actual owned process stop+wait proof;
then validate on-disk prior/new state and its original receipt before reopening.
Without that proof, keep Unknown/NotReady and same-object mutation quarantine.
A mere epoch increment, cancellation flag, expired handle or thread Drop does
not fence a filesystem write already started. Killing a real writer proves it
cannot issue further IO after wait; recovery must still reconcile completed or
interrupted prior IO. Gates use bounded supported writers, not arbitrary hung
thread cleanup claims.

For the supported successful-race fixture, two sessions with the same authorized
base produce one revision winner and one Conflict after the winning permit
settles. While a permit remains unsettled, same-object mutation admission is
NotReady/Unknown, not a second permit. Conflict exposes only the selected
object's current revision to its
authorized holder; it leaves the draft Unsaved and requires explicit reload/
new-base confirmation before another operation. There is no silent merge.
Old immutable content remains readable by its original authorized owner.

| Observed boundary | UI state / reconciliation rule |
|---|---|
| Rejected before mutation admission with a definitive reply | Unsaved/Conflict/error; no selection change, no receipt invented |
| Accepted operation, reply lost before or after permit/commit | Save unknown; preserve operation/binding, dispatch count stays one |
| Deadline/revocation before permit, with verified writer fencing | No later selection publication; definitive noncommit only with complete evidence |
| Deadline/revocation after permit or while journal IO blocks | Unknown, same-object quarantine; original one permitted transition may finish, no replacement permit or automatic retry |
| Read original receipt finds committed outcome | Verify binding and new identity, show Saved for that draft snapshot; never recommit |
| Service proves no permit was issued, or quiesced writer recovery proves no selection committed | Definitive noncommit; preserve draft, a new operation needs explicit user save |
| Missing/truncated/corrupt journal or no sufficient original evidence | Unknown or NotReady; stop mutations/restart-base selection, no inferred failure |
| Receipt read timeout/disconnection | Unknown persists; bounded reads may be explicitly retried, mutations are not replayed |

Crash-phase fixtures interrupt before candidate publication, after candidate
publication/before selection, after journal rename/before acknowledgement, and
after acknowledgement. Only after writer quiescence, service reopen verifies
complete prior state or
complete new selection+receipt; ambiguous/corrupt state fails closed. A recovery
receipt can survive editor/focus restart after all old handles retire. Host CAS
service reopen is real host IO evidence. It is neither power-loss qualification
nor target save across reboot. STORE0 actual device write/flush/read and UI2
separate target boots remain prerequisites for those claims.

## Gate-first assertion inventory

The coordinator registers exact names/counts before handlers. Every required
case runs once, none ignored, and missing/extra cases fail. A fixture leg never
satisfies the real Store inventory. Useful-task cases must initially fail with
absent handlers, not pass on metadata/doc presence. Denial and failure cases
include prior-state comparison and a valid allowed producer/consumer witness.

| Proposed exact test name | Required behavior, denial or failure evidence | Packet |
|---|---|---|
| `ui1_1_keyboard_edits_ascii_and_renders_owned_frame` | Generated press/release corpus changes old→new; cursor/selection/scroll witness; expected composed app pixels and source sequence | a |
| `ui1_1_ascii_bounds_preserve_prior_draft` | Invalid bytes/usage and 4097th byte preserve draft; valid LF/tab/4096-byte edge works | a |
| `ui1_1_focus_routes_reserved_keys_to_trusted_chrome` | Owned trusted Enter route succeeds; editor/nonproducer/spoofed context cannot approve, focus, produce, prepare/confirm/restart | a |
| `ui1_1_focus_epoch_reset_overflow_and_detach` | Queued modifier/Enter/releases never cross focus; 65 events/reset/detach/reattach leave no held key or accidental approval | a |
| `ui1_1_surface_freeze_alias_quota_and_chrome_clip` | Retained writer aliases deny during consumption, fresh Acquire works; stale sequence/index/quota fail; adversarial full frame leaves indicator/chrome pixels unchanged | a |
| `ui1_1_wire_schema_descriptor_and_identity_fail_closed` | Exact malformed protocol/op/direction/length/tail/pad/raw handle, unknown object schema, stride/range overflow and foreign generations fail before data access | a |
| `ui1_1_editor_grants_are_exact_and_preview_single_use` | Mask 63 equals issued resource grants; missing/replayed event or changed manifest/policy/artifact/expiry denies; fresh approval works | a/c |
| `ui1_1_volatile_save_and_reopen_are_explicitly_labeled` | Fixture receipt/reopen old→new works with visible volatile label; it supplies no Store IO evidence | a |
| `ui1_1_revoke_paused_key_frame_and_save` | Barrier before key/frame delivery and before save commit permit, then revoke/expire; no subsequent key/frame/selection effect; old handles deny and own authorized witness succeeds | a/b/c |
| `ui1_1_fault_restart_discards_unsaved_draft_and_old_grants` | Fault after edit shows draft loss; fresh preview opens confirmed base; fresh instance/gens, no silent restart | a/c |
| `ui1_1_focus_compositor_restart_retires_old_epochs` | Each service fault/reset produces recovery, fresh epochs/attachments; no reused surface/input authority | a/c |
| `ui1_1_stalled_session_leaves_other_session_usable` | Paused artifact worker reaches absolute timeout/Unknown; B's different selected object and input/frame/read complete within registered 1000 ms fixture deadline; same-object quarantine, bounded supported-fixture cleanup, no global-lock stall | a/b/c |
| `ui1_1_clock_capacity_and_counter_limits_fail_closed` | Backward times after B advances cannot revive A; bounded sessions/objects/operations/queue/frame history; checked overflow and resource release | a/b |
| `ui1_1_store_commit_reopen_preserves_prior_blob` | Named Store owner actually publishes authenticated content and selection/receipt; service closes/reopens; fresh typed read new bytes, old blob unchanged | b/c |
| `ui1_1_store_same_base_conflict_and_operation_reuse` | Barrier synchronizes two same-base attempts: one success/one Conflict; changed owner/base/bytes operation reuse denies, no extra publication | b/c |
| `ui1_1_store_lost_reply_reconciles_without_mutation_replay` | Drop reply before permit and after permit/selection; separate after-permit revoke/deadline while IO is paused: Unknown, exactly one dispatch/permit/transition, same-object quarantine, original receipt reconciles after bounded writer completion | b/c |
| `ui1_1_store_recovery_validates_atomic_selection_and_receipt` | Refuse new-epoch reopen while old writer unquiesced; after supported completion/join or actual stop+wait, actual IO reopen at each crash phase; corrupt/truncated journal denies, prior or verified new complete state only | b/c |
| `ui1_1_unknown_save_survives_editor_recovery` | Editor/focus fault after permit while save IO pauses retires old grants; permitted original transition may finish; owner-bound original receipt reconciles without replay; unquiesced writer blocks service reopen, unresolved state forbids guessed restart base | c |
| `ui1_1_real_editor_executes_typed_task` | Actual distinct editor PID/pinned snapshot/bootstrap and shared-data exchanges produce keys/frame/save; fault/watchdog/Drop actual kill/reap | d, separate future inventory |

Proposed commands, **not registered or runnable acceptance claims**:

| Proposed command / output | Inventory and acceptance |
|---|---|
| `just foundry-desktop-editor-host-ui1-1a` / `out/desktop/ui1-1a/` | First 13 rows' a legs; volatile fixture and in-process execution explicitly stamped |
| `just foundry-desktop-editor-store-ui1-1b` / `out/desktop/ui1-1b/` | Exactly 7 b rows, including four Store-specific rows; actual Store producer/consumer artifacts |
| `just foundry-desktop-editor-task-ui1-1c` / `out/desktop/ui1-1c/` | Exactly first 18 rows, joined a+b with a separately stamped volatile leg and real Store task; exact keyboard task plus unknown-save recovery |
| `just foundry-desktop-editor-process-ui1-1d` / `out/desktop/ui1-1d/` | Future real editor process inventory; mapped alias safety has explicit proof/limit |

Coordinator retains result/per-case JSON, canonical actual producer/consumer
wire transcripts, input corpus/translation/font digests, full grant manifest,
composed frame images, selected old/new hashes/revisions, operation/receipt
objects, fault barriers, service epochs and bounded resource ledgers. Report
git SHA/tested-diff digest, contract version, actual execution inventory and
backend/fixture provenance. Store artifacts include named IO owner, actual
on-disk journal/blob readback hashes and independent reopen results. Exact
outcome/unknown/dispatched-mutation counts must be machine checked. Red runs
are preserved; absent tooling is INCOMPLETE, runnable missing behavior FAIL.
No marker, hand-authored receipt or app-reported hash replaces observed output.

Keep fixture hash/setup and evidence-file IO outside timing regions where
possible; measure named service admission/response rather than repeated test
binary hashing. Report fixture durations without promising scheduler latency.
Gates sharing fixed paths are coordinator-serialized; workers use focused tests
and unique temporary outputs. UI1.0 plus affected Store consumers and S11/S12/
S13/org checks are integrated by the coordinator after independent review.

## Dependency order, ownership and readiness

1. Coordinator reviews this packet, freezes protocol/message IDs, exact shared
   schemas/grant records, status 0–11 meanings (matching UI1.0), phase/format
   enums, resource ledgers and host lease/opaque-peer APIs. Register codegen,
   module/workspace paths and exact named-case gate inventories. No change to
   protocol 336 without explicit compatible review.
2. Assertion worker writes UI1.1a behavior/denial/failure cases, records initial
   RED, then coordinator dispatches one exact-file writer for host focus/
   compositor/shared-object adapters and a disjoint Rust editor-core writer.
   Coordinator owns their library root, common registry/clock and launch join.
   Host keyboard fixtures are default-off and driver-class authenticated.
3. In parallel after artifact contract freeze, one Store writer owns new
   selected-object schema/transaction/portal files and any explicitly assigned
   Store integration. Write UI1.1b IO/conflict/crash/unknown assertions RED first.
   Do not concurrently mutate projection_cow, agent_task journals, capability
   tables or shared Store handlers. Existing SW0 receipts are source grounding,
   not permission to reinterpret a task-validator transaction as editor save.
4. Independently review a and b against frozen artifacts, integrate one at a
   time, then assign c's launch/keyboard/Store/recovery consumer and integrated
   RED assertions. UI1.1a fixture success cannot close c while b is incomplete.
5. If desired, coordinator admits d with an explicit real-process transport/
   shared-mapping alias contract and fresh tests. RUN0, IN0 and target ports
   remain separate successors. No controller interaction precedes its Vault/
   Oracle, and no physical actuation follows from this packet.

Ready now: contract review, shared allocation and UI1.1a RED assertion preparation.
Blocking executable dispatch: unallocated protocols/messages, exact preview/grant/
receipt schema and opaque-peer/lease API, gate inventory and named writers.
Blocking real Store acceptance: selected-object expected-revision/receipt atomic
transaction and owner-bound recovery lookup, not merely existing ingest/CoW.
Blocking real host editor claim: d's owned process and mapped transport. Blocking
target/device/persistence claims: RUN0 execution/enforcement, IN0 controller
evidence and input, actual retained scanout, Store device IO/flush/reboot tests,
and applicable H0/H1/S12/S13 qualification. No model, paid run or lab access is
needed for the first host contract.

## Source basis and claim limits

- [Current Status](../../CURRENT_STATUS.md), [Next Tasks](../../NEXT_TASKS.md),
  [Roadmap](../../ROADMAP.md) and [Slices](../../SLICES.md) select the human task
  and separate host/device/target readiness. This plan changes none of them.
- [Constitution](../../CONSTITUTION.md), [Agentic Workflow](../AGENTIC_WORKFLOW.md)
  and [IDL Tools](../../idl/tools/README.md) require boundary ownership, generated
  typed control, bounded data and gate-first integration. Shared files remain
  coordinator-owned. Lang and ObsContract are separately specified above.
- [Accepted UI0](desktop-v0.md) supplies ASCII/4096, chrome clipping, frozen
  buffers, input epochs and unknown-save requirements. [UI1.0](../DESKTOP_SESSION_V1.md)
  supplies host opaque identity, exact grants, monotonic clock and independent
  process cleanup. Its observation-only witness is preserved.
- [Store IDL](../../idl/services/store_service_v1.toml) supplies read/verify/ingest,
  not selected-object CAS/receipt semantics. The existing
  [host StoreClient](../../services/store_service/src/client.rs) uses its host
  framed transport and descriptor ingestion; it is not the generated native
  IDL transport. A real adapter must name this boundary if used.
- [Projection CoW](../../services/store_service/src/projection_cow.rs) publishes
  owned immutable blobs and updates a serialized projection; it lacks this
  protocol's expected-revision/operation-receipt transaction. [Domain ownership](../../services/store_service/src/domain_visibility.rs)
  and [artifact core](../../artifact_store_core/src/lib.rs) own publication and
  blob verification. Preserve foreign-content collision/signature behavior.
- [Agent task schema](../../artifact_store_schema/src/agent_task.rs) and
  [host task service](../../services/store_service/src/agent_task/mod.rs) show
  existing receipt binding and fsynced journal recovery patterns. Their pinned
  validation/task semantics cannot be copied into editor authority by assumption.
- [Shared-memory IDL](../../idl/harness/shmem_control_v1.toml),
  [Multi-Domain](../MULTI_DOMAIN.md) and [Ring v0](../RING_BUFFER_V0.md) provide
  bounded foundations; the cooperative ring does not enforce hostile aliases.
  [S10.5 execution inventory](2026-06-17-s10-5-host-to-target-integration.md) does
  not supply target editor execution. [Evidence Levels](../../EVIDENCE_LEVELS.md)
  and [Security Status](../../SECURITY_STATUS.md) govern remaining claims.

Independent review accepted this proposed dependency packet, including the
clarified irreversible save-permit boundary. Final shared allocation and API
freeze remain coordinator prerequisites before executable dispatch. Passing the
future host cases establishes the named task/boundaries on
the tested host only; it does not establish S14/S15 graduation, generalized
noninterference, physical persistence, everyday readiness or human usability.
No hardware/model/spending/public-support/merge/release authority is granted.
