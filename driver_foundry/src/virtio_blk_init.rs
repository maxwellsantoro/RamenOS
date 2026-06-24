//! virtio-blk initialization driver distilled from the Oracle trace.
//!
//! Replays legacy PCI discovery, feature negotiation, queue setup, and capacity
//! observation against a `MockPciDevice`.

use kernel_api::mock::pci_device::{MockPciDevice, PciReplayEvent, ReplayMismatch, ReplayResult};

pub const VIRTIO_VENDOR_ID: u64 = 0x1af4;
pub const VIRTIO_BLK_DEVICE_ID: u64 = 0x1001;

pub const DEVICE_FEATURES_OFFSET: u64 = 0x00;
pub const GUEST_FEATURES_OFFSET: u64 = 0x04;
pub const QUEUE_PFN_OFFSET: u64 = 0x08;
pub const QUEUE_NUM_OFFSET: u64 = 0x0c;
pub const QUEUE_SEL_OFFSET: u64 = 0x0e;
pub const DEVICE_STATUS_OFFSET: u64 = 0x12;
pub const BLK_CONFIG_OFFSET: u64 = 0x14;

pub const STATUS_ACKNOWLEDGE: u64 = 0x1;
pub const STATUS_DRIVER: u64 = 0x2;
pub const STATUS_DRIVER_OK: u64 = 0x4;
pub const STATUS_FEATURES_OK: u64 = 0x8;

pub const QUEUE_INDEX: u64 = 0;
pub const QUEUE_PFN: u64 = 0x100000;
pub const DEFAULT_QUEUE_SIZE: u64 = 256;

/// Run the virtio-blk init sequence recorded in the Reference Vault oracle trace.
pub fn init_virtio_blk(device: &mut MockPciDevice<'_>) -> ReplayResult<()> {
    discover_pci(device)?;
    let host_features = negotiate_features(device)?;
    observe_capacity(device)?;
    setup_queue(device, QUEUE_INDEX, QUEUE_PFN, DEFAULT_QUEUE_SIZE)?;
    set_driver_ok(device, host_features)?;
    device.finish()
}

fn discover_pci(device: &mut MockPciDevice<'_>) -> ReplayResult<()> {
    let vendor = device.pci_config_read(0x00, 2)?;
    if vendor != VIRTIO_VENDOR_ID {
        return Err(ReplayMismatch::Value {
            expected: VIRTIO_VENDOR_ID,
            observed: vendor,
        });
    }

    let device_id = device.pci_config_read(0x02, 2)?;
    if device_id != VIRTIO_BLK_DEVICE_ID {
        return Err(ReplayMismatch::Value {
            expected: VIRTIO_BLK_DEVICE_ID,
            observed: device_id,
        });
    }

    let status = device.mmio_read(0, DEVICE_STATUS_OFFSET, 2)?;
    if status != 0 {
        return Err(ReplayMismatch::Value {
            expected: 0,
            observed: status,
        });
    }

    device.mmio_write(0, DEVICE_STATUS_OFFSET, 2, STATUS_ACKNOWLEDGE)?;
    device.mmio_write(
        0,
        DEVICE_STATUS_OFFSET,
        2,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER,
    )
}

fn negotiate_features(device: &mut MockPciDevice<'_>) -> ReplayResult<u64> {
    let host_features_lo = device.mmio_read(0, DEVICE_FEATURES_OFFSET, 2)?;
    let host_features_hi = device.mmio_read(0, DEVICE_FEATURES_OFFSET + 2, 2)?;
    let host_features = host_features_lo | (host_features_hi << 16);
    device.mmio_write(0, GUEST_FEATURES_OFFSET, 4, host_features)?;

    if host_features != 0 {
        device.mmio_write(
            0,
            DEVICE_STATUS_OFFSET,
            2,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK,
        )?;

        let status = device.mmio_read(0, DEVICE_STATUS_OFFSET, 2)?;
        if status & STATUS_FEATURES_OK == 0 {
            return Err(ReplayMismatch::Value {
                expected: STATUS_FEATURES_OK,
                observed: status,
            });
        }
    }

    Ok(host_features)
}

