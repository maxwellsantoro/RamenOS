# RamenOS Vision

**Last Updated:** 2026-10-08
**Status:** Product direction; implementation evidence lives in Current Status

RamenOS is a modern, post-Unix operating system being built for humans and AI
agents to use efficiently and safely. It aims to combine fast execution and
hardware adaptability with a modular architecture, where drivers and software
communicate through explicit contracts and failures are contained. Humans get
approachable interactions; agents get structured system state and narrowly
scoped, revocable permissions. Compatibility supports useful existing software
while leaving native design free to evolve beyond inherited Unix constraints.
Like the balance between noodles and broth, RamenOS seeks flexibility and
adaptability without sacrificing safety or ease of use.

## The product we are building

The destination is an everyday OS for both humans and agents. Human interaction,
agent interaction, hardware support, application execution, and developer
workflows are parts of the same product. The current agent-task experiment and
hardware bring-up are bounded steps toward that destination.

- **For humans:** a responsive desktop, understandable permissions, predictable
  application behavior, and recoverable failures. People remain the source of
  intent and policy; core interactions must remain usable without an AI model.
- **For agents:** permitted system state and available operations expressed as
  structured, typed data, with explicit authority for observations and actions.
  Core control should not require screen scraping or fragile command-output parsing.
- **For developers, human or agent:** independently develop and replace drivers
  and services behind versioned contracts. Foundry checks conformance, failure
  behavior, and effects across boundaries before broader integration.
- **For hardware support:** qualify concrete device profiles, use documented
  behavior and Oracle traces, and expand support through replaceable native
  components and bounded compatibility paths.
- **For existing software:** preserve useful compatibility while giving native
  software a path to RamenOS interfaces and narrower permissions.

## Architecture serving that product

Three pillars work together:

1. **OS Core:** a Rust-first kernel, services, and runtimes. The kernel supplies
   mechanisms and validates capabilities on fast paths; user-space brokers
   decide grants. Typed Harnesses and Portals carry control, while shared memory
   carries bulk data.
2. **Foundry:** the development and verification system for contract checks,
   trace replay, negative tests, and hardware qualification. It makes independent
   development reviewable through evidence about actual behavior.
3. **Store:** software discovery, permission previews, execution, and a guided
   path from compatibility to native ports, backed by artifacts and Foundry gates.

The intended architecture moves drivers and high-risk stacks into isolated
domains and keeps kernel, service, and Store responsibilities separate. This
reduces coupling and contains failures; it does not eliminate dependencies or
the need to test consumers when a component changes.

Post-Unix means native interfaces follow the needs of this product. POSIX and
legacy stacks remain compatibility tools rather than the blueprint for native
APIs. Useful existing techniques can stay when they serve the design.

## Independent evolution as a measurable goal

The three pillars aim to reduce the cost of safe change. A developer should be
able to improve a driver, service or application against a bounded contract;
Foundry supplies reusable qualification evidence, and Store helps discover and
activate suitable implementations under user policy. Contracts must cover
behavior, authority, resources, failure and recovery as well as message syntax.

Demonstrate this with a concrete replacement and recovery cycle while an unrelated
human task continues. Measure changes outside the component, qualification effort,
resource cost, disruption and recovery success. Independent evolution permits
explicit service interruption or boot-time activation when that is the supported
mode. Hardware failure boundaries additionally depend on real DMA/reset topology
and device evidence. These are goals, not established platform capabilities.

## What the vision requires us to demonstrate

Speed, safety, ease of use, hardware adaptability, and everyday readiness are
product goals. Each needs evidence before it becomes a statement of achieved
behavior: representative performance measurements, boundary and recovery tests,
human interaction validation, per-device qualification, and integrated tasks.
An isolated component can still affect latency or availability for its consumers.

RamenOS is currently public pre-alpha. Kernel/QEMU paths, host services, replay
tooling, and selected bridges exist; a complete target-native environment,
desktop, broad hardware support, and comparative agent benefits remain work.
The [Agent Task Proof](docs/plans/2026-09-16-agent-task-proof.md) tests part of
the agent proposition. It does not evaluate the entire human-facing product.

[Current Status](CURRENT_STATUS.md) records landed behavior and evidence.
[Next Tasks](NEXT_TASKS.md) owns execution order; [Roadmap](ROADMAP.md) connects
that work to the destination. [Constitution](CONSTITUTION.md) defines the design
invariants, and [Evidence Levels](EVIDENCE_LEVELS.md) bounds readiness claims.

## Describing RamenOS

Lead with an everyday post-Unix OS for humans and AI agents, then explain the
architecture and current stage. Keep the broader product visible when describing
an agent experiment, a driver gate, or a research result. Describe intended
behavior as a goal until the matching evidence exists. Historical plans and
trial reports retain their original scope and chronology.

**Short description:** A Rust-first, post-Unix OS for humans and AI agents,
designed for everyday use through explicit contracts, modular components,
and evidence-backed development. Currently public pre-alpha.
