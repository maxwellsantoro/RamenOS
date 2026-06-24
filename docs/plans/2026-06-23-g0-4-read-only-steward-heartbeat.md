# G0.4 Read-Only Steward Heartbeat / Board Brief

**Last Updated:** 2026-06-23
**Status:** Scaffold

## Purpose

G0.4 turns the validated packet set into a human-readable handoff surface. A
next agent should be able to read `out/org/current_board_brief.md` plus the
referenced packets and understand the active task, evidence, gates, and allowed
actions without relying on a pasted chat summary.

Authority remains A0/A1 only.

## Definition of Done

1. Render `out/org/current_board_brief.md` from validated packets.
2. Brief includes active task, authority boundary, required gates, context refs,
   evidence refs, and allowed next-agent actions.
3. Brief generation happens only after packet validation passes.
4. Governance gate checks brief existence and key sections.
5. Preserve A0/A1 only.

## Deliverables

- `tools/org/render_board_brief.py`
- `docs/org/BOARD_BRIEF_V0.md`
- Governance gate step after packet validation and before negative validator
  cases.
- Status, roadmap, decision, and changelog updates.

## Non-Scope

- No autonomous cadence, scheduler, or monitor.
- No merge, release, HIL actuation, public support, credentials, or
  identity-level role separation authority.

## Gate

```bash
just foundry-org-governance-g0
```
