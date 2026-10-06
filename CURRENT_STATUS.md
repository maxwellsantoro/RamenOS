# Current Status

**Last Updated:** 2026-10-06
**Status:** Active and authoritative for landed state
**Current Slice:** UI1.1 host editor; S12.4 HIL appliance physical loop pending
**Software Lane:** SW0 foundations through A2.9 implemented; full A2 and comparison pending

RamenOS is public pre-alpha, building toward the everyday OS for humans and AI
agents described in [VISION.md](VISION.md). This file records implemented behavior
and evidence boundaries. [NEXT_TASKS.md](NEXT_TASKS.md) owns the next work;
[CHANGELOG.md](CHANGELOG.md) holds detailed milestone history and
[DECISIONS.md](DECISIONS.md) holds rationale.

## Foundry development and CI execution

The independently reviewed [execution profile](docs/FOUNDRY_CI_OPTIMIZATION_V0.md)
adds `just dev-check` with one exclusive persistent compiler target, one-build
content-stable generation, and a complete canonical Foundry inventory. CI splits
quality, host/Docker and QEMU work across isolated runners; the stable required
Foundry aggregate fails closed on failed, cancelled or unexpectedly skipped lanes.
The accepted dependency resolution is now tracked in `Cargo.lock`; toolchain and
download caches follow the pinned manifest. NativeRead and NativePreview keep
fresh targets by default and support explicit, source-bound compilation reuse
with separate exclusion/enabled targets, fresh assertions and retained binaries.
Early read-only input and umask checks deny unsafe cache setup before compilation.
Every stage retains actual monotonic timing and exit/reap observations.

The CI optimization integration passed all 52 stages on its assembled
candidate with `RAMEN_CI_STRICT=1`, `RUST_TEST_THREADS=1`,
`RAMEN_FOUNDRY_BUILD_CACHE=1`, a private Cargo home and `umask 022`.
The run took 844.028 seconds and includes the original 36 extended stages,
Docker controls, host consumers, QEMU and storage assertions. Initial permission
and fixture-access failures remain retained; permission repair affected only
owned temporary compilation outputs and preserved executable bytes. This is
host/QEMU evidence, not a kernel fix, physical qualification or release proof.

A measured schema developer check ran 94 library tests on both passes; its
command-time sum fell from 70.871 seconds cold to 1.238 seconds warm. Independently
reviewed focused Linux cache runs reran all nine NativeRead and five NativePreview
cases: command sums fell from 41.590 to 5.697 seconds and 56.784 to 6.412 seconds.
Those focused measurements precede the final umask-only helper amendment and
exclude setup and binary retention. Final integration validates the amended
helper. They do not establish a whole-CI speedup. All sixteen strict package
checks remain because multi-package Cargo unit graphs changed feature units.
The hosted workflow is reviewed configuration; a GitHub-hosted run is not claimed.

## Execution state

The S12.4 physical lane awaits test-hardware setup: first live serial capture,
then Intel AMT 11 power/reset, S12 on SATA, and S13 NVMe boot/update/rollback.
No live capture or actuation is scheduled. SW0 continues independently of lab
access and NVMe graduation. The dependency-driven queue in `NEXT_TASKS.md` also
allows S14/S15 contracts, host/replay work, and QEMU implementation to proceed
without the model comparison or physical qualification. Driver work retains its
own Reference Vault, Oracle and gate requirements; physical integration requires
the prepared observation/actuation loop. The [desktop v0 design](docs/plans/desktop-v0.md)
is accepted after independent review. UI1.0 now implements host permission preview
and launch lifetime with a real
non-rendering Rust witness and generated protocol-336 messages. The default-off
Unix fixture passed its 17-case Foundry gate on macOS and Linux, retaining actual
process identity, typed exchanges and cleanup evidence. UI1.1a now adds a
keyboard-driven volatile editor with typed focus and offscreen surfaces.
UI1.1b now adds a default-off real host Store transaction and joined-owner
recovery. The bounded native Read prerequisite connects approved editor authority
to that Store with live copy checks. The integrated keyboard/editor/Store task,
actual editor process and
RUN0 post-firmware memory ownership are next dependencies. RUN0.0's
pure map/retention admission prerequisite now passes 17 reviewed assertions,
with sticky insertion-overflow rejection and conservative bounded selection.
Actual firmware exit, retained-object collection and allocator installation are
not connected yet. USB
xHCI/HID, the target runtime and target desktop remain future work.

