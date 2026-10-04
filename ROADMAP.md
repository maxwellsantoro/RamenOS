# Roadmap

**Last Updated:** 2026-10-03
**Status:** Directional; prerequisites are requirements, not readiness claims

[VISION.md](VISION.md) defines the destination: an everyday, post-Unix OS for
humans and AI agents, with approachable interaction, fast execution, adaptable
hardware support, explicit authority, and recoverable failures. Compatibility
keeps existing software useful while native interfaces can evolve.

[CURRENT_STATUS.md](CURRENT_STATUS.md) owns landed behavior and evidence.
[NEXT_TASKS.md](NEXT_TASKS.md) owns execution order and acceptance criteria.
This roadmap connects that work to the product; milestone history belongs in
[CHANGELOG.md](CHANGELOG.md) and definitions in [SLICES.md](SLICES.md).

## Current direction: independent physical and software lanes

| Lane | Purpose | Progression |
|------|---------|-------------|
| H0–H3 physical work | Make hardware runs repeatable and qualify the reference machine | S12.4 live serial observation → AMT power/reset → S12 on SATA → S13 NVMe boot and verified update/rollback |
| SW0 Agent Task Proof | Evaluate structured interaction and the implemented substrate on one useful task | Scripted host proof → complete comparison controls → bounded three-arm study → named target-enforcement evidence |
| G0 Org Kernel / Research Office | Keep planning, authority, and evidence reviewable | Validated packets and bounded trials → research inputs and stronger identity separation |

SW0 continues independently of H0–H3 and lab access. Physical runs await setup;
software contracts and fault/recovery gates can proceed. H0–H3 and SW0 are queue
labels, not new slice numbers. Governance cannot widen its own authority or
displace OS execution.

The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) compares
Linux scoped shell, Linux typed, and RamenOS typed workflows for a scoped
configuration repair. Controls must distinguish interface effects from substrate
effects, retain unknown authority, force denials against the backend, and freeze
bank/study/provider controls before model collection. The pilot determines whether
an affordable powered comparison fits the predeclared ceiling; otherwise publish
exploratory evidence with uncertainty. A tie or regression is a valid result.
This study evaluates the agent interface; it does not qualify the whole OS.

S13 hardware graduation requires publication/readback, revisioned boot selection,
a new-slot boot and a separate rollback/recovery boot with fresh provenance.
The current metadata scaffold cannot supply that evidence. Firmware NVMe boot
and embedded Oracle-vector transfers do not establish native device-backed I/O.
See [the S13 design](docs/plans/2026-06-21-s13-persistent-storage-design.md).

## Expansion: human input and desktop

### S14: USB xHCI and HID

Establish reliable keyboard input, then pointer input, on one reference controller.
Use typed USB/HID control contracts, shared-memory data, and Oracle/replay evidence.

Implementation requires a stable H0/H1 appliance loop, reviewed SW0 A1/A2 evidence,
and a recorded proceed/defer decision on the bounded Phase B report. An exploratory
report may satisfy review with explicit uncertainty; a positive RamenOS advantage
is not required. S14 also needs its own design, Reference Vault, Oracle trace,
IDL boundary, and Foundry gate definition. H2/H3 do not block SW0, and Phase C target
enforcement remains a separate follow-up. Exact prerequisites live in `NEXT_TASKS.md`.

### S15: everyday desktop foundations

- A native compositor consumes shared-memory surfaces.
- Typed contracts route input, focus, and application surfaces.
- Compatibility domains export surfaces through the native boundary.
- Human application-launch, permission, and recovery flows work without an AI model.
- Foundry gates check frame delivery, focus, input routing, and failure recovery.

Define concrete human tasks and usability checks alongside component gates before
claiming a usable desktop. Optional agent assistance follows explicit policy and grants.

## Integration and product validation

| Area | Required next design or evidence |
|------|---------------------------------|
| Target runtime and services | Target loader/runtime, live Semantic State aggregation/reactor, and named broker/kernel enforcement paths |
| Native drivers | Device-backed net/block transfers, recorded Oracle comparisons, DMA boundaries, and per-device qualification |
| Compatibility | Guest VFS read evidence, controlled scratch-to-commit flow, and useful application scenarios |
| Store | Permission previews, launch/recovery flows, and wizard orchestration over semantic/projection contracts |
| Execution fabric | Real transport, failure/recovery model, and resource authority beyond simulation |
| Performance | Representative latency, throughput, and resource measurements for human and agent tasks |
| Modularity | Replacement, recovery, latency, and availability checks with affected consumers |

Each follow-up needs a consumer, a bounded contract, and a gate. Everyday readiness
requires integrated human interaction, useful software, hardware, recovery, and
performance evidence; individual host or QEMU gates support only their stated scope.

## Research and governance

The **G0 Org Kernel** and **Research Office** prepare bounded inputs without
granting merge, release, hardware, or public-support authority. RQ-0001 investigates
offer-shaped service boundaries and measurable observation limits; RQ-0002
investigates capability-governed project operation. See [Research](docs/research/INDEX.md).
Request authority (`Lang`) and observable authority (`ObsContract`) remain distinct.
Advancement above A2-local requires explicit identity, credential, independent
review, merge, release, and claim controls.

## Deferred Decisions

These require a short design and a Foundry gate definition before implementation.
The task queue records executable prerequisites; resolved choices belong in
[DECISIONS.md](DECISIONS.md).

| Topic | Required decision |
|-------|-------------------|
| Vector or graph semantic search | Evidence that path/tag projections are insufficient |
| S5.1 wizard orchestration | Concrete consumer flow over semantic index and subscriptions |
| Real execution-fabric transport | Transport boundary, failure model, and kernel validation path |
| Broad kernel broker migration | Scope beyond the proven one-harness bridge |
| V-10 supervisor TCB reduction | Kernel policy ownership and migration plan |
| V-13 portal TOCTOU | Broker transaction and object-lifetime model |
| Offer-shaped runtime boundary | RQ-0001 result, IDL, observable contract, and gate |
| RamenOrg authority above A2-local | Explicit controls, separated approval, and evidence policy |
