# RamenOrg Constitution

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

RamenOrg is the project-control plane for RamenOS. Its job is to let agents
advance a serious OS project without turning the founder into the message bus or
turning agent autonomy into ambient authority.

RamenOrg is not a separate product and not a substitute for RamenOS engineering
discipline. It applies the same doctrine to the organization that RamenOS
applies to computation: typed boundaries, explicit capabilities, evidence
before claims, and replayable work.

## Mission

RamenOrg coordinates planning, implementation, review, evidence, research,
release, and support through bounded artifacts:

- `WorkOrderV0` for scoped tasks.
- `HandoffPacketV0` for moving work between roles.
- `BoardVoteV0` for approval, rejection, or blocking decisions.
- Evidence references for every completion claim.
- Status-drift checks for authoritative planning docs.

## Non-Negotiables

1. No ambient project authority. Agents receive explicit, revocable authority for
   a task, role, repository action, gate, hardware action, or public claim.
2. No undocumented handoff. Work moving between agents must carry context refs,
   claims, constraints, requested output, and required gates.
3. No unsupported claims. Completion, metal, release, security, research, and
   support claims must cite evidence at the level they assert.
4. No same-agent write, approve, merge, and announce path. Roles may cooperate,
   but authority domains must remain separated.
5. No hidden source of truth. `CURRENT_STATUS.md` plus `NEXT_TASKS.md` remain the
   operational truth for OS execution; board packets summarize them but do not
   override them.
6. No research theater. Research work must be tied to a product risk, claim
   boundary, implementation path, and evidence plan.
7. No hardware actuation without a HIL capability. Appliance commands require a
   bounded work order and must preserve target-emitted evidence requirements.

## Founder Role

The founder is the vision channel and escalation authority, not the routine
transport layer. Founder proposals are privileged inputs, but they still become
typed proposals, slices, decisions, or research questions before agents act.

## G0 Definition Of Done

G0 is complete when:

- The Org Kernel docs exist and are indexed.
- A status-drift checker validates the authoritative planning docs agree.
- A governance Foundry gate runs in normal CI-safe mode.
- `WorkOrderV0`, `HandoffPacketV0`, and `BoardVoteV0` have examples and
  validation rules.
- `BoardPacketV0` can summarize the active task and point at validated packet
  examples.
- `docs/org/current_task.yaml` drives packet rendering for the active task.
- Referenced packets are checked for cross-packet consistency.
- RamenOrg automation is explicitly staged and cannot merge, release, actuate
  hardware, or make public claims by default.
