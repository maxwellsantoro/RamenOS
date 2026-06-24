# virtio-net Reference Vault

S11 uses `virtio-net-pci` as the first Driver Factory Oracle target.

This vault is intentionally small at S11.3 scaffold time:
- `traces/oracle_init_trace.json`: schema-valid `driver_protocol_trace_v0` fixture for the initialization replay path.
- `datasheets/virtio-net-v1.3.md`: pinned OASIS VIRTIO source notes for PCI discovery, capabilities, init status, queues, features, and network config.
- `harness.toml`: the target `harness.net` IDL contract copied from `/idl` for agent context.
- `notes.md`: capture scope, constraints, and follow-up inventory.

The replay scoreboard lives in `kernel_api::mock::pci_device`; `driver_foundry`
translates trace events into `PciReplayEvent` arrays before running native
driver code against `MockPciDevice`.
