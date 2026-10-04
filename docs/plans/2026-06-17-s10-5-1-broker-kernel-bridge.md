# S10.5.1: Broker / Kernel Harness Bridge

**Last Updated:** 2026-10-03
**Status:** Maintained reference; bounded host bridge landed

This phase connects broker policy to native-runner harness IPC for
`shared_memory.control_v1` (protocol 8) and `services.semantic_state_v1`
(protocol 10). It does not implement a general target broker.

## Grant and request path

1. The supervisor requests grants from Domain Manager for the selected manifest.
2. `SemanticHarnessGrantOps` validates its interface allowlist and records grants.
3. Typed `get_domain_grant_handles` messages return the selected export handles.
4. The supervisor passes those handles to the host native runner.
5. `KernelHarnessProxy` validates requests against the host grant registry and
   dispatches the supported shared-memory and semantic handlers.

See [Domain Manager IDL](../../idl/harness/domain_manager_v1.toml),
[broker implementation](../../services/domain_manager/src/broker.rs),
[host proxy](../../services/kernel_harness_proxy/), and
[supervisor wiring](../../runtime_supervisor/src/native_wasm_runner.rs).
The IDL and code own message IDs and handle encoding; old sketches with TBD IDs
or informal bit formulas are superseded.

## Authority and data scope

The grant registry and proxy are host implementations, distinct from the target
kernel capability table. The semantic bridge selects a bounded profile; unknown
interfaces, missing exports, and unsupported requests must fail their checked
paths. Generic broker policy and other runner profiles retain their own scope.

Snapshot bytes and shared-memory replies are checked against named deterministic
vectors. Broader host snapshot filtering and subscriptions have their own S10.2
and SW0 gates; they are not inferred from this initial two-interface bridge.

## Verification and remaining work

`just foundry-broker-kernel-bridge-s10-5-1` checks the allowlist, proxy roundtrip,
and host supervisor integration. It is included in extended CI and does not
require QEMU. The later [QEMU bridge](2026-06-17-s10-5-2-qemu-ipc-bridge.md)
uses serial framing, not virtio-serial or a kernel Unix socket.

General kernel grant IPC, a target userspace loader, and whole-task lifetime
integration remain separate. See the
[parent reference](2026-06-17-s10-5-host-to-target-integration.md) and
[Current Status](../../CURRENT_STATUS.md) for the execution split and evidence.
