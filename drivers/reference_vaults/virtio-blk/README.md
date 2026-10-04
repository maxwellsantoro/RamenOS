# virtio-blk Reference Vault

S13 uses `virtio-blk-pci` as the QEMU Oracle stepping stone before metal NVMe graduation.

The landed Oracle/replay context includes:
- `traces/`: Oracle `driver_protocol_trace_v0` fixtures (init + block I/O) — initialization and sector captures are populated.
- `datasheets/`: pinned VIRTIO block device spec notes for agents.
- `harness.toml`: the target `harness.block` IDL contract copied from `/idl` for agent context.
- `notes.md`: capture scope, constraints, and follow-up inventory.

The replay scoreboard reuses `kernel_api::mock::pci_device`; `driver_foundry` translates
trace events into `PciReplayEvent` arrays before running native driver code against
`MockPciDevice`. Block payload validation uses `harness.block` shmem contracts (S13.4+).

`just s13` checks the Oracle/replay and embedded-sector harness lane. Native
device-backed read/write/flush and physical NVMe update/rollback remain unproven;
see [Current Status](../../../CURRENT_STATUS.md).
