# UI1.1d Linux editor process contract

2026-10-06. **FROZEN_CONTRACT_RUNTIME_NOT_IMPLEMENTED.** This contract specifies the next bounded successor to accepted UI1.1c. Executable gate-first assertion review and actual missing-API RED precede handlers; no process runtime acceptance follows from this document.

The wire, authority, bounds, API and assertion inventories are maintained in `docs/contracts/editor-process-v0.json`; exact signatures are in `docs/DESKTOP_EDITOR_PROCESS_API_V0.md`. Existing native Save/task contracts remain referenced without broadening their accepted evidence.

## Behavior and scope

A genuine approved Linux Rust child reads the selected artifact, edits its own bounded ASCII draft from actual focused keyboard deliveries, rasterizes its own640×480 BGRA image through actual shared mappings, and publishes immutable text/state plus pixels. Desktop authenticates and validates that publication, composes protected chrome and owns genuine Save authority. Store retains IO, original execution and recovery. The parent must not call legacy `step_native_editor` or `render_native_editor` to edit or rasterize the process document. UI1.0's observation-only child is not an editor proof.

The model-free route is Ctrl+Esc → selected app preview → fresh focused Enter → valid first child frame → focus → edit → genuine Ctrl+S → full receipt-validated trusted Saved → Ctrl+Q → new preview/approval → child reads confirmed bytes. This adds host process/typed-carrier/sealed-mapping evidence. Logical input and offscreen pixels remain host evidence. There is no claim of hostile same-UID containment, device input, window presentation, target MMU revocation, power-loss durability, noninterference, physical hardware or everyday readiness.

## Frozen source graph and ownership

The shared pure Rust crate is `services/desktop_editor_core`, with `default=[]` and no Desktop or Store dependency. Extract existing pure edit/font/raster code once; both legacy c and actual child use it. It has no issuer, capability, IO or process authority. Preserve c's behavior, key corpus, font and pixels through unchanged consumer assertions.

Use optional Linux-only `libc=0.2.180`, already pinned in Cargo.lock. Desktop `host_editor_process_v0_dev` includes existing `editor_native_save_v0_dev`, optional pure core and optional libc. App parent `host_editor_process_v0_dev` includes existing `host_native_save_v0_dev` and Desktop process. Separate app `host_editor_process_child_v0_dev` includes only pure core, kernel_api and libc. The child binary is `apps/artifact_editor/src/bin/editor-process.rs`, requires that child feature, and is built independently without parent features. Its active dependency closure must exclude Desktop and Store; a parent-feature-built artifact is not accepted. New features remain absent from default builds.

Root owns workspace/Cargo declarations, protocol registry/IDL/codegen/generated output, `justfile`, Foundry inventories, recording schemas, source manifests and maintained status/history. Workers receive disjoint implementation paths only after freeze. Executable assertions belong at `apps/artifact_editor/tests/process_task.rs`; semantic reader at `tools/ci/desktop_editor_process_result.py`; source registry at `tools/ci/editor_process_sources_v0.json`. Future command is `just foundry-desktop-editor-process-ui1-1d`, added only by Root. Existing canonical54/extended38 inventories do not silently change.

## Held executable and actual process

Only the Root-registered independently built Cargo child artifact may be opened, with O_NOFOLLOW|O_CLOEXEC and regular-file checks. Copy at most1GiB through a64KiB bounded buffer into a memfd created with MFD_ALLOW_SEALING|MFD_CLOEXEC. Hash the held copied bytes against the registered build digest, validate ELF, set mode0500, and install WRITE|GROW|SHRINK|SEAL. Retain that exact held sealed executable until actual reap. Close the source FD after verified copy. No arbitrary caller path, PATH lookup, shell, script, child-reported hash or `/proc` re-open selects execution.

Use explicit fork and `execveat(held_fd, "", prepared_argv, prepared_envp, AT_EMPTY_PATH)` on that same sealed FD. Source path replacement or original-inode mutation after the verified snapshot cannot alter these executed bytes. A before-copy digest mismatch denies admission. D02 uses a real barrier between final verification and exec to test path replacement and original-inode mutation; attempted held sealed mutation must fail. A successful exec-error-pipe EOF alone is not binary identity. Parent may inspect actual `/proc/<owned-child>/exe` descriptor identity against the held sealed inode before bootstrap, and records actual held FD identity privately. No raw FD authority getter exists.

