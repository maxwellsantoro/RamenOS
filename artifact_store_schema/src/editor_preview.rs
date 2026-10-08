//! Pure, fixed-size NativePreview shared-data codecs.
//!
//! Validation establishes canonical data only. It establishes no live authority,
//! current clock or epoch, Store selection, publication, or durability. Services
//! must check their actual Core, peer, Gate, descriptors, and original lifetime.
use crate::editor_save::EditorActorV0;
use core::fmt;

pub const PREVIEW_SCHEMA_VERSION: u32 = 2;
pub const PREVIEW_DATA_LEN: usize = 464;
pub const PREVIEW_RECORD_COUNT: u32 = 4;
pub const PREVIEW_CATEGORY_MASK: u32 = 63;
pub const CHROME_SCHEMA_VERSION: u32 = 1;
pub const CHROME_STATUS_LEN: usize = 248;
pub const MAX_TEXT_LEN: u32 = 4096;
pub const SURFACE_WIDTH: u32 = 640;
pub const SURFACE_HEIGHT: u32 = 480;
pub const SURFACE_STRIDE: u32 = 2560;
pub const SURFACE_FORMAT: u32 = 1;
pub const INPUT_CAPACITY: u32 = 64;
pub const DATA_PROTOCOL_VERSION: u32 = 1;
pub const COMPOSED_WIDTH: u32 = 640;
pub const COMPOSED_HEIGHT: u32 = 568;
pub const APP_TOP: u32 = 48;
pub const APP_BOTTOM: u32 = 528;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPreviewErrorV0 {
    Length,
    Version,
    Reserved,
    Constants,
    Phase,
    Identity,
    Handle,
    Resource,
    Epoch,
    Expiry,
    State,
}

impl fmt::Display for EditorPreviewErrorV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "invalid preview data length",
            Self::Version => "unsupported preview data version",
            Self::Reserved => "nonzero preview reserved field",
            Self::Constants => "invalid preview constants",
            Self::Phase => "invalid preview phase",
            Self::Identity => "invalid preview identity",
            Self::Handle => "invalid preview IPC reference",
            Self::Resource => "invalid preview resource binding",
            Self::Epoch => "invalid preview epoch binding",
            Self::Expiry => "invalid preview expiry binding",
            Self::State => "invalid preview chrome state",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EditorPreviewErrorV0 {}

