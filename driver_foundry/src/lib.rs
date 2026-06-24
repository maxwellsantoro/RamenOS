pub mod virtio_blk_init;
pub mod virtio_blk_sector;
pub mod virtio_net_init;
pub mod virtio_net_packet;

use std::fs;
use std::path::Path;

use artifact_store_schema::block_sector_trace::{
    BLOCK_SECTOR_TRACE_SCHEMA_VERSION, BlockSectorTraceEventKind, BlockSectorTraceEventV0,
    BlockSectorTraceMetadataV0, BlockSectorTraceV0, validate_block_sector_trace,
};
use artifact_store_schema::driver_protocol_trace::{
    DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION, DriverProtocolTraceEventKind, DriverProtocolTraceEventV0,
    DriverProtocolTraceMetadataV0, DriverProtocolTraceV0, validate_driver_protocol_trace,
};
use artifact_store_schema::net_packet_trace::{
    NET_PACKET_TRACE_SCHEMA_VERSION, NetPacketTraceEventKind, NetPacketTraceEventV0,
    NetPacketTraceMetadataV0, NetPacketTraceV0, validate_net_packet_trace,
};
use kernel_api::mock::block_harness::{BlockReplayEvent, BlockReplayOp};
use kernel_api::mock::packet_harness::{PacketReplayEvent, PacketReplayOp};
use kernel_api::mock::pci_device::{MockPciDevice, PciReplayEvent};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub enum DriverReplayError {
    Io(String),
    Json(String),
    Schema(String),
    MissingMetadata,
    EmptyJsonl,
    UnsupportedEvent {
        seq: u64,
        kind: DriverProtocolTraceEventKind,
    },
    InvalidPciConfigBar {
        seq: u64,
        bar: u8,
    },
    MissingBar {
        seq: u64,
    },
    MissingField {
        seq: u64,
        field: &'static str,
    },
    Replay(String),
    PacketReplay(String),
    InvalidPacketPayload {
        seq: u64,
    },
    MissingPacketField {
        seq: u64,
        field: &'static str,
    },
    UnsupportedPacketEvent {
        seq: u64,
        kind: NetPacketTraceEventKind,
    },
    MissingLiveTraceId,
    InvalidLiveTraceId {
        trace_id: String,
    },
    ScaffoldTrace {
        trace_id: Option<String>,
    },
    MissingLiveTimestamp {
        seq: u64,
    },
    NonContiguousLiveSeq {
        expected: u64,
        observed: u64,
    },
    DerivedPacketReceive {
        seq: u64,
        notes: String,
    },
}

impl std::fmt::Display for DriverReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::Schema(err) => write!(f, "schema error: {err}"),
            Self::MissingMetadata => write!(f, "jsonl trace missing metadata record"),
            Self::EmptyJsonl => write!(f, "jsonl trace is empty"),
            Self::UnsupportedEvent { seq, kind } => {
                write!(f, "unsupported replay event seq={seq} kind={kind:?}")
            }
            Self::InvalidPciConfigBar { seq, bar } => {
                write!(f, "invalid pci config bar seq={seq} bar={bar}")
            }
            Self::MissingBar { seq } => write!(f, "missing mmio bar seq={seq}"),
            Self::MissingField { seq, field } => {
                write!(f, "missing replay field seq={seq} field={field}")
            }
            Self::Replay(err) => write!(f, "replay mismatch: {err}"),
            Self::PacketReplay(err) => write!(f, "packet replay mismatch: {err}"),
            Self::InvalidPacketPayload { seq } => {
                write!(f, "packet trace payload invalid seq={seq}")
            }
            Self::MissingPacketField { seq, field } => {
                write!(f, "missing packet replay field seq={seq} field={field}")
            }
            Self::UnsupportedPacketEvent { seq, kind } => {
                write!(f, "unsupported packet replay event seq={seq} kind={kind:?}")
            }
            Self::MissingLiveTraceId => write!(f, "live trace missing trace_id"),
            Self::InvalidLiveTraceId { trace_id } => {
                write!(
                    f,
                    "live trace_id must use sha256 provenance trace_id={trace_id:?}"
                )
            }
            Self::ScaffoldTrace { trace_id } => {
                write!(f, "trace is still a scaffold fixture trace_id={trace_id:?}")
            }
            Self::MissingLiveTimestamp { seq } => {
                write!(f, "live trace event missing timestamp_ns seq={seq}")
            }
            Self::NonContiguousLiveSeq { expected, observed } => {
                write!(
                    f,
                    "live trace event sequence gap expected_seq={expected} observed_seq={observed}"
                )
            }
            Self::DerivedPacketReceive { seq, notes } => {
                write!(
                    f,
                    "packet receive event is not hardware-captured seq={seq} notes={notes:?}"
                )
            }
        }
    }
}

