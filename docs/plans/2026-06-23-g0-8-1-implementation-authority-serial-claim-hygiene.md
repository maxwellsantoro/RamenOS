# G0.8.1 Implementation Authority and Serial Observer Claim Hygiene

**Date:** 2026-06-23
**Status:** Implemented
**Authority:** A2-local only

## Purpose

Clean up the authority semantics and evidence mode hygiene from G0.8 before
moving toward S12.4.2 power/reset actuation. Code-writing trials are
implementation authority, so they are A2-local rather than A1. A2-local still
does not grant merge, release, self-approval, HIL actuation, or public support
authority.

## Definition of Done

1. Reclassify the active implementation work order as A2-local.
2. Keep A3+ rejected by packet and current-task validators.
3. Add run-id path-safety validation to the serial observer.
4. Add `serial_input_kind` to HIL appliance evidence.
5. Reject empty serial transcripts.
6. Distinguish development replay evidence from live appliance evidence:
   `PASS/HIL-LOG` for `RAMEN_HIL_SERIAL_LOG`, `PASS/HIL-APPLIANCE` for
   `RAMEN_HIL_SERIAL_DEV`.
7. Add synthetic negative tests for unsafe run ids and empty transcripts.
8. Preserve no merge, no release, no self-approval, no HIL actuation, and no
   public support authority.

## Result

G0.8.1 makes the authority ladder honest while keeping the implementation local
and bounded. The observer now validates run ids before building output paths,
fails closed on empty captures, records `serial_input_kind`, and prevents copied
development logs from looking like live appliance evidence.

## Gate

```text
just hil-appliance
just foundry-org-governance-g0
```
