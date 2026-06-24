# virtio-net PCI Reference Notes

Pinned source: OASIS Virtual I/O Device (VIRTIO) Version 1.3, latest-stage
HTML: https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.html

Citation tag: `[VIRTIO-v1.3]`

These notes are a distilled reference for S11.3 driver distillation. They are
not a replacement for the normative specification; use them to correlate the
Oracle PCI/MMIO trace with section numbers and stable symbolic names.

## Scope

- Device model: `virtio-net-pci`
- Transport: virtio over PCI bus
- Harness target: `harness.net` / `net_v1`
- Replay target: `kernel_api::mock::pci_device::MockPciDevice`

## Discovery Anchors

- Virtio PCI devices use vendor ID `0x1AF4`.
- Non-transitional virtio PCI device IDs are computed as `0x1040 +
  virtio_device_id`.
- Network device ID is `1`, so the non-transitional PCI device ID is `0x1041`.
- Transitional network devices may use PCI device ID `0x1000`.

Source sections:
- `4.1.2 PCI Device Discovery`
- `5.1.1 Device ID`

Trace expectations:
- The Oracle trace should first prove that the observed function is a virtio
  network device before any register interpretation is trusted.
- For S11.3, PCI config reads with vendor `0x1AF4` and device `0x1000` or
  `0x1041` are sufficient to keep replay scoped to the selected QEMU device.

## PCI Capability Map

The virtio PCI transport discovers device regions through vendor-specific PCI
capabilities. The key `cfg_type` values for this vault are:

| Symbol | Value | Meaning |
|--------|-------|---------|
| `VIRTIO_PCI_CAP_COMMON_CFG` | `1` | Common configuration |
| `VIRTIO_PCI_CAP_NOTIFY_CFG` | `2` | Queue notification region |
| `VIRTIO_PCI_CAP_ISR_CFG` | `3` | ISR status byte |
| `VIRTIO_PCI_CAP_DEVICE_CFG` | `4` | Network device configuration |
| `VIRTIO_PCI_CAP_PCI_CFG` | `5` | PCI configuration access window |
| `VIRTIO_PCI_CAP_SHARED_MEMORY_CFG` | `8` | Shared memory region |
| `VIRTIO_PCI_CAP_VENDOR_CFG` | `9` | Vendor-specific data |

Source sections:
- `4.1.3 PCI Device Layout`
- `4.1.4 Virtio Structure PCI Capabilities`

Trace expectations:
- MMIO offsets are relative to the BAR and capability offset discovered through
  PCI config space.
- Device configuration accesses must use the field width and alignment required
  by the spec. Do not infer wider or unaligned MMIO operations from convenience.

## Initialization State Machine

The generic virtio initialization sequence is:

1. reset the device;
2. set `ACKNOWLEDGE`;
3. set `DRIVER`;
4. read offered features and write the accepted subset;
5. set `FEATURES_OK`;
6. verify `FEATURES_OK` remains set;
7. perform device-specific setup, including virtqueue discovery and population;
8. set `DRIVER_OK`.

Source sections:
- `2.1 Device Status Field`
- `2.4 Device Reset`
- `3.1 Device Initialization`

Trace expectations:
- Writes to `device_status` must be monotonic except for reset-to-zero.
- The driver must not notify buffers before `DRIVER_OK`.
- If `FEATURES_OK` is cleared by the device after negotiation, distillation
  should fail closed rather than continuing with guessed features.

## Network Queues

The base queue layout is:

| Queue index | Purpose |
|-------------|---------|
| `0` | `receiveq1` |
| `1` | `transmitq1` |
| `2(N-1)` | `receiveqN` |
| `2(N-1)+1` | `transmitqN` |
| `2N` | `controlq` |

When neither `VIRTIO_NET_F_MQ` nor `VIRTIO_NET_F_RSS` is negotiated, `N = 1`.
The `controlq` exists only when `VIRTIO_NET_F_CTRL_VQ` is negotiated.

Source section:
- `5.1.2 Virtqueues`

Trace expectations:
- The S11.3 init replay should first model queue 0 and queue 1.
- Control-queue behavior is out of scope unless the live trace negotiates
  `VIRTIO_NET_F_CTRL_VQ`.

## Network Feature Bits

Feature bits most relevant to initial bring-up:

| Symbol | Bit | Meaning for distillation |
|--------|-----|--------------------------|
| `VIRTIO_NET_F_CSUM` | `0` | Device accepts partial checksum offload. |
| `VIRTIO_NET_F_GUEST_CSUM` | `1` | Driver accepts partial checksum packets. |
| `VIRTIO_NET_F_MTU` | `3` | `mtu` field is valid. |
| `VIRTIO_NET_F_MAC` | `5` | `mac` field is valid. |
| `VIRTIO_NET_F_STATUS` | `16` | `status` field is valid. |
| `VIRTIO_NET_F_CTRL_VQ` | `17` | Control virtqueue exists. |
| `VIRTIO_NET_F_MQ` | `22` | Multiqueue is available. |
| `VIRTIO_NET_F_CTRL_MAC_ADDR` | `23` | MAC can be set through controlq. |
| `VIRTIO_NET_F_RSS` | `60` | RSS steering is available. |
| `VIRTIO_NET_F_SPEED_DUPLEX` | `63` | `speed` and `duplex` fields are valid. |

Source sections:
- `5.1.3 Feature bits`
- `5.1.3.1 Feature bit requirements`

Trace expectations:
- Optional configuration fields are valid only when their feature bit is
  offered and accepted.
- Dependency bits matter: for example, `VIRTIO_NET_F_CTRL_MAC_ADDR`,
  `VIRTIO_NET_F_MQ`, and `VIRTIO_NET_F_RSS` depend on
  `VIRTIO_NET_F_CTRL_VQ`.

## Network Device Configuration

The network device configuration contains:

| Field | Availability |
|-------|--------------|
| `mac[6]` | Always present, valid with `VIRTIO_NET_F_MAC`. |
| `status` | Present with `VIRTIO_NET_F_STATUS`. |
| `max_virtqueue_pairs` | Present with `VIRTIO_NET_F_MQ` or `VIRTIO_NET_F_RSS`. |
| `mtu` | Present with `VIRTIO_NET_F_MTU`. |
| `speed` / `duplex` | Present with `VIRTIO_NET_F_SPEED_DUPLEX`. |
| RSS/hash fields | Present only with their matching feature families. |

Status bits:
- `VIRTIO_NET_S_LINK_UP = 1`
- `VIRTIO_NET_S_ANNOUNCE = 2`

Source section:
- `5.1.4 Device configuration layout`

Trace expectations:
- Reads from optional fields must be explained by feature negotiation.
- A distilled init driver may read MAC/status for observability, but packet I/O
  must flow through `harness.net` shared-memory descriptors, not dynamic native
  control-plane bytes.