The [UI1.1 editor proposal](docs/plans/desktop-editor-v0.md) and
[external boot-profile Oracle proposal](docs/plans/boot-profile-oracle-v0.md)
are independently reviewed preparation packets. UI1.1 separates the volatile
in-process editor, real Store transaction, integrated task and actual editor
process. The Oracle proposal keeps external inspection distinct from guest
access and requires a reviewed relocated-entry resolution method before capture.
The editor's [wire allocation](docs/DESKTOP_EDITOR_WIRE_V1.md) now has five
canonical IDLs and generated Rust modules: input 802, focus 832, surface 833,
editor session 352 and artifact 368. Independent review checked all 43 messages;
IDL lint, `kernel_api` checks and the existing 17-case launch consumer pass.
The [shared host API](docs/DESKTOP_EDITOR_HOST_API_V0.md) is independently
reviewed and frozen, with a pinned font and independent old/new raster
expectations. Producer held state survives focus changes; confirmation requires
a release and fresh press. Exact rights, checked leases, observable pause
barriers and live save-admission state now support 13 executable assertions.
The default-off in-process Rust editor passes on macOS and the assembled Linux
checkout: real logical key input changes bounded ASCII drafts and offscreen
pixels; protected chrome displays `VOLATILE / IN-PROCESS`. Preview confirmation,
focus changes, surface alias retirement, denial, fault recovery and scoped
counter exhaustion are exercised through issued contexts. One irreversible
volatile save permit survives after-admission uncertainty without replay;
definitively fenced pre-permit timeout permits a fresh explicit `Ctrl+S`.
The gate retains 34 registry witnesses, canonical exchanges, actual composed
frames and original receipts, and checks that the development API is absent by
default. This establishes no Store IO, editor PID, device, target runtime or
process containment. Strict Linux preflight passes on the assembled UI1.1a
revision, including this gate in the extended Foundry suite.
The [Store transaction prerequisite](docs/plans/editor-store-transaction-v0.md)
specifies a default-off Store owner, private per-object CAS, one atomic
selection/receipt journal and joined-writer recovery. The
[exact Store API](docs/DESKTOP_EDITOR_STORE_API_V0.md) is independently reviewed
and frozen: pure records/codecs, opaque fixture authority, bounded admission and
actual join/fence witnesses prepare seven gate-first assertions. Pure payloads
and their canonical journal validator now pass the nine-case
`just foundry-editor-save-schema-ui1-1b` gate on Linux, including strict Clippy
and a no-default-feature build. All 115 schema tests pass on macOS. These records
provide no IO or commit authority. The seven service assertions are independently
reviewed and their original RED compile check failed at the missing Store module
as expected.
The default-off Store owner now passes
`just foundry-desktop-editor-store-ui1-1b` on macOS and assembled Linux. Its seven
behavior cases exercise private per-object CAS, atomic selection/receipt journals,
bounded admission, pre-permit closure, irreversible permits, same-base conflict,
original-receipt reconciliation and supported joined-writer reopen. Sixteen
evidence assertions check 48 fixture legs, 56 producer epochs, 112 snapshots,
actual typed calls and lease bytes, joins/fences, malformed input, descriptor
aliases and explicit owned-root cleanup. Each accepted run also performs 131
native Rust decodes of captured text, receipts and eligible journals; all exit
successfully and are reaped. Strict Clippy and default API exclusion pass. Strict
Linux preflight passes with this gate registered in the extended suite, including
the existing launch, volatile editor, save-schema and host/QEMU consumers.

