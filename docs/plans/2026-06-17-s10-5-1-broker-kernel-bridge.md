# S10.5.1: Broker / Kernel Harness Bridge (one path)

**Last Updated:** 2026-06-17
**Status:** Complete (host gate green)
**Parent:** `docs/plans/2026-06-17-s10-5-host-to-target-integration.md`
**Gate:** `tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh`

---

## Goal

Close the gap between **broker policy decisions** and **native_runner harness IPC** for exactly two interfaces:

| Interface IDL | Protocol | WASM export |
|---------------|----------|-------------|
| `shared_memory.control_v1` | `8` | `RAMEN_CAP_SHMEM_CONTROL` |
| `services.semantic_state_v1` | `10` | `RAMEN_CAP_SEMANTIC_STATE` |

S10.5.0 proved semantic snapshot bytes on **QEMU kernel init**. S10.5.1 proves the **host control-plane path**: broker grant → validated fast-path transact → `get_snapshot` reply with shmem — without migrating the full broker or moving `store_service`.

---

## Inventory blockers (why not “real kernel socket” yet)

| Gap | S10.5.1 approach | Later |
|-----|------------------|-------|
| QEMU kernel has no Unix socket to host | Host **`KernelHarnessProxy`** (Unix socket) | S10.5.2 virtio-serial bridge to QEMU |
| `GrantCapabilitiesReply` returns only `handle_count` | Add **`get_domain_grant_handles`** IPC round-trip | Or v2 grant reply with inline handles |
| `SimulatedKernelOps` ignores interface semantics | **`SemanticHarnessGrantOps`** allowlist + registry | Full kernel `cap_table` grant IPC |
| `native_wasm_runner` stubs grants | Wire supervisor → domain_manager broker IPC | — |

**Scope guard:** No echo/trace/store harnesses. No subscribe push on proxy. No cap-filtered snapshots.

---

## Architecture

```
runtime_supervisor                domain_manager                    KernelHarnessProxy
      |                                 |                                  |
      |-- GrantCapabilities ----------->| broker + SemanticHarnessGrantOps   |
      |<-- grant ok (count=2) ----------|                                  |
      |-- GetDomainGrantHandles ------->| reads active_grants registry     |
      |<-- {SHMEM, SEMANTIC} handles ---|                                  |
      |                                 |                                  |
      |-- native_runner.load_and_run -->|                                  |
      |   kernel_ipc=/run/ramen/... ------------------------------------->|
      |                                 |                    validate cap  |
      |                                 |                    shmem create  |
      |                                 |                    get_snapshot  |
      |<-- exit 0 + stdout markers -----|                                  |
```

### Component ownership

| Component | Owner | Role |
|-----------|-------|------|
| `SemanticHarnessGrantOps` | `domain_manager::broker` | `KernelGrantOps` impl; allowlist-only grants; auditable registry |
| `KernelHarnessProxy` | new `services/kernel_harness_proxy` (host) | Unix socket; envelope transact; cap validation; in-process shmem + semantic handlers |
| Grant handle fetch | `domain_manager` IPC | New typed messages (below) |
| Supervisor wiring | `runtime_supervisor::native_wasm_runner` | Replace grant stub with broker IPC + proxy path |

---

## 1. Broker: `SemanticHarnessGrantOps`

Replace `SimulatedKernelOps` in the **semantic harness profile** only (env `RAMEN_SEMANTIC_HARNESS_BRIDGE=1` or explicit supervisor launch flag).

```rust
const SEMANTIC_HARNESS_ALLOWLIST: &[&str] = &[
    "shared_memory.control_v1",
    "services.semantic_state_v1",
];
```

Behavior:
- `grant(interface, rights, domain_id)` → fail closed (`STATUS_INVALID_INTERFACE`) if not allowlisted
- Deterministic handle encoding: `0x5308_0000_0000_0001 | domain_id` (shmem), `0x5310_0000_0000_0001 | domain_id` (semantic)
- `revoke(handle)` → remove from registry; fail if unknown
- Full broker audit trail (existing `BrokerAuditEvent`)

Policy: extend `ChannelAllowlistPolicy` test profile OR add `SemanticState` channel with the two interfaces only.

---

## 2. IPC: fetch granted handles

`GrantCapabilitiesReply` cannot fit handle map in 64-byte payload. Add domain_manager v1 messages:

**`get_domain_grant_handles`** (msg_type TBD, e.g. 15)
```
request_id: u64
domain_id: u64
```

