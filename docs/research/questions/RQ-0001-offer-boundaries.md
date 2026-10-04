# RQ-0001: Offer-Shaped Service Boundaries

**Last Updated:** 2026-10-04
**Status:** Research question

## Question

How should RamenOS expose agent-facing and cross-domain service boundaries so
consumers receive provider-authored capabilities instead of searchable API
topology, while observable leakage is measured rather than assumed away?

## Product Risk

Typed APIs and capability handles reduce ambient authority, but they can still
expose route topology, workflow states, validation differences, error classes,
timing, replay state, contention, and menu drift. Agentic consumers can optimize
against those signals.

## Doctrine Under Review

The external offers/airlock draft, summarized here, proposes:

- A single key-routed boundary verb: `present(key)`.
- Independent request authority and observable authority:
  - `Lang`: what the holder may ask.
  - `ObsContract`: what the holder may learn.
- Provider-authored offers rather than public catalogs.
- A re-timing airlock for state-dependent discovery.
- Error membranes and observable contracts for output safety.
- A pump/leakage meter and refresh control law for residual timing channels.

## Claim Boundary

This research may justify future service-boundary doctrine. It does not yet
claim that existing RamenOS services provide hidden-affordance noninterference.
Until measurement gates exist, safe claims are limited to design doctrine and
prototype plans.

## Required Outputs

- Prior-art packet covering ocap systems, membranes, DIFC, NRL Pump, QIF, and
  capability URLs/tokens.
- RamenOS-specific `OfferKeyV0`, `ObsContractV0`, and `ErrorMembraneV0` design
  sketch.
- Threat model for agentic consumers and cross-domain holders.
- Evaluation plan for topology hiding, projection monotonicity, and measured
  leakage.
- Foundry gate proposal for an initial offer-boundary prototype.

## Landing Path

Initial landing should be doc and gate-first:

- `docs/plans/<date>-offer-boundary-doctrine.md`
- `idl/...` only after the design pass chooses concrete interfaces.
- Prototype should wrap a narrow service or vault operation, not replace all
  existing IDL at once.

## Dependencies

Research and a concrete contract/gate proposal can proceed independently of
physical HIL. The [R-OFFERS-1 prototype](../slices/R-OFFERS-1-airlock-leakage-meter.md)
depends on this question supporting implementation and on a chosen service
boundary with usable source evidence. That dependency blocks offer-boundary
implementation, not unrelated OS work. The coordinator selects scope and capacity
from [Next Tasks](../../../NEXT_TASKS.md).
