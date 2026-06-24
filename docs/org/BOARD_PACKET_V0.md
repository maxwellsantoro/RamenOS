# BoardPacketV0

**Last Updated:** 2026-06-23
**Status:** G0.8.1 scaffold

`BoardPacketV0` is the read-only packet a steward heartbeat emits from the
current repo state. It summarizes the active task, cites authoritative docs, and
points at generated work-order, handoff, and vote packets.

It is not an approval by itself and grants no authority beyond the packets it
references.

## Required Fields

```json
{
  "schema_version": 1,
  "packet_kind": "board_packet_v0",
  "packet_id": "BP-2026-06-23-s12-4-1",
  "generated_at_unix_ms": 1782259200000,
  "repo_sha": "unknown",
  "authority_level": "A2",
  "current_task_ref": "docs/org/current_task.yaml",
  "active_track": "S12.4 HIL appliance v0 physical loop",
  "active_task": "S12.4.1 HIL appliance serial observer",
  "next_gate": "just hil-appliance",
  "parallel_tracks": ["G0 Org Kernel", "RQ-0001", "RQ-0002"],
  "context_refs": [
    "CURRENT_STATUS.md",
    "NEXT_TASKS.md",
    "docs/plans/2026-06-22-hil-appliance-controller.md",
    "docs/plans/2026-06-23-research-backed-ramenorg.md"
  ],
  "work_order_refs": ["out/org/examples/work_order_s12_4_1_serial_observer.json"],
  "handoff_refs": ["out/org/examples/handoff_planner_to_implementer_s12_4_1.json"],
  "vote_refs": ["out/org/examples/board_vote_foundry_evidence_s12_4_1.json"],
  "claim_boundary": "A2-local only; no merge, no release, no self-approval, no HIL actuation, and no public support authority"
}
```

## Validation Rules

- `authority_level` must not exceed the authority granted for the slice.
- `current_task_ref` must exist and be the source used by the renderer.
- Context refs and packet refs must exist.
- `next_gate` must resolve to a `just` recipe or executable gate path.
- G0.3 board packets carry exactly one work-order ref, one handoff ref, and one
  vote ref.
- The claim boundary must explicitly include all four denials: no merge, no
  release, no HIL actuation, and no public support authority.
- Board packets must not embed new authority beyond their referenced work orders
  and votes.
- Referenced work-order, handoff, and vote packets must agree on repo SHA, task,
  proposal/work-order id, gate set, and authority level.
