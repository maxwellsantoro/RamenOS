# Documentation Index

**Last Updated:** 2026-10-04
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
| Ready work, dependencies and acceptance | [Next Tasks](../NEXT_TASKS.md) |
| Product destination and medium-range direction | [Roadmap](../ROADMAP.md) |
| Slice definitions | [Vertical Slices](../SLICES.md) |
| Contributor setup | [Getting Started](GETTING_STARTED.md) and [Contributing](../CONTRIBUTING.md) |
| Store examples, operator settings, and repository map | [Development Reference](DEVELOPMENT_REFERENCE.md) |
| Planned agent-task experiment | [Agent Task Proof](plans/2026-09-16-agent-task-proof.md) |
| Coordinator/sub-agent workflow and hook limits | [Agentic Workflow](AGENTIC_WORKFLOW.md) |
| Terms and concepts | [Glossary](GLOSSARY.md) |

The operational source of truth is
[CURRENT_STATUS.md](../CURRENT_STATUS.md) plus
[NEXT_TASKS.md](../NEXT_TASKS.md). `ROADMAP.md` is directional.

## Load context for the assigned task

Read [AGENTS.md](../AGENTS.md), the status/task pair and the applicable contract
first. The coordinator assigns a bounded outcome and file ownership using
[Agentic Workflow](AGENTIC_WORKFLOW.md); workers then load only the references
needed for that outcome. This index is a map, not a required reading list.

| Work | Additional starting context |
|------|-----------------------------|
| Native interfaces and target integration | [Constitution](../CONSTITUTION.md), [IDL Tools](../idl/tools/README.md), the relevant S10/S14/S15 contract and affected consumers |
| SW0 controls or evaluation | [Study plan](plans/2026-09-16-agent-task-proof.md), then the matching A0–A2.9 contracts below |
| Drivers, storage or HIL | [Evidence Levels](../EVIDENCE_LEVELS.md), the device [Reference Vault](../drivers/reference_vaults/README.md), and the relevant S11/S12/S13 plan |
| Store or compatibility | [Store Spec](../STORE_SPEC.md), [Development Reference](DEVELOPMENT_REFERENCE.md), the affected artifact/runner contract |
| Governance or research | The relevant [RamenOrg contract](#ramenorg-and-research) or [research question](research/INDEX.md) |

Host/replay/QEMU preparation and physical qualification have separate prerequisites.
Use the queue's dependency edges; a historical phase number or a blocked hardware
run does not impose a global stop on independent software work.

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

## Maintained Plans and Contract References

`docs/plans/` contains both executable plans and maintained architecture
references. A file's presence here does not put every remaining idea on the ready
queue. [Next Tasks](../NEXT_TASKS.md) selects work and records its dependencies.

### OS and Hardware

- [Agent Task Proof](plans/2026-09-16-agent-task-proof.md) — independent SW0 lane; three-arm controls, authority normalization and bounded comparison; scripted foundations landed, model comparison pending
- [Desktop v0](plans/desktop-v0.md) — accepted bounded human task and runtime/input/surface/launch/recovery design; first host launch/lifetime consumer implemented; editor/device/target joins pending
- [Desktop session v1](DESKTOP_SESSION_V1.md) — generated protocol-336 preview/confirmation/lifetime contract and passing default-off Unix host process/wire gate
- [Boot frame ownership v0](BOOT_FRAME_OWNERSHIP_V0.md) — reviewed pure map/retention admission and passing 17-case gate; actual firmware-exit adapter, initial-access evidence and QEMU allocator proof pending
- [Desktop editor v0](plans/desktop-editor-v0.md) — reviewed UI1.1 proposal with bounded host/Store/task/process packets; volatile editor and host Store transaction implemented, integrated task/process successors pending
- [Desktop editor wire v1](DESKTOP_EDITOR_WIRE_V1.md) — five registered canonical IDLs and 43 generated messages consumed by the passing volatile host editor gate
- [Desktop editor host API v0](DESKTOP_EDITOR_HOST_API_V0.md) — implemented default-off opaque peers/leases, logical keyboard editing, offscreen composition and 13-case gate; no Store/device/target/process-containment evidence
- [Desktop editor Store transaction v0](plans/editor-store-transaction-v0.md) — implemented default-off host CAS, atomic selection/receipt and joined-writer recovery; seven behavior cases pass on macOS/Linux, integrated editor/device/target joins remain separate
- [Desktop editor Store API v0](DESKTOP_EDITOR_STORE_API_V0.md) — frozen pure/service API consumed by the passing nine-case schema and seven-case actual host Store gates; opaque authority, admission and original-operation recovery are exercised within the trusted fixture scope
- [Editor Store recording contract v0](contracts/editor-store-recording-v0.json) — closed source/provenance/lease/fence/cleanup shapes and explicit temporal receipt correction consumed by the sixteen-case evidence gate and native Rust decoder
- [Desktop editor native Read API v0](DESKTOP_EDITOR_NATIVE_READ_API_V0.md) and [bounded contract](contracts/editor-native-read-v0.json) — reviewed default-off approved-peer/Store Read prerequisite, original deadlines, live copy checks and owned joins; nine assertions and handlers pending, with full UI1.1c Save/task acceptance separate
- [External boot-profile Oracle v0](plans/boot-profile-oracle-v0.md) — reviewed CPU inspection preparation, bounded actual EFI checkpoints and fixed claim limits; relocated-entry resolution and frozen executable capture still pending
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
gate validates their exact paths. Their historical banners bound their authority;
they are not new work. `current_task.yaml` supplies one retained packet set, not
the coordinator's multi-task queue. See [CurrentTaskV0](org/CURRENT_TASK_V0.md).

## Maintenance and ownership

| Information | Maintained owner |
|-------------|------------------|
| Product framing and description guidance | [Vision](../VISION.md) |
| Stable contributor/agent invariants | [AGENTS.md](../AGENTS.md), [Constitution](../CONSTITUTION.md) |
| Landed behavior and evidence limits | [Current Status](../CURRENT_STATUS.md) |
| Ready work, dependencies and acceptance | [Next Tasks](../NEXT_TASKS.md) |
| Coordinator dispatch, ownership and integration | [Agentic Workflow](AGENTIC_WORKFLOW.md) |
| Direction and deferred choices | [Roadmap](../ROADMAP.md) |
| Milestone history / decision rationale | [Changelog](../CHANGELOG.md), [Decisions](../DECISIONS.md) |
| Exact native layout / artifact validation | IDL, schema source and named gates |
| Superseded plans and investigations | [Archive](archive/README.md) |

Update the relevant owner instead of copying its queue or history into another
reference. The coordinator integrates shared status/queue/history edits once per
landed unit; workers return evidence and proposed deltas. Design docs record the
contract and limits; label proposed behavior and commands. Archive superseded
analysis under the [archive policy](archive/README.md), preserving gate-bound
paths and original trial evidence. Recheck local links/anchors and recipe names
against `justfile`; run the required governance and affected Foundry gates.
