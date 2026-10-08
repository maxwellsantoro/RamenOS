# Next Tasks

**Last Updated:** 2026-10-08
**Status:** Authoritative ready work and acceptance dependencies

[Current Status](CURRENT_STATUS.md) records landed behavior; [Roadmap](ROADMAP.md)
connects checkpoints to [Vision](VISION.md). This queue owns dispatch order. Use
[Agentic Workflow](docs/AGENTIC_WORKFLOW.md) for scope, resource reservations and
handoff; do not copy this queue into instructions or skills.

## Ready work front

**Now:** repair host-task ownership/focus/timeout transitions under corrected CI evidence admission, then complete the real editor process; advance RUN0/input/storage preparation independently.

The first product checkpoint remains one keyboard-first human editing task that
works without a model. Preserve its in-process control, then cross a real process
boundary, target/QEMU, persistent target storage and one qualified machine. The
October 8 review supplies concrete regression scenarios; green existing tests do
not close them. No paid study or physical actuation is needed for the ready repairs.

### First batch: reliability and evidence

Use coordinator + two implementers + one reviewer. Dispatch only two disjoint
implementation packets at once; the reviewer checks assertions before dependent
implementation and rotates onto completed patches. Root owns shared status,
IDL/generated files, gate registrations, manifests and source registries unless
explicitly delegated. Multiple packets touching host.rs or Store lifecycle run
sequentially in that scope; other ready work fills freed capacity. Each row below
is a bounded outcome, not permission to edit every listed area simultaneously.

| Packet | Consumer and owned boundary | Gate-first completion |
|--------|-----------------------------|-----------------------|
| R1 — Store reply ownership | Store data registry → pending reply → native Read/receipt lease; exact files assigned centrally | Concurrent approved sessions on different objects return correct bytes without spurious denial. Pending delivery and held leases survive reclamation; dropped owners release capacity within 32. Join/quiesce actual producers. Cover receipt transfer without replaying mutation. Do not disable reclamation or serialize clients as the fix. |
| R2 — render acquisition and focus | Desktop renderer/surface lifecycle and trusted launch handoff | Pause before Present, switch to launcher, join stale render, explicitly restore focus and require fresh render+composition. Abandoned acquisition retires only its generation and aliases. Approved initial launch receives focus; background redraw preserves launcher focus and cannot consume its keyboard input. Keep these assertions independent. |
| R3 — Save before admission | Desktop intent/admission state and native app orchestration | Expired key creates no pending Save; expiration between intent and allocation has authenticated owner-side definitive closure. Fresh Ctrl+S saves unchanged draft exactly once. No operation/dispatch/permit from expired attempt. Preserve admitted Unknown/original reconciliation and no mutation replay. Depends on R2 if same Desktop files are owned. |
| R4 — journal staging investigation | Store publisher and genuine joined-owner reopen | Inject JournalSync failure; retain disk/trace/original result, join owners and attempt subsequent publication/reopen. Distinguish safe owner-created pre-rename staging from ambiguous post-rename state. Only then implement bounded cleanup/reconciliation with corrupt/foreign/symlink denials. CAS temporary files are a separate investigation. Depends on R1 where Store files overlap. |