**`get_domain_grant_handles_reply`** (msg_type 16)
```
request_id: u64
domain_id: u64
status: u32
count: u32
# up to 2 entries for S10.5.1:
entry0_export_id: u16   # 1=RAMEN_CAP_SHMEM_CONTROL, 2=RAMEN_CAP_SEMANTIC_STATE
entry0_reserved: u16
entry0_handle: u64
entry1_export_id: u16
entry1_reserved: u16
entry1_handle: u64
```

Export ID enum is stable for the semantic harness profile; generalizes later.

Fail-closed: `count == 0` or status != OK → supervisor aborts WASM load.

---

## 3. `KernelHarnessProxy` (host fast-path)

Unix socket default: `/run/ramen/kernel.sock` (configurable via `RAMEN_KERNEL_PROXY_SOCKET`).

Per `transact(envelope)`:
1. Validate `envelope.handle` against grant registry (domain + interface binding)
2. Dispatch by protocol:
   - **8 / shmem_control:** `create_region`, `shmem_write` (in-process byte map keyed by `shm_cap`)
   - **10 / semantic_state:** `get_snapshot` → same deterministic JSON snapshot as S10.5.0 kernel init (`snapshot_sha256_prefix=9c0de4419f03f426`)
3. Unknown protocol → fail-closed empty reply / error status

No Wasmtime in proxy. No store access.

### Determinism contract

Proxy `get_snapshot` payload bytes MUST match S10.5.0 QEMU init snapshot (`SEMANTIC_SNAPSHOT_BYTES` shared constant or cross-crate test vector) so gates can compare sha256 prefix across host proxy and QEMU.

---

## 4. Supervisor wiring

`native_wasm_runner::request_capability_grants`:
1. Connect domain_manager socket
2. `GrantCapabilities` with manifest content_id hash
3. On OK → `GetDomainGrantHandles`
4. Map export IDs → `HashMap<String, u64>` for `RunConfig.granted_handles`
5. `RunnerConfig.kernel_ipc` → proxy socket path

Fail-closed: missing export → `Err` before Wasmtime load.

---

## 5. Foundry gate — `foundry_broker_kernel_bridge_s10_5_1.sh`

### Phase 0 — Design + inventory (PASS)

```
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY grant_reply_handle_count_only=true
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY simulated_kernel_ops=true
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: INVENTORY native_wasm_grant_stub=true
```

### Phase 1 — Broker allowlist (PASS)

```
cargo test -p domain_manager semantic_harness_grant_ops --quiet
```

Assertions:
- Grants `shared_memory.control_v1` + `services.semantic_state_v1` for semantic manifest
- Denies `harness.echo_v1` fail-closed

### Phase 2 — Proxy roundtrip (PASS)

```
cargo test -p kernel_harness_proxy proxy_get_snapshot_roundtrip --quiet
```

Starts proxy in test, transacts `get_snapshot` with granted cap, asserts:
- Reply status OK
- Shmem contains snapshot bytes
- `snapshot_sha256_prefix=9c0de4419f03f426`

### Phase 3 — Supervisor E2E (PASS)

```
cargo test -p runtime_supervisor semantic_harness_bridge_e2e --quiet
```

Spawns domain_manager + proxy fixtures, runs minimal semantic_state WASM (or harness test module), asserts exit 0.

### PASS markers

```
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: METRIC snapshot_sha256_prefix=9c0de4419f03f426
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: PASS
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: ok
```

**CI:** Host-only gate is green. Not added to `foundry_ci_extended` in this change. No QEMU required.

---

## 6. What remains after S10.5.1

| Still host-only / simulated | Addressed in |
|-----------------------------|------------|
| Full broker `KernelGrantOps` for echo/trace/store | Later slices |
| QEMU kernel Unix socket / virtio-serial IPC | S10.5.2 |
| `semantic_state.wasm` inside QEMU | S10.5.2+ / userspace loader |
| Subscribe push to WASM guest | S10.2 v1.1 + S10.5.2 |
| Capability-filtered snapshots | S10.2 v1.1 |

---

## 7. Implementation checklist

1. ✅ IDL: `get_domain_grant_handles` in `domain_manager_v1.toml` + codegen
2. ✅ `SemanticHarnessGrantOps` + semantic channel policy
3. ✅ `services/kernel_harness_proxy` crate (socket + shmem map + semantic handler)
4. ✅ Share `SEMANTIC_SNAPSHOT_BYTES` constant with kernel init test vector
5. ✅ Wire `native_wasm_runner` grant path
6. ✅ Green `foundry_broker_kernel_bridge_s10_5_1.sh`
7. ✅ Update `foundry_host_target_s10_5.sh` Phase 2 comment → point to S10.5.1 gate
