# S10.5: Host-to-Target Integration

**Last Updated:** 2026-10-03
**Status:** Maintained reference; S10.5.0–S10.5.2 selected bridges landed

**CHOSEN:** Option A, a typed Semantic State snapshot on the QEMU kernel/init
path, followed by a host broker/proxy and selected framed QEMU IPC. This proves
bounded target contract behavior while preserving the actual execution split.

## Execution inventory

| Component | Current execution environment |
|-----------|-------------------------------|
| Kernel, init bytecode, snapshot/relay handlers | RamenOS QEMU target |
| Wasmtime native runner and Semantic State WASM | Host |
| Semantic reactor and source aggregation | Host |
| Domain Manager and semantic grant profile | Host |
| Kernel Harness Proxy and Store service | Host Unix sockets |
| Runtime Supervisor | Host |
| Linux compatibility capsule | Separate Linux VM |
| General RamenOS userspace domain loader | Pending |

A Unix socket named `kernel.sock` on the host is not a kernel-native Unix socket
inside RamenOS. The proxy and serial bridge have different ownership and evidence.

## Landed phases

| Phase | Path | Gate |
|-------|------|------|
| S10.5.0 | `semantic_snapshot` init op constructs deterministic snapshot bytes using kernel shared-memory primitives | `just foundry-host-target-s10-5` |
| S10.5.1 | Broker grants → host `KernelHarnessProxy` → selected shmem/semantic handlers | `just foundry-broker-kernel-bridge-s10-5-1` |
| S10.5.2 | Host framed transport → QEMU COM2 UART → selected kernel init relay | `just foundry-qemu-ipc-bridge-s10-5-2` |

All three gates are included in the extended CI suite. Snapshot markers and hash
comparisons assert the named deterministic payload. They do not establish live
aggregation, a complete WASM task on target, or target-wide broker enforcement.

The [broker/proxy reference](2026-06-17-s10-5-1-broker-kernel-bridge.md) and
[serial IPC reference](2026-06-17-s10-5-2-qemu-ipc-bridge.md) define the precise
transport and authority boundaries. Generic host runner IPC now uses absolute
invocation deadlines and avoids automatic replay of uncertain requests; those
properties do not supply whole-task lifetime or target shutdown guarantees.

## Deferred integration

A target domain loader/runtime, general broker-to-kernel grants, target event
subscriptions, live multi-source snapshots, and complete target-native agent
tasks remain separate work. Projection queries on target and broad service
migration need their own typed contracts and consumers. Keep the host/QEMU
split explicit in plans, logs, and performance claims.

[Current Status](../../CURRENT_STATUS.md) owns landed evidence;
[Next Tasks](../../NEXT_TASKS.md) owns integration order. The independent SW0
host task proof does not become target-enforced through these selected bridge gates.
