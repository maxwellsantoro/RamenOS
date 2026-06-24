# Research Program

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

RamenOS should move quickly without breaking things. The path is not to skip
research, and not to turn the OS into an experimental toy. The path is to make
research operational.

## Principle

Doctrine-level unknowns get a research owner, a claim boundary, an evidence
plan, and an implementation landing path.

Research is required when a question affects:

- Capability or authority semantics.
- Agent-facing or cross-domain boundaries.
- Hardware evidence and HIL graduation.
- Driver distillation from Oracle traces.
- Semantic State as a machine-readable OS substrate.
- Execution Fabric scheduling or resource authority.
- RamenOrg autonomy, merge/release authority, or public claims.

## Production Loop

```text
research question
  -> prior-art map
  -> doctrine / model
  -> threat model or assumptions
  -> prototype or measurement harness
  -> implementation slice
  -> Foundry gate
  -> paper / essay / decision record
  -> product behavior
```

## Research Office

The Research Office is product-bound. It may block shallow implementation when a
problem is not understood well enough to support the claim being made, but it
must also keep every question attached to a landing path.

Responsibilities:

- Identify doctrine-level unknowns.
- Maintain research questions tied to slices and product risks.
- Produce prior-art packets, papers, essays, specs, and decisions.
- Convert research claims into implementation requirements.
- Define evidence needed before a public or internal claim is allowed.
- Prevent research from drifting away from shipping.

## Product Rule

RamenOS does not move fast by breaking things. It moves fast by making
uncertainty explicit, authority bounded, evidence machine-checkable, and
research operational.
