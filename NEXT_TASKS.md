# Next Tasks

**Last Updated:** 2026-10-05
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

This queue serves the [Vision](VISION.md): an everyday, post-Unix OS for humans
and AI agents. The next integrated product task is keyboard-driven application
launch, permission preview, artifact editing and failure recovery, delivered
first with host contracts, then QEMU, then qualified reference hardware. Saving
across restart requires real persistence. These are future acceptance goals;
[Current Status](CURRENT_STATUS.md) records what actually works today.

## Ready work front

**Now:** freeze UI1.1c's live editor-to-Store admission and shared-data bridge,
then write its eighteen planned integrated-task assertions and independently
review their RED evidence before handlers.
Use the [editor proposal](docs/plans/desktop-editor-v0.md),
[exact Store API](docs/DESKTOP_EDITOR_STORE_API_V0.md) and
[transaction contract](docs/plans/editor-store-transaction-v0.md). A copied
volatile authorization verdict cannot grant Store admission. Freeze bounded
issuer, lease, ticket and pending-operation capacities, charging, retention and
overflow rules alongside exact API signatures. Request timeout
must fence its original operation/source without retiring a still-live instance;
revocation, expiry and service fault retain their distinct lifetimes. Prepare
RUN0.0's initial table-access evidence and firmware adapter independently.
UI1.1b's seven host Store behavior cases and sixteen evidence assertions now pass
on macOS and assembled Linux, including actual lease/journal decoding, cleanup
and strict Linux preflight. The nine-case pure save-schema prerequisite grants
no IO authority by itself. UI1.1a's thirteen volatile editor assertions also pass;
the integrated task and actual editor process remain separate. RUN0.0's pure map/retention validator is reviewed and passes;
the actual ownership transition is still pending. UI1.0's real host launch/lifetime gate passed on macOS and Linux; its
process/wire artifacts are retained. UI0 design, portable accounting and the
finite named Python-consumer canary are reviewed, with the affected Linux SW0
gates passing. Physical H0–H3 await hardware setup; no live capture or actuation
is scheduled. These ready packets need no model calls or physical actuation.
Packet IDs are planning labels, not new slices or evidence of implementation.

The coordinator keeps at most three worker packets active by default, prioritizes
the integrated human-task dependency path, and backfills blocked capacity with
independent SW0 or recovery work. Do not wait for every packet in a row or wave
to finish before starting a successor whose own prerequisites are satisfied.
Follow [Agentic Workflow](docs/AGENTIC_WORKFLOW.md) for ownership and handoffs.

