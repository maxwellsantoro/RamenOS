# BoardVoteV0

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

`BoardVoteV0` records role-specific approval, rejection, abstention, or blocking
over a proposal. Votes are evidence checks, not popularity signals.

## Required Fields

```json
{
  "schema_version": 1,
  "packet_kind": "board_vote_v0",
  "vote_id": "BV-2026-06-23-foundry-evidence-s12-4-1",
  "proposal_id": "WO-2026-06-23-s12-4-1-serial-observer",
  "repo_sha": "unknown",
  "role": "Foundry Evidence Officer",
  "vote": "approve",
  "claim_checked": "S12.4.1 scaffold is ready for implementation",
  "evidence": {
    "design_evidence_refs": [
      "schemas/org/work_order_v0.schema.json",
      "tools/org/validate_packets.py"
    ],
    "gate_evidence_refs": [
      "tools/ci/foundry_org_governance_g0.sh"
    ],
    "claim_evidence_refs": [
      "NEXT_TASKS.md"
    ],
    "hil_evidence_refs": [],
    "release_evidence_refs": []
  },
  "blocking_conditions": []
}
```

## Vote Values

- `approve`: the role's scoped requirement is satisfied.
- `reject`: the proposal should not proceed as written.
- `abstain`: the role has no scoped judgment.
- `block`: the proposal violates a veto domain and must not proceed.

## Validation Rules

- `block` requires at least one blocking condition.
- `approve` requires at least one typed evidence reference.
- `packet_kind` must be `board_vote_v0`.
- `repo_sha` must match the board packet, work order, and handoff.
- A role must not approve outside its authority domain.
- Public, release, metal, and security claims require the matching evidence role.
- Evidence refs are bucketed as design, gate, claim, HIL, and release evidence so
  later agents cannot confuse "schema exists" with "work succeeded".
- `PASS/METAL` claims require `hil_evidence_refs`.
