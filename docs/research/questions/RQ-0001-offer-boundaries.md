# RQ-0001: Offer-Shaped Service Boundaries

**Last Updated:** 2026-06-23
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

The attached offers/airlock paper proposes:

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

Do not displace the active S12.4/S13 metal track. Use this question to prepare a
post-HIL service-boundary slice.
