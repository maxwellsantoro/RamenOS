# VIRTIO Block Device — pinned source notes (S13.0 scaffold)

**Primary source:** OASIS Virtual I/O Device (VIRTIO) Version 1.3 — Block Device section.

## Agent anchors

- Device ID: `2` (block)
- Feature bits: `VIRTIO_BLK_F_SIZE_MAX`, `VIRTIO_BLK_F_SEG_MAX`, `VIRTIO_BLK_F_GEOMETRY`, `VIRTIO_BLK_F_RO`
- Queue 0: request queue
- Request types: `IN`, `OUT`, `FLUSH`, `GET_ID`
- Status byte: `VIRTIO_BLK_S_OK`, `VIRTIO_BLK_S_IOERR`, `VIRTIO_BLK_S_UNSUPP`

## PCI legacy init anchors

1. acknowledge device (`ACKNOWLEDGE`).
2. set `DRIVER`.
3. negotiate guest features from device features.
4. read capacity from device config (`capacity` + `capacity_hi` when present).
5. program queue 0 PFN and size.
6. set `DRIVER_OK`.

## S13 capture intent

Oracle capture must record PCI config cycles, queue notify, and descriptor chain
setup for one sector read and one sector write. Register names in distilled Rust
must map to trace offsets — not datasheet guesses alone.