type Result<T> = core::result::Result<T, EditorPreviewErrorV0>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPreviewPhaseV0 {
    Preview = 0,
    Active = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPreviewClassV0 {
    SelfStatus = 1,
    FocusRead = 2,
    Surface = 3,
    StoreRead = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorPreviewGrantV0 {
    pub handle: u64,
    pub resource_id: u64,
    pub resource_generation: u64,
    pub service_epoch: u64,
    pub expires_at_ms: u64,
    pub class: EditorPreviewClassV0,
    pub rights: u32,
    pub protocol_version: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorPreviewDataV0 {
    pub phase: EditorPreviewPhaseV0,
    pub schema_version: u32,
    pub total_len: u32,
    pub category_mask: u32,
    pub record_count: u32,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub selected_object_id: u64,
    pub selected_object_generation: u64,
    pub selected_revision: u64,
    pub policy_revision: u64,
    pub plan_id: u64,
    pub preview_revision: u64,
    pub expires_at_ms: u64,
    pub desktop_service_epoch: u64,
    pub application_hash: [u8; 32],
    pub manifest_hash: [u8; 32],
    pub selected_content_hash: [u8; 32],
    pub max_text_len: u32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub surface_stride: u32,
    pub surface_format: u32,
    pub input_capacity: u32,
    pub protocol_version: u32,
    pub reserved: u32,
    pub records: [EditorPreviewGrantV0; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPreviewChromeStateV0 {
    NoSave = 0,
    Unavailable = 4,
}

// EditorActorV0 is deliberately unchanged and is Clone, not Copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorPreviewChromeV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub state: EditorPreviewChromeStateV0,
    pub reserved: u32,
    pub actor: EditorActorV0,
    pub selected_object_id: u64,
    pub selected_object_generation: u64,
    pub current_store_epoch: u64,
    pub original_backend_epoch: u64,
    pub operation_id: u64,
    pub source_handle: u64,
    pub source_generation: u64,
    pub expected_revision: u64,
    pub result_revision: u64,
    pub attachment_version: u64,
    pub status_version: u64,
    pub source_content_hash: [u8; 32],
    pub expected_content_hash: [u8; 32],
    pub receipt_sha256: [u8; 32],
    pub source_body_len: u32,
    pub tail_reserved: u32,
}

const CLASSES: [EditorPreviewClassV0; 4] = [
    EditorPreviewClassV0::SelfStatus,
    EditorPreviewClassV0::FocusRead,
    EditorPreviewClassV0::Surface,
    EditorPreviewClassV0::StoreRead,
];
const RIGHTS: [u32; 4] = [1, 1, 15, 1];

/// Encode checked IPC parts without truncation. Slot zero is canonical data;
/// only the actual issuer registry can establish that a slot is allocated.
pub fn encode_ipc_reference(index: u32, generation: u64) -> Result<u64> {
    if index > u16::MAX as u32 || generation == 0 || generation > u32::MAX as u64 {
        return Err(EditorPreviewErrorV0::Handle);
    }
    Ok((1u64 << 56) | ((index as u64) << 32) | generation)
}

/// Decode a canonical IPC reference, retaining the exact widened 16/32 parts.
pub fn decode_ipc_reference(raw: u64) -> Result<(u32, u64)> {
    if raw >> 56 != 1 || (raw >> 48) & 0xff != 0 || raw & 0xffff_ffff == 0 {
        return Err(EditorPreviewErrorV0::Handle);
    }
    Ok((((raw >> 32) & 0xffff) as u32, raw & 0xffff_ffff))
}

fn read<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N]> {
    let end = offset.checked_add(N).ok_or(EditorPreviewErrorV0::Length)?;
    let source = bytes.get(offset..end).ok_or(EditorPreviewErrorV0::Length)?;
    let mut out = [0; N];
    out.copy_from_slice(source);
    Ok(out)
}

fn read32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(read(bytes, offset)?))
}

fn read64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(read(bytes, offset)?))
}

fn write<const N: usize>(bytes: &mut [u8], offset: usize, value: [u8; N]) -> Result<()> {
    let end = offset.checked_add(N).ok_or(EditorPreviewErrorV0::Length)?;
    bytes
        .get_mut(offset..end)
        .ok_or(EditorPreviewErrorV0::Length)?
        .copy_from_slice(&value);
    Ok(())
}

fn write32(bytes: &mut [u8], offset: usize, value: u32) -> Result<()> {
    write(bytes, offset, value.to_le_bytes())
}

fn write64(bytes: &mut [u8], offset: usize, value: u64) -> Result<()> {
    write(bytes, offset, value.to_le_bytes())
}

