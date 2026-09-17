# Next Tasks

**Last Updated:** 2026-09-16
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

## Parallel Execution Lanes

**Now:** H0 HIL appliance serial observer — first live capture — and SW0 Agent Task Proof Phase A can proceed independently.

H0–H3 are ordered within the physical lane; SW0 is an independent software lane,
not the next item after H3. Start SW0 now without waiting for lab access or NVMe
graduation. Lane labels are queue positions, not new slice identifiers.

## Physical Lane: H0–H3

Run the first live capture on the physically ready Pi↔M900 chain, then provision
and validate the M900's Intel AMT 11 power/reset path.
S12 runs on the installed 240 GB SanDisk SATA SSD. Add a compatible M.2 2280
PCIe NVMe drive before S13 metal graduation. Front-panel relays and a
smart plug/PDU remain deferred until AMT testing shows they are necessary.

| Order | Task | Completion signal |
|----------|------|-------------------|
| H0 | S12.4.1 HIL appliance serial observer — first live capture | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 just hil-appliance` captures live serial and emits valid controller evidence |
| H1 | S12.4.2 Intel AMT power/reset actuator | AMT status, power-on, power-off, reset, and power-cycle are validated from the Pi and represented in controller evidence JSON |
| H2 | S12 physical graduation on the installed SanDisk SATA SSD | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil` produces valid live provenance |
| H3 | Add M.2 2280 PCIe NVMe and run S13 metal graduation | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` produces valid live provenance with `claim_path: appliance-mediated` |

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

**Next software action:** write Phase A's deterministic task, control-protocol,
authority-mapping, denial, and replay assertions, then implement the adapters.
SW0 has no H0–H3 prerequisite. The
[Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines one
consumer task: repair a scoped configuration, execute its pinned validator, and
report the resulting artifact while access to another workspace is denied.

1. Inventory the actual Semantic State, Store, broker, and native runner paths.
   Write the task-success, forced-denial, revocation, conflict, audit, and replay
   assertions first; define missing native operations through IDL/codegen.
2. Implement the fixture and scripted consumer across the host service boundary.
   Ship a deterministic Foundry gate and inspectable evidence bundle. Report
   host enforcement explicitly; no target-native or comparative claim yet.
3. Pilot Linux scoped shell, Linux typed, and RamenOS typed using one evaluator
   and hidden fixture bank. Verify LT/RT protocol equivalence and canonical
   authority mappings. Use the predeclared power rule to size and freeze the
   final comparison, then run it opt-in. Report the three contrasts and separate
   completion, authority, cost, and audit/replay outcomes, including uncertainty.
4. Add target-side enforcement evidence for named task operations. The existing
   QEMU snapshot/IPC bridge alone cannot establish this task's OS boundary.

The proof and its proposed commands are **not implemented**. Completion of the
plan is not completion of the experiment; an unfavorable comparison is a valid
result and should inform the next software slice.

## S14 Expansion Prerequisites

S14 USB xHCI/HID implementation depends on both lanes: a demonstrated stable
H0/H1 observation-and-actuation loop, and review of SW0 Phase A evidence and
Phase B comparison results. It also needs its own short design, Reference Vault
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
