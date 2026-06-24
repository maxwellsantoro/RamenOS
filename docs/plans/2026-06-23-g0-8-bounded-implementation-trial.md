# G0.8 Bounded Implementation Trial for S12.4.1

**Date:** 2026-06-23
**Status:** Implemented
**Authority:** A2-local only, reclassified by G0.8.1

## Purpose

Use the G0.7 bounded-context result to make the first small implementation
patch for S12.4.1 without widening authority. The patch may create the serial
observer, update the HIL appliance gate, and record the trial result. It must
not merge, release, actuate hardware, or claim public support authority.

## Definition of Done

1. Authorize the implementation work order explicitly as A2-local.
2. Implement `tools/hil/appliance_capture_serial.sh`.
3. Touch only the S12.4.1 implementation surface for the implementation patch:
   the serial observer and HIL appliance gate.
4. Keep stale `RAMEN_HIL_SERIAL_LOG` replay development-only and forbidden for
   graduation.
5. Run `just hil-appliance`.
6. Run `just foundry-org-governance-g0`.
7. Record a trial report with outcome, context sufficiency, touched files, and
   gate results.
8. Preserve no merge, release, self-approval, HIL actuation, or public support authority.

## Result

G0.8 implemented the S12.4.1 serial-observer scaffold and added a default-CI
contract test for it. G0.8.1 later clarified that this was A2-local
implementation authority, not A1 proposal authority. The observer archives a
timestamped serial transcript, parses `RAMEN OS`, `golden_machine:*`,
`persistent_storage:*`, and `hil_evidence:*` markers, writes wrapper evidence
under `out/evidence/`, and rejects stale serial-log replay in graduation mode.

## Gate

```text
just hil-appliance
just foundry-org-governance-g0
```
