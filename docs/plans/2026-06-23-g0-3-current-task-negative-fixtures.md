# G0.3 CurrentTaskV0 Schema, Negative Fixtures, and Vote SHA Binding

**Last Updated:** 2026-06-23
**Status:** Scaffold

## Purpose

G0.2 made packets agree about one active task. G0.3 hardens that boundary:

- `docs/org/current_task.yaml` is validated against `CurrentTaskV0` before
  packet rendering.
- `BoardVoteV0` binds to the same repo SHA as board, work-order, and handoff
  packets.
- Board packets are exactly-one for work order, handoff, and vote refs in this
  scaffold phase.
- Negative tests prove the validator rejects malformed organizational state.

Authority remains A0/A1 only.

## Deliverables

- `schemas/org/current_task_v0.schema.json`
- `BoardVoteV0.repo_sha`
- `BoardPacketV0` exactly-one packet refs (`maxItems: 1`)
- `tools/org/test_validate_packets.py`
- Governance gate negative cases for:
  - mismatched SHA
  - missing evidence
  - unknown gate syntax
  - A2 authority
  - stale HIL claim without evidence constraints
  - wrong handoff work-order id
  - vote proposal mismatch
  - vote SHA mismatch
  - too many board work-order refs
  - missing release/public-support denial
  - PASS/METAL without HIL evidence refs
  - malformed current task source

## Gate

```bash
just foundry-org-governance-g0
```

The gate validates `CurrentTaskV0`, renders packets, validates the happy path,
then runs negative validator cases.
