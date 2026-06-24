# virtio-blk Reference Vault

S13 uses `virtio-blk-pci` as the QEMU Oracle stepping stone before metal NVMe graduation.

This vault is intentionally small at S13.0 scaffold time:
- `traces/`: Oracle `driver_protocol_trace_v0` fixtures (init + block I/O) — populated in S13.2+.
- `datasheets/`: pinned VIRTIO block device spec notes for agents.
- `harness.toml`: the target `harness.block` IDL contract copied from `/idl` for agent context.
- `notes.md`: capture scope, constraints, and follow-up inventory.

The replay scoreboard reuses `kernel_api::mock::pci_device`; `driver_foundry` translates
trace events into `PciReplayEvent` arrays before running native driver code against
`MockPciDevice`. Block payload validation uses `harness.block` shmem contracts (S13.4+).