This proves a trusted in-process host CAS fixture using supported fault hooks and
a retained reopening owner. It does not connect the editor's live save authority
to Store: UI1.1c must freeze that bridge and deliver the integrated task. A separate
[native Read API](docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md) and its
[bounded contract](docs/contracts/editor-native-read-v0.json) now connect an actual
approved editor peer to a real host Store Read. Nine reviewed gate-first assertions
pass on macOS and assembled Linux, with original deadlines, current instance and
Store grant checks, canonical selected-data copies and actual owned joins. Strict
Clippy for both services and native API exclusion without the opt-in feature pass.
The source-bound gate retains its actual test binary, process birth identity,
original logs, exit and reap evidence. It does not collect optional fixture exports
or establish a universal transcript validator. Strict integrated Linux preflight
passes for the same source candidate with
`RUST_TEST_THREADS=1`, including the affected host/QEMU consumers. This establishes
the serialized Rust test profile; parallel child-spawn reliability remains
unproved. Save admission and full UI1.1c remain separate.
The [native Store preview Read contract](docs/contracts/editor-native-preview-read-v0.json)
and [pure shared-data codec contract](docs/contracts/editor-native-preview-codec-v0.json)
are independently reviewed and frozen preparation. They specify actual input
tickets, current Store selection, one activation row, protected Read-only chrome,
and five precursor cases. The pure codec now implements schema2's 464-byte
grants and the 248-byte NoSave/Unavailable observation, with seven independently
reviewed assertions passing on macOS and Linux after a retained missing-module
RED. The Mac schema suite passes all 122 tests; the affected save-schema and native
Read gates remain green. Strict Linux preflight passes with
`RUST_TEST_THREADS=1`. Mac and Linux used separately retained dependency
resolutions. An initial preflight sandbox cleanup failure is retained; a fresh
focused sandbox run and the subsequent complete preflight passed without a source
change. These bytes grant no authority, Store IO, current-time or publication
proof. The contract JSON retains its creation-stage preparation snapshot; this
file owns current implementation status. The native preview observation amendment
is independently reviewed and frozen: genuine inactive rows have privileged
denial probes, protected pixels have literal font expectations, and an optional
248-byte record is tied to actual positive-actor frame publication. Pre-instance
preview remains 464-byte metadata. Own query identities are historical observations;
actual retained owners and joined producers must prove the two separate 64 limits.
The default-off NativePreview Read precursor now implements actual input tickets,
Store-current pinning, fresh approval, same-row activation/delivery and protected
Read-only composition. Five independently reviewed source-bound cases pass on
macOS and Linux through `just foundry-editor-native-preview-read-ui1-1c`, with
strict Clippy and exclusion from the older NativeRead feature. They check real
Store copies, inactive/foreign denials, original clocks, paused frame publication,
retained setup/error owners and separate query/producer capacity with actual joins.
The two platforms used the identical 278-source manifest and explicitly captured
Cargo.lock. Affected Mac NativeRead, volatile editor and Store gates pass.
Strict integrated Linux preflight passes with `RAMEN_CI_STRICT=1` and `RUST_TEST_THREADS=1`.

The initial API-absence RED and subsequent 0/5 and 4/1 behavioral failures remain
preserved. Reviewed handler corrections fixed canonical native error replies,
Cancel wire validation, foreign lease denial ordering and original pin/Produce
lifetimes. The 4/1 run also exposed an assertion comparing across rightful delivery;
its reviewed repair now asserts the exact live-pin 1-to-0 transition with all other
counts unchanged before checking foreign pairing leaves that delivered state intact.
The gate retains actual executable, process birth/reap and bounded command logs.
Wire, pixel, lease and service-thread-join checks are assertions in that pinned
binary, with no independent exported transcript or per-thread native identity claim.
Original Produce validation-wait forwarding is source-reviewed; no injected wait
scenario was run. Contract creation-stage snapshots remain provenance, while this
file owns implementation status.
The [pure native Save data contract](docs/contracts/editor-native-save-codec-v0.json)
is implemented in a separate schema3 Save grant codec (464 bytes) and schema2
original Commit outcome codec (248 bytes). Eight independently authored and
reviewed assertion families pass on macOS and Linux through
`just foundry-editor-native-save-codec-ui1-1c`, with strict std Clippy and no_std
checks. Actual missing-module RED preceded implementation; the earlier mixed
compiler failure and reviewed fixture-loop corrections remain retained. All six
Active handle pairs are checked for duplicates. Exact original Binding, epoch,
receipt outcome and 176-byte digest joins are data correlation only.

The new gate freezes all 38 source inputs, builds in a fresh private target, and
retains the Cargo-selected binary, exact unfiltered list/outcomes, owned child
exit/reap, and bounded logs. Affected pure Save (nine), Preview (seven), NativeRead
(nine) and NativePreview (five) gates pass on macOS and in strict Linux preflight.
The complete canonical inventory now has 37 extended and 53 preflight stages;
the original stages and CI overrides remain. Contract creation-stage JSON flags
and source pins remain historical provenance. No live Save API, mutation permit,
Store IO or protected current-draft Saved claim follows from this pure codec.
The remaining live bridge and original eighteen-case integrated task are pending.

