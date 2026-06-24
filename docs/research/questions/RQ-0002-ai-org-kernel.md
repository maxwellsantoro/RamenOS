# RQ-0002: AI-Governed Org Kernel

**Last Updated:** 2026-06-23
**Status:** Research question / G0 scaffold

## Question

How can a capability-governed AI organization safely operate a large open-source
OS project, including planning, implementation, review, evidence, research,
release, and support, without using the founder as the manual message bus?

## Product Risk

RamenOS already uses agents heavily, but coordination can drift into copy/paste,
unstated memory, unsupported claims, and unclear authority. At OS scale, that is
an operational risk, not merely an inconvenience.

## Doctrine Under Review

RamenOrg should be an Org Kernel:

- Work orders bound scope and authority.
- Handoff packets move context between roles.
- Board votes check evidence from role-specific veto domains.
- Heartbeats emit artifacts and drift reports.
- Research is a first-class production lane.
- No same agent writes, approves, merges, and announces the same change.

## Claim Boundary

G0 does not create a fully autonomous organization. It establishes typed
governance artifacts, a drift gate, and a research-backed roadmap for staged
autonomy. Merge, release, hardware, and public support authority remain disabled
unless explicitly granted later.

## Required Outputs

- Org Kernel docs under `docs/org/`.
- Research program docs under `docs/research/`.
- `tools/org/status_drift.py`.
- `tools/ci/foundry_org_governance_g0.sh`.
- `BoardPacketV0` renderer and packet validators.

## Landing Path

G0 lands as a documentation and Foundry-gate slice. Later slices may add:

- Board packet generation.
- Work order and vote JSON validation.
- Handoff artifact storage under `out/org/`.
- Read-only automation heartbeats.
- PR/release/hardware authority only after explicit decisions and controls.