CI0 is accepted: executable-input classification and transitive NativeRead/Preview
source admission have independent review and passing actual Linux affected gates;
[Current Status](CURRENT_STATUS.md#foundry-development-and-ci-execution) owns the
validation scope. R1–R4 are unresolved runtime repairs/investigation. They require new maintained assertions and evidence inventory
entries; none is certified by the prior eighteen-case task. Review each patch and
run its producer plus affected consumers on a fixed source snapshot. Preserve
original 18/Reader93, volatile13, adapter4 and codec/Read/Preview controls rather
than substituting new regressions for old coverage. Do not bake historical source
row counts into future closure acceptance; legitimate additions require reviewed
registry/pin successors.

### Process successor: split at real ownership boundaries

Repair acceptance joins here. Assertion/contract work can proceed in a disjoint
scope while repairs run, but dependent runtime integration waits for R1–R4 and
CI0. Preserve the [process contract](docs/contracts/editor-process-v0.json),
[API inventory](docs/DESKTOP_EDITOR_PROCESS_API_V0.md),
[process plan](docs/plans/editor-process-v0.md) and live Save contracts. The shared
pure core and adapters are already accepted; do not reimplement them.

| Packet | Prerequisite | Deliverable / acceptance |
|--------|--------------|--------------------------|
| P0 — executable assertions and safe owner staging | Frozen process/live Save contracts; reviewed shared-core control | Complete all thirty finite behavior/denial/failure families and original missing-API RED before dependent handlers. Finish genuine constructor/setup, Read/reopen, consuming-join bounds/non-unwinding owner return, Save-pause panic and uncertain parent-death controls. Private smoke/cleanup evidence remains preparation. |
| P1 — authenticated carrier and lifecycle | P0 review; relevant reliability fixes | Held sealed executable identity, authenticated bounded control and exact descriptor/publication ownership; child failure, malformed/foreign/stale channels and parent-death cleanup. Unproved quiescence retains owners/charges and original deadlines; watchdog death is not child reap. |
| P2 — child edit/raster consumer | P0 and fixed P1 interface/ownership contract | Actual Linux editor child uses existing core, edits bounded text and publishes immutable frames through issued channels. Can implement alongside P1 in separate files after interface freeze; acceptance requires their real combined runtime. No copied bootstrap or thread join supplies child authority. |
| P3 — Save/recovery integration | Accepted P1+P2 and R1–R4+CI0 | Genuine Store evidence/recording binds current Core, successors, original Save owners and full receipt. Preserve original operation through child death without replay; exercise denial/crash/recovery and unrelated consumer progress end to end. Run original in-process task as control plus process gate on one fixed candidate. |

Root assigns exact paths and outputs at dispatch; proposed packet IDs are planning
labels, not implemented recipes. A pure record, PID or source inclusion is not
process execution, containment, target persistence or hardware evidence.

### Independent frontier and target joins

Use spare capacity only when it does not delay review/integration of the human task.
Keep Oracle/profile preparation and target memory ownership moving during host
repairs; neither waits for a process-runtime merge or model study.

| Packet | Ready scope | Completion / join |
|--------|-------------|-------------------|
| RUN0.0 → RUN0 | Freeze relocated-entry capture/profile and executable raw-map/retention assertions using the [entry preparation](docs/plans/run0-relocated-entry-preparation-v0.md) | Obtain actual initial table access, complete retained ranges and final firmware-exit map; prove allocation/write/read/reuse plus S8 in QEMU. Then one target application with grants, denial, failure/restart. Pure selector or external memory reads do not prove guest access/runtime. |
| IN0 | Select one input controller and obtain Reference Vault + Oracle protocol_trace, then freeze typed keyboard/failure contract | Actual device-backed keyboard reports with malformed/reset/unplug/bounded-queue/denied delivery assertions. Host injection supports consumer work but cannot qualify the driver. Physical execution waits for H0/H1. |
| STORE0 | Use selected block Vault/Oracle; freeze actual QEMU block read/write/flush and bounded Store durability assertions | Implement one device-backed path and named Store consumer. Target save/reopen joins this evidence; embedded vectors and firmware NVMe detection do not supply it. |
| UI2 | Freeze target compositor/display and service adapter boundaries after host contracts | Port exact required services, then join RUN0+IN0+display with unchanged human task in QEMU; join STORE0 for persistent save/reopen. Name every remaining host service. Qualified machine repeat needs H0/H1, device evidence and relevant S12/S13. |
| REC0 | S13 selection/publication contract and versioned failure schema | Inactive-slot publication/readback, revisioned selection and interrupted phases in host/QEMU. Physical new-slot and rollback boots remain H3. |
| SW-A / SW-E | Remaining authority or provider-supervision controls in separate assigned files | Run named Linux/Docker gates; preserve unknown authority, failed/pending attempts, original cleanup and private bank/funded-work-order controls. Avoid repeating landed canary/accounting controls. Full A2 precedes paid model collection; no agent-study barrier on desktop work. |

After the genuine process task, add a bounded component-replacement experiment
alongside target preparation: one existing service interface, two implementations,
qualified selection, explicit activation/failure/recovery and an unrelated human
task that continues. Define permitted service interruption, generation and unresolved
operation behavior first. Measure changes outside the component, reused evidence,
qualification effort, disruption and recovery; do not open a generic deployment
platform or hardware breadth project. Device variants require real DMA/reset evidence.

### Integration cadence

Integrate one reviewed packet at a time; do not accumulate a giant milestone PR.
Use `just dev-check` and focused gate assertions while editing. On the fixed
combined candidate, run affected consumers, required planning checks and the
canonical CI lanes. Reserve shared outputs/build targets; no overlapping writers
while source-bound gates run. Reuse a result only if its tested inputs/features,
environment and claim are unchanged. Review throughput and measured host-stage
costs determine the next optimization; worker count alone is not progress.

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

Run `just hil-appliance` for appliance changes. Use focused checks while editing. Before integration, run affected consumers and
the canonical CI lanes on the same candidate; full `just preflight` is the serial
Linux equivalent, not an additional duplicate run after all lanes pass.

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