| Packet / initial owner | Bounded edit scope | Prerequisite and consumer | Completion / gate |
|------------------------|--------------------|---------------------------|-------------------|
| UI1.1c / coordinator then bridge and consumer workers | Coordinator freezes exact live authority/data bridge and owns shared exports/features/IDL/gates; assertion writer owns the eighteen-case consumer; one writer per assigned desktop or Store adapter | Accepted UI1.1a volatile editor and UI1.1b actual host CAS gates, [editor proposal](docs/plans/desktop-editor-v0.md), frozen [Store API](docs/DESKTOP_EDITOR_STORE_API_V0.md), [wire interfaces](docs/DESKTOP_EDITOR_WIRE_V1.md) and [host API/corpus](docs/DESKTOP_EDITOR_HOST_API_V0.md) | Review and freeze exact signatures, lock order, descriptor lifetime, emission allowlist and bounded authority/recovery capacities with charging, retention and overflow rules before assertion dispatch. Write the first eighteen maintained cases RED, then deliver keyboard edit/frame/save/reopen with live issuer-checked authority, separately stamped volatile controls and original-operation Unknown recovery without replay. Request timeout must preserve the fresh-save positive for a live instance; revoke/expiry/fault must deny unpermitted admission. d's editor PID, physical input and device/target persistence remain separate. |
| RUN0.0 / coordinator then runtime worker | Coordinator freezes Oracle profile, raw-map/retention collection assertions and boot integration; worker owns exact subsequently assigned adapter files | Reviewed [pure admission contract](docs/BOOT_FRAME_OWNERSHIP_V0.md), passing gate and [Oracle preparation](docs/plans/boot-profile-oracle-v0.md); current UEFI path still lacks post-firmware ownership | Resolve actual relocated-entry capture, freeze the CPU-inspection schema/profile and write RED assertions. Then obtain initial table-access evidence, derive complete retained ranges, use the actual final firmware-exit map and prove allocation/write/read/reuse plus S8 in QEMU. External debug reads or a pure selector cannot prove guest access, firmware exit or user-mode execution. |
| SW-A / authority worker | `tools/agent_task/authority_*`, `requestable_authority*`, corresponding tests and authority contract docs | Landed A2.6/A2.9; three-arm authority report | Choose a remaining host-client/deputy/unexercised or continuous-lifetime gap with real backend probes; do not repeat the landed nine-point named Python-consumer canary or LS retained-descriptor probes. Preserve remaining unknowns. Run `just foundry-agent-task-authority` and `just foundry-agent-task-requestable-authority` plus affected consumer gates on Linux/Docker. A finite addition is not full A2. |
| SW-E / evaluator worker | Exact assigned evaluator/session/accounting files and their tests/docs | Reviewed portable accounting and combined Linux gates; landed A2.7/A2.8; future comparison evaluator | Bind actual provider usage/context capture to frozen plans under deterministic failure fixtures. Portable `just foundry-agent-task-provider-accounting` and combined Linux evaluator/reconciliation pass; actual private bank release and paid runs retain independent operator/funded-work-order controls. |

Check each worker's environment before dispatch. SW-A completion and full SW-E
integration require Linux/Docker. On macOS without those prerequisites, assign
UI1.1 contract preparation, RUN0.0 pure validation, a portable SW-E subpacket, and an independent reviewer or
prerequisite worker. Keep the Linux acceptance explicitly incomplete; the ready
queue does not certify that a particular host can run every listed gate.

At dispatch, narrow each scope to exact files and one deliverable. Shared backend
changes needed by SW-A or SW-E return to the coordinator for ownership assignment;
workers must not concurrently edit `lt_backend.py`, the Rust adapter, Store
handlers or shared lifecycle code. The coordinator owns `justfile`, workspace
configuration, shared schemas/IDL, protocol/registry assignments, codegen outputs,
kernel integration and status/history updates unless explicitly reassigned.

### Admit successors as their dependencies clear

The following are bounded follow-up packets, not instructions to open all work
at once. New gate names/paths are assigned in the accepted design and registered
before implementation; no S14/S15/target-desktop recipe is claimed to exist.

| Packet / owner | Requires | Consumer, edit boundary and completion |
|----------------|----------|----------------------------------------|
| RUN0 / target-runtime worker | RUN0.0 post-firmware ownership proof, UI0's accepted runtime/authority contract and executable denial/failure assertions | One target application. Own only assigned loader/runtime files; integrate kernel glue through the coordinator. Demonstrate target execution, granted access, denial, failure and restart in QEMU; a host Wasmtime or init-bytecode marker is insufficient. |
| IN0 / input worker | UI0 input contract; selected controller Reference Vault and Oracle `protocol_trace` before device interaction | UI1's input consumer. First capture/validate the dossier, then implement one keyboard path in assigned driver/IDL files. Gate malformed reports, unplug/reset, bounded queues and denied delivery using the declared host/replay/QEMU scope. Physical execution waits for H0/H1. |
| UI1 / desktop worker | UI0's accepted contracts and executable behavior/failure assertions | One application, compositor/focus and permission/launch/recovery flow. Own assigned service/client files and run a deterministic host consumer against typed interfaces. Input injection is allowed for this host gate; it does not finish IN0 or RUN0. |
| STORE0 / storage worker | Existing block Vault/Oracle; a bounded device and Store durability contract with failure assertions | Artifact save/reopen. First establish actual QEMU device-backed read/write/flush; then connect the named Store consumer across its IO boundary. Own assigned driver/storage files, preserving schema/IO separation. Embedded-vector success alone cannot satisfy this packet. |
| REC0 / recovery worker | S13 publication/selection contract and versioned evidence schema with fault cases | Update/rollback verifier. Own assigned S13 verifier/gate files; prove publication/readback, revisioned selection and interrupted-phase handling in host/QEMU. Physical new-slot and rollback boots remain H3 work. |
| UI2 / coordinator plus assigned integration worker | RUN0 + IN0 + UI1; STORE0 before persistent save/reopen acceptance | First port the compositor/display path and required service adapters to target, gating actual frame delivery, capability-bound surfaces and recovery; then run the human task in QEMU with unauthorized input/focus, stale surfaces and crash/restart. Assign each port exact files and its own gate; name remaining host services. A qualified physical repeat also needs H0/H1, relevant S12/S13 completion and controller-specific evidence. |

