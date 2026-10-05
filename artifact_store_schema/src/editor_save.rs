//! Pure UI1.1b payloads, bounded journal validation and portable little-endian codecs.
//! Deserialization and constructors establish no grant, commit permit or IO authority.
use crate::{ContentId, Vec};
use core::{fmt, marker::PhantomData};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

pub const VERSION: u32 = 1;
pub const TEXT_HEADER_LEN: usize = 64;
pub const RECEIPT_LEN: usize = 176;
pub const MAX_TEXT_LEN: usize = 4096;
pub const MAX_OPERATIONS: usize = 16;
pub const MAX_JOURNAL_BYTES: usize = 131072;

pub type Hash32 = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorSaveError {
    Version,
    Bounds,
    Identity,
    ContentId,
    Binding,
    State,
    Chain,
    Reserved,
    Ascii,
    Overflow,
    Json,
    NonCanonical,
}

// Derive Serialize + Deserialize + Clone + Debug + PartialEq + Eq for
// every following plain record; deny_unknown_fields on every record.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorActorV0 {
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectedArtifactV0 {
    pub schema_version: u32,
    pub owner_id: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub revision: u64,
    pub content_hash: Hash32,
    pub byte_len: u32,
    pub manifest_hash: Hash32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorSaveAllocationV0 {
    pub schema_version: u32,
    pub actor: EditorActorV0,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub operation_id: u64,
    pub expected_revision: u64,
    pub expected_content_hash: Hash32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorSourceBindingV0 {
    pub source_object_id: u64,  // canonical packed SHM reference
    pub source_generation: u64, // checked widened32, never truncation
    pub byte_len: u32,          // ASCII body length, not header+body
    pub content_hash: Hash32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorSaveBindingV0 {
    pub schema_version: u32,
    pub allocation: EditorSaveAllocationV0,
    pub source: EditorSourceBindingV0,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorCommitPermitV0 {
    pub schema_version: u32,
    pub binding: EditorSaveBindingV0,
    pub successor_revision: u64,
    pub service_epoch: u64,
    pub writer_id: u64,
    pub admission_epoch: u64,
}
#[repr(u32)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorReceiptOutcomeV0 {
    Committed = 0,
    DefinitiveNoncommit = 1,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorSaveReceiptV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub outcome: EditorReceiptOutcomeV0,
    pub reserved: u32,
    pub owner_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub original_instance_id: u64,
    pub original_instance_generation: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub operation_id: u64,
    pub expected_revision: u64,
    pub result_revision: u64,
    pub backend_service_epoch: u64,
    pub committed_at_ms: u64,
    pub expected_content_hash: Hash32,
    pub source_content_hash: Hash32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorTextHeaderV0 {
    pub schema_version: u32,
    pub total_len: u32,
    pub selected_object_id: u64,
    pub revision: u64,
    pub content_hash: Hash32,
    pub byte_len: u32,
    pub reserved: u32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorOperationStateV0 {
    Allocated,
    Submitted,
    Permitted,
    Committed,
    Noncommit,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditorClosureV0 {
    None,
    Deadline,
    Revoked,
    Expired,
    ServiceRetired,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorOperationRecordV0 {
    pub allocation: EditorSaveAllocationV0,
    pub allocated_service_epoch: u64,
    pub binding: Option<EditorSaveBindingV0>,
    pub submitted_service_epoch: Option<u64>,
    pub permit: Option<EditorCommitPermitV0>,
    pub closure: EditorClosureV0,
    pub state: EditorOperationStateV0,
    pub successor: Option<SelectedArtifactV0>,
    pub receipt: Option<EditorSaveReceiptV0>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorSelectionJournalV0 {
    pub schema_version: u32,
    pub owner_id: u64,
    pub selected_object_id: u64,
    pub selected_generation: u64,
    pub signing_policy_hash: Hash32,
    pub initial: SelectedArtifactV0,
    pub current: SelectedArtifactV0,
    pub service_epoch: u64,
    pub operation_high_water: u64,
    pub writer_high_water: u64,
    pub transition_sequence: u64,
    pub prior_journal_hash: Hash32,
    pub operations: BoundedVec<EditorOperationRecordV0, 16>,
}

impl fmt::Display for EditorSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "editor save payload: {self:?}")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for EditorSaveError {}

type Result<T> = core::result::Result<T, EditorSaveError>;
fn ensure(test: bool, error: EditorSaveError) -> Result<()> {
    if test { Ok(()) } else { Err(error) }
}
fn positive(values: &[u64]) -> Result<()> {
    ensure(values.iter().all(|v| *v != 0), EditorSaveError::Identity)
}
fn version(v: u32) -> Result<()> {
    ensure(v == VERSION, EditorSaveError::Version)
}
fn length(v: u32) -> Result<()> {
    ensure(v as usize <= MAX_TEXT_LEN, EditorSaveError::Bounds)
}
fn body(bytes: &[u8]) -> Result<()> {
    ensure(bytes.len() <= MAX_TEXT_LEN, EditorSaveError::Bounds)?;
    ensure(
        bytes.iter().all(|b| matches!(*b, 9 | 10 | 32..=126)),
        EditorSaveError::Ascii,
    )
}
fn digest(bytes: &[u8]) -> Hash32 {
    Sha256::digest(bytes).into()
}

impl EditorActorV0 {
    pub fn try_new(
        owner_id: u64,
        session_id: u64,
        session_generation: u64,
        instance_id: u64,
        instance_generation: u64,
    ) -> Result<Self> {
        let v = Self {
            owner_id,
            session_id,
            session_generation,
            instance_id,
            instance_generation,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        positive(&[
            self.owner_id,
            self.session_id,
            self.session_generation,
            self.instance_id,
            self.instance_generation,
        ])
    }
}
impl SelectedArtifactV0 {
    pub fn try_new(
        owner_id: u64,
        object_id: u64,
        generation: u64,
        revision: u64,
        content_hash: Hash32,
        byte_len: u32,
        manifest_hash: Hash32,
    ) -> Result<Self> {
        let v = Self {
            schema_version: VERSION,
            owner_id,
            selected_object_id: object_id,
            selected_generation: generation,
            revision,
            content_hash,
            byte_len,
            manifest_hash,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        positive(&[
            self.owner_id,
            self.selected_object_id,
            self.selected_generation,
            self.revision,
        ])?;
        length(self.byte_len)
    }
    pub fn content_id(&self) -> Result<ContentId> {
        self.validate()?;
        ContentId::parse(&format!("sha256:{}", hex::encode(self.content_hash)))
            .map_err(|_| EditorSaveError::ContentId)
    }
}
impl EditorSaveAllocationV0 {
    pub fn try_new(
        actor: EditorActorV0,
        base: &SelectedArtifactV0,
        operation_id: u64,
    ) -> Result<Self> {
        base.validate()?;
        ensure(actor.owner_id == base.owner_id, EditorSaveError::Binding)?;
        let v = Self {
            schema_version: VERSION,
            actor,
            selected_object_id: base.selected_object_id,
            selected_generation: base.selected_generation,
            operation_id,
            expected_revision: base.revision,
            expected_content_hash: base.content_hash,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        self.actor.validate()?;
        positive(&[
            self.selected_object_id,
            self.selected_generation,
            self.operation_id,
            self.expected_revision,
        ])
    }
}
impl EditorSourceBindingV0 {
    pub fn try_new(
        object_id: u64,
        generation: u64,
        byte_len: u32,
        content_hash: Hash32,
    ) -> Result<Self> {
        let v = Self {
            source_object_id: object_id,
            source_generation: generation,
            byte_len,
            content_hash,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        positive(&[self.source_object_id, self.source_generation])?;
        ensure(
            self.source_generation <= u64::from(u32::MAX),
            EditorSaveError::Bounds,
        )?;
        length(self.byte_len)
    }
}
impl EditorSaveBindingV0 {
    pub fn try_new(
        allocation: EditorSaveAllocationV0,
        source: EditorSourceBindingV0,
    ) -> Result<Self> {
        let v = Self {
            schema_version: VERSION,
            allocation,
            source,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        self.allocation.validate()?;
        self.source.validate()
    }
}
impl EditorCommitPermitV0 {
    pub fn try_new(
        binding: EditorSaveBindingV0,
        service_epoch: u64,
        writer_id: u64,
        admission_epoch: u64,
    ) -> Result<Self> {
        let successor_revision = binding
            .allocation
            .expected_revision
            .checked_add(1)
            .ok_or(EditorSaveError::Overflow)?;
        let v = Self {
            schema_version: VERSION,
            binding,
            successor_revision,
            service_epoch,
            writer_id,
            admission_epoch,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        self.binding.validate()?;
        positive(&[self.service_epoch, self.writer_id, self.admission_epoch])?;
        ensure(
            self.binding.allocation.expected_revision.checked_add(1)
                == Some(self.successor_revision),
            EditorSaveError::Overflow,
        )
    }
}
impl EditorSaveReceiptV0 {
    fn from_binding(
        binding: &EditorSaveBindingV0,
        epoch: u64,
        outcome: EditorReceiptOutcomeV0,
        revision: u64,
        committed_at_ms: u64,
    ) -> Result<Self> {
        binding.validate()?;
        let a = &binding.allocation;
        let actor = &a.actor;
        let v = Self {
            schema_version: VERSION,
            total_len: RECEIPT_LEN as u32,
            outcome,
            reserved: 0,
            owner_id: actor.owner_id,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            original_instance_id: actor.instance_id,
            original_instance_generation: actor.instance_generation,
            selected_object_id: a.selected_object_id,
            selected_generation: a.selected_generation,
            operation_id: a.operation_id,
            expected_revision: a.expected_revision,
            result_revision: revision,
            backend_service_epoch: epoch,
            committed_at_ms,
            expected_content_hash: a.expected_content_hash,
            source_content_hash: binding.source.content_hash,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn committed(permit: &EditorCommitPermitV0, committed_at_ms: u64) -> Result<Self> {
        permit.validate()?;
        Self::from_binding(
            &permit.binding,
            permit.service_epoch,
            EditorReceiptOutcomeV0::Committed,
            permit.successor_revision,
            committed_at_ms,
        )
    }
    pub fn noncommit(binding: &EditorSaveBindingV0, original_service_epoch: u64) -> Result<Self> {
        Self::from_binding(
            binding,
            original_service_epoch,
            EditorReceiptOutcomeV0::DefinitiveNoncommit,
            0,
            0,
        )
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        ensure(
            self.total_len == RECEIPT_LEN as u32,
            EditorSaveError::Bounds,
        )?;
        ensure(self.reserved == 0, EditorSaveError::Reserved)?;
        positive(&[
            self.owner_id,
            self.session_id,
            self.session_generation,
            self.original_instance_id,
            self.original_instance_generation,
            self.selected_object_id,
            self.selected_generation,
            self.operation_id,
            self.expected_revision,
            self.backend_service_epoch,
        ])?;
        match self.outcome {
            EditorReceiptOutcomeV0::Committed => ensure(
                self.expected_revision.checked_add(1) == Some(self.result_revision),
                EditorSaveError::Overflow,
            ),
            EditorReceiptOutcomeV0::DefinitiveNoncommit => ensure(
                self.result_revision == 0 && self.committed_at_ms == 0,
                EditorSaveError::State,
            ),
        }
    }
    pub fn validate_binding(&self, binding: &EditorSaveBindingV0) -> Result<()> {
        self.validate()?;
        binding.validate()?;
        let a = &binding.allocation;
        let actor = &a.actor;
        ensure(
            self.owner_id == actor.owner_id
                && self.session_id == actor.session_id
                && self.session_generation == actor.session_generation
                && self.original_instance_id == actor.instance_id
                && self.original_instance_generation == actor.instance_generation
                && self.selected_object_id == a.selected_object_id
                && self.selected_generation == a.selected_generation
                && self.operation_id == a.operation_id
                && self.expected_revision == a.expected_revision
                && self.expected_content_hash == a.expected_content_hash
                && self.source_content_hash == binding.source.content_hash,
            EditorSaveError::Binding,
        )
    }
    pub fn encode_le(&self) -> Result<[u8; RECEIPT_LEN]> {
        self.validate()?;
        let mut wire = [0; RECEIPT_LEN];
        for (i, v) in [
            self.schema_version,
            self.total_len,
            self.outcome as u32,
            self.reserved,
        ]
        .iter()
        .enumerate()
        {
            put32(&mut wire, i * 4, *v);
        }
        for (i, v) in [
            self.owner_id,
            self.session_id,
            self.session_generation,
            self.original_instance_id,
            self.original_instance_generation,
            self.selected_object_id,
            self.selected_generation,
            self.operation_id,
            self.expected_revision,
            self.result_revision,
            self.backend_service_epoch,
            self.committed_at_ms,
        ]
        .iter()
        .enumerate()
        {
            put64(&mut wire, 16 + i * 8, *v);
        }
        wire[112..144].copy_from_slice(&self.expected_content_hash);
        wire[144..176].copy_from_slice(&self.source_content_hash);
        Ok(wire)
    }
    pub fn decode_le(bytes: &[u8]) -> Result<Self> {
        ensure(bytes.len() == RECEIPT_LEN, EditorSaveError::Bounds)?;
        let outcome = match get32(bytes, 8) {
            0 => EditorReceiptOutcomeV0::Committed,
            1 => EditorReceiptOutcomeV0::DefinitiveNoncommit,
            _ => return Err(EditorSaveError::State),
        };
        let v = Self {
            schema_version: get32(bytes, 0),
            total_len: get32(bytes, 4),
            outcome,
            reserved: get32(bytes, 12),
            owner_id: get64(bytes, 16),
            session_id: get64(bytes, 24),
            session_generation: get64(bytes, 32),
            original_instance_id: get64(bytes, 40),
            original_instance_generation: get64(bytes, 48),
            selected_object_id: get64(bytes, 56),
            selected_generation: get64(bytes, 64),
            operation_id: get64(bytes, 72),
            expected_revision: get64(bytes, 80),
            result_revision: get64(bytes, 88),
            backend_service_epoch: get64(bytes, 96),
            committed_at_ms: get64(bytes, 104),
            expected_content_hash: array32(bytes, 112),
            source_content_hash: array32(bytes, 144),
        };
        v.validate()?;
        Ok(v)
    }
}
impl EditorTextHeaderV0 {
    pub fn try_new(object_id: u64, revision: u64, bytes: &[u8]) -> Result<Self> {
        body(bytes)?;
        let v = Self {
            schema_version: VERSION,
            total_len: (TEXT_HEADER_LEN + bytes.len()) as u32,
            selected_object_id: object_id,
            revision,
            content_hash: digest(bytes),
            byte_len: bytes.len() as u32,
            reserved: 0,
        };
        v.validate_bytes(bytes)?;
        Ok(v)
    }
    fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        positive(&[self.selected_object_id, self.revision])?;
        length(self.byte_len)?;
        ensure(self.reserved == 0, EditorSaveError::Reserved)?;
        ensure(
            self.byte_len.checked_add(TEXT_HEADER_LEN as u32) == Some(self.total_len),
            EditorSaveError::Bounds,
        )
    }
    pub fn validate_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.validate()?;
        body(bytes)?;
        ensure(
            bytes.len() == self.byte_len as usize,
            EditorSaveError::Bounds,
        )?;
        ensure(digest(bytes) == self.content_hash, EditorSaveError::Binding)
    }
    pub fn encode_le(&self) -> Result<[u8; TEXT_HEADER_LEN]> {
        self.validate()?;
        let mut wire = [0; TEXT_HEADER_LEN];
        put32(&mut wire, 0, self.schema_version);
        put32(&mut wire, 4, self.total_len);
        put64(&mut wire, 8, self.selected_object_id);
        put64(&mut wire, 16, self.revision);
        wire[24..56].copy_from_slice(&self.content_hash);
        put32(&mut wire, 56, self.byte_len);
        put32(&mut wire, 60, self.reserved);
        Ok(wire)
    }
    pub fn decode_le(bytes: &[u8]) -> Result<Self> {
        ensure(bytes.len() == TEXT_HEADER_LEN, EditorSaveError::Bounds)?;
        let v = Self {
            schema_version: get32(bytes, 0),
            total_len: get32(bytes, 4),
            selected_object_id: get64(bytes, 8),
            revision: get64(bytes, 16),
            content_hash: array32(bytes, 24),
            byte_len: get32(bytes, 56),
            reserved: get32(bytes, 60),
        };
        v.validate()?;
        Ok(v)
    }
}
// Callers have already checked exact wire length; no unaligned casts or native layout copies.
fn put32(bytes: &mut [u8], offset: usize, v: u32) {
    bytes[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(bytes: &mut [u8], offset: usize, v: u64) {
    bytes[offset..offset + 8].copy_from_slice(&v.to_le_bytes());
}
fn get32(bytes: &[u8], offset: usize) -> u32 {
    let mut out = [0; 4];
    out.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(out)
}
fn get64(bytes: &[u8], offset: usize) -> u64 {
    let mut out = [0; 8];
    out.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(out)
}
fn array32(bytes: &[u8], offset: usize) -> Hash32 {
    let mut out = [0; 32];
    out.copy_from_slice(&bytes[offset..offset + 32]);
    out
}

/// A vector whose deserializer never allocates an unbounded input collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedVec<T, const N: usize>(Vec<T>);
impl<T, const N: usize> BoundedVec<T, N> {
    pub fn try_from_vec(values: Vec<T>) -> Result<Self> {
        ensure(values.len() <= N, EditorSaveError::Bounds)?;
        Ok(Self(values))
    }
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn try_push(&mut self, value: T) -> Result<()> {
        ensure(self.len() < N, EditorSaveError::Bounds)?;
        self.0.push(value);
        Ok(())
    }
}
impl<T: Serialize, const N: usize> Serialize for BoundedVec<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for BoundedVec<T, N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Self, D::Error> {
        struct BoundedVisitor<T, const N: usize>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for BoundedVisitor<T, N> {
            type Value = BoundedVec<T, N>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "at most {N} elements")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> core::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while values.len() < N {
                    match seq.next_element::<T>()? {
                        Some(v) => values.push(v),
                        None => return Ok(BoundedVec(values)),
                    }
                }
                // Consume no seventeenth record: a rejecting seed observes only whether
                // another element exists, without deserializing or allocating its payload.
                struct RejectExtra;
                impl<'de> serde::de::DeserializeSeed<'de> for RejectExtra {
                    type Value = ();
                    fn deserialize<D: Deserializer<'de>>(
                        self,
                        _: D,
                    ) -> core::result::Result<(), D::Error> {
                        Err(D::Error::custom("bounded vector capacity exceeded"))
                    }
                }
                seq.next_element_seed(RejectExtra)?;
                Ok(BoundedVec(values))
            }
        }
        deserializer.deserialize_seq(BoundedVisitor::<T, N>(PhantomData))
    }
}

impl EditorSelectionJournalV0 {
    pub fn try_new(initial: SelectedArtifactV0, signing_policy_hash: Hash32) -> Result<Self> {
        initial.validate()?;
        let v = Self {
            schema_version: VERSION,
            owner_id: initial.owner_id,
            selected_object_id: initial.selected_object_id,
            selected_generation: initial.selected_generation,
            signing_policy_hash,
            current: initial.clone(),
            initial,
            service_epoch: 1,
            operation_high_water: 0,
            writer_high_water: 0,
            transition_sequence: 1,
            prior_journal_hash: [0; 32],
            operations: BoundedVec::try_from_vec(Vec::new())?,
        };
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<()> {
        version(self.schema_version)?;
        positive(&[
            self.owner_id,
            self.selected_object_id,
            self.selected_generation,
            self.service_epoch,
            self.transition_sequence,
        ])?;
        self.validate_selected(&self.initial)?;
        self.validate_selected(&self.current)?;
        ensure(
            self.operations.len() <= MAX_OPERATIONS,
            EditorSaveError::Bounds,
        )?;
        let mut selected = &self.initial;
        let mut last_operation = 0;
        let mut active = 0;
        for (index, record) in self.operations.as_slice().iter().enumerate() {
            let a = &record.allocation;
            a.validate()?;
            ensure(
                a.actor.owner_id == self.owner_id
                    && a.selected_object_id == self.selected_object_id
                    && a.selected_generation == self.selected_generation,
                EditorSaveError::Binding,
            )?;
            ensure(
                a.operation_id > last_operation && a.operation_id <= self.operation_high_water,
                EditorSaveError::State,
            )?;
            last_operation = a.operation_id;
            positive(&[record.allocated_service_epoch])?;
            ensure(
                record.allocated_service_epoch <= self.service_epoch,
                EditorSaveError::State,
            )?;
            match (&record.binding, record.submitted_service_epoch) {
                (None, None) => {}
                (Some(binding), Some(epoch)) => {
                    binding.validate()?;
                    ensure(binding.allocation == *a, EditorSaveError::Binding)?;
                    ensure(
                        epoch >= record.allocated_service_epoch && epoch <= self.service_epoch,
                        EditorSaveError::State,
                    )?;
                }
                _ => return Err(EditorSaveError::State),
            }
            if let Some(permit) = &record.permit {
                permit.validate()?;
                ensure(
                    Some(&permit.binding) == record.binding.as_ref(),
                    EditorSaveError::Binding,
                )?;
                ensure(
                    Some(permit.service_epoch) == record.submitted_service_epoch
                        && permit.writer_id <= self.writer_high_water,
                    EditorSaveError::State,
                )?;
                ensure(
                    !self.operations.as_slice()[..index].iter().any(|previous| {
                        previous
                            .permit
                            .as_ref()
                            .is_some_and(|p| p.writer_id == permit.writer_id)
                    }),
                    EditorSaveError::State,
                )?;
            }
            if let Some(receipt) = &record.receipt {
                let binding = record.binding.as_ref().ok_or(EditorSaveError::State)?;
                receipt.validate_binding(binding)?;
                ensure(
                    Some(receipt.backend_service_epoch) == record.submitted_service_epoch,
                    EditorSaveError::Binding,
                )?;
            }
            if let Some(successor) = &record.successor {
                self.validate_selected(successor)?;
            }
            match record.state {
                EditorOperationStateV0::Allocated => {
                    ensure(
                        record.binding.is_none()
                            && record.permit.is_none()
                            && record.receipt.is_none()
                            && record.successor.is_none()
                            && record.closure == EditorClosureV0::None,
                        EditorSaveError::State,
                    )?;
                }
                EditorOperationStateV0::Submitted => {
                    ensure(
                        record.binding.is_some()
                            && record.permit.is_none()
                            && record.receipt.is_none()
                            && record.successor.is_none(),
                        EditorSaveError::State,
                    )?;
                    if record.closure == EditorClosureV0::None {
                        active += 1;
                    }
                }
                EditorOperationStateV0::Permitted => {
                    ensure(
                        record.binding.is_some()
                            && record.permit.is_some()
                            && record.receipt.is_none()
                            && record.successor.is_none(),
                        EditorSaveError::State,
                    )?;
                    active += 1;
                }
                EditorOperationStateV0::Committed => {
                    let permit = record.permit.as_ref().ok_or(EditorSaveError::State)?;
                    let receipt = record.receipt.as_ref().ok_or(EditorSaveError::State)?;
                    let successor = record.successor.as_ref().ok_or(EditorSaveError::State)?;
                    ensure(
                        receipt.outcome == EditorReceiptOutcomeV0::Committed
                            && receipt.result_revision == permit.successor_revision
                            && successor.revision == permit.successor_revision
                            && successor.content_hash == permit.binding.source.content_hash
                            && successor.byte_len == permit.binding.source.byte_len,
                        EditorSaveError::Binding,
                    )?;
                    ensure(
                        a.expected_revision == selected.revision
                            && a.expected_content_hash == selected.content_hash,
                        EditorSaveError::Chain,
                    )?;
                    selected = successor;
                }
                EditorOperationStateV0::Noncommit => {
                    ensure(record.successor.is_none(), EditorSaveError::State)?;
                    match (&record.binding, &record.receipt) {
                        (None, None) => ensure(record.permit.is_none(), EditorSaveError::State)?,
                        (Some(_), Some(receipt)) => ensure(
                            receipt.outcome == EditorReceiptOutcomeV0::DefinitiveNoncommit,
                            EditorSaveError::State,
                        )?,
                        _ => return Err(EditorSaveError::State),
                    }
                }
            }
        }
        ensure(active <= 1, EditorSaveError::State)?;
        ensure(*selected == self.current, EditorSaveError::Chain)
    }
    fn validate_selected(&self, selected: &SelectedArtifactV0) -> Result<()> {
        selected.validate()?;
        ensure(
            selected.owner_id == self.owner_id
                && selected.selected_object_id == self.selected_object_id
                && selected.selected_generation == self.selected_generation,
            EditorSaveError::Binding,
        )
    }
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| EditorSaveError::Json)?;
        ensure(bytes.len() <= MAX_JOURNAL_BYTES, EditorSaveError::Bounds)?;
        Ok(bytes)
    }
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        ensure(
            !bytes.is_empty() && bytes.len() <= MAX_JOURNAL_BYTES,
            EditorSaveError::Bounds,
        )?;
        let parsed: Self = serde_json::from_slice(bytes).map_err(|_| EditorSaveError::Json)?;
        ensure(
            parsed.canonical_bytes()?.as_slice() == bytes,
            EditorSaveError::NonCanonical,
        )?;
        Ok(parsed)
    }
    pub fn digest(&self) -> Result<Hash32> {
        Ok(digest(&self.canonical_bytes()?))
    }
}
