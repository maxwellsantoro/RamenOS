# WorkOrderV0

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

`WorkOrderV0` is the bounded task artifact an agent receives before doing
implementation, review, research, or release work.

## Required Fields

```json
{
  "schema_version": 1,
  "packet_kind": "work_order_v0",
  "work_order_id": "WO-2026-06-23-s12-4-1-serial-observer",
  "repo_sha": "unknown",
  "role": "Implementer",
  "authority_level": "A1",
  "task": "Implement S12.4.1 HIL appliance serial observer",
  "scope": ["tools/hil", "tools/ci", "hardware", "NEXT_TASKS.md"],
  "context_refs": [
    "CURRENT_STATUS.md",
    "NEXT_TASKS.md",
    "docs/plans/2026-06-22-hil-appliance-controller.md",
    "EVIDENCE_LEVELS.md"
  ],
  "constraints": [
    "Do not claim PASS/METAL from stale serial logs",
    "Appliance evidence wraps per-gate evidence and does not replace it"
  ],
  "required_gates": ["just hil-appliance"],
  "claim_level_allowed": "scaffold",
  "rollback_plan": "Revert only files touched by this work order"
}
```

## Validation Rules

- `schema_version` must be `1`.
- `packet_kind` must be `work_order_v0`.
- `task`, `role`, `context_refs`, `constraints`, and `required_gates` must be
  non-empty.
- `authority_level` must match the granted level for the slice.
- `required_gates` must use `just <recipe>` or an existing executable-style repo
  path; unknown gate syntax fails closed.
- `scope` must not include unrelated modules unless the work order explains why.
- Hardware tasks must name HIL evidence requirements.
- `claim_level_allowed: PASS/METAL` is only valid when the referenced board vote
  carries HIL evidence refs.
- Research tasks must name product risk, claim boundary, and landing path.
- Release tasks must name the maximum public claim level.
