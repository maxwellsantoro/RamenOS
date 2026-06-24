# MergeGateV0 (A3 conditional merge)

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

`MergeGateV0` is the A3 gate: the set of preconditions an agent work product
must satisfy before it may merge. It is the closure of the work loop
(`WorkOrderV0` → implementer → reviewer → gates → evidence → board vote → merge)
and the first place RamenOrg enforces separation of duties, evidence-bearing
votes, and research-blocks-implementation on real work.

## A3 preconditions (all required)

A `MergeRequestV0` passes only when every condition holds:

1. **Separation of duties.** `implementer_role != reviewer_role`. No agent writes
   and approves the same change (Constitution: no same-agent write, approve,
   merge, and announce path).
2. **Evidence-bearing board vote.** The referenced `BoardVoteV0` is `approve`
   with at least one typed evidence ref, and `vote.proposal_id` equals
   `work_order.work_order_id`.
3. **Required gates green.** Every gate in `required_gates` exists and is
   reported `PASS` in `gate_results`. (The governance gate itself must be among
   them.)
4. **Research blocks implementation.** If `requires_rq` is present, each named
   research question must *support implementation* (status advanced past open
   research). If `doctrine_area` is set, `requires_rq` must be non-empty and
   satisfied. An open research question blocks the merge by construction.
5. **Claim boundary preserved.** The merge request's `claim_boundary` keeps the
   A3 denials (no release, no hardware actuation, no public support) and, while
   no remote branch protection/credentials are configured, records the honest
   outcome as `PASS/LOOP-LOCAL` — never `PASS/MERGE`.

## Honest outcome: LOOP-LOCAL vs MERGE

A real `PASS/MERGE` requires GitHub branch protection (required reviews, status
checks, linear history) plus merge credentials — both are deployment settings
outside this repo. Until an explicit A3-authority decision configures them, the
gate proves the **loop machinery** on a real change and records `PASS/LOOP-LOCAL`:
the writer→reviewer→vote→precondition chain validated locally, with the remote
merge explicitly deferred. This is the org equivalent of `PASS/QEMU` vs
`PASS/METAL`: do not claim a remote merge that did not happen.

## Validation

`tools/org/validate_merge.py` checks all preconditions against a
`MergeRequestV0` and its referenced packets. Negative cases live in
`tools/org/test_validate_merge.py`.