RUN0, IN0 and UI1 may overlap after UI0; STORE0 and REC0 can advance independently
once their own contracts are fixed. Capture INPUT/STORE Oracle evidence from the
chosen environment; missing lab devices block physical capture, not an unrelated
QEMU profile. Keep the native controller/device claim separate from replay.
Split each multi-step packet at its contract, first consumer and integration gate
so that an unfinished subsystem cannot hide behind a single oversized handoff.

### Integration checkpoints

1. Before dispatch: record the base revision, exact write scope, dependencies,
   assertions, consumer and expected evidence level. Resolve shared-contract
   changes once and give every consumer the same revision.
2. After each packet: inspect the diff and evidence, integrate one result at a
   time, then run affected producer/consumer gates on the combined revision.
   Worker-only green checks are not integrated acceptance.
3. At a product checkpoint: run the declared human scenario and denial/recovery
   matrix end to end; record target/host/device provenance and remaining gaps.
   Update status and queue once. Retain A2/A3 review separation for PRs.

H0–H3 remain ordered within the physical lane. SW0, S14/S15 software work and
storage/recovery preparation do not wait for lab access, a Phase B report or NVMe
graduation. The detailed prerequisites below still apply to their own outcomes.

## Physical Lane: H0–H3

When test hardware is available, run the first live capture on the Pi↔M900 chain,
then provision
and validate the M900's Intel AMT 11 power/reset path.
S12 runs on the installed 240 GB SanDisk SATA SSD. Add a compatible M.2 2280
PCIe NVMe drive before S13 metal graduation. Front-panel relays and a
smart plug/PDU remain deferred until AMT testing shows they are necessary.

| Order | Task | Completion signal |
|----------|------|-------------------|
| H0 | S12.4.1 HIL appliance serial observer — first live capture | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 just hil-appliance` captures live serial and emits valid controller evidence |
| H1 | S12.4.2 Intel AMT power/reset actuator | AMT status, power-on, power-off, reset, and power-cycle are validated from the Pi and represented in controller evidence JSON |
| H2 | S12 physical graduation on the installed SanDisk SATA SSD | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil` produces valid live provenance |
| H3 | Add M.2 2280 PCIe NVMe and run S13 metal graduation using the completed reboot/rollback protocol | Provenance-bound boot and rollback captures verify slot/artifact transitions and recovery; existing `just s13-hil` metadata scaffold alone is insufficient |

Before H3 graduation, implement a gate for inactive-slot publication and readback,
revisioned boot selection, a new-slot boot and a rollback/recovery boot with fresh
nonces, and recovered artifact identity. Define interrupted-write/selection cases
and the storage flush/ordering contract first. Hardware runs remain deferred.
S13.7 proves firmware boot from an NVMe ESP; native NVMe `harness.block` I/O needs
its own controller Reference Vault, Oracle, implementation and target evidence.
Neither firmware detection nor a manually set `rollback_ready` variable proves it.

### H0 Acceptance Criteria

- Capture the target COM1 console at 115200 8N1, matching the kernel and
  `hardware/hil_appliance_v0.toml`; QEMU parameter assertions do not replace
  this live Pi/ThinkCentre check.
