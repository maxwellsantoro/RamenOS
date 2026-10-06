//! Pure, fixed-size NativeSave grant and original Commit outcome data codecs.
//!
//! Validation establishes canonical data only. It establishes no live authority,
//! current clock or epoch, Store selection, publication, or durability. Services
//! must check their actual Core, peer, Gate, descriptors, and original lifetime.
use crate::editor_save::{
    EditorActorV0, EditorReceiptOutcomeV0, EditorSaveBindingV0, EditorSaveReceiptV0,
};
use core::fmt;
use sha2::{Digest, Sha256};

pub const SAVE_SCHEMA_VERSION: u32 = 3;
pub const SAVE_DATA_LEN: usize = 464;
pub const SAVE_RECORD_COUNT: u32 = 4;
pub const SAVE_CATEGORY_MASK: u32 = 63;
pub const SAVE_CHROME_SCHEMA_VERSION: u32 = 2;
pub const SAVE_CHROME_STATUS_LEN: usize = 248;
pub const MAX_TEXT_LEN: u32 = 4096;
pub const SURFACE_WIDTH: u32 = 640;
pub const SURFACE_HEIGHT: u32 = 480;
pub const SURFACE_STRIDE: u32 = 2560;
pub const SURFACE_FORMAT: u32 = 1;
pub const INPUT_CAPACITY: u32 = 64;
pub const DATA_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorNativeSaveErrorV0 {
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
    Bounds,
    Binding,
    Receipt,
    Overflow,
}

impl fmt::Display for EditorNativeSaveErrorV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "invalid save data length",
            Self::Version => "unsupported save data version",
            Self::Reserved => "nonzero save reserved field",
            Self::Constants => "invalid save constants",
            Self::Phase => "invalid save phase",
            Self::Identity => "invalid save identity",
            Self::Handle => "invalid save IPC reference",
            Self::Resource => "invalid save resource binding",
            Self::Epoch => "invalid save epoch binding",
            Self::Expiry => "invalid save expiry binding",
            Self::State => "invalid save chrome state",
            Self::Bounds => "invalid save source bounds",
            Self::Binding => "invalid original save binding",
            Self::Receipt => "invalid original save receipt",
            Self::Overflow => "invalid save revision successor",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EditorNativeSaveErrorV0 {}