The eight planned preview identities retain their Save, recovery and IO2
dependencies, and the original eighteen-case integrated task remains separate.
The five reused volatile UI records remain separately stamped. Actual editor PID,
cold-start anti-rollback, device flush, power-loss durability, target execution and
runtime containment remain unproved. The Mac fixture requires its verified
writable Data-volume temporary-directory layout. The API contract and pure gate
alone supply no transaction authority. The Oracle packet adds no boot capture or
guest-runtime evidence.

Strict Linux preflight passes at `a05b0c6` with the reviewed compatibility cleanup correction,
including the 17-case boot admission and 17-case existing desktop launch gates.
The compatibility gate launches built Store/supervisor executables, handles
SIGTERM through the supervisor's child kill/reap path, and bounds teardown of
its own jobs. Forced or unproved shutdown fails with `UNKNOWN`. The real Store
and compatibility VM gates pass with all three serial markers and no remaining
owned QEMU process. `just foundry-compat-cleanup-s2` adds seven Linux private
process regressions, including interruption and missing-marker denials, with
unrelated-process survival checked through held pidfds. These regressions use
an ordinary process stand-in; they do not supply VM or general containment proof.
Test-only coordination of freshly written validator scripts also removes a
reproduced parallel-spawn `ETXTBSY` race without changing production supervision
or its timeout, result and descendant-cleanup assertions.

## Implemented foundations and their boundaries

| Area | Landed behavior | Evidence and limits |
|------|-----------------|---------------------|
| Kernel / S0–S8 | x86_64 and aarch64 boot, typed IPC, capabilities, shared-memory mappings, tracing and SPSC ring foundations | Selected target/QEMU paths; fixed-size tables. Capability-table use after SMP transition is deliberately blocked; general SMP/IRQ support remains incomplete |
| Boot admission / RUN0.0 prerequisite | Allocation-free full-map validation, seven retention reasons, sticky map-overflow denial and bounded deterministic pool selection | `just foundry-boot-frame-pool-run0-0` · [Contract](docs/BOOT_FRAME_OWNERSHIP_V0.md); 17 pure cases, kernel consumer tests/builds and existing S8 integration pass. No actual firmware-exit/collector/allocator or target-runtime proof |
| Typed interfaces | IDL/codegen, protocol/message IDs, bounded wire contracts | Native contracts are defined in `idl/`; generated syntax alone grants no authority |
| Desktop / UI1.0 | Host permission preview, single-use synthetic confirmation, exact self-observation grants, real pinned child, expiry/revocation/fault/restart and independent watchdog | `just foundry-desktop-host-launch-ui1-0` · [Contract](docs/DESKTOP_SESSION_V1.md); default-off trusted Unix fixture, 17 cases and retained process/wire evidence. No editor, compositor, Store, target or process-containment proof |
| Editor / UI1.1a | Logical keyboard editing, focus and preview approval, offscreen composition, volatile save/receipt and explicit recovery | `just foundry-desktop-editor-host-ui1-1a` · [Contract](docs/DESKTOP_EDITOR_HOST_API_V0.md); default-off trusted in-process fixture, 13 cases and source-bound wire/pixel/receipt evidence on macOS/Linux. Real Store IO, actual editor process, device input and target execution remain separate |
| Editor Store / UI1.1b | Private host CAS, atomic selection/receipt journal, irreversible admission and original-operation recovery through joined-owner reopen | `just foundry-desktop-editor-store-ui1-1b` · [API](docs/DESKTOP_EDITOR_STORE_API_V0.md) · [Recording contract](docs/contracts/editor-store-recording-v0.json); seven behavior and sixteen evidence cases pass on macOS/Linux, with captured bytes decoded by the native Rust codec. Integrated editor authority, actual editor PID, device/target durability and containment remain separate |
| Native Save data / UI1.1c prerequisite | Versioned Save grants and original Commit outcomes, canonical references and complete supplied Binding/receipt correlation | `just foundry-editor-native-save-codec-ui1-1c` · [Contract](docs/contracts/editor-native-save-codec-v0.json); eight pure host cases on macOS/Linux, strict std Clippy and no_std checks and 53-stage strict Linux preflight. Live mutation, Store IO, current-draft Saved and full task remain separate |
| Native Read / UI1.1c prerequisite | Approved editor peer, fresh Store Read, original request deadline, live copy authority and shared owned-producer roster | `just foundry-desktop-editor-native-read-ui1-1c-prerequisite` · [API](docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md); nine host cases on macOS/Linux, strict Clippy and feature exclusion. Integrated Save/task, optional-export transcript validation, editor PID and device/target execution remain separate |
| Native preview Read / UI1.1c precursor | Actual Store-current pin, one-use input approval, same-row activation/delivery, protected Read-only frames, live Store reads and retained-owner accounting | `just foundry-editor-native-preview-read-ui1-1c` · [Contract](docs/contracts/editor-native-preview-read-v0.json); five default-off trusted host cases on macOS/Linux, identical source/lock inputs, strict Clippy and feature exclusion. Finite source-bound log assertions; no universal exported transcript, integrated Save/full task, editor PID, target/device or containment proof |
| Native runner / S10 | Host Wasmtime execution, manifests, granted-handle injection and guest deadlines | Host runtime; no complete target userspace loader or Wasmtime environment |
| Semantic State / S10 | Host snapshots, subscriptions/reactor, capability-filtered views; selected QEMU snapshot/IPC paths | Multi-source aggregation and target reactor remain incomplete; default boot/time metadata includes fixtures |
| Store / S1–S10 | Host CAS, signatures, durable ownership, path/tag queries, read-only projections and typed CoW commits | Full user launch/porting flow and target persistence remain incomplete |
| Execution fabric / S10 | Placement, lease, duplicate-observer, launch-plan and trace contracts | Synthetic nodes/load and simulated routing; no distributed transport |
| Driver Foundry / S11 | virtio-net Reference Vault, Linux Oracle capture, replay and typed harness transfers | Embedded Oracle packet vectors in QEMU; device-backed native send/receive unproven |
| Golden machine / S12 | Machine contract, GOP probe, HIL boot/IOMMU gate scaffolds and appliance tooling | First live appliance capture, AMT actuation and physical graduation pending |
| Storage / S13 | Block IDL, virtio-blk Oracle capture/replay and typed harness transfers; NVMe/atomic-update probes | Embedded sector vectors and QEMU scaffolds; native device read/write/flush and physical two-boot rollback remain unproven |
| Compatibility / S2–S9 | Separate Linux capsule VM, host POSIX and GPU quarantine paths with gates | Boundaries differ per runner; default POSIX profile is rlimits-only, not general containment |
| RamenOrg / G0 | Governance schemas, packets, renderers, validators, bounded trials and drift gate | A2-local only; no autonomous merge/release/hardware/public-support authority |

