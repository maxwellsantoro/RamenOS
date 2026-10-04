# Next Tasks

**Last Updated:** 2026-10-04
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

This queue serves the [Vision](VISION.md): an everyday, post-Unix OS for humans
and AI agents. Current hardware and agent-task foundations lead toward human
interactivity, the desktop, and wider software/hardware support; vision alignment
does not change the lane prerequisites below.

## Parallel Execution Lanes

**Now:** SW0 A2 remaining host/deputy/unexercised and continuous authority coverage; HIL appliance hardware runs await setup.
Implemented steps and their precise boundaries are listed in [Current Status](CURRENT_STATUS.md).
Physical H0–H3 await test-hardware setup; no live capture or actuation is scheduled.

H0–H3 are ordered within the physical lane; SW0 is an independent software lane,
not the next item after H3. Continue SW0 without waiting for lab access or NVMe
graduation. Lane labels are queue positions, not new slice identifiers.

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

**Next software action:** complete the remaining A2 controls before model
collection. Run the expanded RT/LT point and subscription transition gates on
Linux/Docker as part of conformance. Preserve the independently runnable A1.1 host proof. SW0 has no
H0–H3 prerequisite; its consumer is a scoped configuration repair, pinned
validation and checked artifact publication.

| Order within SW0 | Work remaining | Acceptance / dependency |
|------------------|----------------|-------------------------|
| A2 authority | Bound host-client/deputy differences and unexercised authority outside the declared interface; extend continuous lifetime coverage | Named backend and OS probes, explicit available/task/probe effects, and honest unknown/inclusion results; finite issued-right equality is insufficient |
| A2 study controls | Freeze real bank/study releases, provider/token accounting and arm supervision | Independent hidden partitions, identical authorized task resources, frozen context/usage accounting and no silent retry or discarded failure rows |
| A2 conformance | Run all deterministic controls across LS/LT/RT and validate canonical mappings | No skipped negative cases or missing protocol/authority mappings; preserve the client/deputy differences in the report |
| Phase B pilot and report | Pilot the three arms, apply the predeclared power rule, then freeze an affordable final comparison or publish exploratory results | Explicit funded ceiling and opt-in work order; separate interface/substrate/total contrasts and completion, authority, cost and audit/replay outcomes with uncertainty |
| Phase C target evidence | Add target enforcement for named task operations | Actual kernel/QEMU grants and forbidden probes per operation; the snapshot/IPC bridge alone is insufficient |

The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) owns the
experimental contract and proposed future commands. The implemented gate/contract
map is in [Current Status](CURRENT_STATUS.md#sw0-runnable-evidence-not-a-completed-experiment).
Full A2 conformance and model comparison are **not implemented**.

Carry forward the [authority inventory](docs/AGENT_TASK_AUTHORITY_V1.md),
[requestable/lifetime boundary](docs/AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md),
[evaluator controls](docs/AGENT_TASK_EVALUATOR_CONTROLS_V1.md), and
[reconciliation](docs/AGENT_TASK_RECONCILIATION_V1.md): retain validator timeouts
and forced failures in the denominator; quarantine unresolved create intents;
certify cleanup only from complete acknowledged-ID evidence. Completion of a
contract is not completion of the study. A tie, regression or budget-limited
report is a valid result for the next software decision.

## S14 Expansion Prerequisites

S14 USB xHCI/HID implementation depends on both lanes: a demonstrated stable
H0/H1 observation-and-actuation loop, review of SW0 A1/A2 evidence, and a recorded
proceed/defer decision on the bounded Phase B report. A budget-limited exploratory
report may satisfy that review with explicit uncertainty, without a powered
claim; a positive RamenOS advantage is not required. It also needs its own short
design, Reference Vault
and Oracle trace, IDL boundary, and Foundry gate definition before implementation.
H2/H3 remain the physical graduation sequence; they do not block SW0. SW0 Phase C
is a separate target-enforcement follow-up, not a prerequisite for the host study.

## Parallel Project-Control Track

This lane can proceed without displacing H0–H3 or SW0.

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

- S14 implementation until the H0/H1 loop is stable, SW0 Phase A/B results are
  reviewed, and the S14 design/IDL/Oracle/gate prerequisites above are met.
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
