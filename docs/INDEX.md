# Documentation Index

**Last Updated:** 2026-06-24
**Status:** Active

This is the navigation hub for maintained documentation. Completed plans and
investigations are preserved under [archive](archive/README.md), where they are
historical and non-authoritative.

## Start Here

| Need | Document |
|------|----------|
| Project overview and first commands | [README](../README.md) |
| Landed state | [Current Status](../CURRENT_STATUS.md) |
| Next executable work | [Next Tasks](../NEXT_TASKS.md) |
| Medium-range direction | [Roadmap](../ROADMAP.md) |
| Slice definitions | [Vertical Slices](../SLICES.md) |
| Contributor setup | [Getting Started](GETTING_STARTED.md) and [Contributing](../CONTRIBUTING.md) |
| Terms and concepts | [Glossary](GLOSSARY.md) |

The operational source of truth is
[CURRENT_STATUS.md](../CURRENT_STATUS.md) plus
[NEXT_TASKS.md](../NEXT_TASKS.md). `ROADMAP.md` is directional.

## Architecture and Policy

- [Constitution](../CONSTITUTION.md): non-negotiable platform invariants.
- [Platform Overview](../PLATFORM_OVERVIEW.md): OS, Foundry, and Store shape.
- [Store Spec](../STORE_SPEC.md): package intelligence and launch-plan model.
- [Driver Capsule Spec](../DRIVER_CAPSULE_SPEC.md): quarantined legacy-driver boundary.
- [Hardware Strategy](HARDWARE_STRATEGY.md): Tier-1 and Golden Machine policy.
- [Evidence Levels](../EVIDENCE_LEVELS.md): allowed gate and hardware claims.
- [Decisions](../DECISIONS.md): ADR-lite decision log.
- [Risks](../RISKS.md): active risk register.
- [Security Status](../SECURITY_STATUS.md): current security posture and residual risk.

## Contracts and Artifact Formats

- [Evidence Policy V0](EVIDENCE_POLICY_V0.md)
- [Claim Artifact V0](CLAIM_V0.md)
- [Trace Artifact V0](TRACE_ARTIFACT_V0.md)
- [Observed Capabilities V0](OBSERVED_CAPS_V0.md)
- [Queue Item V0](QUEUE_ITEM_V0.md)
- [Compat Capsule V0](COMPAT_CAPSULE_V0.md)
- [Ring Buffer V0](RING_BUFFER_V0.md)
- [Multi-Domain Architecture](MULTI_DOMAIN.md)
- [HIL Appliance Evidence V0](HIL_APPLIANCE_EVIDENCE_V0.md)
- [`idl/`](../idl/): canonical typed interfaces and generated-binding inputs.

## Active and Gate-Bound Plans

These files remain under `docs/plans/` because they describe current work,
deferred design surfaces, or contracts consumed directly by Foundry gates.

### OS and Hardware

- [Semantic State substrate](plans/2026-02-20-s10-2-semantic-state-substrate.md)
- [Projection storage](plans/2026-02-20-s10-3-projection-storage.md)
- [Execution fabric](plans/2026-06-17-s10-4-execution-fabric.md)
- [Host-to-target integration](plans/2026-06-17-s10-5-host-to-target-integration.md)
- [Driver Factory MVP](plans/2026-02-20-s11-driver-factory-mvp.md)
- [Golden Machine](plans/2026-06-21-s12-golden-machine-design.md)
- [Persistent storage](plans/2026-06-21-s13-persistent-storage-design.md)
- [HIL Appliance Controller](plans/2026-06-22-hil-appliance-controller.md)

Completed S10 sub-plans are archived; parent architecture documents stay active.
Gate-bound S10.5 bridge plans remain beside their parent because Foundry checks
consume their stable paths.

### Security Operations

- [POSIX runner residual risks](plans/posix_runner_remaining_risks.md)
- [Security remediation program](plans/security_remediation_v006_v007_v012.md)
- [Store service IPC design](plans/v007_phase2_store_service_ipc_design.md)

These older documents remain active because gates, runtime warnings, or security
guidance still cite their stable paths.

## RamenOrg and Research

- [RamenOrg Constitution](org/ORG_CONSTITUTION.md)
- [Authority Levels](org/AUTHORITY_LEVELS.md)
- [Role Charter](org/ROLE_CHARTER.md)
- [Current Task contract](org/CURRENT_TASK_V0.md) and
  [machine-readable task](org/current_task.yaml)
- [Work Order](org/WORK_ORDER_V0.md), [Handoff Packet](org/HANDOFF_PACKET_V0.md),
  [Board Vote](org/BOARD_VOTE_V0.md), and [Board Packet](org/BOARD_PACKET_V0.md)
- [Board Brief](org/BOARD_BRIEF_V0.md), [Intake Bundle](org/INTAKE_BUNDLE_V0.md),
  and [Context Grant](org/CONTEXT_GRANT_V0.md)
- [Claim Safety](org/CLAIM_SAFETY.md) and [Heartbeats](org/HEARTBEATS.md)
- [Research index](research/INDEX.md) and [program](research/RESEARCH_PROGRAM.md)
- [RamenOrg plan](plans/2026-06-23-research-backed-ramenorg.md)

The G0 milestone plans and trial reports remain in place because the governance
gate validates their exact paths.

## Maintenance

- Update `CURRENT_STATUS.md` and `CHANGELOG.md` when a milestone lands.
- Update `NEXT_TASKS.md` when execution order changes.
- Move completed, non-gate-bound plans to `docs/archive/plans/`.
- Repair inbound links in the same change as any move.
- Do not duplicate current status in design docs; link to the authoritative pair.