[PLATFORM_OVERVIEW.md](PLATFORM_OVERVIEW.md) explains responsibilities;
[SLICES.md](SLICES.md) defines slice scope. [SECURITY_STATUS.md](SECURITY_STATUS.md)
and [RISKS.md](RISKS.md) record residual risks.

## SW0: runnable evidence, not a completed experiment

The useful task repairs one configuration, runs a pinned WASM validator, commits
an immutable artifact, and denies named unauthorized operations. RT means RamenOS
typed, LT Linux typed, and LS Linux scoped shell. These gates use scripted
consumers and trusted host fixtures, not model trials or target-native task clients.

| Step | Implemented scope | Gate / contract |
|------|-------------------|-----------------|
| A0 | Pure transaction model and deterministic synthetic fixtures; no IO enforcement | `just foundry-agent-task-contract-a0` · [Contract](docs/AGENT_TASK_CONTRACT_V0.md) |
| A1.0 | Generated protocol-14 layouts and allocation-free request preflight | `just foundry-agent-task-protocol-a1-0` · [Protocol](docs/AGENT_TASK_PROTOCOL_V1.md) |
| A1.1 | Useful RT host service task, immutable staging, supervised validator, durable receipts, denials and replay | `just foundry-agent-task-proof-rt` · [Service proof](docs/AGENT_TASK_SERVICE_PROOF_V1.md) |
| A2.1 | Linux scripted repair, inspected Docker containment, pinned worker and named OS probes | `just foundry-agent-task-linux-control` · [Linux control](docs/AGENT_TASK_LINUX_CONTROL_V1.md) |
| A2.2 | Shared strict JSON codec/descriptions and opt-in RT IPC adapter | `just foundry-agent-task-adapter` · [Adapter](docs/AGENT_TASK_ADAPTER_V1.md) |
| A2.3 | Independent LT broker, grants, sealed validation, durable transactions and named RT/LT cases | `just foundry-agent-task-lt` · [LT backend](docs/AGENT_TASK_LT_BACKEND_V1.md) |
| A2.4 | Contained LS commands/launcher, shared Linux transactions, peer checks and original-receipt recovery | `just foundry-agent-task-ls-transactions` · [LS transactions](docs/AGENT_TASK_LS_TRANSACTIONS_V1.md) |
| A2.5 | Shared v2 pull/poll/cancel subscriptions and typed lifecycle comparison | `just foundry-agent-task-subscriptions` · [Subscriptions](docs/AGENT_TASK_SUBSCRIPTIONS_V2.md) |
| A2.6 | Finite canonical inventory, 33 common cases and separate LS OS probes; unknown authority retained | `just foundry-agent-task-authority` · [Authority](docs/AGENT_TASK_AUTHORITY_V1.md) |
| A2.7 | Synthetic bank/release contract, bounded sessions, 45 development attempts and retained failures | `just foundry-agent-task-evaluator-controls` · [Evaluator controls](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md) |
| A2.8 | Named acknowledged-ID cleanup, unresolved-create quarantine and explicit interrupted-commit receipt recovery | `just foundry-agent-task-reconciliation` · [Reconciliation](docs/AGENT_TASK_RECONCILIATION_V1.md) |
| A2.9 | 31 issued-right subsets under two policies, single-right effects and named lifetime witnesses | `just foundry-agent-task-requestable-authority` · [Requestable authority](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) |

