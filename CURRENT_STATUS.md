# Current Status

**Last Updated:** 2026-10-05
**Status:** Active and authoritative for landed state
**Current Slice:** UI1.1 host editor; S12.4 HIL appliance physical loop pending
**Software Lane:** SW0 foundations through A2.9 implemented; full A2 and comparison pending

RamenOS is public pre-alpha, building toward the everyday OS for humans and AI
agents described in [VISION.md](VISION.md). This file records implemented behavior
and evidence boundaries. [NEXT_TASKS.md](NEXT_TASKS.md) owns the next work;
[CHANGELOG.md](CHANGELOG.md) holds detailed milestone history and
[DECISIONS.md](DECISIONS.md) holds rationale.

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
Real Store transactions, the integrated task, actual editor process and RUN0
post-firmware memory ownership are next dependencies. RUN0.0's
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
The Oracle packet adds no capture or runtime evidence.

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
