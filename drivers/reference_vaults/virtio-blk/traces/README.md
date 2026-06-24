# virtio-blk Oracle traces

Populated by S13.2 capture + promotion tooling.

Expected fixtures:
- `oracle_init_trace.json` — PCI/MMIO init path (virtio-blk `DRIVER_OK`)
- `oracle_block_trace.json` — read/write sector harness trace (S13.4+)