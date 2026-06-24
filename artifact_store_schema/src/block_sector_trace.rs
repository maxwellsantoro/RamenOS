use serde::{Deserialize, Serialize};

use crate::prelude::*;

pub const BLOCK_SECTOR_TRACE_SCHEMA_VERSION: u32 = 1;
pub const BLOCK_SECTOR_TRACE_KIND: &str = "block_sector_trace_v0";

#[derive(Debug)]
pub struct BlockSectorTraceValidationError(pub String);

impl core::fmt::Display for BlockSectorTraceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BlockSectorTraceValidationError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockSectorTraceV0 {
    pub schema_version: u32,
    pub metadata: BlockSectorTraceMetadataV0,
    pub events: Vec<BlockSectorTraceEventV0>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockSectorTraceMetadataV0 {
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
pub enum BlockSectorTraceEventKind {
    ReadBlocks,
    WriteBlocks,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockSectorTraceEventV0 {
    pub seq: u64,
    pub kind: BlockSectorTraceEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ns: Option<u64>,
    pub request_id: u64,
    pub lba: u64,
    pub block_count: u32,
    pub block_size: u32,
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

pub fn validate_block_sector_trace(
    trace: &BlockSectorTraceV0,
) -> Result<(), BlockSectorTraceValidationError> {
    if trace.schema_version != BLOCK_SECTOR_TRACE_SCHEMA_VERSION {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace schema_version unsupported: {}",
            trace.schema_version
        )));
    }
    validate_metadata(&trace.metadata)?;
    if trace.events.is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.events empty".into(),
        ));
    }

    let mut last_seq: Option<u64> = None;
    for (idx, event) in trace.events.iter().enumerate() {
        if let Some(prev) = last_seq {
            if event.seq <= prev {
                return Err(BlockSectorTraceValidationError(format!(
                    "block_sector_trace.events[{}] sequence not monotonic",
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
    metadata: &BlockSectorTraceMetadataV0,
) -> Result<(), BlockSectorTraceValidationError> {
    if metadata.oracle.trim().is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.metadata.oracle required".into(),
        ));
    }
    if metadata.device_model.trim().is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.metadata.device_model required".into(),
        ));
    }
    if metadata.harness.trim().is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.metadata.harness required".into(),
        ));
    }
    if metadata.harness_version.trim().is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.metadata.harness_version required".into(),
        ));
    }
    if metadata.capture_tool.trim().is_empty() {
        return Err(BlockSectorTraceValidationError(
            "block_sector_trace.metadata.capture_tool required".into(),
        ));
    }
    Ok(())
}

fn validate_event(
    idx: usize,
    event: &BlockSectorTraceEventV0,
) -> Result<(), BlockSectorTraceValidationError> {
    if event.block_count == 0 || event.block_size == 0 {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] block_count and block_size must be nonzero",
            idx
        )));
    }
    let expected_len = event
        .block_size
        .checked_mul(event.block_count)
        .ok_or_else(|| {
            BlockSectorTraceValidationError(format!(
                "block_sector_trace.events[{}] block_size * block_count overflow",
                idx
            ))
        })?;
    if event.len != expected_len {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] len must equal block_size * block_count",
            idx
        )));
    }
    if !event.payload_hex.len().is_multiple_of(2) {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] payload_hex has odd length",
            idx
        )));
    }
    let payload = hex::decode(&event.payload_hex).map_err(|_| {
        BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] payload_hex invalid hex",
            idx
        ))
    })?;
    if payload.len() != event.len as usize {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] payload_hex length must match len",
            idx
        )));
    }
    if event.bytes > event.len {
        return Err(BlockSectorTraceValidationError(format!(
            "block_sector_trace.events[{}] bytes exceeds len",
            idx
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn virtio_blk_sector_trace() -> BlockSectorTraceV0 {
        BlockSectorTraceV0 {
            schema_version: 1,
            metadata: BlockSectorTraceMetadataV0 {
                trace_id: Some("virtio-blk-s13-sector-scaffold".into()),
                timestamp_start: None,
                timestamp_end: None,
                oracle: "linux-virtio-blk".into(),
                device_model: "virtio-blk-pci".into(),
                harness: "harness.block".into(),
                harness_version: "1".into(),
                capture_tool: "harness_sector_scaffold".into(),
            },
            events: vec![
                BlockSectorTraceEventV0 {
                    seq: 1,
                    kind: BlockSectorTraceEventKind::ReadBlocks,
                    timestamp_ns: Some(1),
                    request_id: 1,
                    lba: 0,
                    block_count: 1,
                    block_size: 512,
                    shm_cap: 4096,
                    offset: 0,
                    len: 512,
                    payload_hex: "00".repeat(512),
                    status: 0,
                    bytes: 512,
                    result: Some("ok".into()),
                    notes: None,
                },
                BlockSectorTraceEventV0 {
                    seq: 2,
                    kind: BlockSectorTraceEventKind::WriteBlocks,
                    timestamp_ns: Some(2),
                    request_id: 2,
                    lba: 1,
                    block_count: 1,
                    block_size: 512,
                    shm_cap: 4096,
                    offset: 512,
                    len: 512,
                    payload_hex: "ff".repeat(512),
                    status: 0,
                    bytes: 512,
                    result: Some("ok".into()),
                    notes: None,
                },
            ],
        }
    }

    #[test]
    fn block_sector_trace_validates_virtio_blk_scaffold() {
        validate_block_sector_trace(&virtio_blk_sector_trace()).unwrap();
    }

    #[test]
    fn block_sector_trace_rejects_len_mismatch() {
        let mut trace = virtio_blk_sector_trace();
        trace.events[0].len = 256;

        assert!(validate_block_sector_trace(&trace).is_err());
    }

    #[test]
    fn virtio_blk_reference_vault_sector_fixture_validates() {
        let fixture = include_str!(
            "../../drivers/reference_vaults/virtio-blk/traces/oracle_block_trace.json"
        );
        let trace: BlockSectorTraceV0 = serde_json::from_str(fixture).unwrap();

        validate_block_sector_trace(&trace).unwrap();
        assert_eq!(trace.metadata.harness, "harness.block");
    }
}
