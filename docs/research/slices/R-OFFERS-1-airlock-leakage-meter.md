# R-OFFERS-1: Re-Timing Airlock, Offer-Key Redemption, and the Leakage Meter

**Last Updated:** 2026-06-23
**Status:** prototype-plan (research-bound slice; not yet implemented)
**Research track:** `R-OFFERS`
**Namespace:** research-bound (see `docs/research/SLICE_NAMESPACING.md`)

## Source

`~/Desktop/offers paper.md` — "From APIs to Offers: Provider-Authored
Capabilities, Re-Timing Airlocks, and Governed Leakage at the Service Boundary."
Research question: `docs/research/questions/RQ-0001-offer-boundaries.md`.

> Note: the paper originally labeled this prototype `S12.0`. That collides with
> the OS `S12.0` golden-machine contract, so it is renamed `R-OFFERS-1`.

## Scope

Implement the offer-shaped boundary doctrine around a narrow service or vault
operation (per RQ-0001 landing path), not a replacement of all IDL at once.

Deliverables (paper §13):

- Primitives: `InvitationKeyV0`, `OuterChamberV0`, `InnerChamberV0`, `PumpV0`,
  `RetrievalChamberV0`, `OfferKeyV0`, `RedemptionPacketV0`,
  `Execution/VaultOfferV0`, `AffordanceProjectionV0`, `ErrorMembraneV0`,
  `ObsContractV0`, `OutputProjectorV0`, `IdempotentReplayLedgerV0`,
  `LeakageMeterV0` (`rho`, `Q`, `eps*`, refresh trigger).
- Harness: chamber state machines; self-verifying offer keys; idempotent ledger;
  ObsContract enforcement; error membrane; baseline API comparison;
  non-enumerability tests; projection-hiding tests; attenuation-monotonicity
  tests; the **M1–M11 pump measurement harness**; a P1/P2 refresh verifier.

## requires_rq

- `RQ-0001` — offer-shaped service boundaries.

`RQ-0001` status is currently `Research question` (open). Per
research-blocks-implementation, **R-OFFERS-1 implementation is blocked until
RQ-0001 advances to `prototype-plan`/`measured`** and a service-boundary design
pass chooses concrete interfaces. This slice currently exists only as a plan and
claim boundary; no runtime code is claimed.

## Claim boundary (honest)

Following the paper §13.2 and `docs/org/CLAIM_SAFETY.md`:

- Claim **Level 1** (functional) and **Level 2** (topology hiding) once
  implemented and gated.
- Test **Level 3** (attenuation monotonicity, projection post-processing)
  partially.
- Claim **Level 4** (hidden-affordance noninterference) **only** for the modeled
  pump channel, and only as `L ≤ eps*` under the §10 control law — never as zero
  leakage, and never for unmodeled channels.

Until the `LeakageMeter` (M1–M11) lands, the headline `L ≤ eps*` is an assertion,
not a measurement, and must not be claimed as product behavior. The meter is the
artifact that makes the paper's central claim falsifiable.

## Landing path

doc/gate-first: design pass → IDL sketch → narrow vault-operation prototype →
M1–M11 measurement harness → Foundry gate. Does not displace the active S12.4/S13
metal track.
