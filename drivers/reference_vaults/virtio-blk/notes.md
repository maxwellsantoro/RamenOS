# virtio-blk Oracle notes (S13)

## Capture scope (S13.2 target)

1. PCI discovery + virtio feature negotiation
2. Queue setup (queue 0 = request queue for block)
3. `DRIVER_OK` transition
4. Single-sector read + write against a known LBA pattern in the capsule

## Constraints

- Capture runs in the Linux Oracle capsule with `pci_mmio_tracer` (reuse S11.1 tooling).
- Live traces must carry `sha256:` provenance when `REQUIRE_LIVE_ORACLE_TRACE=1`.
- Do not distill from datasheet alone — Oracle trace is authoritative.

## Metal follow-up (S13.7+)

Replace virtio-blk Oracle with NVMe admin/IO queue trace from the lab controller only
after QEMU replay path is green. Metal graduation is gated separately from this vault.