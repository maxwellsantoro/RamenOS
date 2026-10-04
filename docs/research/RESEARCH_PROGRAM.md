# Research Program

**Last Updated:** 2026-10-03
**Status:** G0 scaffold

Research serves the [Vision](../../VISION.md) of an everyday, post-Unix OS for
humans and AI agents. It turns uncertainty about safety, performance, human
interaction, agent control, and hardware adaptability into evidence and
implementation requirements. Each question needs a concrete product landing path.

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
- Human permission comprehension, interaction, and recovery when the design
  introduces an unknown that affects usability or safety.
- Driver/service fault containment and consumer behavior across replacement.
- Performance or hardware-adaptability claims that need representative measurements.

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
