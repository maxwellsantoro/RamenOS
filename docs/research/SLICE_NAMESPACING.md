# Slice Namespacing

**Last Updated:** 2026-10-04
**Status:** Authoritative

RamenOS uses **three** slice namespaces; each can contain multiple parallel tasks.
Their identifiers must not collide. This document fixes the namespaces so a single slice id has
one unambiguous meaning across the OS, the research program, and the org.

## Namespaces

| Prefix | Track | Meaning | Example |
|--------|-------|---------|---------|
| `S##` / `S##.#` | OS execution | Boot, kernel, services, drivers, HIL, store | `S12.4`, `S13.7` |
| `R-<PROGRAM>-<n>` | Research-bound | A research output's implementation landing slice, gated by an RQ | `R-OFFERS-1` |
| `G#` / `G#.#` | Org / governance | RamenOrg kernel, packets, gates, authority | `G0.9` |

## Why this exists

The offers/airlock paper (external) called its first prototype slice
`S12.0: Re-Timing Airlock`. The repo's `S12.0` is the **golden-machine
contract** (First Metal). Two different `S12.0`s is exactly the drift an
AI-governed org must catch mechanically. The airlock prototype is therefore
`R-OFFERS-1`, not `S12.0`, and the status-drift checker rejects OS-style headings in the tracked
research-slice documents.

## Rules

1. A slice id belongs to exactly one namespace.
2. OS slices keep `S##`. Do not assign `S##` to research or org work.
3. A research-bound slice (`R-<PROGRAM>-<n>`) must bind to at least one research
   question via `requires_rq` and state a claim boundary per
   `docs/org/CLAIM_SAFETY.md`.
4. The drift checker ([status_drift.py](../../tools/org/status_drift.py)) checks
   required active references and research-slice headings. It is not a complete
   scan of every identifier in prose; review allocations and cross-references too.
5. Queue/lane labels and coordinator work-item IDs are scheduling identifiers,
   not new slices. Keep the owning slice and its gate/evidence boundary explicit;
   split work without allocating a new slice number for every agent.

## Current allocations

- OS: `S0`–`S15` (see [Slices](../../SLICES.md)); later numbers describe
  planned scope, not landed behavior.
- Org: `G0`–`G0.9` (see `docs/org/`).
- Research: `R-OFFERS-1` (airlock + leakage meter; see
  `docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md`).
