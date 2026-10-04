# S10.2: Semantic State Substrate

**Last Updated:** 2026-10-03
**Status:** Maintained architecture reference; host snapshots, reactor, and filtering landed

Semantic State provides typed, structured views of permitted OS state for humans
and agents. JSON supports programmatic consumers; Markdown supports readable
inspection. The current service aggregates named host sources and fixture state,
not the complete live target OS topology.

## Ownership and contract

[semantic_state](../../services/semantic_state/) owns snapshot construction,
source aggregation, capability-filtered views, and the subscription reactor.
The canonical protocol is
[semantic_state_v1.toml](../../idl/services/semantic_state_v1.toml); generated
SDK imports and native-runner host shims come from `just codegen`.

Snapshots cover domains, grants, harnesses, and selected resource/execution
metadata supported by their producers. Placeholder boot/time metadata and
fixture producers must remain distinguishable from observed target state.
Snapshot formats and filtering rules belong to their schema/producer; clients
must not infer authority from the mere presence of an interface name.

## Observation authority

A caller's grants determine which rows and fields it may observe. Filtering
includes snapshots and subscription events; hidden rows must not leak through
unfiltered aggregate counts or change notifications. Invalid authority or
unsupported formats fail closed on the named checked paths.

Request permission and observation permission are separate contracts. The SW0
[authority inventory](../AGENT_TASK_AUTHORITY_V1.md) and
[subscription contract](../AGENT_TASK_SUBSCRIPTIONS_V2.md) add finite host-task
assertions; they do not establish hidden-affordance noninterference for arbitrary
OS state or protocols.

## Subscriptions

The host reactor offers bounded subscription state and change handling. Consumers
must preserve cancellation, ownership, and observation filters across changes.
This is distinct from a general target event transport or hotplug inventory.
SW0's typed pull/cancel lifecycle has its own contract and gates rather than
being inferred from the original reactor milestone.

## Evidence and remaining integration

Run `just foundry-semantic-state-s10-2` for the host assertions. The selected
QEMU snapshot/IPC bridge is documented in
[S10.5](2026-06-17-s10-5-host-to-target-integration.md); it runs kernel init
handlers and deterministic snapshot bytes, while Wasmtime and the broader
service remain on the host.

Live target producers, a target userspace runtime, cross-source completeness,
and end-to-end notification transport remain integration work. Sub-50ms latency
and compact context budgets are design targets, not measurements established by
these deterministic gates. Human policy and enforcement must remain usable
without an AI model.

Historical implementation decisions are retained in the
[reactor plan](../archive/plans/2026-06-17-s10-2-1-subscribe-reactor.md) and
[filtering plan](../archive/plans/2026-06-17-s10-2-v1-1-cap-filtered-snapshots.md).
[Current Status](../../CURRENT_STATUS.md) records landed evidence;
[Next Tasks](../../NEXT_TASKS.md) owns next work.
