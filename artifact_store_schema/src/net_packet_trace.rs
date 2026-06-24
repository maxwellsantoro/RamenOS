use serde::{Deserialize, Serialize};

use crate::prelude::*;

pub const NET_PACKET_TRACE_SCHEMA_VERSION: u32 = 1;
pub const NET_PACKET_TRACE_KIND: &str = "net_packet_trace_v0";

#[derive(Debug)]
pub struct NetPacketTraceValidationError(pub String);

impl core::fmt::Display for NetPacketTraceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NetPacketTraceValidationError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetPacketTraceV0 {
    pub schema_version: u32,
    pub metadata: NetPacketTraceMetadataV0,
    pub events: Vec<NetPacketTraceEventV0>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetPacketTraceMetadataV0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<String>,
    pub oracle: String,
    pub device_model: String,
    pub harness: String,
    pub harness_version: String,
    pub capture_tool: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NetPacketTraceEventKind {
    SendPacket,
    ReceivePacket,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetPacketTraceEventV0 {
    pub seq: u64,
    pub kind: NetPacketTraceEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ns: Option<u64>,
    pub request_id: u64,
    pub shm_cap: u64,
    pub offset: u64,
    pub len: u32,
    pub payload_hex: String,
    pub status: u32,
    pub bytes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

pub fn validate_net_packet_trace(
    trace: &NetPacketTraceV0,
) -> Result<(), NetPacketTraceValidationError> {
    if trace.schema_version != NET_PACKET_TRACE_SCHEMA_VERSION {
        return Err(NetPacketTraceValidationError(format!(
            "net_packet_trace schema_version unsupported: {}",
            trace.schema_version
        )));
    }
    validate_metadata(&trace.metadata)?;
    if trace.events.is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.events empty".into(),
        ));
    }

    let mut last_seq: Option<u64> = None;
    for (idx, event) in trace.events.iter().enumerate() {
        if let Some(prev) = last_seq {
            if event.seq <= prev {
                return Err(NetPacketTraceValidationError(format!(
                    "net_packet_trace.events[{}] sequence not monotonic",
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
    metadata: &NetPacketTraceMetadataV0,
) -> Result<(), NetPacketTraceValidationError> {
    if metadata.oracle.trim().is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.metadata.oracle required".into(),
        ));
    }
    if metadata.device_model.trim().is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.metadata.device_model required".into(),
        ));
    }
    if metadata.harness.trim().is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.metadata.harness required".into(),
        ));
    }
    if metadata.harness_version.trim().is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.metadata.harness_version required".into(),
        ));
    }
    if metadata.capture_tool.trim().is_empty() {
        return Err(NetPacketTraceValidationError(
            "net_packet_trace.metadata.capture_tool required".into(),
        ));
    }
    Ok(())
}

fn validate_event(
    idx: usize,
    event: &NetPacketTraceEventV0,
) -> Result<(), NetPacketTraceValidationError> {
    if event.len == 0 {
        return Err(NetPacketTraceValidationError(format!(
            "net_packet_trace.events[{}] len must be nonzero",
            idx
        )));
    }
    if !event.payload_hex.len().is_multiple_of(2) {
        return Err(NetPacketTraceValidationError(format!(
            "net_packet_trace.events[{}] payload_hex has odd length",
            idx
        )));
    }
    let payload = hex::decode(&event.payload_hex).map_err(|_| {
        NetPacketTraceValidationError(format!(
            "net_packet_trace.events[{}] payload_hex invalid hex",
            idx
        ))
    })?;
    if payload.len() != event.len as usize {
        return Err(NetPacketTraceValidationError(format!(
            "net_packet_trace.events[{}] payload_hex length must match len",
            idx
        )));
    }
    if event.bytes > event.len {
        return Err(NetPacketTraceValidationError(format!(
            "net_packet_trace.events[{}] bytes exceeds len",
            idx
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn virtio_net_packet_trace() -> NetPacketTraceV0 {
        NetPacketTraceV0 {
            schema_version: 1,
            metadata: NetPacketTraceMetadataV0 {
                trace_id: Some("virtio-net-s11-packet-scaffold".into()),
                timestamp_start: None,
                timestamp_end: None,
                oracle: "linux-virtio-net".into(),
                device_model: "virtio-net-pci".into(),
                harness: "harness.net".into(),
                harness_version: "1".into(),
                capture_tool: "harness_packet_scaffold".into(),
            },
            events: vec![
                NetPacketTraceEventV0 {
                    seq: 1,
                    kind: NetPacketTraceEventKind::SendPacket,
                    timestamp_ns: Some(1),
                    request_id: 1,
                    shm_cap: 0x1000,
                    offset: 0,
                    len: 2,
                    payload_hex: "aabb".into(),
                    status: 0,
                    bytes: 2,
                    result: Some("ok".into()),
                    notes: None,
                },
                NetPacketTraceEventV0 {
                    seq: 2,
                    kind: NetPacketTraceEventKind::ReceivePacket,
                    timestamp_ns: Some(2),
                    request_id: 2,
                    shm_cap: 0x1000,
                    offset: 2048,
                    len: 4,
                    payload_hex: "ccddeeff".into(),
                    status: 0,
                    bytes: 4,
                    result: Some("ok".into()),
                    notes: None,
                },
            ],
        }
    }

    #[test]
    fn net_packet_trace_validates_virtio_net_scaffold() {
        validate_net_packet_trace(&virtio_net_packet_trace()).unwrap();
    }

    #[test]
    fn net_packet_trace_rejects_payload_len_mismatch() {
        let mut trace = virtio_net_packet_trace();
        trace.events[0].len = 3;

        assert!(validate_net_packet_trace(&trace).is_err());
    }

    #[test]
    fn virtio_net_reference_vault_packet_fixture_validates() {
        let fixture = include_str!(
            "../../drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json"
        );
        let trace: NetPacketTraceV0 = serde_json::from_str(fixture).unwrap();

        validate_net_packet_trace(&trace).unwrap();
        assert_eq!(trace.metadata.harness, "harness.net");
    }
}
