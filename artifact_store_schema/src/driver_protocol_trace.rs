use serde::{Deserialize, Serialize};

use crate::prelude::*;

pub const DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION: u32 = 1;
pub const DRIVER_PROTOCOL_TRACE_KIND: &str = "driver_protocol_trace_v0";

#[derive(Debug)]
pub struct DriverProtocolTraceValidationError(pub String);

impl core::fmt::Display for DriverProtocolTraceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DriverProtocolTraceValidationError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriverProtocolTraceV0 {
    pub schema_version: u32,
    pub metadata: DriverProtocolTraceMetadataV0,
    pub events: Vec<DriverProtocolTraceEventV0>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriverProtocolTraceMetadataV0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule_id: Option<String>,
    pub oracle: String,
    pub device_model: String,
    pub pci_vendor_id: u16,
    pub pci_device_id: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pci_bdf: Option<String>,
    pub capture_tool: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DriverProtocolTraceEventKind {
    PciConfigRead,
    PciConfigWrite,
    MmioRead,
    MmioWrite,
    Irq,
    DmaMap,
    DmaUnmap,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriverProtocolTraceEventV0 {
    pub seq: u64,
    pub kind: DriverProtocolTraceEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ns: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

pub fn validate_driver_protocol_trace(
    trace: &DriverProtocolTraceV0,
) -> Result<(), DriverProtocolTraceValidationError> {
    if trace.schema_version != DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION {
        return Err(DriverProtocolTraceValidationError(format!(
            "driver_protocol_trace schema_version unsupported: {}",
            trace.schema_version
        )));
    }
    validate_metadata(&trace.metadata)?;
    if trace.events.is_empty() {
        return Err(DriverProtocolTraceValidationError(
            "driver_protocol_trace.events empty".into(),
        ));
    }

    let mut last_seq: Option<u64> = None;
    for (idx, event) in trace.events.iter().enumerate() {
        if let Some(prev) = last_seq {
            if event.seq <= prev {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] sequence not monotonic",
                    idx
                )));
            }
        }
        last_seq = Some(event.seq);
        validate_event(idx, event)?;
    }

    Ok(())
}

fn validate_metadata(
    metadata: &DriverProtocolTraceMetadataV0,
) -> Result<(), DriverProtocolTraceValidationError> {
    if metadata.oracle.trim().is_empty() {
        return Err(DriverProtocolTraceValidationError(
            "driver_protocol_trace.metadata.oracle required".into(),
        ));
    }
    if metadata.device_model.trim().is_empty() {
        return Err(DriverProtocolTraceValidationError(
            "driver_protocol_trace.metadata.device_model required".into(),
        ));
    }
    if metadata.capture_tool.trim().is_empty() {
        return Err(DriverProtocolTraceValidationError(
            "driver_protocol_trace.metadata.capture_tool required".into(),
        ));
    }
    if metadata.pci_vendor_id == 0 || metadata.pci_device_id == 0 {
        return Err(DriverProtocolTraceValidationError(
            "driver_protocol_trace.metadata pci ids must be nonzero".into(),
        ));
    }
    Ok(())
}

