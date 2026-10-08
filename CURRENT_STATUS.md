# Current Status

**Last Updated:** 2026-10-08
**Status:** Active and authoritative for landed state
**Current Slice:** UI1.1 host editor reliability, then UI1.1d process boundary
**Physical Lane:** S12.4 HIL appliance observation/actuation awaits setup
**Software Lane:** SW0 foundations through A2.9; full A2 and comparison pending

RamenOS is public pre-alpha, building toward the everyday OS for humans and AI
agents in [Vision](VISION.md). This file owns landed behavior and limitations;
[Next Tasks](NEXT_TASKS.md) owns dispatch and acceptance. Detailed integration
chronology and private evidence digests are retained in the
[historical status snapshot](docs/archive/plans/2026-10-06-status-integration-snapshot.md)
and [Changelog](CHANGELOG.md), outside routine agent intake.

## Execution state

The default-off UI1.1c host task reads an approved artifact, processes logical
keyboard input, edits bounded text, renders offscreen frames, saves through typed
Store authority and reconciles original outcomes through joined-owner reopen.
Its eighteen original assertions and complete 93-scenario recording checks passed
on macOS/Linux. This is a trusted in-process host consumer. An actual editor child,
authenticated process transport, target execution and target persistence remain
unfinished. Models are unnecessary for this task.

