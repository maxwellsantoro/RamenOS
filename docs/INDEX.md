# Documentation Index

**Last Updated:** 2026-10-03
**Status:** Active

This is the navigation hub for maintained documentation. Completed plans and
investigations are preserved under [archive](archive/README.md), where they are
historical and non-authoritative.

The [Vision](../VISION.md) is an everyday, post-Unix OS for humans and AI agents.
Architecture, hardware, agent-task, desktop, Foundry, and Store documents describe
parts of that product, with implementation status and evidence recorded separately.

## Start Here

| Need | Document |
|------|----------|
| Product vision and description guidance | [Vision](../VISION.md) |
| Project overview and first commands | [README](../README.md) |
| Landed state | [Current Status](../CURRENT_STATUS.md) |
| Next executable work | [Next Tasks](../NEXT_TASKS.md) |
| Product destination and medium-range direction | [Roadmap](../ROADMAP.md) |
| Slice definitions | [Vertical Slices](../SLICES.md) |
| Contributor setup | [Getting Started](GETTING_STARTED.md) and [Contributing](../CONTRIBUTING.md) |
| Store examples, operator settings, and repository map | [Development Reference](DEVELOPMENT_REFERENCE.md) |
| Planned agent-task experiment | [Agent Task Proof](plans/2026-09-16-agent-task-proof.md) |
| Coding-agent workflow and hook limits | [Agentic Workflow](AGENTIC_WORKFLOW.md) |
| Terms and concepts | [Glossary](GLOSSARY.md) |

The operational source of truth is
[CURRENT_STATUS.md](../CURRENT_STATUS.md) plus
[NEXT_TASKS.md](../NEXT_TASKS.md). `ROADMAP.md` is directional.

## Architecture and Policy

- [Constitution](../CONSTITUTION.md): non-negotiable platform invariants.
- [Platform Overview](../PLATFORM_OVERVIEW.md): human and agent interfaces, modular OS, Foundry, and Store with Landed / Partial / Target architecture markers.
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
The [Current Status gate table](../CURRENT_STATUS.md#sw0-runnable-evidence-not-a-completed-experiment)
owns implemented SW0 scope and commands. Contract references, in milestone order:

| Step | Contract |
|------|----------|
| A0 | [Task semantics and fixtures](AGENT_TASK_CONTRACT_V0.md) |
| A1.0 | [Native protocol and preflight](AGENT_TASK_PROTOCOL_V1.md) |
| A1.1 | [Scripted RT service proof](AGENT_TASK_SERVICE_PROOF_V1.md) |
| A2.1 | [Linux scoped-shell foundation](AGENT_TASK_LINUX_CONTROL_V1.md) |
| A2.2 | [JSON adapter and RT bridge](AGENT_TASK_ADAPTER_V1.md) |
| A2.3 | [Independent LT backend](AGENT_TASK_LT_BACKEND_V1.md) |
| A2.4 | [LS transactions and launcher](AGENT_TASK_LS_TRANSACTIONS_V1.md) |
| A2.5 | [Typed subscription lifecycle](AGENT_TASK_SUBSCRIPTIONS_V2.md) |
| A2.6 | [Finite authority inventory](AGENT_TASK_AUTHORITY_V1.md) |
| A2.7 | [Scripted evaluator controls](AGENT_TASK_EVALUATOR_CONTROLS_V1.md) |
| A2.8 | [Named reconciliation](AGENT_TASK_RECONCILIATION_V1.md) |
| A2.9 | [Requestable rights and lifetime witnesses](AGENT_TASK_REQUESTABLE_AUTHORITY_V1.md) |

- [IDL Tools](../idl/tools/README.md): required IDs/types, regeneration, versioning.
- [Reference Vaults](../drivers/reference_vaults/README.md): driver context and Oracle evidence.

## Active and Gate-Bound Plans

These files remain under `docs/plans/` because they describe current work,
deferred design surfaces, or contracts consumed directly by Foundry gates.

### OS and Hardware

- [Agent Task Proof](plans/2026-09-16-agent-task-proof.md) — independent SW0 lane; three-arm controls, authority normalization, and pilot/powered comparison plan before S14; scripted foundations landed, model comparison pending
- [Semantic State substrate](plans/2026-02-20-s10-2-semantic-state-substrate.md)
- [Projection storage](plans/2026-02-20-s10-3-projection-storage.md)
- [Execution fabric](plans/2026-06-17-s10-4-execution-fabric.md)
- [Host-to-target integration](plans/2026-06-17-s10-5-host-to-target-integration.md),
  [host broker/proxy](plans/2026-06-17-s10-5-1-broker-kernel-bridge.md), and
  [QEMU serial IPC](plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md)
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

These stable paths now contain current references; the original analyses are
archived. Also read the [POSIX guide](../runtime_supervisor/POSIX_RUNNER_SECURITY.md)
and [vulnerability reporting policy](../SECURITY.md).

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
- [Implementer bot](org/RAMEN_IMPLEMENTER_BOT.md), [Merge Gate (A3)](org/MERGE_GATE_V0.md), and [Human Directive](org/HUMAN_DIRECTIVE_V0.md)
- [Research index](research/INDEX.md) and [program](research/RESEARCH_PROGRAM.md)
- [RamenOrg plan](plans/2026-06-23-research-backed-ramenorg.md)

The G0 milestone plans and trial reports remain in place because the governance
gate validates their exact paths.

## Maintenance and ownership

| Information | Maintained owner |
|-------------|------------------|
| Product framing and description guidance | [Vision](../VISION.md) |
| Stable contributor/agent invariants | [AGENTS.md](../AGENTS.md), [Constitution](../CONSTITUTION.md) |
| Landed behavior and evidence limits | [Current Status](../CURRENT_STATUS.md) |
| Next work, dependencies and acceptance | [Next Tasks](../NEXT_TASKS.md) |
| Direction and deferred choices | [Roadmap](../ROADMAP.md) |
| Milestone history / decision rationale | [Changelog](../CHANGELOG.md), [Decisions](../DECISIONS.md) |
| Exact native layout / artifact validation | IDL, schema source and named gates |
| Superseded plans and investigations | [Archive](archive/README.md) |

Update the relevant owner instead of copying its queue or history into another
reference. Design docs may record the implemented contract and its limits; proposed
behavior and commands must be labeled. Preserve gate-bound paths and original
trial evidence. When archiving an analysis behind a stable reference, repair its
links and add a historical banner. Recheck local links/anchors and recipe names
against `justfile`; run the required governance and affected Foundry gates.