Parent prepares all argv/envp pointers, fixed descriptor remap/close plans, error bytes and barrier before fork. The post-fork child branch performs only reviewed async-safe syscalls, fixed errno write and `_exit` on failure: no Rust allocation, logging, locks, unwind or Drop. Deliberately inherited channel/stdio descriptors are explicit; fixture descriptors outside the inheritance allowlist are close-on-exec or closed by the prepared plan. The finite inheritance assertion checks the actual fixture; no global hostile-process containment is inferred. Empty environment, no executable fallback. Dynamic system loader/libraries remain explicit host prerequisites, outside held main-ELF byte identity.

The child waits at a bounded preexec barrier until parent installs the fork-derived PID/birth, acquired pidfd and authentic supervisor wait ownership. Failure retains actual cleanup ownership and never admits bootstrap. Unsupported executable memfd, pidfd or execveat is INCOMPLETE rather than a simulated fallback.

## One supervisor and one waiter

Internal `ProcessSupervisor` exclusively owns `OwnedLinuxProcess`: actual fork-derived PID, native birth, pidfd and waitpid authority. There is no std::process::Child requirement and no second waiter. Public opaque `OwnedEditorChild` owns the supervised record, channel, actual supervisor JoinHandle and charges, not direct signal/wait authority. Ensure SIGCHLD/reaper policy prevents other waiters from taking this child.

Retire first denies ordinary effects and retains original Save owners, then requests signaling from that supervisor's held pidfd. Consuming join validates its authentic terminal waitpid result and then joins the actual supervisor. Failures return the sole opaque supervised owner and all charges. pidfd readiness, EOF, kill_requested, Drop, a payload PID or a Rust thread join alone is not a reap proof. Supervisor join, child reap and Store producer joins stay distinct. Existing `JoinedProducerProof` observes a parent Rust JoinHandle and is never repurposed as child proof.

Getter-only `ProcessChildObservation` reports actual fork PID, parent-observed birth ticks, held executable digest, acknowledged input sequence and accepted publication generation. Exit proof additionally reports genuine wait outcome Exited(code) or Signaled(signal) and supervisor_joined. All fields/constructors remain private; copied observations grant no authority. Actual descriptor/inode/pidfd/channel bindings live in the private recording schema, never a caller-supplied witness.

## Authenticated typed channel

One pre-created AF_UNIX SOCK_SEQPACKET pair carries the existing canonical88-byte Envelope:20-byte header,64-byte payload, four zero tail bytes. All integers encode explicit LE; no repr(C), dynamic payload, stream length prefix or ioctl escape. Proposed namespace `portal.desktop_editor_process`, version1, protocol384 remains unallocated until Root checks the complete registry. Existing protocols and grants/chrome remain byte-exact.

Enable SO_PASSCRED before any packet. Every received record requires exactly one kernel SCM_CREDENTIALS for the actual owned sender PID and expected real UID/GID. SO_PEERCRED on the pre-spawn socketpair identifies creation-time credentials and cannot authenticate the child alone. Channel/bootstrap/actor/PID fields are checked references, never constructors. Gate credential controls are unprivileged actual forged-credential kernel rejection and an actual owned helper intentionally inheriting the endpoint with wrong native PID. Received privileged wrong-UID/GID parsing is separate, not required or claimed. Preserve actual kernel errno and received denial evidence distinctly.

Receive ancillary FDs with close-on-exec and bound actual ownership before validation. Reject MSG_TRUNC/MSG_CTRUNC, unknown/duplicate/missing credentials, duplicate SCM_RIGHTS and wrong exact FD count. Every actually received alias is closed on denial; no application closure is fabricated for kernel-discarded aliases. A malformed channel retires before mapping/adoption. Check sender/direction, length/frame/padding, protocol/type, exact IDs/sequence/current attachment before any effect.

The exact eight payload layouts are in the wire inventory for adoption into `docs/contracts/editor-process-v0.json`:

| Type | Message | Direction | Exact FD roles |
|---|---|---|---|
|1|Bootstrap|Parent → child|sealed selected Text64, offered TextState, offered Surface|
|2|DeliverInput|Parent → child|none; offer_id zero|
|3|InputAck|Child → parent|none|
|4|Stop|Parent → child|none|
|5|Publish|Child → parent|exact offered TextState and Surface duplicates|
|6|PublishAck|Parent → child|none|
|7|ReceiptNotice|Parent → child|none|
|8|RequestPublication|Parent → child|offered TextState and Surface|

Only these messages/directions/counts are valid; type9 is invalid. One input and one publication can be pending, with no next input during publication. Request, publication and generation counters use checked additions; no wrap or eviction. Error replies retain existing status numbers and canonical zero fields. InputAck success may indicate unchanged or applied-unpublished state, with per-event generation equality/checked+1; it does not update parent pixels or mint Save.

## Input and bounded publication policy

Every genuine Focus delivery reaches and is individually acknowledged by the child. Publication batching never drops, coalesces or parent-processes key events. Force a publication after **128 acknowledged deliveries OR64 key-down events, including modifier downs, whichever occurs first**, before the next delivery. Releases count toward delivery count but not key-down count. A human render request may flush earlier. The counters reset after terminal accepted publication, never refund lifetime offers.

Exactly64 ordinary process queue slots plus one separately reserved reset slot gives65 total; existing Core input queue capacity64 and its actual overflow-reset semantics remain unchanged. The65th ordinary event is denied without overwriting; the reset remains admissible at ordinary saturation; a second queued reset is separately denied. Focus/reset/service changes retain enforcing epoch and retirement rules.

The capacity scenario starts from a genuine selected empty ASCII artifact and delivers4096 unmodified supported ASCII a presses plus4096 releases:8192 DeliverInput and8192 InputAck records. This yields64 edit publications. Bootstrap plus immediate Ctrl+S capture gives66 pairs; one optional final close-boundary publication allows67 for the whole scenario. Ctrl down/S down/S up/Ctrl up controls are retained; the actual genuine S-down captures the original Save Instant and forces its flush before subsequent releases/input. No initial deletion or chord is silently omitted. The contract remains ASCII with4096 body bytes; arbitrary Unicode key translation is not claimed.

DeliverInput flags are NORMAL0, RESET1 or SAVE_CAPTURE2, no combinations. Genuine Ctrl+S mints a private capture ticket before any child response and forces matching RequestPublication immediately after all preceding acknowledgments. Child freezes that local state until terminal publication. Only this exact request may consume the ticket. No child message can ask for Save, renew the original deadline or replay a ticket. Early human frame flushes consume real immutable offers; exhausting128 lifetime pairs retires truthfully with last accepted state and original Save owners retained.

## Sealed mapped data and internal copies

Selected input is unchanged Text64 plus validated ASCII body, length64..4160, with genuine selected object/current revision/hash. TextState is128-byte header plus0..4096 body. Header0..63 is existing EditorTextHeaderV0; LE u32 cursor/selection_start/selection_end/first_visible_line at64/68/72/76; LE u64 text/view generations at80/88; full companion Surface SHA256 at96..127. Body begins128. Cursor and selection obey actual body bounds; absent selection uses both0xffffffff. Generations match latest acknowledgment and publication. Current base must match parent-authorized confirmed tuple; child cannot invent receipt advance.

Surface is exactly640×480×4=1,228,800 BGRA opaque bytes, stride2560. Parent never exposes scanout or protected chrome mappings. Parent creates and retains every offered memfd and actual(dev,ino), reserves before allocation, and never keeps a shared writable view. Child writes through actual MAP_SHARED, resizes bounded TextState, unmaps every shared writable alias, then installs WRITE|GROW|SHRINK|SEAL on both objects. F_SEAL_FUTURE_WRITE, mprotect or producer booleans are insufficient. Real retained writable aliases must cause EBUSY and leave prior accepted state unchanged.

Parent authenticates the returned pair, requires exact offered identities/regular type/lengths/all four actual seals, then maps read-only and validates complete text/state/body digest/full frame digest/base. Recheck held identity, size and seals before exposure; recheck enforcing authority, pending request, attachment, focus/service epoch and original deadline before atomic adoption. No enforcing lock spans recv/send/map/full hash/raster/Store IO/signal/wait/join. A concurrent retirement wins ordinary admission; issued Store permits remain independent. Invalid pair releases temporary aliases and preserves prior current pair.

