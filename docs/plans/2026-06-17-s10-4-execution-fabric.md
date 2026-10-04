# S10.4: Capability-Scheduled Execution Fabric

**Last Updated:** 2026-10-03
**Status:** Maintained reference; v0 contracts and S10.4.1 host wiring landed

The execution fabric is a policy/control service that selects a runner, domain,
and node, grants bounded resources, and records execution outcomes. Runners
perform execution. The current implementation is a host simulation and local
integration boundary; distributed execution is deferred.

## Canonical contracts

[execution_fabric.rs](../../artifact_store_schema/src/execution_fabric.rs) owns
node, lease, launch-plan, and execution-trace schemas. The typed control interface
is [execution_fabric_v1.toml](../../idl/services/execution_fabric_v1.toml).
Rich state is referenced by bounded artifact IDs rather than dynamic wire strings
or pointer-bearing payloads.

`ExecutionLaunchPlanV0` is canonical across Store CLI and runtime-supervisor
consumers. Its explicit runner selection/configuration, artifact reference, domain,
node, resource/policy references, and outputs must be validated according to the
schema and selected runner. A serializable plan is not an authority grant, signed
publication, or proof that a runner is available on the target.

Execution lifecycle traces have their own schema; protocol/scenario traces
retain their original meanings. Semantic State exposes supported node/lease/
execution metadata without implying a live distributed cluster.

## Component boundaries

| Component | Responsibility |
|-----------|----------------|
| [Execution Fabric service](../../services/execution_fabric/) | Host simulation, node/lease/execution policy and state |
| [Store CLI](../../store_cli/) | Catalog selection and canonical launch-plan emission |
| [Runtime Supervisor](../../runtime_supervisor/) | Validate the plan and dispatch the selected host runner |
| [Domain Manager](../../services/domain_manager/) | Domain lifecycle and broker decisions |
| Kernel | Validate capabilities on implemented native fast paths |

Scheduling policy does not belong in the kernel or Domain Manager IDL. Native
control remains typed messages; bulk data belongs to validated shared memory.
A simulated lease is distinct from enforced CPU/memory accounting on target.

## Evidence and deferred work

Run `just foundry-execution-fabric-s10-4` for contract/simulation and local plan
wiring assertions. Gate definitions and schema tests specify the exact boundary.
There are no real remote workers, SSH transport, cluster scheduler, or enforced
distributed resource leases established by this milestone.

Target execution, scheduler fairness, recovery under partial failures, remote
trust, and fabric performance need separate decisions and gates. Current runner
and bridge limits are documented in
[S10.5](2026-06-17-s10-5-host-to-target-integration.md) and
[Security Status](../../SECURITY_STATUS.md). Use
[Current Status](../../CURRENT_STATUS.md) and [Next Tasks](../../NEXT_TASKS.md)
for landed state and next work rather than repeating their milestone history here.
