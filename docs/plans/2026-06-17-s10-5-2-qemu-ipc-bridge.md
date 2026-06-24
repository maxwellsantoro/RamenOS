# S10.5.2: QEMU IPC Bridge (Design Pass)

**Last Updated:** 2026-06-17
**Status:** Complete (2026-06-17)
**Parent:** `docs/plans/2026-06-17-s10-5-host-to-target-integration.md`
**Prerequisite:** S10.5.0 (init semantic snapshot) + S10.5.1 (host broker/proxy bridge) **complete**

---

## Problem

S10.5.1 proves host `native_runner` → `KernelHarnessProxy` (Unix socket) → in-process semantic handlers. QEMU kernel still has **no host-facing IPC transport**; semantic bytes on target today flow through init op `OP_SEMANTIC_SNAPSHOT` only.

S10.5.2 chooses and specifies the first **host↔QEMU control-plane transport** so Wasmtime on the host can transact typed envelopes against the running kernel without the init-bridge shortcut.

## Options

| Option | Transport | Pros | Cons |
|--------|-----------|------|------|
| **A — virtio-serial** | QEMU `-device virtio-serial` + host chardev | Matches capsule relay pattern; bounded framing | Requires init/kernel virtio driver or firmware channel |
| **B — shared memory doorbell** | Existing shmem region + MMIO notify | Aligns with data-plane model | Needs kernel MMIO page + host mmap in QEMU |
| **C — target userspace loader** | Native domain on QEMU runs proxy client | No host socket | Requires userspace loader (large scope) |

**Recommendation:** Option **A (virtio-serial)** for S10.5.2 — reuse `capsule_relay` virtio-serial framing discipline and Foundry trace/replay gates.

## Proposed architecture

```
host native_runner / supervisor
        |
        v
KernelHarnessProxy (host) --[virtio-serial]--> QEMU init relay
        |                                              |
        |                                              v
        |                                    kernel IPC envelope handler
        |                                              |
        +<-------- typed reply + shmem cap --------------+
```

### Components

| Component | Owner | Role |
|-----------|-------|------|
| `virtio_serial_bridge` (host) | new `services/` crate or extend `kernel_harness_proxy` | Frame envelopes; connect to QEMU chardev |
| Init relay profile | `tools/init/build_init_image.py` | Read/write virtio-serial; forward to kernel |
| Kernel forwarder | `kernel/src/init.rs` or dedicated IPC op | Validate + dispatch envelope; return reply |
| Foundry gate | `foundry_qemu_ipc_bridge_s10_5_2.sh` | Host transact roundtrip; negative malformed frame |

## Wire framing (v0 sketch)

Reuse capsule relay length-prefixed little-endian envelope bytes:

```
[u32 le length][Envelope bytes]
```

Max frame size: 4096 bytes (fail closed). Malformed length → `STATUS_ERR` + gate FAIL.

## Gate definition (red before implementation)

1. QEMU boots `semantic_ipc_bridge` init profile.
2. Host sends `get_snapshot` envelope; serial shows `semantic_state: get_snapshot ok`.
3. Reply `shm_cap` non-zero; snapshot sha256 prefix matches S10.5.0 contract.
4. Negative: oversize frame rejected; invalid protocol rejected.

## Scope guard

- No store service on target
- No full broker migration
- No subscribe push over virtio-serial (pull `get_snapshot` only in v0)
- No Wasmtime inside QEMU

## Sequencing

1. Land this design + red gate script (assert `NOT_IMPLEMENTED` or skip with `RAMEN_CI_STRICT=0` until green).
2. Implement virtio-serial init relay + host bridge.
3. Wire `runtime_supervisor` to select chardev transport via `RAMEN_KERNEL_IPC_TRANSPORT=chardev-serial` (or launch-plan `kernel_ipc_transport`).
