# virtio-net Reference Vault

S11 uses `virtio-net-pci` as the first Driver Factory Oracle target.

The landed Oracle/replay context includes:
- `traces/oracle_init_trace.json`: Linux-captured `driver_protocol_trace_v0` for initialization replay.
- `traces/oracle_packet_trace.json`: packet Oracle payloads and capture provenance.
- `datasheets/virtio-net-v1.3.md`: pinned OASIS VIRTIO source notes for PCI discovery, capabilities, init status, queues, features, and network config.
- `harness.toml`: the target `harness.net` IDL contract copied from `/idl` for agent context.
- `notes.md`: capture scope, constraints, and follow-up inventory.

The replay scoreboard lives in `kernel_api::mock::pci_device`; `driver_foundry`
translates trace events into `PciReplayEvent` arrays before running native
driver code against `MockPciDevice`.

`just s11` checks replay, required Oracle provenance and embedded-vector harness
transfers. Linux Oracle RX is distinct from native RamenOS device RX, which remains
unproven. See [Current Status](../../../CURRENT_STATUS.md).