Parent privately copies at most4096 body bytes into existing Desktop SnapshotData and uses existing CPU surface copy/composition. Process transport is actual shared memory; Save is not claimed end-to-end copy-free. This preserves Store's existing genuine source/original ownership without a simultaneous data representation redesign.

After the first validated child publication, private pump uses existing genuine Surface.Create → Focus.Assign → Surface.Acquire → Surface.Present with actual Core-owned bindings/CreateReply values and child pixels. It preserves existing epoch/cap/error/retention checks; no implicit grant or caller numeric surface ID. The consumer then calls existing `compose_native_save` with its actual compositor endpoint. Legacy parent rasterization remains denied for process mode.

## Genuine enrollment, private Save and death

Existing genuine Read creates NativeEditor through SaveGateGuard::initialize_editor. Preparation consumes that actual editor and atomically tags the shared DocumentLife ProcessOwned once. Preparation error returns the unchanged editor. Legacy aliases cannot step/render it; repeated Read/take_editor cannot issue another ordinary editor for that process-owned document. Ordinary c documents retain existing behavior. No new public body/image/receipt setter or caller-supplied Verified trait exists.

Private DeliveredProcessInput, VerifiedProcessPublication and ProcessSaveCapture retain genuine Focus/FD/capture ownership. Private adoption alone copies verified snapshot and mints existing NativeSaveIntent from the genuine Ctrl+S ticket. Exact proposed consumer signatures are supplied for adoption into `docs/DESKTOP_EDITOR_PROCESS_API_V0.md`. Root's gate-first consumer can call genuine initialize → prepare → registered executable → spawn → pump → take intent → existing SaveChains → retire → join, without inventing authority.

Parent's common app SaveChains remains owner of existing Read/Allocate/Commit/Status/Recovery and actual original source/ticket/permit/IO. Preserve original Instant limits: min(original Ctrl+S+1000ms, original stage entry+1000ms); child work/publication cannot renew them. ReceiptNotice is derived privately from genuine parent receipt validation and confirmed tuple; no public receipt injection method. Protected Saved requires full existing176-byte receipt checks and exact current draft tuple. Older Save completion with a newer draft stays Unsaved.

Before capture admission, child death denies new Save. Before permit retain known allocation and existing definitive/Unknown distinctions. After permit, immutable private snapshot/original/source/ticket/actual IO owner survives child/socket death. Reap cannot cancel a permit, prove NoAllocation, join Store Rust IO, restore a grant or invent receipt. Original-only lookup/reconcile uses genuine origin and fresh bounded query; no Allocate/Commit replay. Restart requires fresh preview/approval, a new owner/channel generation and preserved old original recovery.

## Exact accounting and deadlines

Per child, data FD/mapping owners cap12; fixed-process FD owners cap16; combined cap28, or56 for two children. Fixed inventory is source1 +sealed executable1 +socketpair2 +pidfd1 +stdoutpair2 +stderrpair2 +stdin-null1 +exec-errorpipe2 +preexec-barrierpipe2 +remap scratch2 =16. This is a conservative maximum, not fabricated occupancy. Every actual alias/transfer/close is recorded and charged before create/dup/import/map. Source closes after copy; sealed executable remains until true reap. No hidden Command-created descriptors exist. Ambient host descriptors and fork-child aliases are outside this finite parent-owner claim.

Data peak is current pair2FD+2maps, pending offered2FD, returned2FD and verified2maps=10, with bootstrap selected FD+map2 giving12. Parent creation gives lifetime128 pairs plus one selected object=257 objects; maximum data bytes128×(4224+1228800)+4160=157,831,232 per child,315,662,464 for two. ACK/drop never refunds lifetime even if child retains aliases. This bounds broker-created bytes, not hostile OS allocations. Retained executable bytes have the separate1GiB-per-child bound.

Max two live editor children, one per existing session; max two supervisor handles in a separate actual ledger; max two owned gate-only negative helpers, so four actual OS children including editors. Existing native Save producers64 and Store IO2 remain distinct. Child stdout and stderr each cap16384; overflow retains real partial diagnostics with failure. Existing service caps remain unchanged.