- `tools/hil/appliance_capture_serial.sh` captures from the configured appliance
  serial device without accepting stale graduation logs.
- Empty transcripts and unsafe run ids fail closed.
- Output distinguishes `PASS/HIL-LOG` replay from live
  `PASS/HIL-APPLIANCE` evidence.
- `just hil-appliance` remains green in its default docs/manifest mode and its
  opt-in appliance mode.
- No `PASS/METAL` claim is emitted without the required target provenance.
- S13 per-gate evidence distinguishes standalone
  `claim_path: operator-golden-machine` from appliance-mediated
  `claim_path: appliance-mediated` runs.

Live per-gate runs validate the prepared `provenance.json` beside their EFI image.
For graduation, set a unique `RAMEN_HIL_RUN_ID`, `RAMEN_HIL_APPLIANCE_ID`, and
`RAMEN_HIL_EXPECTED_NONCE`; stage that same nonzero nonce on the target before
boot. Use a fresh nonce for each boot and run the individual physical gates when
manual media/nonce staging is needed. See [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md).

### H1 Acceptance Criteria

- Provision AMT 11 through MEBx on a trusted wired lab network.
- Add AMT-backed status, power-on, power-off, reset, and power-cycle commands.
- Keep AMT credentials out of evidence, logs, and the repository.
- Dry-run behavior is deterministic and covered by the appliance gate.
- Controller evidence records action, transport, target, run id, and result.
- Validate reachability while the target is running, soft-off, and hung in the
  target OS before deciding whether a smart plug/PDU or relay fallback is needed.
- Physical actuation remains opt-in; governance scaffolding grants no ambient
  HIL actuation authority.

## Software Lane: SW0 Agent Task Proof

Complete the remaining A2 controls before model collection. Authority work and
offline evaluator/study-control implementation may proceed in separate scopes;
all-arm conformance is their integration checkpoint. Run the expanded RT/LT point
and subscription transition gates on Linux/Docker and preserve the independently
runnable A1.1 host proof. SW0 has no H0–H3 prerequisite; its consumer is a scoped
configuration repair, pinned validation and checked artifact publication.

| Workstream | Work remaining | Acceptance / dependency |
|------------------|----------------|-------------------------|
| A2 authority | Bound host-client/deputy differences and unexercised authority outside the declared interface; extend continuous lifetime coverage | Named backend and OS probes, explicit available/task/probe effects, and honest unknown/inclusion results; finite issued-right equality is insufficient |
| A2 study controls | Freeze real bank/study releases, provider/token accounting and arm supervision | Independent hidden partitions, identical authorized task resources, frozen context/usage accounting and no silent retry or discarded failure rows |
| A2 conformance | Run all deterministic controls across LS/LT/RT and validate canonical mappings | No skipped negative cases or missing protocol/authority mappings; preserve the client/deputy differences in the report |
| Phase B pilot and report | Pilot the three arms, apply the predeclared power rule, then freeze an affordable final comparison or publish exploratory results | Explicit funded ceiling and opt-in work order; separate interface/substrate/total contrasts and completion, authority, cost and audit/replay outcomes with uncertainty |
| Phase C target evidence | Add target enforcement for named task operations as their target contracts become available, independently of Phase B | Actual kernel/QEMU grants and forbidden probes per operation; the snapshot/IPC bridge alone is insufficient |

