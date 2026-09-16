# Next Tasks

**Last Updated:** 2026-09-16
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

## Active Execution Track

**Now:** Run the first live HIL appliance serial capture on the physically ready Pi↔M900
chain, then provision and validate the M900's Intel AMT 11 power/reset path.
S12 runs on the installed 240 GB SanDisk SATA SSD. Add a compatible M.2 2280
PCIe NVMe drive before S13 metal graduation. Front-panel relays and a
smart plug/PDU remain deferred until AMT testing shows they are necessary.

| Priority | Task | Completion signal |
|----------|------|-------------------|
| P0 | S12.4.1 HIL appliance serial observer — first live capture | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_SERIAL_DEV=/dev/ttyUSB0 just hil-appliance` captures live serial and emits valid controller evidence |
| P1 | S12.4.2 Intel AMT power/reset actuator | AMT status, power-on, power-off, reset, and power-cycle are validated from the Pi and represented in controller evidence JSON |
| P2 | S12 physical graduation on the installed SanDisk SATA SSD | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil` produces valid live provenance |
| P3 | Add M.2 2280 PCIe NVMe and run S13 metal graduation | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` produces valid live provenance with `claim_path: appliance-mediated` |
| P4 | S14 USB xHCI and HID design pass | Approved short plan, IDL boundary, and Foundry gate definition before implementation |

### P0 Acceptance Criteria

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

### P1 Acceptance Criteria

- Provision AMT 11 through MEBx on a trusted wired lab network.
- Add AMT-backed status, power-on, power-off, reset, and power-cycle commands.
- Keep AMT credentials out of evidence, logs, and the repository.
- Dry-run behavior is deterministic and covered by the appliance gate.
- Controller evidence records action, transport, target, run id, and result.
- Validate reachability while the target is running, soft-off, and hung in the
  target OS before deciding whether a smart plug/PDU or relay fallback is needed.
- Physical actuation remains opt-in; governance scaffolding grants no ambient
  HIL actuation authority.

## Parallel Project-Control Track

This lane can proceed without displacing P0-P4.

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

- S14 implementation until the appliance loop is stable and a design pass lands.
- Smart plug/PDU and front-panel relay purchases until AMT validation establishes
  a concrete recovery gap.
- Full execution-fabric transport and broad real-kernel broker migration.
- S5.1 wizard orchestration beyond the existing policy proposal path.
- Offer-shaped runtime interfaces until RQ-0001 produces an IDL and evidence plan.
- RamenOrg authority above A2-local until explicit controls and decisions land.

Resolved decisions and their evidence live in [DECISIONS.md](DECISIONS.md), not
in this queue.
