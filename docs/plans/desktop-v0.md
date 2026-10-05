# Desktop v0: one keyboard-operated artifact task

**Date:** 2026-10-04  
**Status:** UI0 design accepted after independent review; contracts and gates below remain proposed  
**Consumer:** one bounded Rust artifact editor, operated without an AI model

This packet defines the next human task selected by
[Next Tasks](../../NEXT_TASKS.md): launch an application from the keyboard,
inspect its permission preview, edit one artifact, save and reopen it, and
recover when an application fails. It supplies implementation boundaries and
assertions for S14/S15 and their runtime/storage dependencies. Acceptance of
this design completes UI0 only. No interface, recipe, driver, runtime or desktop
described below is claimed to exist merely because it is specified here.

## Grounding and evidence boundaries

| Existing source | What can be reused | What still has to be implemented and checked |
|---|---|---|
| [S8 shared-memory IDL](../../idl/harness/shmem_control_v1.toml), [multi-domain memory](../MULTI_DOMAIN.md), [ring contract](../RING_BUFFER_V0.md) | Bounded mapping/accounting primitives and typed descriptors | Application/service lifetime, hostile descriptor validation and target domain execution. The current ring copies bytes and assumes cooperating peers; it is not an untrusted-input boundary |
| [S10.5 execution inventory](2026-06-17-s10-5-host-to-target-integration.md) and its [broker](2026-06-17-s10-5-1-broker-kernel-bridge.md)/[serial](2026-06-17-s10-5-2-qemu-ipc-bridge.md) references | Host native-runner grants and selected host/QEMU IPC | Target loader, service ports, general kernel grants and runtime enforcement. Host Wasmtime and kernel init bytecode do not run the desktop application on target |
| [Domain Manager IDL](../../idl/harness/domain_manager_v1.toml), [kernel capability table](../../kernel/src/cap_table.rs) | Lifecycle vocabulary and generation-bearing handles | Desktop resource/operation rights and authenticated user-mode caller context; the existing IPC table's handle/domain validation alone does not implement these rights |
| [S12 design](2026-06-21-s12-golden-machine-design.md), [GOP probe](../../kernel_uefi/src/gop_probe.rs), [boot metadata](../../kernel/src/boot.rs) | Mode/fill probe and physical qualification process | A retained framebuffer mapping, stride/length/format boot handoff and actual application-frame delivery after firmware boot services. Existing `GopProbeInfo` has no framebuffer address or stride |
| [Store IDL](../../idl/services/store_service_v1.toml), [semantic query IDL](../../idl/harness/semantic_store_v1.toml), [CoW implementation](../../services/store_service/src/projection_cow.rs) | Authenticated immutable blobs, owner checks and host CoW operations | The bounded artifact portal below, atomic expected-revision publication, uncertain-save receipts and target persistent readback. Existing native Store IDL does not expose that revision/receipt protocol |
| [Roadmap S14/S15](../../ROADMAP.md#human-input-and-desktop-scope), [Reference Vault rule](../../drivers/reference_vaults/README.md) | Dependency and dossier requirements | Selected xHCI/HID controller dossier, Oracle trace, device-backed input and failure handling |

Host development can inject typed key events into a deterministic consumer and
use host Store services. These are host contracts/fixture evidence. A replayed
HID report is replay evidence. QEMU integration requires the editor, desktop
services, keyboard device path and scanout adapter actually to execute on the
RamenOS target; every host helper must be named. Reopening from a host CAS does
not qualify target persistence. Physical qualification additionally follows
[Evidence Levels](../../EVIDENCE_LEVELS.md), corresponding S12/S13 gates and
the prepared H0/H1 observation/actuation loop. This packet schedules no physical
actuation and grants no spending, model, merge or release authority.

The first executable successor is **UI1.0**, a host permission-preview and
launch-lifetime consumer. It confirms/cancels a pinned plan, creates one real
bounded host application instance, issues the exact allowlisted grants, exercises
an authorized request and denied deputy request, and retires the instance on
revocation/expiry/fault. It can use a tiny non-rendering Rust witness application
and volatile selected-artifact fixture; it does not require target user-mode
groundwork, the full editor, USB input, surfaces or Store publication. **UI1.1**
adds the editor, compositor/focus, artifact portal and real host Store consumer
against the same accepted contract. UI1.0 cannot satisfy UI1.1 or target gates.

## Task and simplest viable implementation

The named application is `artifact-editor-v0`, a Rust editor for one selected
plain-text artifact. Its v0 text alphabet is printable ASCII plus LF and tab,
with a 4096-byte limit. Unsupported input and oversized artifacts yield a
visible error without replacing the current draft. This deliberately bounded
editor is sufficient to test application execution, surfaces, input authority,
immutable save and recovery. Unicode editing, pointer input, clipboard, network,
GPU acceleration, POSIX applications and remote execution are later consumers.

The deterministic fixture starts with `note=old\n` and edits it to `note=new\n`.
The application and artifact have separate immutable content IDs. The launcher
shows a trusted label for the artifact, the application's pinned content ID and
an explicit permission list: receive keys while focused, present one surface,
read the selected artifact, and publish replacement revisions of that artifact.
It displays the byte/resource limits, session lifetime and restart behavior.
No directory, device, screen-capture or unrelated-artifact grant is requested.

The human flow is:

1. `Ctrl+Esc` opens trusted launcher chrome. Arrow keys select the only initial
   application; Enter opens its preview. Escape cancels with no launch/grants.
2. Enter on the preview confirms the exact displayed plan. The broker creates
   a fresh application instance and grants its fixed resource set. The editor
   becomes focused only after its surface is ready.
3. `Ctrl+A` selects the draft; typing the fixture replacement changes it.
   Arrow/Home/End, Backspace and Enter operate locally. `Ctrl+S` requests save.
   The chrome distinguishes Unsaved, Saving, Saved, Conflict and Save unknown.
4. After a confirmed save, `Ctrl+Q` closes the application. Launching it again
   reads the committed revision through a newly granted artifact portal.
5. A fault, deadline expiration or service disconnection yields trusted recovery
   chrome. `Ctrl+R` offers a fresh preview/restart using the last confirmed
   revision. Unknown save outcomes must be reconciled before selecting a base.
   Unsaved draft loss is stated explicitly; v0 promises no crash autosave.

`Ctrl+Esc`, preview/recovery confirmation and `Ctrl+Q` are reserved session
commands. The focus service consumes them, including their key releases; the
compositor owns a reserved chrome region and system indicator, clips application
pixels to the application region, and the authenticated chrome focus route owns
confirmation. App-rendered imitation inside the app region is still possible;
visual separation does not establish resistance to every spoofing attempt. An
application may request exit but cannot prepare/confirm a launch or restart.
Focus loss clears the editor's pressed-key/selection transient state using a synthetic reset event.
The fixed US keyboard translation and reserved-key policy belong to the session
consumer, not the USB driver. V0 generates no synthetic key repeat.

**Host choice:** Rust desktop/session logic and editor core with a deterministic
offscreen renderer; a host runner adapter executes the editor and uses the same
typed interfaces. Using host Wasmtime is allowed for this adapter but must be
reported as such. Host transport, caller binding and process containment are
separate properties and must be stated in evidence.

**Target choice:** a statically linked, `no_std` Rust x86_64 ELF64 executable,
loaded from a pinned immutable byte snapshot into a real user-mode domain.
Choose a bounded ET_EXEC loader with no relocation, interpreter, dynamic linking
or POSIX/WASI imports. Reject unsupported ELF types/machines, overlapping or
overflowing segments, writable executable segments, invalid entry points and
out-of-range addresses. Bound the image to 4 MiB, eight load segments and an
8 MiB private-memory budget including stack; mapped surfaces count separately.
The build fixture records the exact entry/layout expected by this loader.

RUN0 needs user-mode entry/return, trap-origin caller binding, user-copy checks,
private-page permissions, fault delivery, IPC blocking/yield and an interrupt
deadline that regains control of an infinite application loop. These are future
prerequisites, not inferred from existing MMU tests. Keep architecture-specific
work under `kernel/src/arch/`; keep kernel parsing/accounting allocation-free.
Use one CPU with serialized kernel dispatch. Interrupt handlers must not race
capability/mapping tables; enqueue bounded events or flags for serialized work.
The present SMP prohibition is not permission to use these tables reentrantly.
A cooperative infinite-loop timeout is insufficient for the recovery assertion.
No new kernel dependency is necessary for the proposed small loader; a proposed
dependency change must go back to the coordinator for a recorded decision.

User-space policy decides grants; kernel dispatch validates caller, typed endpoint,
resource rights and lifetime before fast-path operations. Desktop services use
`kernel_api`, not kernel implementation imports. Loader mechanics remain kernel
mechanisms, not embedded Store or launcher policy. A boot image may initially
carry the pinned application bytes and fixtures for RUN0, clearly marked volatile.

**Display choice:** one opaque, software-rendered BGRA8888 surface of 640×480
pixels, with two 1,228,800-byte buffers and trusted chrome composed above it.
The target viewport requires at least 640×568 pixels: 48 rows of reserved top
chrome, a 640×480 app region, and 40 rows of reserved status/recovery chrome.
An unsupported smaller mode returns a NotReady condition; it cannot pass frame
delivery. The compositor clips every app pixel to rows 48–527, renders system
indicator/preview/recovery itself, and never maps chrome pixels into the app.
The target display adapter converts to a supported retained GOP framebuffer
format and respects its actual stride/range. BLT-only or unsupported layouts
fail explicitly; firmware BLT probe success cannot substitute for scanout.
No application receives the physical framebuffer or chrome mapping. A host
offscreen image and a QEMU screenshot are different delivery evidence.

## Proposed typed contracts

The coordinator allocates protocols/message IDs after review. All names below
are proposed new files, not allocated namespaces or generated interfaces:

- `idl/harness/input_v1.toml`: driver-to-focus keyboard transport.
- `idl/services/desktop_surface_v1.toml`: application/compositor buffers and presentation.
- `idl/portals/desktop_session_v1.toml`: trusted preview, focus, launch and recovery.
- `idl/portals/desktop_artifact_v1.toml`: selected-artifact reads, revision commits and receipt lookup.

The wire contract uses the existing bounded Envelope and IDL types. Each request
uses the authenticated endpoint capability in `Envelope.handle`; it carries no
authoritative caller-domain field. The target kernel derives caller identity
from the executing trap context and verifies the endpoint capability belongs to
it. Host adapters bind an authenticated connection to a broker-issued identity;
passing a `domain_id` in JSON is not authentication. Resource IDs and generation
fields are references to validate, never evidence of identity. The existing
Envelope has no wire-version field: each incompatible IDL version must get a
distinct allocated protocol ID, which identifies its wire version. Unsupported
protocols, wrong operations/handle kinds, nonzero reserved fields, short/oversized
payloads and arithmetic overflow fail before data consumption or state mutation.

Notation below expands to individual ordered primitive fields in IDL. `req`,
`session`, `generation`, `surface`, `revision`, `sequence`, `operation` and handles
are `u64`; `status`, lengths, dimensions and flags are `u32`; hash is `bytes32`.
All layouts must fit the 64-byte payload including actual `repr(C)` padding.
The coordinator writes/initializes padding explicitly and tests generated layout
sizes; sketches here are not a portable wire codec.

| Operation / ordered request fields | Required resource/right and caller | Reply and effect |
|---|---|---|
| Input `attach`: req, session, generation | Focus service holds `INPUT_ATTACH` for one selected keyboard endpoint | req, status, queue_generation; driver binds only this recipient |
| Input `key`: session, generation, sequence, usage:u32, phase:u32, modifiers:u32, reserved:u32 | Driver holds `INPUT_PRODUCE`; focus validates device/queue generation | status, sequence; bounded press/release/reset event, no arbitrary text payload |
| Session `poll_keys`: req, session, generation, focus_epoch:u64 | Application holds `FOCUSED_INPUT_READ` bound to its instance | req, status, sequence, usage:u32, phase:u32, modifiers:u32, reserved:u32; at most one event |
| Session `prepare_launch`: req, session, generation, application_hash:bytes32 | Trusted launcher holds `SESSION_LAUNCH_PREVIEW` | req, status, plan_id:u64, preview_shm:u64, preview_len:u32, reserved:u32 |
| Session `confirm_launch`: req, session, generation, plan_id:u64, preview_revision:u64 | Trusted confirmation route holds `SESSION_CONFIRM`; actual focused chrome event required | req, status, instance_id:u64, instance_generation:u64; single-use plan consumed |
| Session `set_focus`: req, session, generation, surface, surface_generation:u64 | Focus service/session chrome holds `FOCUS_ASSIGN` | req, status, focus_epoch:u64; old input discarded before new epoch |
| Session `status`: req, session, generation | Owner holds `SESSION_OBSERVE` | req, status, instance_id:u64, instance_generation:u64, focus_epoch:u64, state:u32, reason:u32; app can obtain its current focus epoch without assigning focus |
| Session `close` / `prepare_restart`: req, session, generation, instance_id:u64, instance_generation:u64 | Trusted chrome holds `SESSION_CLOSE` / `SESSION_RECOVER` | req, status, plan_id:u64; close retires instance, restart returns a new preview |
| Surface `create`: req, session, generation, width:u32, height:u32, format:u32, reserved:u32 | Instance holds `SURFACE_CREATE` with one-surface quota | req, status, surface, surface_generation:u64, buffer0_shm:u64, buffer1_shm:u64; fixed dimensions/stride known by profile |
| Surface `present`: req, surface, surface_generation:u64, buffer_index:u32, reserved:u32, sequence, mapping_generation:u64 | Instance holds `SURFACE_PRESENT` for this surface | req, status, sequence; publication freezes that buffer until release |
| Surface `acquire`: req, surface, surface_generation:u64, buffer_index:u32, reserved:u32 | Owning instance holds `SURFACE_BUFFER_WRITE` | req, status, mapping_generation:u64, buffer_shm:u64; fresh writable mapping handle issued only after prior frame release |
| Surface `release_frame`: req, surface, surface_generation:u64, sequence | Display adapter holds `FRAME_CONSUME` | req, status, sequence; compositor may release the buffer and then permit reacquisition |
| Surface `destroy`: req, surface, surface_generation:u64 | Owner or recovery service holds `SURFACE_DESTROY` | req, status; quiesce readers, revoke mappings and retire generation |
| Artifact `read_selected`: req, session, generation | Instance holds `ARTIFACT_READ` for the selected object only | req, status, revision, data_shm:u64, len:u32, reserved:u32; read-only bytes |
| Artifact `commit`: req, session, generation, expected_revision:u64, operation, src_shm:u64, len:u32, reserved:u32 | Instance holds `ARTIFACT_REPLACE` for the selected object only | req, status, revision, receipt_id:u64; commit selected object only |
| Artifact `save_status`: req, session, generation, operation | Current authorized owner/recovery chrome holds `ARTIFACT_RECEIPT_READ` | req, status, revision, receipt_id:u64; original outcome or Unknown, never a new commit |

Errors are finite status values: `Denied`, `Invalid`, `Unsupported`, `Stale`,
`Exhausted`, `NotReady`, `Conflict`, `Timeout`, `Disconnected`, `Unknown` and
`Internal`. `Ok` is zero; numeric allocation belongs to accepted IDL. Invalid
requests produce no handles and no state change. Responses carry the matching
request ID; asynchronous device events carry a strictly increasing sequence.
Sequence/generation exhaustion retires the object; never wrap into validity.

Preview bytes use a versioned, bounded read-only shared object containing the
application hash, artifact identity, plan/preview/session revisions, resource
limits and complete grants. The maximum preview is 2048 bytes. The service keeps
its authoritative plan internally and the user approves its revision; mutable
shared bytes cannot become the grant source. Confirmation rechecks application
bytes/manifest identity, policy revision, artifact revision, session generation
and preview expiry. Any mismatch requires a fresh preview and new confirmation.
The editor manifest is derived from named observed-capability scenarios before
launch acceptance, then compared with an explicit finite allowlist; one observed
run does not establish every possible authority requirement.
The preview object and referenced data descriptors include an explicit
`schema_version:u32` in their shared-memory headers; reject unknown values and
invalid declared lengths before parsing. Artifact text is raw bounded ASCII
behind that validated descriptor, not an implicit versioned wire payload.

## Resource, lifetime and observable authority

Every grant binds `(owner identity, session ID, session generation, instance ID,
instance generation, resource identity, rights, expiry)`. The broker grants the
editor access only after confirmation. Exit, cancel, session closure, expiry,
policy revocation or service-generation change stops delivery and mutation,
quiesces data readers/writers, revokes mappings and retires handles. V0 lasts at
most ten minutes per confirmed instance, using monotonic runtime time; expiry
is visible and requires a fresh preview. Preview expiry is thirty seconds.
An application deadline is five seconds without a completed dispatch/yield;
surface and artifact requests have a one-second profile deadline. Gates can use
a controlled monotonic clock; target tests also check the actual timer path.

| Holder | `Lang`: requestable authority | `ObsContract`: permitted observations and retained data |
|---|---|---|
| Keyboard driver | Produce validated reports/reset events for attached device; cannot choose application focus | Device reports and queue status; no artifact bytes, other surface contents or permission decisions |
| Focus/session service | Assign focus, own trusted launcher/recovery commands, route keys and prepare/confirm the selected plan | Focused session inventory, approved preview and named failures. Per-session errors omit other owners' resource IDs/content |
| Editor instance | Poll focused keys, create/present its one surface, read/replace selected artifact, inspect its own save receipt | Its delivered keys, selected draft/revision, own frames, own save outcome and own lifecycle reason. No desktop screenshot, global input, directory enumeration or foreign receipts |
| Compositor/display adapter | Read committed frames, compose chrome, write selected scanout | Visible frame pixels and bounded surface metadata; no raw artifact portal or launch grant authority |
| Artifact portal/Store service | Validate delegated selected-object grant; publish/reconcile owner-bound revisions | Selected bytes and receipt metadata needed for IO; cannot use the caller's grant to enumerate other objects |
| Recovery chrome | Retire the failed instance, observe/reconcile its named operation, prepare a new preview | Last confirmed artifact revision and bounded fault/save state; no silent transfer of old instance handles |

Revocation prevents future requests and delivery; it cannot erase pixels already
displayed, keys already delivered, artifact bytes copied into the editor or
trusted service logs. Shared mappings require actual quiescence and unmapping,
not merely invalidating a control handle. Responses/error timing, resource
pressure and compositor scheduling may reveal shared activity. Finite denial
gates do not establish hidden-affordance noninterference or complete isolation.
Test caller/deputy checks at enforcing adapters and the target kernel, not solely
in a UI button handler. A caller holding `SURFACE_PRESENT` cannot repurpose it
for input, focus, Store IO or a different instance's surface.

Bounds for the first profile are two sessions (one main and one gate witness),
one running editor per session, one surface per editor, two frame buffers per
surface, 64 key events per session, one active preview and one in-flight save per
session. Capacity exhaustion returns `Exhausted` without evicting another owner.
Desktop/service domains must fit the existing 16-domain target bound; the
coordinator assigns domain/endpoint/table budgets before RUN0 integration.
Trusted fixture injection uses a separate default-off host feature and is absent
from target/release profiles. It cannot obtain `SESSION_CONFIRM` as an application.

Queue overflow clears all pending presses/releases, increments the queue epoch,
emits exactly one reset/overflow notification and requires a fresh key press.
Unplug/reset follows the same rule. Focus changes discard the old focus queue and
reset modifier state. Event epochs must bind routing decisions when the event is
enqueued and when it is delivered, preventing keys for a preview from leaking to
an editor that launches later. No release event alone can synthesize approval.

Surface publication must also protect data from an adversarial writer. The
producer's writable mapping is revoked before a committed buffer is consumed;
the compositor reads only the frozen mapping. Completion permits a new writable
mapping with a fresh mapping generation. Never allow producer mutation during
scanout or use the cooperative SPSC ring as proof of immutability. Checked
`stride * height`, buffer length, offset/range, format and mapping rights are
validated before every acquisition/presentation. Bulk pixels and artifact bytes
are shared-memory data; control messages transfer descriptors only. CPU scanout
conversion still copies pixels and must not be advertised as end-to-end copy-free.
The creation reply's buffer references grant no writable mapping until acquire.
The editor has no unrestricted shared-memory map/close endpoint that could bypass
this lifecycle; the surface service owns region setup and teardown. Buffer
mutation through every alias, including old acquired handles, must fail while
the compositor is reading. Reacquisition returns a fresh mapping capability.

## Save and recovery contract

The artifact portal is a narrow deputy in front of Store IO. It resolves the
selected object from its grant, never from an application-supplied pathname or
unrestricted content ID. Shared schemas belong in `artifact_store_schema`;
publication/readback belong in `artifact_store_core`/`store_service`. No editor
or schema consumer takes ownership of CAS IO. The host CoW function and native
`ingest_artifact` can inform implementation but do not already supply this portal.

Save binds the instance, selected object, expected revision, operation ID and
immutable source snapshot. Publication compares the selected object's current
revision and revalidates authority at the commit point. A conflict or revocation
leaves the prior revision selected. New content is immutable and the old blob
remains readable by its authorized owner. A confirmed receipt binds old/new
revisions, selected object, source hash and operation ID. Reject reuse of an
operation ID for different bytes/base/owner. Two sessions editing the same object
must produce one successful expected-revision commit and one visible conflict.

After dispatch, timeout/disconnection is `Unknown` unless the enforcing service
knows the mutation did not commit. The UI retains the operation ID and does not
retry the commit automatically. Receipt lookup is a read and may reconcile the
original outcome; Unknown stays Unknown when retained evidence is insufficient.
Restart must not reinterpret it as failure or success. If unresolved, recovery
permits returning to the launcher and explains why save/reopen acceptance remains
incomplete. The old instance cannot retain grants while waiting for recovery.
Fresh recovery authority may inspect only the owner/session's original receipt.

Host UI1 can initially expose editing and a volatile fixture save, visibly marked
volatile. Its host Store acceptance leg requires real host publication/reopen
and the revision/receipt protocol; a mock receipt cannot pass that leg. UI2's
nonpersistent gate can use an explicitly volatile target artifact service, but
cannot claim save across reboot. Persistent UI2 requires STORE0 device-backed
write/flush/read, durable selected-revision and receipt records, and two separate
target boots. Process/service restart alone does not establish persistence.

Recovery performs stop-delivery → quiesce → retire surfaces/mappings/handles →
record confirmed/unknown save state → show chrome → fresh preview. A stopped
instance is never resurrected with an old generation. A compositor/focus restart
increments its service epoch, resets focus and invalidates all previous surface
attachments. Apps reattach through fresh grants; automatic restart is disabled.
The target service ports must keep trusted recovery usable after an editor fault
or infinite loop. If the display/session service cannot recover, report degraded
recovery and fail the relevant gate rather than count a serial marker as usable UI.

## Failure matrix and executable assertions

These are mandatory executable cases, not documentation-presence assertions.
The coordinator registers a runner that records each case's inputs, observed
outputs and assertion result before assigning handlers. Host cases use fake time
and fault adapters where specified; target cases execute the actual path. Every
required case must run exactly once or be explicitly failed/incomplete. An empty
case list, absent consumer or unexpected `NotImplemented` cannot pass.

| Case ID | Stimulus | Required observation / state assertion | First gate scope |
|---|---|---|---|
| UI-H01 | Cancel preview, then approve fresh fixture preview | Cancel creates no instance/grants; approval launches one pinned instance; exact approved resource list equals issued grants | Host contract/UI |
| UI-H02 | Edit fixture using typed press/release events | Draft equals `note=new\n`; renderer outputs expected pixel regions; frame metadata identifies that instance | Host UI |
| UI-H03 | Read private artifact/receipt, present foreign surface, poll foreign keys, assign focus or prepare/confirm/restart as editor; emit input as non-driver; present pixels over chrome | Each enforcing endpoint returns Denied with zero foreign bytes/handles/state changes. Only attached driver can produce input and authenticated trusted route can confirm; working trusted/driver/own-app witnesses still succeed. App pixels remain clipped; reserved indicator pixels unchanged | Host contract/UI, target equivalents |
| UI-H04 | Change manifest/policy/artifact revision or expire preview before approval; duplicate confirm | Stale/expired/consumed plan fails; no grants or execution; new preview requires new confirmation | Host contract/UI |
| UI-H05 | Focus change with queued modifier/Enter/release events, then 65 queued keys; unplug/replug | Old epoch events never reach new focus; bounded overflow/reset behavior; no stuck modifiers, synthetic approval or lost reset | Host/input replay |
| UI-H06 | Unsupported protocol/operation, unknown shared-object schema version, truncated/oversized payload, nonzero reserved fields, wrong-kind/stale handles, overflowing descriptor | Invalid/Stale/Unsupported before pointer/range access; prior draft/frame/grants unchanged | Contract and enforcing consumer |
| UI-H07 | Publish then attempt write to frozen buffer; stale reacquire/present; exhaust surface quota | Frozen write denied; displayed bytes match published frame; no use-after-unmap; quota error affects only requesting owner | Host model plus actual target MMU |
| UI-H08 | Revoke/expire a grant while a save/frame/key delivery is paused | Publication/delivery rechecks lifetime; quiesce before teardown; no post-revocation commit/key/frame; no recycled-generation access | Host and target |
| UI-H09 | Crash editor with unsaved draft; confirm restart; call using old handles | Recovery states draft loss; fresh preview/instance opens last confirmed revision; all old-generation requests fail | Host UI/runtime |
| UI-H10 | Stall one session's artifact worker; issue read/launch/frame on second session; restart worker | Second session completes within profile deadline; bounded first-session timeout; owner-bound uncertain state retained; no leaked quota after teardown | Host services; target service ports |
| UI-H11 | Fault/restart focus or compositor with attached apps | Trusted recovery returns, service epoch changes, stale input/surfaces denied; apps reacquire safely; unrelated session has no authority transfer | Host UI; target surface |
| UI-S01 | Save fixture, close, launch and read host Store again | Real publication receipt and new content match replacement; prior blob intact; selected revision matches reopened bytes | Host Store leg |
| UI-S02 | Drop commit reply after publication, then before publication; lookup original operation | No automatic redispatch; committed receipt reconciles first case; second case reports established failure or Unknown without invented success | Host Store and target persistence |
| UI-S03 | Two same-base commits; mismatched operation reuse; revoke during prepare | Exactly one current-revision winner; other Conflict; operation reuse denied; authority failure leaves old selection | Artifact portal/Store |
| UI-R01 | Load bad/oversized ELF, unsupported image and W+X/overflow/overlap fixtures | Loader rejects before execution; no leaked frames/domains/caps; valid image still loads afterward | Target runtime |
| UI-R02 | Valid image writes private memory and invokes typed allowed/forbidden IPC; attempts kernel/foreign-memory/device access | Execution occurs in user mode; allowed IPC works; forbidden calls/loads fault or deny without changing witness domain/kernel; identity cannot be forged | Target runtime |
| UI-R03 | Deliberate user fault and infinite loop | Kernel regains control by interrupt deadline, records bounded reason and retires instance; recovery service can launch fresh image | Target runtime |
| UI-I01 | Valid/short/oversized/rollover HID report corpus; duplicate sequence, unplug/reset | Report decoder produces specified typed keys/reset; malformed reports never fabricate keys; queues remain bounded | Input replay |
| UI-I02 | QEMU keyboard device sends fixture keys through selected native controller/HID stack | Target editor receives device-correlated events; denial/reset cases exercised through this path; no host typed-event injection substituted | Input device/QEMU |
| UI-D01 | Application publishes distinct frame regions and sequence, attempts to paint system indicator/preview and spoof chrome confirmation; invalid stride/format/mapping; adapter restart | Actual target scanout shows app clipped to its region, compositor-owned indicator/preview pixels unchanged, and confirmation accepted only on authenticated chrome route; frame correlated with publication. Invalid frames do not alter framebuffer; restart produces fresh frame | Target surface/QEMU |
| UI-T01 | Boot, keyboard preview/confirm, edit, fault and restart | Actual target runtime, keyboard, focus, compositor, broker and volatile artifact service execute; observed screen and typed trace prove flow; host helpers listed | Target human task/QEMU |
| UI-P01 | Commit/flush, terminate target, boot new target, reopen; interrupted publication boot | Actual persistent backend retains receipt/revision/bytes; fresh nonces/artifact identities bind separate boots; interrupted selection is verified old or verified new, never partial | Persistent human task/QEMU; separate metal leg |

Initial implementation assertions H01/H02/H09/S01/R02/R03/I02/D01/T01/P01
must fail with an absent launch/editor/save/runtime/device/frame/recovery path.
Denial-only stubs cannot pass useful-task acceptance. H03–H08/S02–S03/R01/I01
must also assert prior state and a working authorized witness, preventing a
service that rejects everything from passing. No missing target prerequisite is
converted into a host result. Missing tooling/qualified inputs report INCOMPLETE;
an available runner whose required behavior is absent reports FAIL.

For QEMU, image assertions consume captured scanout pixels, not solely an
application-supplied frame hash. Use deterministic known-color/font regions and
sequence correlation; the serial transcript separately records app/instance,
focus, grants, fault and frame identities. The test harness may send keys to the
QEMU keyboard device, but cannot directly enqueue native events for UI-I02/T01.
Human usability remains a separate measured check: record an operator's task
completion, errors, recovery understanding and observed response times. Scripted
success alone establishes neither everyday readiness nor general responsiveness.

## Proposed recipes, artifacts and ownership

All commands/paths in this table are proposed. They become executable only after
the coordinator registers scripts, schemas and `justfile` entries. No worker may
substitute a passing documentation grep for these acceptance cases.

| Proposed recipe | Proposed script / outputs | Mandatory cases |
|---|---|---|
| `just foundry-desktop-contract-ui0` | `tools/ci/foundry_desktop_contract_ui0.sh`; `out/desktop/ui0/` | Layout/codec checks plus H03–H08 against enforcing host adapters; useful witnesses required |
| `just foundry-desktop-launch-ui1-0` | `tools/ci/foundry_desktop_launch_ui1_0.sh`; `out/desktop/ui1-0/` | H01, H04, H06, launch/grant portions of H03/H08/H09; non-rendering host instance and typed witness. Records unimplemented surface/editor/Store cases as outside this bounded gate, never as passing |
| `just foundry-desktop-host-ui1` | `tools/ci/foundry_desktop_host_ui1.sh`; `out/desktop/ui1/` | H01–H11; S01–S03 separately marked real-host-Store leg, with fixture-only leg unable to satisfy it |
| `just foundry-target-runtime-run0` | `tools/ci/foundry_target_runtime_run0.sh`; `out/desktop/run0/` | R01–R03, actual grants/revocation and repeat launch after teardown |
| `just foundry-keyboard-replay-in0` | `tools/ci/foundry_keyboard_replay_in0.sh`; `out/desktop/in0-replay/` | I01, H05, denial and resource exhaustion |
| `just foundry-keyboard-device-in0` | `tools/ci/foundry_keyboard_device_in0.sh`; `out/desktop/in0-device/` | I02 plus device reset/unplug and denied delivery |
| `just foundry-desktop-target-surface-ui2` | `tools/ci/foundry_desktop_target_surface_ui2.sh`; `out/desktop/ui2-surface/` | D01, H07/H08/H11 with real mappings and scanout |
| `just foundry-desktop-target-task-ui2` | `tools/ci/foundry_desktop_target_task_ui2.sh`; `out/desktop/ui2-task/` | T01 plus target H01/H03/H04/H05/H09/H10; volatile data explicitly stamped |
| `just foundry-desktop-persistent-task-ui2` | `tools/ci/foundry_desktop_persistent_task_ui2.sh`; `out/desktop/ui2-persistent/` | P01 and S02/S03 on actual target Store/device IO across boots |

Each gate retains `result.json`, per-case results, logs, input/corpus hashes and
frame images when relevant. The coordinator assigns a versioned result schema
with gate ID, contract revision, git SHA plus tested-diff digest, run ID, component
execution inventory, fixtures/devices/backends, required/executed case IDs,
outcome, failures/incomplete reasons, artifact hashes and resource-leak checks.
Host/replay are recorded as scope values, not invented metal evidence levels;
target/HIL legs use the applicable existing evidence/provenance conventions.
Save evidence includes operation/receipt/base/new identity and each boot nonce.
Reserve these output paths and shared build/QEMU resources; the coordinator
serializes conflicting final gates and preserves outputs before reuse.

This is a successor scope proposal, not authorization to edit every listed path.
The coordinator narrows each dispatch to exact files and one frozen revision.

| Packet / writer | Proposed exclusive paths and boundary | Start/completion condition |
|---|---|---|
| Contract registration / coordinator | Four new IDLs above; `tools/ci/run_codegen.sh`, generated outputs, `kernel_api` inclusion/wire glue, workspace/registries, all new gate entry points, `justfile` and shared status/history | Independent UI0 review; freeze layouts/IDs/rights/schema and install executable assertions before implementation fan-out |
| RUN0 / runtime worker | New `target_runtime/` loader/client library and `apps/artifact_editor/` target entry adapter; architecture/kernel integration is coordinator-owned unless explicitly reassigned | Frozen runtime/authority assertions; R01–R03 and enforcing denial/lifetime gates. Return needed syscall/timer/domain glue to coordinator rather than embedding policy in kernel |
| IN0 / input worker | New `drivers/keyboard/` bounded decoder/driver, selected `drivers/reference_vaults/` dossier/corpus paths assigned by coordinator | Input contract permits decoder/replay work; device register work waits for exact controller Vault and Oracle `protocol_trace`. I01 then I02; physical work waits for H0/H1 and authorization |
| UI1.0 / launch-lifetime worker | New `services/desktop/src/launch.rs`, `services/desktop/src/authority.rs`, `services/desktop/tests/launch_lifetime.rs`, `services/desktop/tests/fixtures/launch_witness.rs`; crate registration/module root via coordinator | Accepted contract subset and initial failing launch assertions; H01/H04/H06 and launch-specific H03/H08/H09. Host typed producer/consumer and actual witness instance; no target runtime prerequisite |
| UI1.1 / desktop worker | New remaining session/focus/compositor/host adapter files under `services/desktop/` and editor core in `apps/artifact_editor/`, assigned by exact filename | UI1.0's frozen grant/lifetime boundary; H01–H11 and explicit fixture/real Store legs. Coordinator gives RUN0 and UI1.1 disjoint files within the app directory |
| STORE0 / storage worker | Exact assigned existing block-driver files and new selected-artifact portal/Store IO files | Existing block Vault/Oracle plus accepted durable revision/receipt fault contract; actual device-backed read/write/flush before persistent consumer acceptance. Shared Store handlers/schema changes have one named writer |
| UI2 / integration writer and coordinator | New target adapter files under `services/desktop/`; boot framebuffer handoff, kernel glue and service registry via coordinator | RUN0 + IN0 + UI1 before target task; actual surface/service ports first; STORE0 before persistent task; target gates verify each joined consumer |
| Independent reviewer | No implementation write ownership | Review immutable scoped diff/evidence before coordinator acceptance, check useful/denied/failure/unrelated-session behavior and claim limits |

REC0's S13 update/rollback protocol is adjacent work, not supplied by desktop
restart or P01 artifact persistence. Conversely, broad model comparisons and
physical NVMe qualification are not prerequisites for bounded UI1 host work.
Unexpected scope overlap, missing controller evidence, incompatible layouts or
unavailable enforcing authority return to the coordinator for replanning.

## Acceptance and remaining prerequisites

UI0 is complete only when the coordinator accepts this independently reviewed
task, runtime choice, typed boundaries, lifetime/observation contract and
assertions, records consequential choices, and registers the next executable
packet. UI1 completion is useful host behavior with denial/recovery evidence;
RUN0 is target execution and enforcement; IN0 distinguishes replay from device
completion; UI2 joins actual target consumers. None individually is S14/S15
graduation or a completed roadmap.

The next prerequisite is contract registration and initial failing assertions.
The coordinator may register only the UI1.0 launch/session/selected-artifact
authority subset first, allocating the remaining protocols before their own
consumers are dispatched. UI1.0 must not expose a generic arbitrary-operation
escape hatch while waiting for those contracts. Its mandatory useful, denial and
failure assertions run before launch handlers; editor and target implementation
then join as their own dependencies clear.
Target acceptance still needs user-mode loader/traps/timer, endpoint/right
enforcement, target policy/session/compositor/artifact ports, verified mapping
teardown, retained GOP framebuffer handoff and scanout, controller Vault/Oracle
and native device input. Durable save adds actual storage IO and crash-safe
revision/receipt publication. Physical runs add qualified S12/S13 evidence and
explicit HIL operation authority. These requirements cannot be filled by fixture
markers, an empty denial suite or assumed runtime isolation.
