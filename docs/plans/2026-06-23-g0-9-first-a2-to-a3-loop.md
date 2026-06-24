# Plan: G0.9 — First A2→A3 Loop + Research-Blocks-Implementation + Human Directives + Slice Namespacing

**Date:** 2026-06-23
**Slice:** `G0.9` (org/governance namespace; see `docs/research/SLICE_NAMESPACING.md`)
**Authority:** A2-local (docs + Foundry governance gates)
**Claim boundary:** no merge, no release, no self-approval, no HIL actuation, and no public support authority.

## Goal

Close the first end-to-end `A2 -> A3` work loop with separated roles, and land the
five pressure points identified in the 2026-06-23 project review:

1. **Slice namespacing** (`P1`): fix the `S12.0` collision (offers airlock vs OS
   golden machine) by namespacing research slices `R-<PROGRAM>-<n>`, and dogfood
   the drift checker to enforce it.
2. **Research blocks implementation** (`P2`): add `requires_rq` + `doctrine_area`
   to `WorkOrderV0`; doctrine-level work may not land until a referenced research
   question supports implementation.
3. **A3 merge gate + first loop** (`P3`): `MergeRequestV0` + `validate_merge.py`
   enforcing separation, evidence-bearing vote, green gates, research block, and
   honest `PASS/LOOP-LOCAL` labelling; run it on the P1 change.
4. **R-OFFERS-1 research-bound slice** (`P4`): declare the offers §13 airlock /
   leakage-meter prototype as a research-bound slice with its claim boundary,
   honestly blocked on `RQ-0001`.
5. **HumanDirectiveV0** (`P5`): typed, hashed founder vision-injection primitive.

## Non-goals

- No real remote merge (`PASS/MERGE`): requires GitHub branch protection +
  credentials, which is a separate A3-authority decision. The loop closes as
  `PASS/LOOP-LOCAL`.
- No `RQ-0001` closure and no LeakageMeter runtime code: R-OFFERS-1 is declared,
  not implemented.
- No authority increase: G0.9 stays A2-local.

## Artifacts

Schemas: `human_directive_v0`, `merge_request_v0`; `requires_rq`/`doctrine_area`
on `work_order_v0`.
Docs: `HUMAN_DIRECTIVE_V0.md`, `MERGE_GATE_V0.md`, `SLICE_NAMESPACING.md`,
`R-OFFERS-1-airlock-leakage-meter.md`, this plan, and the trial record.
Tools: `validate_human_directive.py`, `validate_merge.py`, `render_g0_9.py`, and
negative-test harnesses for both; `status_drift.py` extended for slice namespaces.
Gate: `foundry_org_governance_g0.sh` extended with the G0.9 steps.

## Honesty

This slice proves the **loop machinery** (writer → reviewer → vote → preconditions)
on a real change in the working tree. It is the org equivalent of `PASS/QEMU`: do
not call it a merge. A `PASS/MERGE` is a future, separately-authorized step.
