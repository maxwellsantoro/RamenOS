# G0.6 Intake-Only Agent Trial

**Date:** 2026-06-23
**Status:** Implemented
**Authority:** A0/A1 only

## Purpose

G0.6 tests the G0.5 bundle as an actual agent intake surface instead of adding
another internal packet mechanism. A fresh agent receives no thread history and
only the generated brief, manifest, and four referenced packet files.

## Definition of Done

1. Generate the intake bundle with `just foundry-org-governance-g0`.
2. Start a fresh agent with inherited thread context disabled.
3. Supply only the board brief, intake manifest, board packet, work order,
   handoff, and vote.
4. Request a bounded S12.4.1 plan that cites packet refs, gates, and authority.
5. Record whether hidden chat context or external file reads were needed.
6. Preserve missing-context failures as trial findings rather than prompt fixes.
7. Keep the slice at A0/A1.

## Result

The trial passed for plan intake. The fresh agent recovered the active task,
work-order identity, packet refs, A1 boundary, four authority denials, and both
required gates without inherited chat context or external file reads.

The same six-file bundle was not sufficient for a responsible patch. The agent
identified absent controller plans, evidence policy, status/task sources, and
scoped implementation files. That limitation is recorded in the trial report;
G0.6 does not silently widen the prompt or bundle.

## Evidence

```text
docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md
```

## Gate

```text
just foundry-org-governance-g0
```
