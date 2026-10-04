# RISKS

**Last Updated:** 2026-10-04
**Status:** Active

These risks are evaluated against the [Vision](VISION.md) of an everyday,
post-Unix OS for humans and AI agents. Mitigations are plans or bounded controls;
landed behavior and evidence remain in [Current Status](CURRENT_STATUS.md).

## R1: Compatibility gravity (Linux becomes the "real OS")
Mitigation:
- Native shell + portals are first-class.
- Store UX always labels native level and offers a graduation path.
- Ports require Foundry gates and dossier artifacts.

## R2: Performance bottleneck in capability broker
Mitigation:
- Broker decides grants; kernel validates handles on fast path.
- Never require synchronous broker calls in hot-path data movement.

## R3: GPU latency makes the desktop unusable
Mitigation:
- Display Export Harness must meet latency budget.
- Roadmap: Quarantine → Native scanout → copy engines → scheduling.

## R4: Tooling sprawl (Foundry + Store too big too early)
Mitigation:
- Vertical slices only.
- Every subsystem must ship with a consumer + a gate.

## R5: Hardware support long tail
Mitigation:
- Foundry pipeline from day 1 (trace/replay/minimize).
- “Golden machine” Tier-1 contract (IOMMU-class) before broad targets.

## R6: Trace corpus privacy + size explosion
Mitigation:
- Default to protocol traces (typed harness transcripts) as the spec.
- Treat bus-level evidence as an opt-in, tiered artifact.
- Redact/sanitize traces before upload; support local-only retention.
- Enforce size caps and retention policies per artifact type.

## R7: Static resource limits in #![no_std] kernel
Severity: Medium (Denial of Service)
Confidence: High
Evidence:
- `CAP_TABLE_SIZE = 64` in `kernel/src/cap_table.rs` limits IPC capability handles
- `MAX_REGIONS = 16` in `kernel/src/shmem.rs` limits shared memory regions
- `MAX_FRAMES = 131072` (512 MiB) in `kernel/src/mm/bump.rs` limits frame allocator
- All use bounded static backing in the current no-heap kernel design
Mitigation:
- Keep capacity/exhaustion behavior explicit; increasing a bound requires memory and consumer validation
- S8 landed `FrameAllocator` plus reusable `BitmapAllocator` paths, but the
  kernel still relies on bounded static backing structures during bring-up.
- Current limits sufficient for early bring-up and controlled workloads
- Gates include resource exhaustion assertions (`table_exhaustion_returns_no_memory`)

## R8: Hardware evidence overclaim
Severity: High (incorrect readiness or security claim)
Confidence: High
Mitigation:
- Keep QEMU, replay, live HIL, appliance, and metal evidence levels distinct.
- Require target provenance markers for graduation mode.
- Treat appliance observations as controller evidence, not target truth.
- Keep physical gates opt-in and fail closed on stale logs.

## R9: Product scope narrows to the current agent experiment
Mitigation:
- Keep human interaction, the desktop, hardware adaptability, and useful software
  compatibility visible in product descriptions and directional planning.
- Treat SW0 as a bounded proof of the agent proposition, preserving S14/S15's
  human-facing purpose. A paid model study is not a dependency of input/desktop
  software work; retain the technical and evidence prerequisites in `NEXT_TASKS.md`.
- Evaluate human usability, performance, component recovery, and hardware profiles
  separately before claiming everyday readiness.

## R10: Modularity is mistaken for absence of dependencies
Mitigation:
- Define versioned driver/service contracts and explicit shared-resource limits.
- Check affected consumers, recovery, latency, and availability when replacing
  or changing a component; isolated execution alone does not prove containment.
- Qualify memory and DMA boundaries in the actual execution environment before
  making broad safety claims.

## R11: Parallel work creates integration debt
Mitigation:
- Start with a bounded set of dependency-ready packets, sized to available agents
  and validation capacity; prioritize the next integrated human task.
- Give shared contracts, generated output, build registration, and planning files
  one integration owner. Split consumer/implementation work only after the contract
  and failure assertions are agreed.
- Reserve shared gate outputs, ports, CAS state, and hardware; use isolated worktrees
  when file ownership alone cannot prevent collisions.
- Integrate and review small increments before dispatching more work. Measure
  completed gates, blocked time, and integration failures instead of agent count.
- Keep spending, physical actuation, approval, merge, and release authority separate
  from local delegation. See [Agentic Workflow](docs/AGENTIC_WORKFLOW.md).

## Residual security and concurrency risk

| Risk | Current boundary | Next evidence needed |
|------|------------------|----------------------|
| V-10: supervisor TCB breadth | Host policy and compatibility execution remain trusted | Explicit policy migration and affected-consumer gates |
| V-13: portal TOCTOU | Capability tokens reduce access risk; object lifetime/transaction guarantees remain incomplete | Broker lifetime and commit-boundary assertions |
| Multi-core transition | Frame/address-space backing uses locks; legacy capability/trace paths have SMP guards | Per-domain MMU barriers/TLB behavior and synchronized shared-memory control |
| POSIX host execution | Default-off, verified-artifact paths, best-effort Linux rlimits | Selected default containment profile with end-to-end enforcement evidence |

Trace publication-order fixes and helper tests do not establish whole-kernel
SMP safety. The shared-memory control table assumes a single control thread;
architecture-specific MMU operations still need multi-core qualification.

[Security Status](SECURITY_STATUS.md) owns current controls. Individual
remediation milestones and the former finding lists are retained in
[Changelog](CHANGELOG.md) and the [security archive](docs/archive/README.md).
Finding counts from old reviews are not a current risk score.
