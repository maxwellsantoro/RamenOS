# Roadmap

**Last Updated:** 2026-06-24
**Status:** Directional

This document describes medium- and long-range sequencing. The authoritative
operational pair is [CURRENT_STATUS.md](CURRENT_STATUS.md) plus
[NEXT_TASKS.md](NEXT_TASKS.md).

## Now

1. Stabilize the S12.4.1 HIL appliance serial observer.
2. Add the S12.4.2 fail-safe power/reset actuator.
3. Graduate S13 storage on metal through appliance-mediated live capture.
4. Re-run S12 physical graduation through the same evidence loop.

The G0 Org Kernel and Research Office continue in parallel as a bounded
project-control track. They may not displace hardware execution or widen their
own authority.

## Next

### S14: Interactivity

- Select one USB xHCI controller profile from the Tier-1 machine.
- Capture an Oracle trace before writing native hardware interactions.
- Define typed USB/HID control messages and shared-memory data paths.
- Land keyboard input first, then pointer input, each with a replay gate.

### S15: Sane Desktop

- Native compositor consumes shared-memory surfaces.
- Input routes through typed HID and window-focus contracts.
- Compatibility domains export surfaces without becoming the native API model.
- Foundry gates cover frame delivery, focus, input routing, and recovery.

### Platform Follow-Ups

- Multi-source Semantic State aggregation and guest reactor loop.
- Real execution-fabric transport and broader kernel broker integration.
- Compat guest VFS read gate and scratch-to-commit flow.
- Store wizard orchestration over the landed semantic and projection layers.

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
| S11 | Complete virtio-net Driver Factory loop through runtime packet I/O |
| S12 | Golden-machine contract, GOP, HIL boot, IOMMU, and appliance scaffold |
| S13 | Persistent-storage contract, Oracle/replay loop, runtime block I/O, and metal gate scaffolds |

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
