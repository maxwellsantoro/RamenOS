# Research-Backed RamenOrg Plan

**Last Updated:** 2026-06-23
**Status:** G0 scaffold / planning track

## Context

Two attached drafts introduce planning changes that belong in the project, not
only in chat:

- The offers/airlock paper frames service boundaries as provider-authored
  offers with separate `Lang` and `ObsContract` objects, plus measured leakage
  for residual timing channels.
- The AI governance draft frames RamenOrg as an Org Kernel so agents coordinate
  through work orders, handoffs, votes, evidence, and heartbeats instead of
  using the founder as the transport layer.

The combined project doctrine is:

```text
RamenOS is a research-backed, agent-native OS.
RamenOrg is the capability-governed organization building it.
Research is a production lane when novelty or risk makes guessing unsafe.
```

## Scope Now

This plan adds a G0 project-control slice:

- Define Org Kernel docs in `docs/org/`.
- Define research-backed development docs in `docs/research/`.
- Add RQ-0001 for offer-shaped service boundaries.
- Add RQ-0002 for the AI-governed Org Kernel.
- Add a status-drift checker and governance Foundry gate.
- Sync `AGENTS.md`, `CURRENT_STATUS.md`, `NEXT_TASKS.md`, and `ROADMAP.md`.

## Non-Scope

G0 does not:

- Replace the active S12.4/S13 HIL execution track.
- Grant agents merge, release, hardware, or public support authority.
- Implement the offer-boundary runtime.
- Claim hidden-affordance noninterference for existing services.
- Treat research papers as evidence for product behavior without gates.

## G0 Definition Of Done

1. `docs/org/ORG_CONSTITUTION.md`, role, authority, heartbeat, work order,
   handoff, vote, and claim-safety docs exist.
2. `docs/research/RESEARCH_PROGRAM.md` and current research questions exist.
3. `tools/org/status_drift.py` checks that active planning docs agree.
4. `tools/ci/foundry_org_governance_g0.sh` runs in CI-safe mode.
5. `NEXT_TASKS.md` tracks G0 as a parallel planning/control track.
6. `CHANGELOG.md` records the scaffold.

## Future Work

- `BoardPacketV0` schema and renderer.
- JSON validators for work orders, handoffs, and votes.
- Read-only daily steward heartbeat.
- Evidence-aware release packet generation.
- Offer-boundary design pass after the HIL appliance loop is stable.
- Staged autonomy above A1 only after explicit decisions and controls.
