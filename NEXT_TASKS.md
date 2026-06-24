# Next Tasks

**Last Updated:** 2026-06-24
**Status:** Active and authoritative for execution order

> [CURRENT_STATUS.md](CURRENT_STATUS.md) records what landed. This file records
> what to execute next. [ROADMAP.md](ROADMAP.md) is directional, not operational.

## Active Execution Track

**Now:** Implement the S12.4 HIL appliance v0 physical loop: stabilize the serial observer, then add the power/reset actuator. Run S13 metal HIL graduation through the appliance once that loop is stable.

| Priority | Task | Completion signal |
|----------|------|-------------------|
| P0 | S12.4.1 HIL appliance serial observer | `RAMEN_HIL_APPLIANCE=1 just hil-appliance` captures live serial and emits valid controller evidence |
| P1 | S12.4.2 HIL appliance power/reset actuator | Power and reset scripts are fail-safe, dry-run tested, and represented in controller evidence JSON |
| P2 | S13 metal HIL graduation through the appliance | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil` produces valid live provenance |
| P3 | S12 physical graduation through the appliance | `RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil` produces valid live provenance |
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

### P1 Acceptance Criteria

- Add `tools/hil/appliance_press_power.sh` and
  `tools/hil/appliance_press_reset.sh`.
- Relays default to inactive and enforce bounded pulse durations.
- Dry-run behavior is deterministic and covered by the appliance gate.
- Controller evidence records action, channel, duration, run id, and result.
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
- Full execution-fabric transport and broad real-kernel broker migration.
- S5.1 wizard orchestration beyond the existing policy proposal path.
- Offer-shaped runtime interfaces until RQ-0001 produces an IDL and evidence plan.
- RamenOrg authority above A2-local until explicit controls and decisions land.

Resolved decisions and their evidence live in [DECISIONS.md](DECISIONS.md), not
in this queue.
