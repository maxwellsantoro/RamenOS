# Next Tasks

**Last Updated:** 2026-09-30
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

## Parallel Execution Lanes

**Now:** SW0 A2 LS transactions, model subscriptions and protocol/authority conformance; HIL appliance hardware runs await setup.
A0/A1.0 contracts, A1.1/A2.1 host foundations and the opt-in A2.2 shared JSON/RT adapter and A2.3 independent LT transactions are implemented.
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

**Next software action:** A2 — add LS durable transaction commands, model
subscription semantics and canonical authority conformance. A2.3's
[independent LT backend](docs/AGENT_TASK_LT_BACKEND_V1.md) runs the shared
[JSON contract](docs/AGENT_TASK_ADAPTER_V1.md) and named RT/LT point cases.
A2.1's
[Linux scoped-shell foundation](docs/AGENT_TASK_LINUX_CONTROL_V1.md) is runnable
with real containment probes. Preserve A1.1's independently runnable host gate
and its explicit
[fixture boundary](docs/AGENT_TASK_SERVICE_PROOF_V1.md).
SW0 has no H0–H3 prerequisite. The
[Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines one
consumer task: repair a scoped configuration, execute its pinned validator, and
report the resulting artifact while access to another workspace is denied.

0. A0 is implemented: `just foundry-agent-task-contract-a0` checks the pure
   contract model and synthetic fixtures. It is not a useful task or an
   enforcement boundary. See [Agent Task Contract V0](docs/AGENT_TASK_CONTRACT_V0.md).
1. A1.0 is implemented: `just foundry-agent-task-protocol-a1-0` checks generated
   fixed control layouts and fail-closed request preflight. The documented
   call-path inventory and A1.1 matrix do not supply service enforcement.
   A1.1 is implemented: `just foundry-agent-task-proof-rt` checks the useful
   scripted host task, forced denials, worker bounds, durable receipt recovery,
   scoped events and audit/receipt replay. Production registration, separately
   isolated clients and target-kernel task enforcement remain outside its scope.
2. A2.1 implements a scripted Linux scoped-shell repair, shared development
   fixture, pinned validation, measured Docker containment and named probes. It
   does not supply LS durable commits or all-arm conformance.
   A2.2 implements the shared JSON codec/descriptions and opt-in RT bridge with
   independent executable/schema assertions. A2.3 adds independent LT transactions,
   direct broker denials/recovery and shared named point-case checks. Native
   subscriptions are not exposed in JSON. Continue with LS transactions,
   model subscriptions, common protocol fixtures,
   authority mapping/conformance and all-arm negative cases. Keep A1 runnable
   independently; comparative data collection requires all A2 controls to pass.
3. Pilot Linux scoped shell, Linux typed, and RamenOS typed using one evaluator
   and hidden fixture bank. Verify LT/RT protocol equivalence and canonical
   authority mappings. Use the predeclared power rule to size and freeze the
   final comparison within a funded, predeclared ceiling, then run it opt-in.
   If power is unaffordable, publish a bounded exploratory report and record the
   proceed/defer decision with its limitations. Report the three contrasts and
   separate
   completion, authority, cost, and audit/replay outcomes, including uncertainty.
4. Add target-side enforcement evidence for named task operations. The existing
   QEMU snapshot/IPC bridge alone cannot establish this task's OS boundary.

A0/A1.0, the A1.1/A2.1 host foundations and A2.2 JSON/RT plus A2.3 LT transactions are implemented;
full A2 conformance and model comparison are **not implemented**. Completion of
the contract is not completion
of the experiment; an unfavorable comparison is a valid
result and should inform the next software slice.

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
| GP0 | G0.8.1 implementation authority and serial claim hygiene | `just foundry-org-governance-g0` |
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
- After this branch merges, update the GitHub repository description to:
  "An experimental Rust OS for agents: typed capabilities, machine-readable
  system state, and evidence-gated hardware support."
- Smart plug/PDU and front-panel relay purchases until AMT validation establishes
  a concrete recovery gap.
- Full execution-fabric transport and broad real-kernel broker migration.
- S5.1 wizard orchestration beyond the existing policy proposal path.
- Offer-shaped runtime interfaces until RQ-0001 produces an IDL and evidence plan.
- RamenOrg authority above A2-local until explicit controls and decisions land.

Resolved decisions and their evidence live in [DECISIONS.md](DECISIONS.md), not
in this queue.