fn observe_capacity(device: &mut MockPciDevice<'_>) -> ReplayResult<()> {
    device.mmio_read(0, BLK_CONFIG_OFFSET, 4)?;
    device.mmio_read(0, BLK_CONFIG_OFFSET + 4, 4)?;
    Ok(())
}

fn setup_queue(
    device: &mut MockPciDevice<'_>,
    queue_index: u64,
    pfn: u64,
    queue_size: u64,
) -> ReplayResult<()> {
    device.mmio_write(0, QUEUE_SEL_OFFSET, 2, queue_index)?;
    device.mmio_read(0, QUEUE_NUM_OFFSET, 2)?;
    device.mmio_write(0, QUEUE_NUM_OFFSET, 2, queue_size)?;
    device.mmio_write(0, QUEUE_PFN_OFFSET, 4, pfn)?;
    Ok(())
}

fn set_driver_ok(device: &mut MockPciDevice<'_>, host_features: u64) -> ReplayResult<()> {
    let status = if host_features != 0 {
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK
    } else {
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK
    };

    device.mmio_write(0, DEVICE_STATUS_OFFSET, 2, status)
}

/// Replay a driver protocol trace through the distilled init driver.
pub fn replay_init_from_events(events: &[PciReplayEvent]) -> ReplayResult<()> {
    let mut device = MockPciDevice::new(events);
    init_virtio_blk(&mut device)
}

/// Replay the checked-in Reference Vault init trace fixture.
pub fn replay_vault_init_trace() -> Result<(), crate::DriverReplayError> {
    let trace = crate::load_trace(vault_fixture_path())?;
    replay_init_trace(&trace)
}

/// Replay a driver protocol trace file through the distilled init driver.
pub fn replay_init_trace(
    trace: &artifact_store_schema::driver_protocol_trace::DriverProtocolTraceV0,
) -> Result<(), crate::DriverReplayError> {
    let events = crate::trace_to_replay_events(trace)?;
    let mut device = MockPciDevice::new(&events);
    init_virtio_blk(&mut device).map_err(|err| crate::DriverReplayError::Replay(format!("{err:?}")))
}

fn vault_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::mock::pci_device::PciReplayOp;

    #[test]
    fn init_virtio_blk_rejects_wrong_vendor() {
        let trace = [PciReplayEvent::pci_config_read(0x00, 2, 0x1234)];
        let mut device = MockPciDevice::new(&trace);
        let err = init_virtio_blk(&mut device).unwrap_err();
        assert_eq!(
            err,
            ReplayMismatch::Value {
                expected: VIRTIO_VENDOR_ID,
                observed: 0x1234
            }
        );
    }

    #[test]
    fn replay_init_from_events_rejects_incomplete_trace() {
        let trace = [
            PciReplayEvent::pci_config_read(0x00, 2, VIRTIO_VENDOR_ID),
            PciReplayEvent::pci_config_read(0x02, 2, VIRTIO_BLK_DEVICE_ID),
            PciReplayEvent::mmio_read(0, DEVICE_STATUS_OFFSET, 2, 0),
            PciReplayEvent::mmio_write(0, DEVICE_STATUS_OFFSET, 2, STATUS_ACKNOWLEDGE),
        ];
        let err = replay_init_from_events(&trace).unwrap_err();
        assert!(matches!(
            err,
            ReplayMismatch::Incomplete { .. }
                | ReplayMismatch::UnexpectedAccess { .. }
                | ReplayMismatch::Op { .. }
                | ReplayMismatch::Offset { .. }
                | ReplayMismatch::Value { .. }
        ));
    }

    #[test]
    fn replay_vault_init_trace_matches_oracle_fixture() {
        replay_vault_init_trace().expect("vault init trace must replay");
    }

    #[test]
    fn oracle_trace_starts_with_pci_reads() {
        let trace = crate::load_trace(vault_fixture_path()).expect("vault trace");
        let events = crate::trace_to_replay_events(&trace).expect("translate");
        assert!(events.len() > 4);
        assert_eq!(events[0].op, PciReplayOp::PciConfigRead);
        assert_eq!(events[1].op, PciReplayOp::PciConfigRead);
    }
}