type Result<T> = core::result::Result<T, EditorNativeSaveErrorV0>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EditorNativeSavePhaseV0 {
    Preview = 0,
    Active = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EditorNativeSaveClassV0 {
    SelfStatus = 1,
    FocusRead = 2,
    Surface = 3,
    StoreArtifact = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorNativeSaveGrantV0 {
    pub handle: u64,
    pub resource_id: u64,
    pub resource_generation: u64,
    pub service_epoch: u64,
    pub expires_at_ms: u64,
    pub class: EditorNativeSaveClassV0,
    pub rights: u32,
    pub protocol_version: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorNativeSaveDataV0 {
    pub phase: EditorNativeSavePhaseV0,
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
    pub records: [EditorNativeSaveGrantV0; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EditorNativeSaveChromeStateV0 {
    NoSave = 0,
    Unknown = 1,
    Committed = 2,
    DefinitiveNoncommit = 3,
    Unavailable = 4,
}

// EditorActorV0 is deliberately unchanged and is Clone, not Copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorNativeSaveChromeV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub state: EditorNativeSaveChromeStateV0,
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

const CLASSES: [EditorNativeSaveClassV0; 4] = [
    EditorNativeSaveClassV0::SelfStatus,
    EditorNativeSaveClassV0::FocusRead,
    EditorNativeSaveClassV0::Surface,
    EditorNativeSaveClassV0::StoreArtifact,
];
const RIGHTS: [u32; 4] = [1, 1, 15, 7];

/// Encode checked IPC parts without truncation. Slot zero is canonical data;
/// only the actual issuer registry can establish that a slot is allocated.
pub fn encode_ipc_reference(index: u32, generation: u64) -> Result<u64> {
    if index > u16::MAX as u32 || generation == 0 || generation > u32::MAX as u64 {
        return Err(EditorNativeSaveErrorV0::Handle);
    }
    Ok((1u64 << 56) | ((index as u64) << 32) | generation)
}

/// Decode a canonical IPC reference, retaining the exact widened 16/32 parts.
pub fn decode_ipc_reference(raw: u64) -> Result<(u32, u64)> {
    if raw >> 56 != 1 || (raw >> 48) & 0xff != 0 || raw & 0xffff_ffff == 0 {
        return Err(EditorNativeSaveErrorV0::Handle);
    }
    Ok((((raw >> 32) & 0xffff) as u32, raw & 0xffff_ffff))
}

/// Encode a checked shared-memory reference; slot zero is never canonical SHM.
pub fn encode_shmem_reference(index: u32, generation: u64) -> Result<u64> {
    if index == 0 || index > u16::MAX as u32 || generation == 0 || generation > u32::MAX as u64 {
        return Err(EditorNativeSaveErrorV0::Handle);
    }
    Ok((2u64 << 56) | ((index as u64) << 32) | generation)
}

/// Canonical data alone establishes no allocated object or lease authority.
pub fn decode_shmem_reference(raw: u64) -> Result<(u32, u64)> {
    if raw >> 56 != 2
        || (raw >> 48) & 0xff != 0
        || raw & 0xffff_ffff == 0
        || (raw >> 32) & 0xffff == 0
    {
        return Err(EditorNativeSaveErrorV0::Handle);
    }
    Ok((((raw >> 32) & 0xffff) as u32, raw & 0xffff_ffff))
}

fn read<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or(EditorNativeSaveErrorV0::Length)?;
    let source = bytes
        .get(offset..end)
        .ok_or(EditorNativeSaveErrorV0::Length)?;
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
    let end = offset
        .checked_add(N)
        .ok_or(EditorNativeSaveErrorV0::Length)?;
    bytes
        .get_mut(offset..end)
        .ok_or(EditorNativeSaveErrorV0::Length)?
        .copy_from_slice(&value);
    Ok(())
}

fn write32(bytes: &mut [u8], offset: usize, value: u32) -> Result<()> {
    write(bytes, offset, value.to_le_bytes())
}

fn write64(bytes: &mut [u8], offset: usize, value: u64) -> Result<()> {
    write(bytes, offset, value.to_le_bytes())
}

impl EditorNativeSaveDataV0 {
    // Shared by typed validation and decoding before raw enum discriminants are
    // interpreted. This preserves Length/Version/Reserved/Constants precedence.
    fn validate_prefix(&self) -> Result<()> {
        if self.total_len != SAVE_DATA_LEN as u32 {
            return Err(EditorNativeSaveErrorV0::Length);
        }
        if self.schema_version != SAVE_SCHEMA_VERSION {
            return Err(EditorNativeSaveErrorV0::Version);
        }
        if self.reserved != 0 || self.records.iter().any(|r| r.reserved != 0) {
            return Err(EditorNativeSaveErrorV0::Reserved);
        }
        if self.category_mask != SAVE_CATEGORY_MASK
            || self.record_count != SAVE_RECORD_COUNT
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
            return Err(EditorNativeSaveErrorV0::Constants);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_prefix()?;
        let inferred = match (self.instance_id == 0, self.instance_generation == 0) {
            (true, true) => EditorNativeSavePhaseV0::Preview,
            (false, false) => EditorNativeSavePhaseV0::Active,
            _ => return Err(EditorNativeSaveErrorV0::Phase),
        };
        if self.phase != inferred {
            return Err(EditorNativeSaveErrorV0::Phase);
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
            return Err(EditorNativeSaveErrorV0::Identity);
        }
        // Validate every handle before any resource/epoch/expiry error.
        for r in &self.records {
            match self.phase {
                EditorNativeSavePhaseV0::Preview if r.handle != 0 => {
                    return Err(EditorNativeSaveErrorV0::Handle);
                }
                EditorNativeSavePhaseV0::Active => {
                    decode_ipc_reference(r.handle)?;
                }
                EditorNativeSavePhaseV0::Preview => {}
            }
        }
        if self.phase == EditorNativeSavePhaseV0::Active
            && self.records.iter().enumerate().any(|(i, r)| {
                self.records[..i]
                    .iter()
                    .any(|previous| previous.handle == r.handle)
            })
        {
            return Err(EditorNativeSaveErrorV0::Handle);
        }
        for r in &self.records[..2] {
            if r.resource_id != self.instance_id
                || r.resource_generation != self.instance_generation
            {
                return Err(EditorNativeSaveErrorV0::Resource);
            }
        }
        let surface = &self.records[2];
        let surface_valid = match self.phase {
            EditorNativeSavePhaseV0::Preview => {
                surface.resource_id == 0 && surface.resource_generation == 0
            }
            EditorNativeSavePhaseV0::Active => {
                surface.resource_id != 0 && surface.resource_generation != 0
            }
        };
        let store = &self.records[3];
        if !surface_valid
            || store.resource_id != self.selected_object_id
            || store.resource_generation != self.selected_object_generation
        {
            return Err(EditorNativeSaveErrorV0::Resource);
        }
        if self.records[..3]
            .iter()
            .any(|r| r.service_epoch != self.desktop_service_epoch)
            || store.service_epoch == 0
        {
            return Err(EditorNativeSaveErrorV0::Epoch);
        }
        if self
            .records
            .iter()
            .any(|r| r.expires_at_ms != self.expires_at_ms)
        {
            return Err(EditorNativeSaveErrorV0::Expiry);
        }
        Ok(())
    }

    pub fn validate_for_phase(&self, expected: EditorNativeSavePhaseV0) -> Result<()> {
        self.validate()?;
        if self.phase != expected {
            return Err(EditorNativeSaveErrorV0::Phase);
        }
        Ok(())
    }

    pub fn encode_le(&self) -> Result<[u8; SAVE_DATA_LEN]> {
        self.validate()?;
        let mut out = [0; SAVE_DATA_LEN];
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
        if bytes.len() != SAVE_DATA_LEN {
            return Err(EditorNativeSaveErrorV0::Length);
        }
        // Placeholder classes are never returned until their raw values have
        // been checked after prefix validation but before phase validation.
        let mut records = [EditorNativeSaveGrantV0 {
            handle: 0,
            resource_id: 0,
            resource_generation: 0,
            service_epoch: 0,
            expires_at_ms: 0,
            class: EditorNativeSaveClassV0::SelfStatus,
            rights: 0,
            protocol_version: 0,
            reserved: 0,
        }; 4];
        let mut raw_classes = [0u32; 4];
        for (i, r) in records.iter_mut().enumerate() {
            let offset = 240 + i * 56;
            *r = EditorNativeSaveGrantV0 {
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
                EditorNativeSavePhaseV0::Preview
            } else {
                EditorNativeSavePhaseV0::Active
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
            return Err(EditorNativeSaveErrorV0::Constants);
        }
        out.validate()?;
        Ok(out)
    }

    pub fn decode_le_for_phase(bytes: &[u8], expected: EditorNativeSavePhaseV0) -> Result<Self> {
        let out = Self::decode_le(bytes)?;
        out.validate_for_phase(expected)?;
        Ok(out)
    }
}

impl EditorNativeSaveChromeV0 {
    fn validate_prefix(&self) -> Result<()> {
        if self.total_len != SAVE_CHROME_STATUS_LEN as u32 {
            return Err(EditorNativeSaveErrorV0::Length);
        }
        if self.schema_version != SAVE_CHROME_SCHEMA_VERSION {
            return Err(EditorNativeSaveErrorV0::Version);
        }
        if self.reserved != 0 || self.tail_reserved != 0 {
            return Err(EditorNativeSaveErrorV0::Reserved);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_prefix()?;
        // The typed enum has only the five supported states; raw decoding checks
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
            return Err(EditorNativeSaveErrorV0::Identity);
        }
        match self.state {
            EditorNativeSaveChromeStateV0::NoSave | EditorNativeSaveChromeStateV0::Unavailable => {
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
                    return Err(EditorNativeSaveErrorV0::State);
                }
            }
            EditorNativeSaveChromeStateV0::Unknown
            | EditorNativeSaveChromeStateV0::Committed
            | EditorNativeSaveChromeStateV0::DefinitiveNoncommit => {
                if self.original_backend_epoch == 0
                    || self.operation_id == 0
                    || self.expected_revision == 0
                {
                    return Err(EditorNativeSaveErrorV0::State);
                }
                match self.state {
                    EditorNativeSaveChromeStateV0::Unknown
                        if self.result_revision != 0 || self.receipt_sha256 != [0; 32] =>
                    {
                        return Err(EditorNativeSaveErrorV0::State);
                    }
                    EditorNativeSaveChromeStateV0::DefinitiveNoncommit
                        if self.result_revision != 0 =>
                    {
                        return Err(EditorNativeSaveErrorV0::State);
                    }
                    _ => {}
                }
                if self.source_body_len > MAX_TEXT_LEN {
                    return Err(EditorNativeSaveErrorV0::Bounds);
                }
                let (_, generation) = decode_shmem_reference(self.source_handle)?;
                if generation != self.source_generation {
                    return Err(EditorNativeSaveErrorV0::Handle);
                }
                if self.state == EditorNativeSaveChromeStateV0::Committed
                    && self.expected_revision.checked_add(1) != Some(self.result_revision)
                {
                    return Err(EditorNativeSaveErrorV0::Overflow);
                }
            }
        }
        Ok(())
    }

    pub fn encode_le(&self) -> Result<[u8; SAVE_CHROME_STATUS_LEN]> {
        self.validate()?;
        let mut out = [0; SAVE_CHROME_STATUS_LEN];
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
        if bytes.len() != SAVE_CHROME_STATUS_LEN {
            return Err(EditorNativeSaveErrorV0::Length);
        }
        let raw_state = read32(bytes, 8)?;
        let mut out = Self {
            schema_version: read32(bytes, 0)?,
            total_len: read32(bytes, 4)?,
            state: EditorNativeSaveChromeStateV0::NoSave,
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
            0 => EditorNativeSaveChromeStateV0::NoSave,
            1 => EditorNativeSaveChromeStateV0::Unknown,
            2 => EditorNativeSaveChromeStateV0::Committed,
            3 => EditorNativeSaveChromeStateV0::DefinitiveNoncommit,
            4 => EditorNativeSaveChromeStateV0::Unavailable,
            _ => return Err(EditorNativeSaveErrorV0::State),
        };
        out.validate()?;
        Ok(out)
    }

    /// Correlate supplied original operation data, without establishing authority.
    /// The caller must independently obtain the binding and original epoch from
    /// the actual service; the current observation epoch may differ.
    pub fn validate_binding(
        &self,
        binding: &EditorSaveBindingV0,
        original_backend_epoch: u64,
    ) -> Result<()> {
        self.validate()?;
        if !matches!(
            self.state,
            EditorNativeSaveChromeStateV0::Unknown
                | EditorNativeSaveChromeStateV0::Committed
                | EditorNativeSaveChromeStateV0::DefinitiveNoncommit
        ) {
            return Err(EditorNativeSaveErrorV0::State);
        }
        binding
            .validate()
            .map_err(|_| EditorNativeSaveErrorV0::Binding)?;
        if original_backend_epoch == 0 {
            return Err(EditorNativeSaveErrorV0::Epoch);
        }
        let (_, generation) = decode_shmem_reference(binding.source.source_object_id)?;
        if generation != binding.source.source_generation {
            return Err(EditorNativeSaveErrorV0::Handle);
        }
        let a = &binding.allocation;
        let source = &binding.source;
        if self.actor != a.actor
            || self.selected_object_id != a.selected_object_id
            || self.selected_object_generation != a.selected_generation
            || self.operation_id != a.operation_id
            || self.expected_revision != a.expected_revision
            || self.expected_content_hash != a.expected_content_hash
            || self.source_handle != source.source_object_id
            || self.source_generation != source.source_generation
            || self.source_body_len != source.byte_len
            || self.source_content_hash != source.content_hash
            || self.original_backend_epoch != original_backend_epoch
        {
            return Err(EditorNativeSaveErrorV0::Binding);
        }
        Ok(())
    }

    /// Validate the exact original 176-byte receipt and its complete source join.
    /// This is data correlation, not permit, journal or current-draft Saved proof.
    pub fn validate_receipt(
        &self,
        binding: &EditorSaveBindingV0,
        original_backend_epoch: u64,
        receipt_bytes: &[u8],
    ) -> Result<()> {
        self.validate()?;
        if !matches!(
            self.state,
            EditorNativeSaveChromeStateV0::Committed
                | EditorNativeSaveChromeStateV0::DefinitiveNoncommit
        ) {
            return Err(EditorNativeSaveErrorV0::State);
        }
        self.validate_binding(binding, original_backend_epoch)?;
        let receipt = EditorSaveReceiptV0::decode_le(receipt_bytes)
            .map_err(|_| EditorNativeSaveErrorV0::Receipt)?;
        receipt
            .validate_binding(binding)
            .map_err(|_| EditorNativeSaveErrorV0::Binding)?;
        let outcome = match self.state {
            EditorNativeSaveChromeStateV0::Committed => EditorReceiptOutcomeV0::Committed,
            EditorNativeSaveChromeStateV0::DefinitiveNoncommit => {
                EditorReceiptOutcomeV0::DefinitiveNoncommit
            }
            _ => return Err(EditorNativeSaveErrorV0::State),
        };
        if receipt.backend_service_epoch != original_backend_epoch
            || receipt.outcome != outcome
            || receipt.result_revision != self.result_revision
        {
            return Err(EditorNativeSaveErrorV0::Binding);
        }
        let digest: [u8; 32] = Sha256::digest(receipt_bytes).into();
        if digest != self.receipt_sha256 {
            return Err(EditorNativeSaveErrorV0::Binding);
        }
        Ok(())
    }
}
