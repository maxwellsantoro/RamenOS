# virtio-net Notes

## Scope

S11.4 extends `virtio-net-pci` initialization replay through feature negotiation, RX/TX queue setup, and MAC observation. S11.5 distills harness-level packet send/receive against `traces/oracle_packet_trace.json`. S11.7 captures live packet receive via kernel netdev (`virtio_net.ko` + AF_PACKET ARP), not userspace virtqueue RX.

## Oracle

- Device model: `virtio-net-pci`
- Init Oracle: Linux guest PCI/MMIO observed through `tools/trace/virtio_net_oracle_capture.c`
- Packet Oracle: kernel netdev path in `tools/trace/virtio_net_packet_oracle_capture.c` (module load → `eth0` → AF_PACKET)
- Replay target (init): `kernel_api::mock::pci_device::MockPciDevice`
- Replay target (packet): `kernel_api::mock::packet_harness::MockPacketHarness`

## Constraints

- Do not infer registers from memory or pre-training alone.
- Use only offsets and values present in `traces/oracle_init_trace.json`, then name them with `datasheets/virtio-net-v1.3.md`.
- Packet buffers use shared memory handles from `harness.net`; native control messages must not carry dynamic byte arrays.
- Packet receive fixtures must pass `assert-hardware-packet-trace` (rejects `slirp-arp-reply-derived` notes).

## Follow-Up Inventory

- Re-capture init with `tools/trace/capture_virtio_net_oracle.sh` when QEMU/device parameters change.
- Re-capture packet I/O with `tools/trace/capture_virtio_net_packet_oracle.sh` when harness parameters change; `fetch_virtio_net_modules.sh` refreshes bundled kernel modules.
- Run `REQUIRE_LIVE_ORACLE_TRACE=1 bash tools/ci/foundry_s11_reference_vault_s11_3.sh` before calling the vault live (init provenance + packet provenance + hardware RX assertion).
- S11.8 complete: runtime packet I/O under native `harness.net` control in QEMU (`foundry_s11_runtime_net_s11_8.sh`); S11 closed via `just s11`.