The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) owns the
experimental contract and proposed future commands. The implemented gate/contract
map is in [Current Status](CURRENT_STATUS.md#sw0-runnable-evidence-not-a-completed-experiment).
Full A2 conformance and model comparison are **not implemented**.

Invocation-wide Store connect/write deadlines and remaining durable-publication,
quota and crash-orphan cleanup limits are separate follow-ups. Use the
[landed boundary fixes](CURRENT_STATUS.md#recent-boundary-fixes) when scoping
them; do not repeat completed lifecycle work.

Carry forward the [authority inventory](docs/AGENT_TASK_AUTHORITY_V1.md),
[requestable/lifetime boundary](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md),
[evaluator controls](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md), and
[reconciliation](docs/AGENT_TASK_RECONCILIATION_V1.md): retain validator timeouts
and forced failures in the denominator; quarantine unresolved create intents;
certify cleanup only from complete acknowledged-ID evidence. Completion of a
contract is not completion of the study. A tie, regression or budget-limited
report is a valid result for the next software decision.

## S14/S15 prerequisites by evidence stage

| Stage | Required before starting or claiming completion |
|-------|-----------------------------------------------|
| Design, contracts and host/replay consumers | Bounded human task, reviewed boundary and executable Foundry assertions before implementation; no SW0 study or H0–H3 prerequisite |
| Native USB/HID device implementation | Selected controller Reference Vault, Oracle `protocol_trace`, generated IDL and denial/failure assertions; replay alone cannot prove a driver |
| QEMU human-task integration | Real target loader/runtime and actual device-backed input/display paths plus named required services; persistence requires actual storage IO |
| Physical S14 integration | Demonstrated stable H0/H1 observation-and-actuation loop, prepared controller evidence and explicitly authorized physical operation |
| Reference desktop / persistence qualification | Corresponding S12/S13 and device gates with live provenance, persistent readback and recovery; physical success on one profile is not broad readiness |

S15 input/surface/focus and host consumers may proceed before the S14 device path
is finished; their integration joins only after both contracts conform. Review
SW0 findings for agent-facing follow-ups without making human interaction wait
for a paid experiment. This supersedes the earlier global S14 hold; rationale
is recorded in the 2026-10-04 sequencing decision in [DECISIONS.md](DECISIONS.md).

## Parallel Project-Control Track

This lane supplies concrete inputs to product packets. Keep it bounded and use
spare capacity; it must not displace the integrated human task, H0–H3 or SW0.

| Priority | Task | Gate or artifact |
|----------|------|------------------|
| GP0 | Maintain G0.8.1 implementation authority and serial claim hygiene | `just foundry-org-governance-g0` |
| GP1 | RQ-0002 AI-governed Org Kernel research packet | [RQ-0002](docs/research/questions/RQ-0002-ai-org-kernel.md) |
| GP2 | RQ-0001 offer-shaped service-boundary research packet | [RQ-0001](docs/research/questions/RQ-0001-offer-boundaries.md) |
| GP3 | Identity-level role separation | Future design; no authority increase |
| GP4 | Fresh isolated implementation-agent reproduction | New bounded trial before any authority widening |

G0 remains A0/A1 for board, planning, docs, and research. G0.8.1 permits only
explicitly bounded A2-local implementation work. It grants no merge, release,
self-approval, HIL actuation, or public-support authority.

## Keep Green

```bash
just s12
just s13
just s11
just foundry-org-governance-g0
```

Run `just hil-appliance` for appliance changes. Use the full `just preflight`
before pushing when practical.

## Deferred

- Physical S14 execution until the H0/H1 loop and its device-specific evidence
  are ready; software work follows the staged prerequisites above.
- After the product-framing change is merged, reconcile the GitHub repository
  description with the short description in [VISION.md](VISION.md#describing-ramenos).
  Repository metadata is an external follow-up, not a local documentation edit.
- Smart plug/PDU and front-panel relay purchases until AMT validation establishes
  a concrete recovery gap.
- Full execution-fabric transport and broad real-kernel broker migration.
- S5.1 wizard orchestration beyond the existing policy proposal path.
- Offer-shaped runtime interfaces until RQ-0001 produces an IDL and evidence plan.
- RamenOrg authority above A2-local until explicit controls and decisions land.

Resolved decisions and their evidence live in [DECISIONS.md](DECISIONS.md), not
in this queue.

## Native Device Evidence Boundary

S11.8 and S13.6 currently validate typed IPC/shared-memory transfers against
embedded Oracle vectors. Device-backed native virtio-net packet I/O and
virtio-blk sector read/write/flush remain separate work. Define the device
attachment, recorded Oracle comparison, and persistence assertions before
implementing either path; successful vector gates do not close this evidence gap.
