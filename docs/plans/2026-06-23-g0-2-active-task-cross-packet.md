# G0.2 Active Task Frontmatter and Cross-Packet Consistency

**Last Updated:** 2026-06-23
**Status:** Scaffold

## Purpose

G0.1 proved that RamenOrg packets can be rendered and validated. G0.2 makes the
packet set agree about the same world:

- The active task is read from `docs/org/current_task.yaml`, not hardcoded in
  the renderer.
- Referenced packets are loaded together and checked relationally.
- Gate references fail closed unless they are `just <recipe>` or an existing
  executable-style repo path.
- Board votes distinguish design, gate, claim, HIL, and release evidence refs.

This keeps authority at A0/A1 and does not grant merge, release, HIL actuation,
or public support authority.

## Deliverables

- `docs/org/current_task.yaml` for the active task packet source.
- `tools/org/render_board_packet.py` reads the current-task file.
- `tools/org/validate_packets.py` performs cross-packet consistency checks.
- Packet schemas updated for:
  - `BoardPacketV0.current_task_ref`
  - `HandoffPacketV0.work_order_id`
  - typed `BoardVoteV0.evidence` buckets.
- Governance gate updated to require and validate the new files.

## Validation Scope

The governance gate now enforces:

- `board_packet.repo_sha == work_order.repo_sha == handoff.repo_sha`
- `vote.proposal_id == work_order.work_order_id`
- `handoff.work_order_id == work_order.work_order_id`
- `handoff.task == work_order.task`
- `board_packet.active_task == work_order.task`
- `handoff.required_gates == work_order.required_gates`
- `board_packet.authority_level == work_order.authority_level`
- unknown gate-ref syntax fails closed.

## Non-Scope

- Identity-level agent separation.
- GitHub, release, HIL, or support credentials.
- Full YAML or JSON Schema implementations.
- Runtime offer-boundary work.

## Gate

```bash
just foundry-org-governance-g0
```
