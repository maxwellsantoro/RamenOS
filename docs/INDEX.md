# Documentation Index

**Last Updated:** 2026-06-23
**Status:** Active

## Authoritative Runtime Docs
- `AGENTS.md` - contributor/agent operating constraints and active execution track.
- `CURRENT_STATUS.md` - latest milestone state and validated completion evidence.
- `NEXT_TASKS.md` - execution backlog and near-term implementation sequence.
- `ROADMAP.md` - slice-level sequencing and medium/long-range priorities.
- `docs/AGENTIC_WORKFLOW.md` - Guidelines for building the OS with AI coding agents.
- `docs/org/ORG_CONSTITUTION.md` - RamenOrg governance invariants and G0 definition of done.
- `docs/research/INDEX.md` - research-backed OS program index.
- `RISKS.md` - active risk register and mitigations.
- `DECISIONS.md` - ADR-lite decision log.
- `CHANGELOG.md` - release and unreleased change history.

## Architecture and Contracts
- `CONSTITUTION.md` - non-negotiable platform invariants.
- `PLATFORM_OVERVIEW.md` - high-level architecture.
- `SLICES.md` - slice definitions.
- `STORE_SPEC.md` - store platform specification.
- `DRIVER_CAPSULE_SPEC.md` - capsule boundary specification.
- `docs/HARDWARE_STRATEGY.md` - Tiered hardware and Golden Machine policy.
- `idl/` - typed IDL contracts used by codegen and runtime boundaries.

## Active Plans
- `docs/plans/2026-02-20-s10-3-projection-storage.md` - S10.3 Projection Storage (phases S10.3.0-S10.3.4 complete).
- `docs/plans/2026-06-17-s10-3-1-projection-index-backend.md` - S10.3.1 projection index backend decision and gate.
- `docs/plans/2026-06-17-s10-3-3-read-only-vfs-projection.md` - S10.3.3 read-only VFS projection (virtio-9p decision + gate).
- `docs/plans/2026-06-17-s10-3-4-cow-projection-writes.md` - S10.3.4 CoW projection writes (complete).
- `docs/plans/2026-06-17-s10-4-execution-fabric.md` - S10.4 Execution Fabric (v0 scaffold + S10.4.1 wiring complete).
- `docs/plans/2026-06-17-s10-2-1-subscribe-reactor.md` - S10.2.1 subscribe reactor (complete).
- `docs/plans/2026-06-17-s10-2-v1-1-cap-filtered-snapshots.md` - S10.2 v1.1 capability-filtered snapshots (complete).
- `docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md` - S10.5.1 broker/proxy harness bridge (complete).
- `docs/plans/2026-06-17-s10-5-2-qemu-ipc-bridge.md` - S10.5.2 QEMU IPC bridge (complete).
- `docs/plans/2026-06-17-s10-5-host-to-target-integration.md` - S10.5 host→QEMU integration (S10.5.0-S10.5.2 complete).
- `docs/plans/2026-02-20-s11-driver-factory-mvp.md` - S11 Driver Factory MVP (complete).
- `docs/plans/2026-06-21-s12-golden-machine-design.md` - S12 First Metal / golden machine (S12.0 scaffold complete).
- `docs/plans/2026-06-21-s13-persistent-storage-design.md` - S13 Persistent Storage (S13.0 contract scaffold).
- `docs/plans/2026-06-22-hil-appliance-controller.md` - S12.4 / S13.9 HIL Appliance Controller scaffold.
- `docs/plans/2026-06-23-research-backed-ramenorg.md` - G0 RamenOrg / research-backed OS planning track.
- `docs/plans/2026-06-23-g0-1-board-packet-validators.md` - G0.1 board packet and packet-validator scaffold.
- `docs/plans/2026-06-23-g0-2-active-task-cross-packet.md` - G0.2 active-task source and cross-packet consistency scaffold.
- `docs/plans/2026-06-23-g0-3-current-task-negative-fixtures.md` - G0.3 current-task schema, vote SHA binding, and negative validator cases.
- `docs/plans/2026-06-23-g0-3-1-governance-label-claim-boundary.md` - G0.3.1 governance label and claim-boundary hygiene.
- `docs/plans/2026-06-23-g0-4-read-only-steward-heartbeat.md` - G0.4 read-only steward heartbeat and board brief.
- `docs/plans/2026-06-23-g0-5-agent-intake-freshness-binding.md` - G0.5 hash-bound agent intake bundle.
- `docs/plans/2026-06-23-g0-6-intake-only-agent-trial.md` - G0.6 fresh-agent intake-only usage trial.
- `docs/plans/2026-06-23-g0-7-bounded-context-grant.md` - G0.7 bounded context grant and patch-plan trial.
- `docs/plans/2026-06-23-g0-8-bounded-implementation-trial.md` - G0.8 bounded implementation trial for S12.4.1.
- `docs/plans/2026-06-23-g0-8-1-implementation-authority-serial-claim-hygiene.md` - G0.8.1 authority and serial evidence hygiene.

## RamenOrg and Research
- `docs/org/ROLE_CHARTER.md` - role authority and veto domains.
- `docs/org/AUTHORITY_LEVELS.md` - staged autonomy levels A0-A6.
- `docs/org/current_task.yaml` - machine-readable active task for packet rendering.
- `docs/org/CURRENT_TASK_V0.md` - current-task source contract.
- `docs/org/HEARTBEATS.md` - recurring artifact-producing loops.
- `docs/org/WORK_ORDER_V0.md` - bounded task packet schema.
- `docs/org/HANDOFF_PACKET_V0.md` - agent-to-agent handoff schema.
- `docs/org/BOARD_VOTE_V0.md` - evidence-backed vote schema.
- `docs/org/BOARD_PACKET_V0.md` - read-only board packet schema.
- `docs/org/BOARD_BRIEF_V0.md` - generated human-readable board brief contract.
- `docs/org/INTAKE_BUNDLE_V0.md` - hash-bound agent intake bundle and manifest contract.
- `docs/org/CONTEXT_GRANT_V0.md` - bounded context grant contract.
- `docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md` - isolated agent trial evidence and findings.
- `docs/org/trials/2026-06-23-g0-7-bounded-context-patch-plan.md` - bounded context patch-plan trial evidence and findings.
- `docs/org/trials/2026-06-23-g0-8-bounded-implementation-trial.md` - bounded implementation trial evidence and findings.
- `docs/org/trials/2026-06-23-g0-8-1-authority-serial-claim-hygiene.md` - G0.8.1 authority and serial evidence hygiene trial.
- `docs/org/CLAIM_SAFETY.md` - claim-level discipline for org and research outputs.
- `docs/research/RESEARCH_PROGRAM.md` - operational model for research-backed development.
- `docs/research/questions/RQ-0001-offer-boundaries.md` - offer-shaped service boundary research question.
- `docs/research/questions/RQ-0002-ai-org-kernel.md` - AI-governed Org Kernel research question.

## Operations and Security
- `SECURITY_STATUS.md` - security posture summary and remediation status.
- `runtime_supervisor/POSIX_RUNNER_SECURITY.md` - POSIX runner security model and controls.
- `docs/EVIDENCE_POLICY_V0.md` - evidence policy contract.
- `docs/LINT_DEBT.md` - lint governance and debt tracking.

## Historical / Archived Material
- `docs/archive/README.md` - archive policy for tracked documentation.
- `docs/archive/plans/` - superseded plans and historical design/investigation documents.

## Usage Notes
- Prefer files in this index for planning and implementation decisions.