impl std::error::Error for DriverReplayError {}

pub struct PacketReplayBundle {
    payloads: Vec<Vec<u8>>,
    specs: Vec<PacketReplaySpec>,
    shm_cap: u64,
}

struct PacketReplaySpec {
    op: PacketReplayOp,
    request_id: u64,
    shm_cap: u64,
    offset: u64,
    len: u32,
    payload_index: usize,
    status: u32,
    bytes: u32,
}

impl PacketReplayBundle {
    pub fn shm_cap(&self) -> u64 {
        self.shm_cap
    }

    pub fn with_events<R>(&self, f: impl FnOnce(&[PacketReplayEvent<'_>]) -> R) -> R {
        let mut events = Vec::with_capacity(self.specs.len());
        for spec in &self.specs {
            events.push(PacketReplayEvent {
                op: spec.op,
                request_id: spec.request_id,
                shm_cap: spec.shm_cap,
                offset: spec.offset,
                len: spec.len,
                payload: &self.payloads[spec.payload_index],
                status: spec.status,
                bytes: spec.bytes,
            });
        }
        f(&events)
    }
}

pub fn load_packet_trace(path: impl AsRef<Path>) -> Result<NetPacketTraceV0, DriverReplayError> {
    let bytes = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let trace: NetPacketTraceV0 =
        serde_json::from_str(&bytes).map_err(|err| DriverReplayError::Json(err.to_string()))?;
    validate_net_packet_trace(&trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn packet_trace_to_replay_bundle(
    trace: &NetPacketTraceV0,
) -> Result<PacketReplayBundle, DriverReplayError> {
    validate_net_packet_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;

    let mut payloads = Vec::with_capacity(trace.events.len());
    let mut specs = Vec::with_capacity(trace.events.len());
    let shm_cap =
        trace
            .events
            .first()
            .map(|event| event.shm_cap)
            .ok_or(DriverReplayError::Schema(
                "net_packet_trace.events empty".into(),
            ))?;

    for event in &trace.events {
        let payload = hex::decode(&event.payload_hex)
            .map_err(|_| DriverReplayError::InvalidPacketPayload { seq: event.seq })?;
        if payload.len() != event.len as usize {
            return Err(DriverReplayError::InvalidPacketPayload { seq: event.seq });
        }

        let op = match event.kind {
            NetPacketTraceEventKind::SendPacket => PacketReplayOp::SendPacket,
            NetPacketTraceEventKind::ReceivePacket => PacketReplayOp::ReceivePacket,
        };

        let payload_index = payloads.len();
        payloads.push(payload);
        specs.push(PacketReplaySpec {
            op,
            request_id: event.request_id,
            shm_cap: event.shm_cap,
            offset: event.offset,
            len: event.len,
            payload_index,
            status: event.status,
            bytes: event.bytes,
        });
    }

    Ok(PacketReplayBundle {
        payloads,
        specs,
        shm_cap,
    })
}

pub fn replay_packet_trace_fixture(path: impl AsRef<Path>) -> Result<usize, DriverReplayError> {
    let trace = load_packet_trace(path)?;
    virtio_net_packet::replay_packet_trace(&trace)?;
    Ok(trace.events.len())
}

pub struct SectorReplayBundle {
    payloads: Vec<Vec<u8>>,
    specs: Vec<SectorReplaySpec>,
    shm_cap: u64,
}

struct SectorReplaySpec {
    op: BlockReplayOp,
    request_id: u64,
    lba: u64,
    block_count: u32,
    block_size: u32,
    shm_cap: u64,
    offset: u64,
    len: u32,
    payload_index: usize,
    status: u32,
    bytes: u32,
}

impl SectorReplayBundle {
    pub fn shm_cap(&self) -> u64 {
        self.shm_cap
    }

    pub fn with_events<R>(&self, f: impl FnOnce(&[BlockReplayEvent<'_>]) -> R) -> R {
        let mut events = Vec::with_capacity(self.specs.len());
        for spec in &self.specs {
            events.push(BlockReplayEvent {
                op: spec.op,
                request_id: spec.request_id,
                lba: spec.lba,
                block_count: spec.block_count,
                block_size: spec.block_size,
                shm_cap: spec.shm_cap,
                offset: spec.offset,
                len: spec.len,
                payload: &self.payloads[spec.payload_index],
                status: spec.status,
                bytes: spec.bytes,
            });
        }
        f(&events)
    }
}

pub fn load_sector_trace(path: impl AsRef<Path>) -> Result<BlockSectorTraceV0, DriverReplayError> {
    let bytes = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let trace: BlockSectorTraceV0 =
        serde_json::from_str(&bytes).map_err(|err| DriverReplayError::Json(err.to_string()))?;
    validate_block_sector_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn sector_trace_to_replay_bundle(
    trace: &BlockSectorTraceV0,
) -> Result<SectorReplayBundle, DriverReplayError> {
    validate_block_sector_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;

    let mut payloads = Vec::with_capacity(trace.events.len());
    let mut specs = Vec::with_capacity(trace.events.len());
    let shm_cap =
        trace
            .events
            .first()
            .map(|event| event.shm_cap)
            .ok_or(DriverReplayError::Schema(
                "block_sector_trace.events empty".into(),
            ))?;

    for event in &trace.events {
        let payload = hex::decode(&event.payload_hex)
            .map_err(|_| DriverReplayError::InvalidPacketPayload { seq: event.seq })?;
        if payload.len() != event.len as usize {
            return Err(DriverReplayError::InvalidPacketPayload { seq: event.seq });
        }

        let op = match event.kind {
            BlockSectorTraceEventKind::ReadBlocks => BlockReplayOp::ReadBlocks,
            BlockSectorTraceEventKind::WriteBlocks => BlockReplayOp::WriteBlocks,
        };

        let payload_index = payloads.len();
        payloads.push(payload);
        specs.push(SectorReplaySpec {
            op,
            request_id: event.request_id,
            lba: event.lba,
            block_count: event.block_count,
            block_size: event.block_size,
            shm_cap: event.shm_cap,
            offset: event.offset,
            len: event.len,
            payload_index,
            status: event.status,
            bytes: event.bytes,
        });
    }

    Ok(SectorReplayBundle {
        payloads,
        specs,
        shm_cap,
    })
}

pub fn replay_sector_trace_fixture(path: impl AsRef<Path>) -> Result<usize, DriverReplayError> {
    let trace = load_sector_trace(path)?;
    virtio_blk_sector::replay_sector_trace(&trace)?;
    Ok(trace.events.len())
}

pub fn load_sector_jsonl(path: impl AsRef<Path>) -> Result<BlockSectorTraceV0, DriverReplayError> {
    let text = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let mut trace = sector_jsonl_to_trace(&text)?;
    stamp_missing_sector_trace_id(&mut trace, text.as_bytes());
    validate_block_sector_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn write_sector_trace_json(
    trace: &BlockSectorTraceV0,
    path: impl AsRef<Path>,
) -> Result<(), DriverReplayError> {
    validate_block_sector_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let json = serde_json::to_string_pretty(trace)
        .map_err(|err| DriverReplayError::Json(err.to_string()))?;
    fs::write(path, format!("{json}\n")).map_err(|err| DriverReplayError::Io(err.to_string()))
}

pub fn sector_jsonl_to_trace(text: &str) -> Result<BlockSectorTraceV0, DriverReplayError> {
    let mut metadata: Option<BlockSectorTraceMetadataV0> = None;
    let mut events = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|err| DriverReplayError::Json(err.to_string()))?;
        if let Some(meta) = value.get("metadata") {
            metadata = Some(
                serde_json::from_value(meta.clone())
                    .map_err(|err| DriverReplayError::Json(err.to_string()))?,
            );
            continue;
        }

        let event: BlockSectorTraceEventV0 = serde_json::from_value(value)
            .map_err(|err| DriverReplayError::Json(err.to_string()))?;
        events.push(event);
    }

    if metadata.is_none() && events.is_empty() {
        return Err(DriverReplayError::EmptyJsonl);
    }

    let trace = BlockSectorTraceV0 {
        schema_version: BLOCK_SECTOR_TRACE_SCHEMA_VERSION,
        metadata: metadata.ok_or(DriverReplayError::MissingMetadata)?,
        events,
    };
    validate_block_sector_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn assert_live_sector_trace(trace: &BlockSectorTraceV0) -> Result<(), DriverReplayError> {
    validate_block_sector_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let trace_id = trace
        .metadata
        .trace_id
        .as_deref()
        .ok_or(DriverReplayError::MissingLiveTraceId)?;
    if trace_id.contains("scaffold") {
        return Err(DriverReplayError::ScaffoldTrace {
            trace_id: trace.metadata.trace_id.clone(),
        });
    }
    if !is_sha256_trace_id(trace_id) {
        return Err(DriverReplayError::InvalidLiveTraceId {
            trace_id: trace_id.into(),
        });
    }
    let mut expected_seq = 1;
    for event in &trace.events {
        if event.seq != expected_seq {
            return Err(DriverReplayError::NonContiguousLiveSeq {
                expected: expected_seq,
                observed: event.seq,
            });
        }
        if event.timestamp_ns.is_none() {
            return Err(DriverReplayError::MissingLiveTimestamp { seq: event.seq });
        }
        expected_seq += 1;
    }
    Ok(())
}

pub fn assert_live_sector_trace_file(path: impl AsRef<Path>) -> Result<(), DriverReplayError> {
    let trace = load_sector_trace(path)?;
    assert_live_sector_trace(&trace)
}

pub fn stamp_missing_sector_trace_id(trace: &mut BlockSectorTraceV0, source: &[u8]) {
    if trace.metadata.trace_id.is_none() {
        trace.metadata.trace_id = Some(source_trace_id(source));
    }
}

pub fn load_packet_jsonl(path: impl AsRef<Path>) -> Result<NetPacketTraceV0, DriverReplayError> {
    let text = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let mut trace = packet_jsonl_to_trace(&text)?;
    stamp_missing_packet_trace_id(&mut trace, text.as_bytes());
    validate_net_packet_trace(&trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn write_packet_trace_json(
    trace: &NetPacketTraceV0,
    path: impl AsRef<Path>,
) -> Result<(), DriverReplayError> {
    validate_net_packet_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let json = serde_json::to_string_pretty(trace)
        .map_err(|err| DriverReplayError::Json(err.to_string()))?;
    fs::write(path, format!("{json}\n")).map_err(|err| DriverReplayError::Io(err.to_string()))
}

pub fn packet_jsonl_to_trace(text: &str) -> Result<NetPacketTraceV0, DriverReplayError> {
    let mut metadata: Option<NetPacketTraceMetadataV0> = None;
    let mut events = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|err| DriverReplayError::Json(err.to_string()))?;
        if let Some(meta) = value.get("metadata") {
            metadata = Some(
                serde_json::from_value(meta.clone())
                    .map_err(|err| DriverReplayError::Json(err.to_string()))?,
            );
            continue;
        }

        let event: NetPacketTraceEventV0 = serde_json::from_value(value)
            .map_err(|err| DriverReplayError::Json(err.to_string()))?;
        events.push(event);
    }

    if metadata.is_none() && events.is_empty() {
        return Err(DriverReplayError::EmptyJsonl);
    }

    let trace = NetPacketTraceV0 {
        schema_version: NET_PACKET_TRACE_SCHEMA_VERSION,
        metadata: metadata.ok_or(DriverReplayError::MissingMetadata)?,
        events,
    };
    validate_net_packet_trace(&trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn assert_live_packet_trace(trace: &NetPacketTraceV0) -> Result<(), DriverReplayError> {
    validate_net_packet_trace(trace).map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let trace_id = trace
        .metadata
        .trace_id
        .as_deref()
        .ok_or(DriverReplayError::MissingLiveTraceId)?;
    if trace_id.contains("scaffold") {
        return Err(DriverReplayError::ScaffoldTrace {
            trace_id: trace.metadata.trace_id.clone(),
        });
    }
    if !is_sha256_trace_id(trace_id) {
        return Err(DriverReplayError::InvalidLiveTraceId {
            trace_id: trace_id.into(),
        });
    }
    let mut expected_seq = 1;
    for event in &trace.events {
        if event.seq != expected_seq {
            return Err(DriverReplayError::NonContiguousLiveSeq {
                expected: expected_seq,
                observed: event.seq,
            });
        }
        if event.timestamp_ns.is_none() {
            return Err(DriverReplayError::MissingLiveTimestamp { seq: event.seq });
        }
        expected_seq += 1;
    }
    Ok(())
}

pub fn assert_live_packet_trace_file(path: impl AsRef<Path>) -> Result<(), DriverReplayError> {
    let trace = load_packet_trace(path)?;
    assert_live_packet_trace(&trace)
}

pub fn assert_hardware_packet_rx(trace: &NetPacketTraceV0) -> Result<(), DriverReplayError> {
    assert_live_packet_trace(trace)?;
    for event in &trace.events {
        if event.kind != NetPacketTraceEventKind::ReceivePacket {
            continue;
        }
        let notes = event.notes.as_deref().unwrap_or("");
        if notes.contains("slirp-arp-reply-derived") {
            return Err(DriverReplayError::DerivedPacketReceive {
                seq: event.seq,
                notes: notes.into(),
            });
        }
    }
    Ok(())
}

pub fn assert_hardware_packet_rx_file(path: impl AsRef<Path>) -> Result<(), DriverReplayError> {
    let trace = load_packet_trace(path)?;
    assert_hardware_packet_rx(&trace)
}

pub fn stamp_missing_packet_trace_id(trace: &mut NetPacketTraceV0, source: &[u8]) {
    if trace.metadata.trace_id.is_none() {
        trace.metadata.trace_id = Some(source_trace_id(source));
    }
}

pub fn load_trace(path: impl AsRef<Path>) -> Result<DriverProtocolTraceV0, DriverReplayError> {
    let bytes = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let trace: DriverProtocolTraceV0 =
        serde_json::from_str(&bytes).map_err(|err| DriverReplayError::Json(err.to_string()))?;
    validate_driver_protocol_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn load_tracer_jsonl(
    path: impl AsRef<Path>,
) -> Result<DriverProtocolTraceV0, DriverReplayError> {
    let text = fs::read_to_string(path).map_err(|err| DriverReplayError::Io(err.to_string()))?;
    let mut trace = tracer_jsonl_to_trace(&text)?;
    stamp_missing_trace_id(&mut trace, text.as_bytes());
    validate_driver_protocol_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn write_trace_json(
    trace: &DriverProtocolTraceV0,
    path: impl AsRef<Path>,
) -> Result<(), DriverReplayError> {
    validate_driver_protocol_trace(trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let json = serde_json::to_string_pretty(trace)
        .map_err(|err| DriverReplayError::Json(err.to_string()))?;
    fs::write(path, format!("{json}\n")).map_err(|err| DriverReplayError::Io(err.to_string()))
}

pub fn tracer_jsonl_to_trace(text: &str) -> Result<DriverProtocolTraceV0, DriverReplayError> {
    let mut metadata: Option<DriverProtocolTraceMetadataV0> = None;
    let mut events = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|err| DriverReplayError::Json(err.to_string()))?;
        if let Some(meta) = value.get("metadata") {
            metadata = Some(
                serde_json::from_value(meta.clone())
                    .map_err(|err| DriverReplayError::Json(err.to_string()))?,
            );
            continue;
        }

        let event: DriverProtocolTraceEventV0 = serde_json::from_value(value)
            .map_err(|err| DriverReplayError::Json(err.to_string()))?;
        events.push(event);
    }

    if metadata.is_none() && events.is_empty() {
        return Err(DriverReplayError::EmptyJsonl);
    }

    let trace = DriverProtocolTraceV0 {
        schema_version: DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION,
        metadata: metadata.ok_or(DriverReplayError::MissingMetadata)?,
        events,
    };
    validate_driver_protocol_trace(&trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    Ok(trace)
}

pub fn assert_live_oracle_trace(trace: &DriverProtocolTraceV0) -> Result<(), DriverReplayError> {
    validate_driver_protocol_trace(trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;
    let trace_id = trace
        .metadata
        .trace_id
        .as_deref()
        .ok_or(DriverReplayError::MissingLiveTraceId)?;
    if trace_id.contains("scaffold") {
        return Err(DriverReplayError::ScaffoldTrace {
            trace_id: trace.metadata.trace_id.clone(),
        });
    }
    if !is_sha256_trace_id(trace_id) {
        return Err(DriverReplayError::InvalidLiveTraceId {
            trace_id: trace_id.into(),
        });
    }
    let mut expected_seq = 1;
    for event in &trace.events {
        if event.seq != expected_seq {
            return Err(DriverReplayError::NonContiguousLiveSeq {
                expected: expected_seq,
                observed: event.seq,
            });
        }
        if event.timestamp_ns.is_none() {
            return Err(DriverReplayError::MissingLiveTimestamp { seq: event.seq });
        }
        expected_seq += 1;
    }
    Ok(())
}

pub fn stamp_missing_trace_id(trace: &mut DriverProtocolTraceV0, source: &[u8]) {
    if trace.metadata.trace_id.is_none() {
        trace.metadata.trace_id = Some(source_trace_id(source));
    }
}

pub fn source_trace_id(source: &[u8]) -> String {
    let digest = Sha256::digest(source);
    let mut trace_id = String::from("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut trace_id, "{byte:02x}");
    }
    trace_id
}

fn is_sha256_trace_id(trace_id: &str) -> bool {
    let Some(digest) = trace_id.strip_prefix("sha256:") else {
        return false;
    };
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn trace_to_replay_events(
    trace: &DriverProtocolTraceV0,
) -> Result<Vec<PciReplayEvent>, DriverReplayError> {
    validate_driver_protocol_trace(trace)
        .map_err(|err| DriverReplayError::Schema(err.to_string()))?;

    trace
        .events
        .iter()
        .map(|event| {
            if matches!(
                event.kind,
                DriverProtocolTraceEventKind::Irq
                    | DriverProtocolTraceEventKind::DmaMap
                    | DriverProtocolTraceEventKind::DmaUnmap
            ) {
                return Err(DriverReplayError::UnsupportedEvent {
                    seq: event.seq,
                    kind: event.kind.clone(),
                });
            }

            let offset = event.offset.ok_or(DriverReplayError::MissingField {
                seq: event.seq,
                field: "offset",
            })?;
            let width = event.width.ok_or(DriverReplayError::MissingField {
                seq: event.seq,
                field: "width",
            })?;
            let value = event.value.ok_or(DriverReplayError::MissingField {
                seq: event.seq,
                field: "value",
            })?;

            match event.kind {
                DriverProtocolTraceEventKind::PciConfigRead => {
                    validate_pci_config_bar(event.seq, event.bar)?;
                    Ok(PciReplayEvent::pci_config_read(offset, width, value))
                }
                DriverProtocolTraceEventKind::PciConfigWrite => {
                    validate_pci_config_bar(event.seq, event.bar)?;
                    Ok(PciReplayEvent::pci_config_write(offset, width, value))
                }
                DriverProtocolTraceEventKind::MmioRead => {
                    let bar = event
                        .bar
                        .ok_or(DriverReplayError::MissingBar { seq: event.seq })?;
                    Ok(PciReplayEvent::mmio_read(bar, offset, width, value))
                }
                DriverProtocolTraceEventKind::MmioWrite => {
                    let bar = event
                        .bar
                        .ok_or(DriverReplayError::MissingBar { seq: event.seq })?;
                    Ok(PciReplayEvent::mmio_write(bar, offset, width, value))
                }
                DriverProtocolTraceEventKind::Irq
                | DriverProtocolTraceEventKind::DmaMap
                | DriverProtocolTraceEventKind::DmaUnmap => unreachable!(),
            }
        })
        .collect()
}

pub fn replay_events_through_mock_device(
    events: &[PciReplayEvent],
) -> Result<usize, DriverReplayError> {
    let mut device = MockPciDevice::new(events);
    for event in events {
        match event.op {
            kernel_api::mock::pci_device::PciReplayOp::PciConfigRead => {
                let value = device
                    .pci_config_read(event.offset, event.width)
                    .map_err(|err| DriverReplayError::Replay(format!("{err:?}")))?;
                if value != event.value {
                    return Err(DriverReplayError::Replay(format!(
                        "read value mismatch expected={} observed={}",
                        event.value, value
                    )));
                }
            }
            kernel_api::mock::pci_device::PciReplayOp::PciConfigWrite => device
                .pci_config_write(event.offset, event.width, event.value)
                .map_err(|err| DriverReplayError::Replay(format!("{err:?}")))?,
            kernel_api::mock::pci_device::PciReplayOp::MmioRead => {
                let value = device
                    .mmio_read(event.bar, event.offset, event.width)
                    .map_err(|err| DriverReplayError::Replay(format!("{err:?}")))?;
                if value != event.value {
                    return Err(DriverReplayError::Replay(format!(
                        "read value mismatch expected={} observed={}",
                        event.value, value
                    )));
                }
            }
            kernel_api::mock::pci_device::PciReplayOp::MmioWrite => device
                .mmio_write(event.bar, event.offset, event.width, event.value)
                .map_err(|err| DriverReplayError::Replay(format!("{err:?}")))?,
        }
    }
    device
        .finish()
        .map_err(|err| DriverReplayError::Replay(format!("{err:?}")))?;
    Ok(events.len())
}

pub fn replay_trace_fixture(path: impl AsRef<Path>) -> Result<usize, DriverReplayError> {
    let trace = load_trace(path)?;
    let events = trace_to_replay_events(&trace)?;
    replay_events_through_mock_device(&events)
}

pub fn assert_live_trace_file(path: impl AsRef<Path>) -> Result<(), DriverReplayError> {
    let trace = load_trace(path)?;
    assert_live_oracle_trace(&trace)
}

fn validate_pci_config_bar(seq: u64, bar: Option<u8>) -> Result<(), DriverReplayError> {
    match bar {
        None | Some(0xff) => Ok(()),
        Some(bar) => Err(DriverReplayError::InvalidPciConfigBar { seq, bar }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use artifact_store_schema::driver_protocol_trace::{
        DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION, DriverProtocolTraceEventV0,
        DriverProtocolTraceMetadataV0,
    };
    use kernel_api::mock::pci_device::PciReplayOp;
    use std::path::PathBuf;

    fn fixture_trace() -> DriverProtocolTraceV0 {
        serde_json::from_str(include_str!(
            "../../drivers/reference_vaults/virtio-net/traces/oracle_init_trace.json"
        ))
        .unwrap()
    }

    #[test]
    fn trace_to_replay_events_translates_vault_fixture() {
        let trace = fixture_trace();
        let events = trace_to_replay_events(&trace).unwrap();

        assert_eq!(events.len(), 20);
        assert_eq!(events[0].op, PciReplayOp::PciConfigRead);
        assert_eq!(events[0].offset, 0);
        assert_eq!(events[0].width, 2);
        assert_eq!(events[0].value, 0x1af4);
        assert_eq!(events[2], PciReplayEvent::mmio_read(0, 0x12, 2, 0));
        assert_eq!(events[3], PciReplayEvent::mmio_write(0, 0x12, 2, 1));
        assert_eq!(events[19], PciReplayEvent::mmio_write(0, 0x12, 2, 7));
    }

    #[test]
    fn replay_trace_fixture_runs_through_mock_device() {
        let observed = replay_trace_fixture(vault_fixture_path()).unwrap();

        assert_eq!(observed, 20);
    }

    #[test]
    fn trace_to_replay_events_rejects_invalid_pci_config_bar() {
        let mut trace = fixture_trace();
        trace.events[0].bar = Some(0);

        let err = trace_to_replay_events(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::InvalidPciConfigBar { seq: 1, bar: 0 }
        );
    }

    #[test]
    fn trace_to_replay_events_rejects_unsupported_irq() {
        let mut trace = fixture_trace();
        trace.events.push(DriverProtocolTraceEventV0 {
            seq: 21,
            kind: DriverProtocolTraceEventKind::Irq,
            timestamp_ns: None,
            bar: None,
            offset: None,
            width: None,
            value: Some(1),
            bytes_hex: None,
            result: Some("ok".into()),
            notes: None,
        });

        let err = trace_to_replay_events(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::UnsupportedEvent {
                seq: 21,
                kind: DriverProtocolTraceEventKind::Irq
            }
        );
    }

    #[test]
    fn trace_to_replay_events_rejects_schema_invalid_trace() {
        let trace = DriverProtocolTraceV0 {
            schema_version: DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION,
            metadata: DriverProtocolTraceMetadataV0 {
                trace_id: None,
                timestamp_start: None,
                timestamp_end: None,
                capsule_id: None,
                oracle: "linux-virtio-net".into(),
                device_model: "virtio-net-pci".into(),
                pci_vendor_id: 0x1af4,
                pci_device_id: 0x1000,
                pci_bdf: None,
                capture_tool: "pci_mmio_tracer".into(),
            },
            events: Vec::new(),
        };

        assert!(matches!(
            trace_to_replay_events(&trace),
            Err(DriverReplayError::Schema(_))
        ));
    }

    #[test]
    fn tracer_jsonl_to_trace_converts_debugfs_stream() {
        let jsonl = r#"
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
{"seq":2,"timestamp_ns":11,"kind":"mmio_write","bar":0,"offset":18,"width":2,"value":1,"result":"ok"}
"#;

        let trace = tracer_jsonl_to_trace(jsonl).unwrap();

        assert_eq!(trace.schema_version, DRIVER_PROTOCOL_TRACE_SCHEMA_VERSION);
        assert_eq!(trace.metadata.device_model, "virtio-net-pci");
        assert_eq!(trace.events.len(), 2);
        assert_eq!(trace.events[0].timestamp_ns, Some(10));
    }

    #[test]
    fn stamp_missing_trace_id_uses_source_sha256() {
        let jsonl = r#"
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
"#;
        let mut trace = tracer_jsonl_to_trace(jsonl).unwrap();

        stamp_missing_trace_id(&mut trace, jsonl.as_bytes());

        assert_eq!(
            trace.metadata.trace_id,
            Some(source_trace_id(jsonl.as_bytes()))
        );
    }

    #[test]
    fn assert_live_oracle_trace_rejects_missing_trace_id() {
        let jsonl = r#"
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
"#;
        let trace = tracer_jsonl_to_trace(jsonl).unwrap();

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(err, DriverReplayError::MissingLiveTraceId);
    }

    #[test]
    fn assert_live_oracle_trace_rejects_non_sha256_trace_id() {
        let jsonl = r#"
{"metadata":{"trace_id":"manual-live-id","oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
"#;
        let trace = tracer_jsonl_to_trace(jsonl).unwrap();

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::InvalidLiveTraceId {
                trace_id: "manual-live-id".into()
            }
        );
    }

    #[test]
    fn assert_live_oracle_trace_rejects_short_sha256_trace_id() {
        let jsonl = r#"
{"metadata":{"trace_id":"sha256:0123456789abcdef","oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}
{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}
"#;
        let trace = tracer_jsonl_to_trace(jsonl).unwrap();

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::InvalidLiveTraceId {
                trace_id: "sha256:0123456789abcdef".into()
            }
        );
    }

    #[test]
    fn assert_live_oracle_trace_rejects_scaffold_trace_id() {
        let mut trace = fixture_trace();
        trace.metadata.trace_id = Some("virtio-net-s11-init-scaffold".into());

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::ScaffoldTrace {
                trace_id: Some("virtio-net-s11-init-scaffold".into())
            }
        );
    }

    #[test]
    fn assert_live_oracle_trace_accepts_timestamped_non_scaffold_trace() {
        let jsonl = format!(
            r#"
{{"metadata":{{"trace_id":"{}","oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}}}
{{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}}
"#,
            source_trace_id(b"live")
        );
        let trace = tracer_jsonl_to_trace(&jsonl).unwrap();

        assert_live_oracle_trace(&trace).unwrap();
    }

    #[test]
    fn assert_live_oracle_trace_rejects_missing_timestamps() {
        let mut trace = fixture_trace();
        trace.events[0].timestamp_ns = None;

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(err, DriverReplayError::MissingLiveTimestamp { seq: 1 });
    }

    #[test]
    fn assert_live_oracle_trace_rejects_wrapped_sequence_start() {
        let jsonl = format!(
            r#"
{{"metadata":{{"trace_id":"{}","oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}}}
{{"seq":4097,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}}
"#,
            source_trace_id(b"wrapped")
        );
        let trace = tracer_jsonl_to_trace(&jsonl).unwrap();

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::NonContiguousLiveSeq {
                expected: 1,
                observed: 4097
            }
        );
    }

    #[test]
    fn assert_live_oracle_trace_rejects_sequence_gap() {
        let jsonl = format!(
            r#"
{{"metadata":{{"trace_id":"{}","oracle":"linux-virtio-net","device_model":"virtio-net-pci","pci_vendor_id":6900,"pci_device_id":4096,"pci_bdf":"0000:00:03.0","capture_tool":"pci_mmio_tracer"}}}}
{{"seq":1,"timestamp_ns":10,"kind":"pci_config_read","bar":255,"offset":0,"width":2,"value":6900,"result":"ok"}}
{{"seq":3,"timestamp_ns":11,"kind":"mmio_write","bar":0,"offset":18,"width":2,"value":1,"result":"ok"}}
"#,
            source_trace_id(b"gap")
        );
        let trace = tracer_jsonl_to_trace(&jsonl).unwrap();

        let err = assert_live_oracle_trace(&trace).unwrap_err();

        assert_eq!(
            err,
            DriverReplayError::NonContiguousLiveSeq {
                expected: 2,
                observed: 3
            }
        );
    }

    fn vault_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../drivers/reference_vaults/virtio-net/traces/oracle_init_trace.json")
    }

    fn vault_packet_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json")
    }

    #[test]
    fn packet_trace_to_replay_bundle_translates_vault_fixture() {
        let trace = load_packet_trace(vault_packet_fixture_path()).unwrap();
        let bundle = packet_trace_to_replay_bundle(&trace).unwrap();

        assert_eq!(bundle.shm_cap(), 0x1000);
        bundle.with_events(|events| {
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].op, PacketReplayOp::SendPacket);
            assert_eq!(events[0].len, 42);
            assert_eq!(events[1].op, PacketReplayOp::ReceivePacket);
            assert_eq!(events[1].offset, 2048);
        });
    }

    #[test]
    fn replay_packet_trace_fixture_runs_through_mock_harness() {
        let observed = replay_packet_trace_fixture(vault_packet_fixture_path()).unwrap();
        assert_eq!(observed, 2);
    }

    #[test]
    fn packet_jsonl_to_trace_converts_capture_stream() {
        let jsonl = r#"
{"metadata":{"oracle":"linux-virtio-net","device_model":"virtio-net-pci","harness":"harness.net","harness_version":"1","capture_tool":"virtio_net_packet_oracle_capture"}}
{"seq":1,"kind":"send_packet","timestamp_ns":10,"request_id":1,"shm_cap":4096,"offset":0,"len":2,"payload_hex":"aabb","status":0,"bytes":2,"result":"ok"}
{"seq":2,"kind":"receive_packet","timestamp_ns":11,"request_id":2,"shm_cap":4096,"offset":2048,"len":2,"payload_hex":"ccdd","status":0,"bytes":2,"result":"ok"}
"#;

        let trace = packet_jsonl_to_trace(jsonl).unwrap();

        assert_eq!(trace.metadata.harness, "harness.net");
        assert_eq!(trace.events.len(), 2);
        assert_eq!(trace.events[0].kind, NetPacketTraceEventKind::SendPacket);
    }

    #[test]
    fn assert_live_packet_trace_rejects_scaffold_trace_id() {
        let mut trace = load_packet_trace(vault_packet_fixture_path()).unwrap();
        trace.metadata.trace_id = Some("virtio-net-s11-packet-scaffold".into());

        let err = assert_live_packet_trace(&trace).unwrap_err();
        assert_eq!(
            err,
            DriverReplayError::ScaffoldTrace {
                trace_id: Some("virtio-net-s11-packet-scaffold".into())
            }
        );
    }

    #[test]
    fn assert_hardware_packet_rx_accepts_live_vault_fixture() {
        let trace = load_packet_trace(vault_packet_fixture_path()).unwrap();
        assert_hardware_packet_rx(&trace).unwrap();
    }

    #[test]
    fn assert_hardware_packet_rx_rejects_slirp_derived_receive() {
        let mut trace = load_packet_trace(vault_packet_fixture_path()).unwrap();
        trace.events[1].notes = Some("qemu-slirp-arp-reply-derived from hardware MAC".into());

        let err = assert_hardware_packet_rx(&trace).unwrap_err();
        assert!(matches!(
            err,
            DriverReplayError::DerivedPacketReceive { seq: 2, .. }
        ));
    }
}
