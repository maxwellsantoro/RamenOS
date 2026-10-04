# Roadmap

**Last Updated:** 2026-10-03
**Status:** Directional

This document describes medium- and long-range sequencing. The authoritative
operational pair is [CURRENT_STATUS.md](CURRENT_STATUS.md) plus
[NEXT_TASKS.md](NEXT_TASKS.md).

## Destination: Everyday Use for Humans and AI Agents

The [Vision](VISION.md) is a modern, post-Unix OS that aims to combine fast
execution, hardware adaptability, safety, and ease of use. Humans get an
approachable desktop and explicit control over policy; agents get structured
state and scoped, revocable authority. Drivers and software evolve behind typed
contracts, with bounded failures and evidence about their effects on consumers.
Compatibility supports existing software while native interfaces remain free
to evolve.

The lanes below build toward that product. SW0 tests the agent interaction
model; H0–H3 establish physical evidence; S14/S15 develop human interactivity
and the desktop. Agent results alone do not qualify everyday readiness. This
direction preserves the current prerequisites and execution order.

## Now: Parallel Hardware and Software Lanes

These queue labels are independent lanes, not a single global priority list or
new slice numbers. SW0 continues independently; it does not wait for H3 or lab access.

### Physical lane: H0–H3

1. **H0:** first live S12.4.1 HIL appliance serial capture and observer validation.
2. **H1:** provision and validate S12.4.2 Intel AMT 11 power/reset actuation.
3. **H2:** run S12 physical work on the installed 240 GB SanDisk SATA SSD.
4. **H3:** add compatible M.2 2280 PCIe NVMe storage and graduate S13 on metal
   through appliance-mediated live capture of the implemented reboot/rollback
   protocol. A metadata-probe pass is insufficient; implement slot publication,
   readback/selection, recovery and the protocol verifier before graduation.

Physical runs await test-hardware setup. Software contracts/gates can proceed.

### Software lane: SW0 Agent Task Proof

- Before S14 expansion, integrate one useful task across intent, observation,
  scoped grants, artifact modification, validation execution, and evidence.
- A0/A1 foundations and bounded A2 controls are implemented; see
  [Current Status](CURRENT_STATUS.md) for their exact scope. Continue remaining
  host/deputy/unexercised and continuous authority coverage, with denied operations
  forced against the enforcement backend independently of model behavior.
- Complete Linux scoped shell and Linux typed controls against the RamenOS path,
  using the shared typed protocol and canonical authority manifest. Freeze real
  bank/study releases and provider/token accounting before comparative collection.
- Pilot the three-arm experiment within a predeclared funded ceiling, then
  freeze an affordable powered final comparison or report exploratory evidence.
  Separate interface effects, substrate effects, and the total proposition;
  report completion, authority, cost, and audit/replay claims individually.
- Add QEMU enforcement evidence per operation; keep host, simulation, and target
  behavior explicit. A host result does not establish a target-native runtime.

The [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md) defines
the fixture, gate assertions, comparison protocol, and landing sequence. This
software lane proceeds independently of the physical track.
Scripted host tasks and bounded control gates are executable; full A2 conformance,
model comparison, and target integration remain pending. No comparative advantage
is claimed. This proof evaluates one part of the broader product vision.

The G0 Org Kernel and Research Office continue in parallel as a bounded
project-control track. They may not displace either execution lane or widen their
own authority.

## Next: Expansion After Lane Prerequisites

### S14: Interactivity

**Product purpose:** establish reliable human input on the reference hardware
through the same explicit contracts used by native software.

- Require a stable H0/H1 appliance loop, SW0 A1/A2 evidence, and a recorded
  proceed/defer decision on the bounded Phase B report before implementation.
  An exploratory report may satisfy review with named uncertainty, never a
  powered claim. H2/H3 do not block SW0.
- Land the S14 design, IDL boundary, and Foundry gate definition first.
- Select one USB xHCI controller profile from the Tier-1 machine.
- Capture an Oracle trace before writing native hardware interactions.
- Define typed USB/HID control messages and shared-memory data paths.
- Land keyboard input first, then pointer input, each with a replay gate.

### S15: Sane Desktop

**Product purpose:** make the OS approachable for everyday human use, with
predictable interaction and visible control over applications and agent authority.

- Native compositor consumes shared-memory surfaces.
- Input routes through typed HID and window-focus contracts.
- Compatibility domains export surfaces without becoming the native API model.
- Foundry gates cover frame delivery, focus, input routing, and recovery.
- Define human permission, application-launch, and recovery flows without
  requiring an AI model; agent assistance uses explicit policy and grants.

### Platform Follow-Ups

- Multi-source Semantic State aggregation and guest reactor loop.
- Real execution-fabric transport and broader kernel broker integration.
- Compat guest VFS read gate and scratch-to-commit flow.
- Store wizard orchestration over the landed semantic and projection layers.

### Product Validation Across Future Slices

- Measure latency, throughput, and resource use on representative human and
  agent workflows before making speed claims.
- Exercise driver/service replacement and recovery with affected consumers;
  qualify hardware profiles individually before expanding support claims.
- Validate human interaction, permission comprehension, and recovery alongside
  bounded agent tasks. Define concrete consumers and gates before implementation.
- Demonstrate useful compatibility and a gated native migration path through
  the Store. Everyday readiness needs integrated evidence across these areas.

## Research and Governance

- **G0 Org Kernel:** keep work orders, handoffs, votes, context grants, and
  evidence refs machine-checkable.
- **Research Office:** mature RQ-0001 and RQ-0002 into bounded design inputs tied
  to product risks and implementation landing paths.
- **Authority staging:** do not advance above A2-local without explicit
  identity, credential, review, merge, release, and claim controls.
- **Service boundaries:** separate request authority (`Lang`) from observable
  authority (`ObsContract`) before implementing agent-facing offers.

## Landed Foundation

| Slice | Outcome |
|-------|---------|
| S0-S6 | Boot, IPC, Store, compatibility, portals, queues, and domain management |
| S7-S9 | GPU quarantine scaffold, shared memory, and phased security hardening |
| S10 | Native runner, Semantic State, projections, execution fabric, and QEMU IPC bridge |
| S11 | virtio-net Oracle/replay and embedded-vector harness validation; native device I/O remains |
| S12 | Golden-machine contract, GOP, HIL boot, IOMMU, and appliance scaffold |
| S13 | Storage contract, Oracle/replay, embedded-vector harness validation, and metal gate scaffolds |

See [SLICES.md](SLICES.md) for definitions and [CHANGELOG.md](CHANGELOG.md) for
chronology.

## Deferred Decisions

These require a short design document and a Foundry gate definition before
implementation.

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

Resolved choices belong in [DECISIONS.md](DECISIONS.md).
