# Slice Namespacing

**Last Updated:** 2026-06-23
**Status:** Authoritative

RamenOS now runs **three** parallel work tracks, and their slice identifiers
must not collide. This document fixes the namespaces so a single slice id has
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
`R-OFFERS-1`, not `S12.0`, and the status-drift checker enforces that no
research slice id collides with an OS slice id.

## Rules

1. A slice id belongs to exactly one namespace.
2. OS slices keep `S##`. Do not assign `S##` to research or org work.
3. A research-bound slice (`R-<PROGRAM>-<n>`) must bind to at least one research
   question via `requires_rq` and state a claim boundary per
   `docs/org/CLAIM_SAFETY.md`.
4. The drift checker (`tools/org/status_drift.py`) scans docs for slice ids and
   fails on cross-namespace collisions and on a research slice reusing an OS
   number.

## Current allocations

- OS: `S0`–`S14` (see `CURRENT_STATUS.md`, `SLICES.md`).
- Org: `G0`–`G0.9` (see `docs/org/`).
- Research: `R-OFFERS-1` (airlock + leakage meter; see
  `docs/research/slices/R-OFFERS-1-airlock-leakage-meter.md`).
