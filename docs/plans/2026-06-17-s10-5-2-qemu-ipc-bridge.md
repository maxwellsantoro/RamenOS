# S10.5.2: QEMU IPC Bridge

**Last Updated:** 2026-10-03
**Status:** Maintained reference; selected chardev-serial bridge landed

The implemented bridge carries a selected typed request from a host Unix socket
through QEMU's **COM2 UART** to the kernel init relay. It uses QEMU chardev serial,
not a virtio-serial driver. COM1 retains boot/evidence logs.

## Transport and framing

The gate's second `-serial` endpoint is a Unix socket. The host
`ChardevKernelBridge` sends frames to that endpoint; QEMU delivers bytes to
`arch::serial::ipc`, and `OP_SEMANTIC_IPC_RELAY` handles the selected request.
The `semantic_ipc_bridge` init profile enables that relay.

[ipc_frame.rs](../../kernel_api/src/ipc_frame.rs) owns the encoding:

```text
[u32 little-endian frame length][encoded Envelope]
```

The encoded envelope is 88 bytes, including its fixed 64-byte payload and header/
padding. The general codec bounds frame length; the current target relay accepts
exactly that envelope size and rejects bad lengths, decoding, routes, and formats.
The selected operation is Semantic State `get_snapshot`, not arbitrary kernel IPC.

## Capability and shared-memory boundary

The init handler exercises deterministic snapshot bytes and kernel shared-memory
primitives. Host/QEMU markers and hashes correlate the selected request/reply.
A returned target shared-memory handle does not map target memory into the host
process. The gate does not establish a complete guest WASM task, general target
broker grants, or cross-machine shared-memory transport.

The host native runner supports explicit Unix and `chardev-serial` transports.
Its invocation deadline includes connect and partial-frame waits; uncertain
requests are not automatically replayed. Whole-task cleanup and backend lifetime
remain separately scoped by their contracts.

## Verification

```bash
just foundry-qemu-ipc-bridge-s10-5-2
```

The [gate](../../tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh) checks framing and
host bridge units, then boots the selected QEMU relay and validates its response.
It is included in extended CI. A PASS covers this selected host/QEMU transport,
not a target userspace runtime or hardware qualification.

See [S10.5](2026-06-17-s10-5-host-to-target-integration.md),
[broker/proxy bridge](2026-06-17-s10-5-1-broker-kernel-bridge.md), and
[Current Status](../../CURRENT_STATUS.md) for remaining integration.