External review of merged `d3a95551` reports four runtime defects, supported by
subsequent independent static inspection: concurrent Store reply reclamation,
abandoned surface acquisitions, render-time focus reassignment and a Save left
pending after definitive pre-admission timeout. Existing green gates do not cover
those scenarios. A `JournalSync` staging-file failure path also needs a focused
reproduction. These are current reliability limitations, not repaired behavior;
[the ready queue](NEXT_TASKS.md#ready-work-front) makes them prerequisites to process
integration. They concern the default-off host fixture and do not establish a
target exploit or physical durability failure.

### UI1.1d shared adapter migration — accepted host preservation

Both `EditorClient` and `NativeTextEditor` use `desktop_editor_core` for pure
editing/navigation/scroll/raster. Checked snapshots transfer seven fields;
legacy/native policies and detached-scratch owner admission remain distinct.
The core has no Desktop, Store, Focus or Save authority. Its 26 pure assertions
and four explicit required-feature adapter assertions passed on macOS/Linux;
strict Linux55 preserves original13/18, strict3 and Reader93 requirements.
Protocol384/version1 and the process contracts are preparation. Private process
smoke/cleanup experiments do not establish child execution, authenticated channels,
sealed publication or retained Save ownership after child death. Re-freeze current
source before reusing historical evidence; the old 217-row task closure is not a
permanent count requirement after legitimate dependency additions.

RUN0.0 has a reviewed 17-case allocation-free map/retention admission gate.
Actual relocated-entry capture, complete retained-range collection, final firmware
exit map, allocator installation and target application execution remain pending.
Native USB/HID input and device-backed storage also remain pending. Independent
software preparation proceeds without SW0 model trials or physical H0–H3.

## Foundry development and CI execution

[The execution profile](docs/FOUNDRY_CI_OPTIMIZATION_V0.md) provides a locked warm
`just dev-check`, content-stable codegen, one canonical 55-stage local inventory
and four isolated CI lanes. Required Foundry/merge aggregates reject failed,
cancelled, missing or unexpectedly skipped results. Compiler reuse never restores
acceptance results; assertions, source checks and evidence execution remain fresh.
Historical 52/54-stage results apply only to their recorded source snapshots.

PR #34 merged as `d3a95551` after both reviewed-head runs passed all eight jobs.
The [post-merge main run](https://github.com/maxwellsantoro/RamenOS/actions/runs/37730644587)
also passed all eight on October 8. Its host job took 820 seconds, agent 337,
quality 309 and QEMU 292; creation-to-final-update was 845 seconds. These are one
run's job durations, including setup/artifact work, not a before/after speedup.
Host is the measured bottleneck for that run.

CI0 evidence-admission repairs are independently reviewed and validated. Executable
contracts/fixtures and gate-consumed Markdown require Foundry, including deletion;
ordinary prose and governed org packets keep their inexpensive path. NativeRead
and NativePreview retain their old inventories and add conservative transitive
crate/include coverage (351/353 inputs), with pinned helper admission and rejection
of unsupported explicit Cargo target layouts. The existing collector, canonical
55/39 stages, runtime assertions, real deadlines and cache policies are unchanged.

The assembled Linux candidate passes the CI tooling gate (including seven Rust
codegen controls), policy regressions and actual nine-case NativeRead/five-case
NativePreview gates with strict checks and feature exclusions. Its source remains
unchanged during execution. Gate-first failures and independent review findings
are retained. The affected gates passed; the complete Linux preflight and a new
hosted run were not repeated for this patch. Required S11/S12/S13, appliance docs
mode and org-governance checks pass locally. Final status/queue wording is a
subsequent documentation-only delta outside the two runtime gate closures.

The dev `sha2` optimization and `native_runner` debug stripping have focused
measurements in the execution profile and historical snapshot. Neither establishes
stable whole-CI latency. No model-comparison, metal or release claim follows from
hosted green checks.

## Implemented foundations and their boundaries

| Area | Landed behavior | Evidence and limits |
|------|-----------------|---------------------|
| Kernel / S0–S8 | x86_64 and aarch64 boot, typed IPC, capabilities, shared-memory mappings, tracing and SPSC ring foundations | Selected target/QEMU paths; fixed-size tables. Capability-table use after SMP transition is deliberately blocked; general SMP/IRQ support remains incomplete |
| Boot admission / RUN0.0 prerequisite | Allocation-free full-map validation, seven retention reasons, sticky map-overflow denial and bounded deterministic pool selection | `just foundry-boot-frame-pool-run0-0` · [Contract](docs/BOOT_FRAME_OWNERSHIP_V0.md); 17 pure cases, kernel consumer tests/builds and existing S8 integration pass. No actual firmware-exit/collector/allocator or target-runtime proof |
| Typed interfaces | IDL/codegen, protocol/message IDs, bounded wire contracts | Native contracts are defined in `idl/`; generated syntax alone grants no authority |
| Desktop / UI1.0 | Host permission preview, single-use synthetic confirmation, exact self-observation grants, real pinned child, expiry/revocation/fault/restart and independent watchdog | `just foundry-desktop-host-launch-ui1-0` · [Contract](docs/DESKTOP_SESSION_V1.md); default-off trusted Unix fixture, 17 cases and retained process/wire evidence. No editor, compositor, Store, target or process-containment proof |
| Editor / UI1.1a | Logical keyboard editing, focus and preview approval, offscreen composition, volatile save/receipt and explicit recovery | `just foundry-desktop-editor-host-ui1-1a` · [Contract](docs/DESKTOP_EDITOR_HOST_API_V0.md); default-off trusted in-process fixture, 13 cases and source-bound wire/pixel/receipt evidence on macOS/Linux. Real Store IO, actual editor process, device input and target execution remain separate |
| Editor Store / UI1.1b | Private host CAS, atomic selection/receipt journal, irreversible admission and original-operation recovery through joined-owner reopen | `just foundry-desktop-editor-store-ui1-1b` · [API](docs/DESKTOP_EDITOR_STORE_API_V0.md) · [Recording contract](docs/contracts/editor-store-recording-v0.json); seven behavior and sixteen evidence cases pass on macOS/Linux, with captured bytes decoded by the native Rust codec. Integrated authority is exercised by UI1.1c; actual editor PID, device/target durability and containment remain separate |
| Native Save data / UI1.1c prerequisite | Versioned Save grants and original Commit outcomes, canonical references and complete supplied Binding/receipt correlation | `just foundry-editor-native-save-codec-ui1-1c` · [Contract](docs/contracts/editor-native-save-codec-v0.json); eight pure host cases on macOS/Linux, strict std Clippy and no_std checks and 53-stage strict Linux preflight. Data-only gate; live mutation and the full task are exercised separately by UI1.1c |
| Native Read / UI1.1c prerequisite | Approved editor peer, fresh Store Read, original request deadline, live copy authority and shared owned-producer roster | `just foundry-desktop-editor-native-read-ui1-1c-prerequisite` · [API](docs/DESKTOP_EDITOR_NATIVE_READ_API_V0.md); nine host cases on macOS/Linux, strict Clippy and feature exclusion. Read-only gate; UI1.1c separately exercises Save/task. Optional-export transcript validation, editor PID and device/target execution remain separate |
| Native preview Read / UI1.1c precursor | Actual Store-current pin, one-use input approval, same-row activation/delivery, protected Read-only frames, live Store reads and retained-owner accounting | `just foundry-editor-native-preview-read-ui1-1c` · [Contract](docs/contracts/editor-native-preview-read-v0.json); five default-off trusted host cases on macOS/Linux, identical source/lock inputs, strict Clippy and feature exclusion. Finite source-bound log assertions; no universal exported transcript, editor PID, target/device or containment proof; UI1.1c separately exercises Save/task |
| Native Save/task / UI1.1c | Focused keyboard editing, protected frames, actual Store Save/reopen and original-only failure recovery | `just foundry-desktop-editor-task-ui1-1c` · [Contract](docs/contracts/editor-native-save-task-v0.json); eighteen original host assertions and complete 93-scenario recordings pass on macOS/Linux with strict checks, exact affected consumers and historical full54 and successor full55 Linux preflights. Trusted in-process, default-off scope; editor child, target persistence, device and containment remain separate |
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

The current roadmap uses a short ready frontier, dependency joins and one
coordinator with disjoint workers and reserved review/validation capacity. The
[workflow](docs/AGENTIC_WORKFLOW.md) owns dispatch mechanics; project skills route
to it rather than copying the queue. Pinned contracts and immutable trial evidence
retain their original paths and bytes. Historical plans do not authorize work.

## Validation entry points

Resolve recipes in [justfile](justfile). Use focused `just dev-check` while editing,
affected consumer gates before handoff and the canonical lanes on a fixed
integration candidate. Roadmap/org/research changes require `just s11`, `just s12`,
`just s13` and `just foundry-org-governance-g0`; appliance docs also require
`just hil-appliance`. Full `just preflight` needs Linux, Python `jsonschema`,
Docker/seccomp and the installed pinned image. macOS host/QEMU results do not
certify the full Linux profile. Physical gates remain separately opt-in.
