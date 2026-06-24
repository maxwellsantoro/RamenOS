# S10.4: Capability-Scheduled Execution Fabric

**Last Updated:** 2026-06-24
**Status:** Reference design; v0 scaffold and S10.4.1 wiring complete
**Related:** CONSTITUTION.md, PLATFORM_OVERVIEW.md, ROADMAP.md, SLICES.md

## Executive Summary

S10.4 defines the contract layer for capability-scheduled execution without committing RamenOS to a distributed scheduler implementation yet.

The core decision is:

```text
Compute Fabric is not a runner.
Compute Fabric is the policy/control service that grants resources, selects a runner/domain/node, and records replayable execution outcomes.
```

The first S10.4 milestone should be contract/readiness work:
- canonical execution and launch-plan schemas in `artifact_store_schema`,
- an `execution_fabric_v1` service IDL,
- Semantic State visibility for nodes, leases, and executions,
- launch-plan alignment between `store_cli` and `runtime_supervisor`,
- a simulation-only Foundry gate.

No SSH, containers, remote workers, or real distributed execution belong in S10.4 v0.

## 1. Motivation

Current launch plans are useful but not yet canonical. `store_cli` and `runtime_supervisor` have historically carried similar local plan structures for runners such as `linux_vm_v0`, `gpu_quarantine_v1`, deprecated POSIX, and `native_wasm_v0`.

That was reasonable for earlier slices. It becomes the pressure point for execution-fabric work because scheduling policy, resource leases, and trace emission need one durable artifact shape rather than duplicated ad hoc JSON structs.

S10.4 makes execution a first-class RamenOS contract:

```text
subject + artifact state + requested action + resource lease
+ capability bundle + runner/domain selection
+ output contract + execution trace
```

Local execution remains an optimization, not the ontology.

## 2. Non-Goals for v0

- No real remote execution.
- No SSH, Nomad, Kubernetes, Ray, or container orchestration integration.
- No scheduler logic inside `domain_manager_v1`.
- No widening of existing protocol/scenario trace artifacts to hold execution lifecycle traces.
- No kernel policy expansion beyond existing typed control-plane and capability rules.

## 3. Contract Surface

### 3.1 IDL

Add a service contract:

```text
idl/services/execution_fabric_v1.toml
```

The IDL should be thin and artifact-based. Rich scheduler state moves through content references, not nested wire objects:

```toml
namespace = "services.execution_fabric"
version = "1"
protocol = 11

[message.register_node]
msg_type = 1
fields = ["request_id:u64", "node_manifest_ref:string", "capability_bytes:bytes"]

[message.register_node_reply]
msg_type = 2
fields = ["request_id:u64", "status:u32", "node_id:u64"]

[message.report_node_load]
msg_type = 3
fields = ["request_id:u64", "node_id:u64", "load_snapshot_ref:string", "capability_bytes:bytes"]

[message.report_node_load_reply]
msg_type = 4
fields = ["request_id:u64", "status:u32"]

[message.request_lease]
msg_type = 5
fields = ["request_id:u64", "resource_request_ref:string", "capability_bytes:bytes"]

[message.request_lease_reply]
msg_type = 6
fields = ["request_id:u64", "status:u32", "lease_id:u64", "lease_ref:string"]

[message.submit_execution]
msg_type = 7
fields = ["request_id:u64", "execution_request_ref:string", "lease_id:u64", "capability_bytes:bytes"]

[message.submit_execution_reply]
msg_type = 8
fields = ["request_id:u64", "status:u32", "execution_id:u64", "execution_trace_ref:string"]

[message.attach_execution]
msg_type = 9
fields = ["request_id:u64", "execution_id:u64", "capability_bytes:bytes"]

[message.attach_execution_reply]
msg_type = 10
fields = ["request_id:u64", "status:u32", "stream_id:u64"]

[message.cancel_execution]
msg_type = 11
fields = ["request_id:u64", "execution_id:u64", "capability_bytes:bytes"]

[message.cancel_execution_reply]
msg_type = 12
fields = ["request_id:u64", "status:u32"]
```

The codegen path should match existing service contracts such as `store_service_v1` and `semantic_state_v1`.

### 3.2 Artifact Schemas

Add an execution-fabric schema module:

```text
artifact_store_schema/src/execution_fabric.rs
```

Minimum v0 types:

- `ExecutionRequestV0`
- `ResourceRequestV0`
- `ResourceLeaseV0`
- `ComputeNodeManifestV0`
- `NodeLoadSnapshotV0`
- `ExecutionLaunchPlanV0`
- `ExecutionTraceV0`

`ExecutionTraceV0` should remain separate from existing `TraceArtifactV0`. Protocol traces and scenario traces describe replayable protocol/scenario events; execution traces describe lifecycle, resource, runner, node, output, and status outcomes.

### 3.3 Canonical Launch Plan

Promote launch plans into `artifact_store_schema` so both Store-side emission and supervisor-side dispatch consume the same validated type.

Candidate shape:

```rust
pub struct ExecutionLaunchPlanV0 {
    pub schema_version: u32,
    pub program_id: String,
    pub artifact_ref: String,
    pub runner: RunnerSelectorV0,
    pub resource_request: Option<ResourceRequestV0>,
    pub node_selector: Option<NodeSelectorV0>,
    pub capability_policy_ref: Option<String>,
    pub input_mounts: Vec<InputMountV0>,
    pub output_contract: OutputContractV0,
    pub runner_config: RunnerConfigV0,
}
```

`runtime_supervisor` should keep its runner dispatch table, but dispatch from this canonical plan type. `store_cli` should emit this same schema.

## 4. Service Relationships

### Execution Fabric

Owns:
- resource lease grants and denials,
- runner/domain/node selection,
- duplicate suppression for equivalent work,
- execution lifecycle state,
- execution trace artifact emission,
- Semantic State compute-fabric contribution.

### Domain Manager

Keeps lifecycle responsibility:
- start, stop, status, list, report-exit,
- restart policy and generation tracking,
- domain capability mediation.

Execution Fabric may depend on Domain Manager. Domain Manager should not become the scheduler.

### Native Runner

Remains one runner backend. Native WASM manifests keep their capability-declaration and fail-closed validation model. Execution Fabric grants leases and selects `native_wasm_v0`; it does not absorb Native Runner.

### Store

Remains artifact authority:
- resolves artifacts,
- validates schemas,
- stores traces/logs/results,
- emits or signs canonical launch plans.

Execution Fabric manages the run lifecycle. Store does not become the scheduler.

## 5. Semantic State Visibility

Add compute-fabric visibility either as an optional field on `PlatformSnapshotV0` or as a new `PlatformSnapshotV1`.

Preferred long-term shape:

```rust
pub struct ComputeFabricSnapshotV0 {
    pub nodes: Vec<ComputeNodeSnapshotV0>,
    pub active_leases: Vec<ResourceLeaseSummaryV0>,
    pub running_executions: Vec<ExecutionSummaryV0>,
    pub queued_executions: Vec<ExecutionSummaryV0>,
    pub duplicate_groups: Vec<DuplicateExecutionGroupV0>,
}
```

The first implementation must prove backwards-compatible deserialization if it extends `PlatformSnapshotV0`.

## 6. Simulation Gate

Add:

```text
tools/ci/foundry_execution_fabric_s10_4.sh
just foundry-execution-fabric-s10-4
```

The S10.4 v0 gate should assert:

1. IDL codegen succeeds for `execution_fabric_v1`.
2. Execution request, resource lease, launch plan, and execution trace schemas validate.
3. Simulated cheap command routes local.
4. Simulated heavy command routes remote.
5. Duplicate execution request attaches to the existing execution ID.
6. Denied resource lease fails closed.
7. Semantic snapshot includes nodes, leases, and executions.
8. Execution trace lifecycle events are monotonic.

The "remote" node in v0 is simulated. The gate validates contracts, not transport.

## 7. Phased PR Plan

### PR 1: Contract Draft

Files:
- `docs/plans/2026-06-17-s10-4-execution-fabric.md`
- `SLICES.md`
- `ROADMAP.md`
- `CURRENT_STATUS.md`
- `NEXT_TASKS.md`
- `CONSTITUTION.md`
- `PLATFORM_OVERVIEW.md`

### PR 2: IDL and Schemas

Files:
- `idl/services/execution_fabric_v1.toml`
- `artifact_store_schema/src/execution_fabric.rs`
- `artifact_store_schema/src/lib.rs`
- `justfile`
- generated code outputs
- `store_cli` validation commands
- `tools/ci/foundry_execution_fabric_s10_4.sh`

### PR 3: Simulated Service and Semantic State

Files:
- `services/execution_fabric/`
- workspace `Cargo.toml`
- `services/semantic_state/`
- `artifact_store_schema/src/semantic_state.rs`
- `tools/ci/foundry_execution_fabric_s10_4.sh`

## 9. Phase 2 — Wiring (S10.4.1) — COMPLETE

- `store_cli emit-plan` emits canonical `ExecutionLaunchPlanV0` with runner-specific payload in `runner_config.config_json`
- `runtime_supervisor` `fabric_policy::consult_always_local` (simulation lease/trace; always-local dispatch)
- `launch_plan` extracts compat/GPU/native config from canonical runner payload; legacy JSON still parses
- Gate: `emit_plan_canonical_roundtrip` in `foundry_execution_fabric_s10_4.sh`
- Updated `foundry_compat_s2.sh` and `foundry_gpu_quarantine_s7.sh` for canonical plan shape

## 10. Design Decisions to Carry Forward

- Execution Fabric is policy/control, not a runner.
- Canonical launch plans live in `artifact_store_schema`.
- Rich scheduler objects are artifact schemas; IDL messages carry content references and capability bytes.
- Execution traces are distinct from protocol/scenario trace artifacts.
- S10.4 v0 is simulation-first and gate-first.
