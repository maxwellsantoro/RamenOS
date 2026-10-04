# Research-Backed RamenOrg Plan

**Last Updated:** 2026-10-03
**Status:** Maintained direction; G0 foundations and bounded trials landed

## Context

Two external drafts motivated the research questions below. They are design
inputs, not repository evidence of runtime behavior:

- The offers/airlock paper frames service boundaries as provider-authored
  offers with separate `Lang` and `ObsContract` objects, plus measured leakage
  for residual timing channels.
- The AI governance draft frames RamenOrg as an Org Kernel so agents coordinate
  through work orders, handoffs, votes, evidence, and heartbeats instead of
  using the founder as the transport layer.

The combined project doctrine is:

```text
RamenOS is a research-backed, post-Unix OS for humans and AI agents.
RamenOrg is the capability-governed organization building it.
Research is a production lane when novelty or risk makes guessing unsafe.
```

The [Vision](../../VISION.md) defines everyday use as the destination. RamenOrg
coordinates evidence-bearing progress toward human interaction, agent control,
modular drivers/services, hardware adaptability, and useful compatibility.

## Landed foundation

The G0 project-control foundation includes:

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

## Remaining decisions and research

Board packet rendering, work-order/handoff/vote validators, read-only steward,
freshness/context binding and bounded implementation/local-loop trials are landed
with named gates. Their stable plans and trial reports remain referenced by the
governance gate. They do not settle [RQ-0002](../research/questions/RQ-0002-ai-org-kernel.md).

Remaining work includes identity-level role separation, fresh isolated trial
reproduction, evidence-aware release policy, and explicit decisions before any
A3+/hardware/public-support authority. G0.8.1 permits bounded A2-local work only.
[RQ-0001](../research/questions/RQ-0001-offer-boundaries.md) still needs an
IDL/evidence landing plan before offer-shaped runtime changes. Research can
proceed independently; implementation priorities live in [Next Tasks](../../NEXT_TASKS.md).