Child execution budget1..5000ms, default5000, is captured before spawn and never renewed by a poll/roundtrip. Existing Save1000ms bound is stronger. Cleanup reserve2000ms is observation/retirement only, never more execution authority. Sole supervisor actually waits; uncertainty retains process/charges and makes gate INCOMPLETE. Existing session-B progress while A stalls must meet its unchanged1000ms assertion. Case work20000ms plus cleanup2000ms; whole gate1200000ms. Source feasibility does not prove these runtime bounds.

## Complete bounded evidence and gate-first order

Thirty finite assertion families cover positive behavior, all caps+1, denial/failure, correct sender, exact eight-message wire/FD inventories, held-byte exec TOCTOU, writable-map EBUSY, private capture/death/original recovery, true wait and compatibility. Every denial compares actual prior output/effect/counts and an authorized witness. Unknown is never definitive noncommit. No fake PID, seal, EBUSY or join is admissible.

Complete scenario controls use one compact append-only JSONL leaf, at most32768 rows/scenario,131072 rows aggregate,512 bytes/row and16MiB per control leaf. Each row preserves canonical control bytes, direction, sequence/request, actual credentials and owned FD/object-ledger references. Receipt bytes are separate complete raw leaves. Scenario metadata binds controls leaf size/SHA/row count/range; raw_refs_per_scenario1024 counts file refs, not events. No sampling, implicit segmentation, truncation or borrowed rows. Raster/text/raw receipt leaves stay2MiB; metadata leaves32MiB. All raw/control/metadata files charge the2GiB corpus,16384 files and17000 entries; retained child binaries have explicit separate1GiB-per-binary charge. Source manifest512KiB/1024 rows; path256 bytes/depth16. Overflow fails before growth with truthful partial evidence.

Root first adopts independently reviewed closed contract/API/assertions and source graph. Workers then write executable gate-first assertions, record actual missing-API compile RED separately from behavior RED, and obtain independent assertion review **before handlers**. Root owns IDL/codegen/registry integration; disjoint workers implement pure extraction/carrier/private adoption/common app composition; independent source review precedes actual Linux RED/GREEN. Full exact active child graph, retained artifact, raw process/channel/maps/controls/receipt and distinct joins are mandatory. Missing Linux prerequisites are INCOMPLETE; no macOS emulation proves them.

New typed interfaces and extraction legitimately change source closures. Old Task203/source745 pins remain historical accepted c evidence, never permanent allowlists. Regenerate/review new current closure and rerun unchanged c18, Reader93, the1000ms other-session assertion, all five affected consumers and integrated canonical checks before accepting d. Any future canonical inventory successor needs independent review; this packet does not silently add/remove lanes or grant speedup/release/HIL authority.

## Source references and remaining work

API details above are fixed Root choices. Remaining work is implementation and evidence: prove actual Linux prerequisite availability, correctly implement the prepared async-safe child branch and sole supervisor, preserve active graph/extraction compatibility, and meet finite deadlines/caps. These are executable review/gate questions, not permission to choose weaker fallback APIs.

Linux man-pages consulted2026-10-06 specify APIs, not observed RamenOS behavior:

- [memfd_create](https://man7.org/linux/man-pages/man2/memfd_create.2.html): sealing-enabled anonymous shared objects.
- [F_GET_SEALS/F_ADD_SEALS](https://man7.org/linux/man-pages/man2/F_GET_SEALS.2const.html): inode-wide irreversible seals, WRITE/EBUSY and FUTURE_WRITE distinction.
- [unix](https://man7.org/linux/man-pages/man7/unix.7.html) and [recvmsg](https://man7.org/linux/man-pages/man2/recvmsg.2.html): per-message credentials, creation-time peer credentials, rights transfer, ancillary truncation/CLOEXEC.
- [pidfd_open](https://man7.org/linux/man-pages/man2/pidfd_open.2.html) and [pidfd_send_signal](https://man7.org/linux/man-pages/man2/pidfd_send_signal.2.html): held process references/signaling, distinct from sole wait/reap.
- [execveat](https://man7.org/linux/man-pages/man2/execveat.2.html): AT_EMPTY_PATH executes a held descriptor; ELF-only avoids script interpreter FD issues.
- [fork](https://man7.org/linux/man-pages/man2/fork.2.html): post-fork child of a multithreaded parent must restrict work until exec to async-signal-safe operations; inherited descriptor/memory state requires explicit setup.