impl EditorPreviewDataV0 {
    // Shared by typed validation and decoding before raw enum discriminants are
    // interpreted. This preserves Length/Version/Reserved/Constants precedence.
    fn validate_prefix(&self) -> Result<()> {
        if self.total_len != PREVIEW_DATA_LEN as u32 {
            return Err(EditorPreviewErrorV0::Length);
        }
        if self.schema_version != PREVIEW_SCHEMA_VERSION {
            return Err(EditorPreviewErrorV0::Version);
        }
        if self.reserved != 0 || self.records.iter().any(|r| r.reserved != 0) {
            return Err(EditorPreviewErrorV0::Reserved);
        }
        if self.category_mask != PREVIEW_CATEGORY_MASK
            || self.record_count != PREVIEW_RECORD_COUNT
            || self.max_text_len != MAX_TEXT_LEN
            || self.surface_width != SURFACE_WIDTH
            || self.surface_height != SURFACE_HEIGHT
            || self.surface_stride != SURFACE_STRIDE
            || self.surface_format != SURFACE_FORMAT
            || self.input_capacity != INPUT_CAPACITY
            || self.protocol_version != DATA_PROTOCOL_VERSION
            || self.records.iter().enumerate().any(|(i, r)| {
                r.class != CLASSES[i]
                    || r.rights != RIGHTS[i]
                    || r.protocol_version != DATA_PROTOCOL_VERSION
            })
        {
            return Err(EditorPreviewErrorV0::Constants);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_prefix()?;
        let inferred = match (self.instance_id == 0, self.instance_generation == 0) {
            (true, true) => EditorPreviewPhaseV0::Preview,
            (false, false) => EditorPreviewPhaseV0::Active,
            _ => return Err(EditorPreviewErrorV0::Phase),
        };
        if self.phase != inferred {
            return Err(EditorPreviewErrorV0::Phase);
        }
        if [
            self.session_id,
            self.session_generation,
            self.selected_object_id,
            self.selected_object_generation,
            self.selected_revision,
            self.policy_revision,
            self.plan_id,
            self.preview_revision,
            self.expires_at_ms,
            self.desktop_service_epoch,
        ]
        .contains(&0)
        {
            return Err(EditorPreviewErrorV0::Identity);
        }
        // Validate every handle before any resource/epoch/expiry error.
        for r in &self.records {
            match self.phase {
                EditorPreviewPhaseV0::Preview if r.handle != 0 => {
                    return Err(EditorPreviewErrorV0::Handle);
                }
                EditorPreviewPhaseV0::Active => {
                    decode_ipc_reference(r.handle)?;
                }
                EditorPreviewPhaseV0::Preview => {}
            }
        }
        for r in &self.records[..2] {
            if r.resource_id != self.instance_id
                || r.resource_generation != self.instance_generation
            {
                return Err(EditorPreviewErrorV0::Resource);
            }
        }
        let surface = &self.records[2];
        let surface_valid = match self.phase {
            EditorPreviewPhaseV0::Preview => {
                surface.resource_id == 0 && surface.resource_generation == 0
            }
            EditorPreviewPhaseV0::Active => {
                surface.resource_id != 0 && surface.resource_generation != 0
            }
        };
        let store = &self.records[3];
        if !surface_valid
            || store.resource_id != self.selected_object_id
            || store.resource_generation != self.selected_object_generation
        {
            return Err(EditorPreviewErrorV0::Resource);
        }
        if self.records[..3]
            .iter()
            .any(|r| r.service_epoch != self.desktop_service_epoch)
            || store.service_epoch == 0
        {
            return Err(EditorPreviewErrorV0::Epoch);
        }
        if self
            .records
            .iter()
            .any(|r| r.expires_at_ms != self.expires_at_ms)
        {
            return Err(EditorPreviewErrorV0::Expiry);
        }
        Ok(())
    }

    pub fn validate_for_phase(&self, expected: EditorPreviewPhaseV0) -> Result<()> {
        self.validate()?;
        if self.phase != expected {
            return Err(EditorPreviewErrorV0::Phase);
        }
        Ok(())
    }

    pub fn encode_le(&self) -> Result<[u8; PREVIEW_DATA_LEN]> {
        self.validate()?;
        let mut out = [0; PREVIEW_DATA_LEN];
        for (i, value) in [
            self.schema_version,
            self.total_len,
            self.category_mask,
            self.record_count,
        ]
        .into_iter()
        .enumerate()
        {
            write32(&mut out, i * 4, value)?;
        }
        for (i, value) in [
            self.session_id,
            self.session_generation,
            self.instance_id,
            self.instance_generation,
            self.selected_object_id,
            self.selected_object_generation,
            self.selected_revision,
            self.policy_revision,
            self.plan_id,
            self.preview_revision,
            self.expires_at_ms,
            self.desktop_service_epoch,
        ]
        .into_iter()
        .enumerate()
        {
            write64(&mut out, 16 + i * 8, value)?;
        }
        write(&mut out, 112, self.application_hash)?;
        write(&mut out, 144, self.manifest_hash)?;
        write(&mut out, 176, self.selected_content_hash)?;
        for (i, value) in [
            self.max_text_len,
            self.surface_width,
            self.surface_height,
            self.surface_stride,
            self.surface_format,
            self.input_capacity,
            self.protocol_version,
            self.reserved,
        ]
        .into_iter()
        .enumerate()
        {
            write32(&mut out, 208 + i * 4, value)?;
        }
        for (i, r) in self.records.iter().enumerate() {
            let offset = 240 + i * 56;
            for (j, value) in [
                r.handle,
                r.resource_id,
                r.resource_generation,
                r.service_epoch,
                r.expires_at_ms,
            ]
            .into_iter()
            .enumerate()
            {
                write64(&mut out, offset + j * 8, value)?;
            }
            for (j, value) in [r.class as u32, r.rights, r.protocol_version, r.reserved]
                .into_iter()
                .enumerate()
            {
                write32(&mut out, offset + 40 + j * 4, value)?;
            }
        }
        Ok(out)
    }

    pub fn decode_le(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PREVIEW_DATA_LEN {
            return Err(EditorPreviewErrorV0::Length);
        }
        // Placeholder classes are never returned until their raw values have
        // been checked after prefix validation but before phase validation.
        let mut records = [EditorPreviewGrantV0 {
            handle: 0,
            resource_id: 0,
            resource_generation: 0,
            service_epoch: 0,
            expires_at_ms: 0,
            class: EditorPreviewClassV0::SelfStatus,
            rights: 0,
            protocol_version: 0,
            reserved: 0,
        }; 4];
        let mut raw_classes = [0u32; 4];
        for (i, r) in records.iter_mut().enumerate() {
            let offset = 240 + i * 56;
            *r = EditorPreviewGrantV0 {
                handle: read64(bytes, offset)?,
                resource_id: read64(bytes, offset + 8)?,
                resource_generation: read64(bytes, offset + 16)?,
                service_epoch: read64(bytes, offset + 24)?,
                expires_at_ms: read64(bytes, offset + 32)?,
                class: CLASSES[i],
                rights: read32(bytes, offset + 44)?,
                protocol_version: read32(bytes, offset + 48)?,
                reserved: read32(bytes, offset + 52)?,
            };
            raw_classes[i] = read32(bytes, offset + 40)?;
        }
        let instance_id = read64(bytes, 32)?;
        let instance_generation = read64(bytes, 40)?;
        let out = Self {
            phase: if instance_id == 0 && instance_generation == 0 {
                EditorPreviewPhaseV0::Preview
            } else {
                EditorPreviewPhaseV0::Active
            },
            schema_version: read32(bytes, 0)?,
            total_len: read32(bytes, 4)?,
            category_mask: read32(bytes, 8)?,
            record_count: read32(bytes, 12)?,
            session_id: read64(bytes, 16)?,
            session_generation: read64(bytes, 24)?,
            instance_id,
            instance_generation,
            selected_object_id: read64(bytes, 48)?,
            selected_object_generation: read64(bytes, 56)?,
            selected_revision: read64(bytes, 64)?,
            policy_revision: read64(bytes, 72)?,
            plan_id: read64(bytes, 80)?,
            preview_revision: read64(bytes, 88)?,
            expires_at_ms: read64(bytes, 96)?,
            desktop_service_epoch: read64(bytes, 104)?,
            application_hash: read(bytes, 112)?,
            manifest_hash: read(bytes, 144)?,
            selected_content_hash: read(bytes, 176)?,
            max_text_len: read32(bytes, 208)?,
            surface_width: read32(bytes, 212)?,
            surface_height: read32(bytes, 216)?,
            surface_stride: read32(bytes, 220)?,
            surface_format: read32(bytes, 224)?,
            input_capacity: read32(bytes, 228)?,
            protocol_version: read32(bytes, 232)?,
            reserved: read32(bytes, 236)?,
            records,
        };
        out.validate_prefix()?;
        if raw_classes
            .iter()
            .enumerate()
            .any(|(i, &raw)| raw != CLASSES[i] as u32)
        {
            return Err(EditorPreviewErrorV0::Constants);
        }
        out.validate()?;
        Ok(out)
    }

    pub fn decode_le_for_phase(bytes: &[u8], expected: EditorPreviewPhaseV0) -> Result<Self> {
        let out = Self::decode_le(bytes)?;
        out.validate_for_phase(expected)?;
        Ok(out)
    }
}

impl EditorPreviewChromeV0 {
    fn validate_prefix(&self) -> Result<()> {
        if self.total_len != CHROME_STATUS_LEN as u32 {
            return Err(EditorPreviewErrorV0::Length);
        }
        if self.schema_version != CHROME_SCHEMA_VERSION {
            return Err(EditorPreviewErrorV0::Version);
        }
        if self.reserved != 0 || self.tail_reserved != 0 {
            return Err(EditorPreviewErrorV0::Reserved);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_prefix()?;
        // The typed enum has only the two supported states; raw decoding checks
        // the discriminant before identity and forbidden original fields.
        if [
            self.actor.owner_id,
            self.actor.session_id,
            self.actor.session_generation,
            self.actor.instance_id,
            self.actor.instance_generation,
            self.selected_object_id,
            self.selected_object_generation,
            self.current_store_epoch,
            self.attachment_version,
            self.status_version,
        ]
        .contains(&0)
        {
            return Err(EditorPreviewErrorV0::Identity);
        }
        if self.original_backend_epoch != 0
            || self.operation_id != 0
            || self.source_handle != 0
            || self.source_generation != 0
            || self.expected_revision != 0
            || self.result_revision != 0
            || self.source_content_hash != [0; 32]
            || self.expected_content_hash != [0; 32]
            || self.receipt_sha256 != [0; 32]
            || self.source_body_len != 0
        {
            return Err(EditorPreviewErrorV0::State);
        }
        Ok(())
    }

    pub fn encode_le(&self) -> Result<[u8; CHROME_STATUS_LEN]> {
        self.validate()?;
        let mut out = [0; CHROME_STATUS_LEN];
        for (i, value) in [
            self.schema_version,
            self.total_len,
            self.state as u32,
            self.reserved,
        ]
        .into_iter()
        .enumerate()
        {
            write32(&mut out, i * 4, value)?;
        }
        for (i, value) in [
            self.actor.owner_id,
            self.actor.session_id,
            self.actor.session_generation,
            self.actor.instance_id,
            self.actor.instance_generation,
            self.selected_object_id,
            self.selected_object_generation,
            self.current_store_epoch,
            self.original_backend_epoch,
            self.operation_id,
            self.source_handle,
            self.source_generation,
            self.expected_revision,
            self.result_revision,
            self.attachment_version,
            self.status_version,
        ]
        .into_iter()
        .enumerate()
        {
            write64(&mut out, 16 + i * 8, value)?;
        }
        write(&mut out, 144, self.source_content_hash)?;
        write(&mut out, 176, self.expected_content_hash)?;
        write(&mut out, 208, self.receipt_sha256)?;
        write32(&mut out, 240, self.source_body_len)?;
        write32(&mut out, 244, self.tail_reserved)?;
        Ok(out)
    }

    pub fn decode_le(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != CHROME_STATUS_LEN {
            return Err(EditorPreviewErrorV0::Length);
        }
        let raw_state = read32(bytes, 8)?;
        let mut out = Self {
            schema_version: read32(bytes, 0)?,
            total_len: read32(bytes, 4)?,
            state: EditorPreviewChromeStateV0::NoSave,
            reserved: read32(bytes, 12)?,
            actor: EditorActorV0 {
                owner_id: read64(bytes, 16)?,
                session_id: read64(bytes, 24)?,
                session_generation: read64(bytes, 32)?,
                instance_id: read64(bytes, 40)?,
                instance_generation: read64(bytes, 48)?,
            },
            selected_object_id: read64(bytes, 56)?,
            selected_object_generation: read64(bytes, 64)?,
            current_store_epoch: read64(bytes, 72)?,
            original_backend_epoch: read64(bytes, 80)?,
            operation_id: read64(bytes, 88)?,
            source_handle: read64(bytes, 96)?,
            source_generation: read64(bytes, 104)?,
            expected_revision: read64(bytes, 112)?,
            result_revision: read64(bytes, 120)?,
            attachment_version: read64(bytes, 128)?,
            status_version: read64(bytes, 136)?,
            source_content_hash: read(bytes, 144)?,
            expected_content_hash: read(bytes, 176)?,
            receipt_sha256: read(bytes, 208)?,
            source_body_len: read32(bytes, 240)?,
            tail_reserved: read32(bytes, 244)?,
        };
        out.validate_prefix()?;
        out.state = match raw_state {
            0 => EditorPreviewChromeStateV0::NoSave,
            4 => EditorPreviewChromeStateV0::Unavailable,
            _ => return Err(EditorPreviewErrorV0::State),
        };
        out.validate()?;
        Ok(out)
    }
}