A2.6's frozen suite has zero successful forbidden probes, but does not prove whole
`E_max`, continuous `E(t)`, or narrower authority. A2.9 establishes equality only
for the declared-interface issued-right projection. LS mounted files, retained
observations/descriptors and raw broker access remain broader observations;
typed host clients and transitive deputy authority remain incompletely bounded.
A finite host-file witness now records nine named observations across RT/LT/LS
before expiry, after expiry and after revocation. The trusted Python evaluator/host
consumer in RT/LT reads an unrelated owner-only canary; the contained LS Python
consumer is denied access to that exact unmounted path. Actual PID/UID/GID and
namespace identities identify these actors, separately from adapters or model
interfaces. Read/grant/revocation witnesses bind the same resource, capability
and generation; malformed attribution is rejected. Both authority gates passed
in an isolated Linux/Docker checkout, with 33 common cases per arm and
31 right subsets under two policies per arm. This does not complete whole-authority
inclusion, continuous lifetime coverage or full A2.
Unacknowledged Docker create intents cannot certify cleanup from an empty inventory.

Portable SW-E accounting now freezes bank/release/context/provider/rate identities
and a finite three-arm schedule. Strict reports retain failed, unknown,
over-budget and pending attempts; integer uncached-token estimates cannot certify
the declared ceiling with missing usage. `just foundry-agent-task-provider-accounting`
checks ten unit assertions and a deterministic synthetic consumer. It makes no
provider calls and supplies no billing, attestation or funding authority. Combined
`foundry-agent-task-evaluator-controls` and `foundry-agent-task-reconciliation`
passed in an isolated Linux/Docker checkout with the pinned toolchain/image:
45 retained scripted repair attempts and six interrupted-commit cases, plus
late-create and abrupt-publication recovery assertions. Reports are retained
under `out/roadmap-linux/`; these checks do not complete full A2.

Full A2, real hidden-bank/study releases, actual provider supervision/usage capture, model
comparison, production registration and target task enforcement remain pending.
The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines their
acceptance and the three separate contrasts. No comparative agent advantage is claimed.

## Recent boundary fixes

The StoreClient transport follow-up applies socket timeouts on initial and
replacement connections and one absolute deadline across response-frame reads.
Retained clients reconnect when peer closure is observable before dispatch;
transport failures discard the stream for the next explicit operation. Uncertain
requests, including ingestion, are never silently replayed. Real-server idle
expiry and fake-server lost/malformed/truncated reply and withheld/trickled
response regressions cover both ordinary reads and descriptor ingestion. This
is host transport evidence; connect and individual writes are not covered by
the response-frame deadline.