fn validate_event(
    idx: usize,
    event: &DriverProtocolTraceEventV0,
) -> Result<(), DriverProtocolTraceValidationError> {
    match event.kind {
        DriverProtocolTraceEventKind::PciConfigRead
        | DriverProtocolTraceEventKind::PciConfigWrite
        | DriverProtocolTraceEventKind::MmioRead
        | DriverProtocolTraceEventKind::MmioWrite => {
            let width = event.width.ok_or_else(|| {
                DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] width required",
                    idx
                ))
            })?;
            if !matches!(width, 1 | 2 | 4 | 8) {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] width invalid",
                    idx
                )));
            }
            if event.offset.is_none() {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] offset required",
                    idx
                )));
            }
            if event.value.is_none() {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] value required",
                    idx
                )));
            }
        }
        DriverProtocolTraceEventKind::Irq => {
            if event.value.is_none() {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] irq value required",
                    idx
                )));
            }
        }
        DriverProtocolTraceEventKind::DmaMap | DriverProtocolTraceEventKind::DmaUnmap => {
            if event.bytes_hex.is_none() {
                return Err(DriverProtocolTraceValidationError(format!(
                    "driver_protocol_trace.events[{}] dma bytes_hex required",
                    idx
                )));
            }
        }
    }

    if let Some(bytes_hex) = event.bytes_hex.as_ref() {
        if bytes_hex.len() % 2 != 0 {
            return Err(DriverProtocolTraceValidationError(format!(
                "driver_protocol_trace.events[{}] bytes_hex has odd length",
                idx
            )));
        }
        hex::decode(bytes_hex).map_err(|_| {
            DriverProtocolTraceValidationError(format!(
                "driver_protocol_trace.events[{}] bytes_hex invalid hex",
                idx
            ))
        })?;
    }

    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn virtio_net_trace() -> DriverProtocolTraceV0 {
        DriverProtocolTraceV0 {
            schema_version: 1,
            metadata: DriverProtocolTraceMetadataV0 {
                trace_id: None,
                timestamp_start: None,
                timestamp_end: None,
                capsule_id: Some("linux-oracle".into()),
                oracle: "linux-virtio-net".into(),
                device_model: "virtio-net-pci".into(),
                pci_vendor_id: 0x1af4,
                pci_device_id: 0x1000,
                pci_bdf: Some("0000:00:03.0".into()),
                capture_tool: "pci_mmio_tracer".into(),
            },
            events: vec![
                DriverProtocolTraceEventV0 {
                    seq: 1,
                    kind: DriverProtocolTraceEventKind::PciConfigRead,
                    timestamp_ns: Some(1),
                    bar: None,
                    offset: Some(0x00),
                    width: Some(2),
                    value: Some(0x1af4),
                    bytes_hex: None,
                    result: Some("ok".into()),
                    notes: None,
                },
                DriverProtocolTraceEventV0 {
                    seq: 2,
                    kind: DriverProtocolTraceEventKind::MmioWrite,
                    timestamp_ns: Some(2),
                    bar: Some(0),
                    offset: Some(0x12),
                    width: Some(2),
                    value: Some(0x0001),
                    bytes_hex: None,
                    result: Some("ok".into()),
                    notes: None,
                },
            ],
        }
    }

    #[test]
    fn driver_protocol_trace_validates_virtio_net_oracle() {
        validate_driver_protocol_trace(&virtio_net_trace()).unwrap();
    }

    #[test]
    fn driver_protocol_trace_rejects_non_monotonic_events() {
        let mut trace = virtio_net_trace();
        trace.events[1].seq = 1;

        assert!(validate_driver_protocol_trace(&trace).is_err());
    }

    #[test]
    fn driver_protocol_trace_rejects_invalid_width() {
        let mut trace = virtio_net_trace();
        trace.events[0].width = Some(3);

        assert!(validate_driver_protocol_trace(&trace).is_err());
    }

    #[test]
    fn virtio_net_reference_vault_trace_fixture_validates() {
        let fixture =
            include_str!("../../drivers/reference_vaults/virtio-net/traces/oracle_init_trace.json");
        let trace: DriverProtocolTraceV0 = serde_json::from_str(fixture).unwrap();

        validate_driver_protocol_trace(&trace).unwrap();
        assert_eq!(trace.metadata.device_model, "virtio-net-pci");
    }

    #[test]
    fn virtio_blk_reference_vault_trace_fixture_validates() {
        let fixture =
            include_str!("../../drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json");
        let trace: DriverProtocolTraceV0 = serde_json::from_str(fixture).unwrap();

        validate_driver_protocol_trace(&trace).unwrap();
        assert_eq!(trace.metadata.device_model, "virtio-blk-pci");
        assert_eq!(trace.metadata.pci_device_id, 0x1001);
    }
}
