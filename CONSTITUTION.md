# CONSTITUTION

This document captures system invariants. If a change violates any item here,
it must be revised or explicitly superseded in DECISIONS.md.

## Non-Negotiables
- **Agents are First-Class Citizens:** The OS must expose state and affordances as structured, typed data (Semantic State API) designed for LLM ingestion. No screen-scraping or brittle CLI text parsing should be required for core system control.
- **Quantized AI Authority:** AI agents operate under strict, revocable Capability Budgets. They do not receive "ambient authority" or pseudo-root access. An agent can only affect the resources for which it holds explicit, unforgeable handles.
- **RamenOrg Uses the Same Doctrine:** Project agents operate through bounded work orders, handoffs, votes, evidence refs, and role capabilities. They do not receive ambient repo, merge, release, hardware, or public-claim authority.
- **Research-Backed, Not Research OS:** Doctrine-level novelty must produce research questions, claim boundaries, evidence plans, and implementation landing paths. Research is part of shipping correctly, not a license to remain experimental.
- **Request Authority Is Not Observable Authority:** Agent-facing and cross-domain service boundaries must distinguish `Lang` (what a holder may ask) from `ObsContract` (what a holder may learn). Topology hiding, request minimization, and cryptographic unforgeability must not be advertised as noninterference without measured evidence.
- Native interfaces are typed harnesses/portals. No ioctl-like escape hatches.
- POSIX is compatibility-only; it must not define native APIs.
- Capability validation for fast-path operations is kernel-side; brokers decide grants.
- Control plane uses typed messages; data plane is zero-copy shared memory.
- Preserve boundaries: kernel ≠ services ≠ store.
- Local execution is an optimization, not an ontology. Native execution is modeled as a capability-bounded request over artifacts, resources, domains, and replayable outputs.

## Development Model
- Build vertical slices. Every new capability ships with a minimal consumer and a Foundry gate.
- New interfaces must be defined in /idl and code-generated.

## Driver & AI Invariants
- **The Driver Dossier Rule:** A driver is not just source code; it is a living dossier. No driver component may be merged without a **Reference Vault** containing its ground-truth documentation, a reproducible fuzzing corpus, and a `protocol_trace` proving its conformance to a Harness.
- **The Oracle Rule:** Legacy operating systems (like Linux) are used as *behavioral oracles*. We do not blindly port C code; we trace the Oracle's hardware interactions in a Quarantine Domain, and distill those traces into native Rust components.
- **AI Translation, Not Dictation:** AI agents interact with the OS exclusively through the Semantic State API and typed IDL contracts. For user UX, LLMs act as *translators* for declarative policy constraints, never as the hidden source-of-truth for system configuration.
