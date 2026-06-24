# G0.1 Board Packet and Packet Validators

**Last Updated:** 2026-06-23
**Status:** Scaffold

## Purpose

G0 created the first RamenOrg control surface. G0.1 makes that surface
machine-checkable enough for the next agent handoff: board packets, work orders,
handoffs, and votes are now JSON artifacts validated by a Foundry gate.

This is still A0/A1 only. It does not grant merge, release, HIL actuation, or
public support authority.

## Deliverables

- JSON schemas:
  - `schemas/org/work_order_v0.schema.json`
  - `schemas/org/handoff_packet_v0.schema.json`
  - `schemas/org/board_vote_v0.schema.json`
  - `schemas/org/board_packet_v0.schema.json`
- Packet tools:
  - `tools/org/render_board_packet.py`
  - `tools/org/validate_packets.py`
- Generated examples under `out/org/examples/` during the governance gate:
  - S12.4.1 work order.
  - Planner-to-implementer handoff.
  - Foundry Evidence Officer vote.
  - Read-only board packet.
- Gate integration in `tools/ci/foundry_org_governance_g0.sh`.

## Validation Scope

`validate_packets.py` checks:

- Required fields, scalar constants, enums, arrays, and object shapes.
- Role separation for handoffs.
- Evidence refs for approval votes.
- Blocking conditions for block votes.
- Context refs and packet refs exist.
- `just ...` gate refs resolve to local recipes.
- G0.1 examples do not exceed A1 authority.
- HIL work orders mention evidence constraints.

## Non-Scope

- Full JSON Schema draft implementation.
- GitHub, HIL, release, or credential authority.
- Machine-readable frontmatter in all planning docs.
- Runtime offer-boundary implementation.

## Gate

```bash
just foundry-org-governance-g0
```

The gate renders packet examples into `out/org/`, validates them against schemas
and RamenOrg custom rules, and preserves the existing active-track drift check.