The 2026-10-04 follow-up defines `validation_current` as observation freshness in
RT/LT, independently of commit eligibility. RT direct/poll regressions cover failed
outcomes, truncation, expiry and revocation; portable LT predicate checks pass.
Expanded paired Linux cases run in the required Linux/Docker gates; local macOS
checks establish RT execution and portable LT predicates.
Ordinary Store preparation now has configurable byte/concurrency/deadline bounds,
runs outside the global registry/projection locks, and cancels on disconnect.
A two-client stalled-worker test checks unrelated reads and cleanup; publication
revalidates authority and preserves owner-bound intents. Durable publication IO
still uses the locks; total CAS quota and crash-orphan staging cleanup remain work.
The README now leads with the runnable RT host proof and a retained fixture result.
These changes add host evidence, not a model comparison or target/hardware claim.

The 2026-10-03 review changes are implemented and recorded in `CHANGELOG.md`:

- Store reads bind the requested content ID to authenticated metadata and blob bytes;
  native execution hashes the byte snapshot it consumes.
- Host ingestion uses caller-opened regular-file descriptors; pathname-only requests
  fail closed. Native ingestion retains its IDL shared-memory source contract.
- Owner/manifest publication intents precede CAS visibility; restart/retry recovers
  partial publication and unrelated orphans remain denied.
- Native Unix/chardev IPC shares the invocation's absolute deadline through connect
  and partial transfers; uncertain dispatch is not automatically replayed.
- LT duplicate staging preserves capability/validation and counts unique candidates,
  matching RT; portable and executable regressions cover capacity.
- CI and preflight use the same complete implemented SW0 sequence. This does not
  turn that sequence into full A2 conformance or a completed model study.

Earlier memory, tracing, WASM, projection and durability fixes remain in the
changelog and their contract/gate documents. They establish host/QEMU behavior,
not complete SMP, client isolation, physical durability or production assurance.

## Physical inventory and graduation boundary

The x86_64 COM1 console uses 115200 8N1, matching the HIL appliance contract
and capture tools. The S12 GOP gate checks the emulated UART's programmed
parameters in a QEMU trace; first live Pi/ThinkCentre validation remains pending.

The pinned reference is the Lenovo ThinkCentre M900 SFF, machine type 10FH,
Core i7-6700, 8 GiB RAM, with a 240 GB SanDisk SATA SSD. The Raspberry Pi 4
(4 GiB), FTDI USB-to-RS-232 adapter and null-modem chain are physically installed.
Firmware/AMT preflight and the first live serial capture remain pending; a compatible
M.2 2280 PCIe NVMe drive is still required for S13 graduation.

`PASS/QEMU` is not metal evidence. `PASS/HIL-LOG`, `PASS/HIL-LIVE`,
`PASS/HIL-APPLIANCE`, and `PASS/METAL` have separate provenance requirements in
[EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md). Standalone `operator-golden-machine`
and `appliance-mediated` metal claims must be stamped separately.

S13.8 currently probes A/B metadata. Graduation still requires an implemented
publication/readback/selection verifier, a new-slot boot and a separate rollback
boot with fresh nonces and matching artifact identities. Firmware NVMe detection
and vector-backed block transfers establish neither native NVMe I/O nor that protocol.

## Documentation maintenance

The 2026-10-04 roadmap review replaces global sequencing barriers with bounded
parallel work and explicit integration checkpoints. The coordinator owns the
shared contract and status files; sub-agents own disjoint changes and return
gate evidence for review. Project skills share one source across agent clients.
The governance drift gate now checks that agent instructions route to maintained
planning owners, with negative cases for missing links and duplicated queues.
This adds workflow/documentation validation, not OS, model, or hardware evidence.

The 2026-10-03 documentation review consolidates status here, execution criteria
in `NEXT_TASKS.md`, and direction in `ROADMAP.md`. Current references and agent
skills were reconciled with source/gates; historical security plans are archived
behind maintained references at their existing paths. This is documentation work
and adds no runtime, model, hardware, security or release-readiness evidence.

## Validation entry points

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

SW0 gates above expose their individual fixture scopes. Full `just preflight`
requires Linux, Python `jsonschema`, Docker/seccomp, and the installed pinned image.
Physical gates are opt-in and require documented preparation/provenance.

Strict Linux preflight passed for the committed UI1.0/accounting/finite-authority
batch `a44993e` with the pinned compatibility kernel and Docker image. Its isolated
checkout matched that revision's tracked sources except two trailing spaces in
the desktop contract; source/artifact digests and the difference are retained in
the coordinator checkpoint. This run predates the subsequent RUN0.0 pure changes;
their focused gates, consumer checks and target builds are recorded separately.
