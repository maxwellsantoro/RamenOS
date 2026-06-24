# HandoffPacketV0

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

`HandoffPacketV0` replaces copy/paste between agents. It carries enough bounded
context for the next role to continue without guessing.

## Required Fields

```json
{
  "schema_version": 1,
  "packet_kind": "handoff_packet_v0",
  "handoff_id": "HO-2026-06-23-planner-to-implementer",
  "work_order_id": "WO-2026-06-23-s12-4-1-serial-observer",
  "from_role": "Planner",
  "to_role": "Implementer",
  "repo_sha": "unknown",
  "task": "Implement S12.4.1 HIL appliance serial observer",
  "context_refs": [
    "CURRENT_STATUS.md",
    "NEXT_TASKS.md",
    "docs/plans/2026-06-22-hil-appliance-controller.md",
    "EVIDENCE_LEVELS.md"
  ],
  "claims": [
    {
      "claim": "S12.4.1 is the next active OS task",
      "source": "NEXT_TASKS.md"
    }
  ],
  "constraints": [
    "Pi GPIO UART is TTL-only; do not wire directly to RS-232",
    "Graduation mode forbids stale serial-log replay"
  ],
  "requested_output": "Patch plus gate result summary",
  "required_gates": ["just hil-appliance"]
}
```

## Validation Rules

- `from_role` and `to_role` must be different unless the packet is explicitly a
  self-review draft.
- `packet_kind` must be `handoff_packet_v0`.
- `work_order_id` must match the referenced work order.
- Every claim must cite a source.
- Requested output must be concrete enough to verify.
- Required gates must match the claim level.
- Handoffs must not smuggle new authority beyond the destination role.
