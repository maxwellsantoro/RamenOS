//! Private, bounded host CAS IO. This module owns paths; consumers own no IO.
use super::{FixtureProfile, IoPoint, PausePoint, ProvisioningBinding, StoreStatus};
use crate::domain_visibility::{ArtifactOwner, DomainArtifactRegistry};
use artifact_store_schema::{ContentId, Manifest, editor_save::*, signature::*};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

pub(super) fn hash(bytes: &[u8]) -> Hash32 {
    Sha256::digest(bytes).into()
}
pub(super) trait Observer {
    fn journal_prepared(&self, journal: &EditorSelectionJournalV0) -> Result<(), StoreStatus>;
    fn before(&self, point: IoPoint, hash: Option<Hash32>) -> Result<(), StoreStatus>;
    fn after(&self, point: IoPoint, hash: Option<Hash32>, success: bool);
    fn pause(&self, point: PausePoint) -> Result<(), StoreStatus>;
}
fn io<T>(
    observer: &impl Observer,
    point: IoPoint,
    artifact: Option<Hash32>,
    action: impl FnOnce() -> Result<T, StoreStatus>,
) -> Result<T, StoreStatus> {
    observer.before(point, artifact)?;
    let result = action();
    observer.after(point, artifact, result.is_ok());
    result
}
fn err<T, E>(value: Result<T, E>) -> Result<T, StoreStatus> {
    value.map_err(|_| StoreStatus::Unknown)
}
pub(super) fn read(path: &Path, max: usize) -> Result<Vec<u8>, StoreStatus> {
    let mut file = err(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path))?;
    let meta = err(file.metadata())?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(StoreStatus::Unknown);
    }
    let mut out = Vec::new();
    err((&mut file).take(max as u64 + 1).read_to_end(&mut out))?;
    if out.len() > max {
        return Err(StoreStatus::Unknown);
    }
    Ok(out)
}
fn sync_dir(path: &Path) -> Result<(), StoreStatus> {
    let meta = err(fs::symlink_metadata(path))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(StoreStatus::Unknown);
    }
    err(err(File::open(path))?.sync_all())
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<File, StoreStatus> {
    let mut file = err(OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path))?;
    err(file.write_all(bytes))?;
    Ok(file)
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreStatus> {
    let tmp = path.with_extension("tmp");
    if tmp.try_exists().map_err(|_| StoreStatus::Unknown)? {
        return Err(StoreStatus::Unknown);
    }
    let file = write_file(&tmp, bytes)?;
    err(file.sync_all())?;
    drop(file);
    err(fs::rename(&tmp, path))?;
    sync_dir(path.parent().ok_or(StoreStatus::Unknown)?)?;
    Ok(())
}

// File-size checks precede typed decoding; nested visitors reject excess cardinality.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictManifest {
    schema_version: u32,
    content_id: String,
    size_bytes: u64,
    kind: String,
    channels: BoundedVec<String, 1>,
    signatures: BoundedVec<String, 1>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictSignature {
    algorithm: SignatureAlgorithm,
    signature_data: String,
    key_id: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    owner: ArtifactOwner,
    manifest: Manifest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictIntent {
    owner: ArtifactOwner,
    manifest: StrictManifest,
}
pub(super) fn make_manifest(
    profile: &FixtureProfile,
    bytes: &[u8],
) -> Result<Manifest, StoreStatus> {
    let mut manifest = Manifest {
        schema_version: 1,
        content_id: format!("sha256:{}", hex::encode(hash(bytes))),
        size_bytes: bytes.len() as u64,
        kind: "editor-selected-private".into(),
        channels: vec!["dev-host-editor".into()],
        signatures: vec![],
    };
    let key = SigningKey::from_bytes(&profile.fixture_signing_seed);
    let signature = ManifestSignature {
        algorithm: SignatureAlgorithm::Ed25519,
        signature_data: STANDARD.encode(
            key.sign(&err(manifest_signing_bytes(&manifest))?)
                .to_bytes(),
        ),
        key_id: profile.fixture_key_id.clone(),
        timestamp: None,
        signer: None,
    };
    manifest
        .signatures
        .push(err(serde_json::to_string(&signature))?);
    Ok(manifest)
}
fn verify_manifest(raw: StrictManifest, profile: &FixtureProfile) -> Result<Manifest, StoreStatus> {
    if raw.schema_version != 1
        || raw.content_id.len() != 71
        || ContentId::parse(&raw.content_id).is_err()
        || raw.size_bytes > 4096
        || raw.kind != "editor-selected-private"
        || raw.channels.as_slice() != ["dev-host-editor"]
        || raw.signatures.len() != 1
    {
        return Err(StoreStatus::Unknown);
    }
    let signatures = raw.signatures.as_slice().to_vec();
    if signatures[0].len() > 1024 {
        return Err(StoreStatus::Unknown);
    }
    let sig: StrictSignature = err(serde_json::from_str(&signatures[0]))?;
    if sig.algorithm != SignatureAlgorithm::Ed25519
        || sig.key_id != profile.fixture_key_id
        || sig.key_id.len() > 64
        || sig.signature_data.len() > 128
    {
        return Err(StoreStatus::Unknown);
    }
    let manifest = Manifest {
        schema_version: raw.schema_version,
        content_id: raw.content_id,
        size_bytes: raw.size_bytes,
        kind: raw.kind,
        channels: raw.channels.as_slice().to_vec(),
        signatures,
    };
    let mut keys = TrustedKeys::new();
    err(keys.add_ed25519_key(
        profile.fixture_key_id.clone(),
        SigningKey::from_bytes(&profile.fixture_signing_seed)
            .verifying_key()
            .as_bytes(),
    ))?;
    let config = SignatureValidationConfig {
        policy: SignaturePolicy::RequireSignature,
        trusted_key_ids: vec![profile.fixture_key_id.clone()],
        max_signature_age_secs: None,
        trusted_keys: keys,
    };
    if validate_manifest_signatures(
        &manifest.signatures,
        &err(manifest_signing_bytes(&manifest))?,
        &config,
    ) != SignatureValidationResult::Valid
    {
        return Err(StoreStatus::Unknown);
    }
    Ok(manifest)
}
fn manifest(path: &Path, profile: &FixtureProfile) -> Result<Manifest, StoreStatus> {
    verify_manifest(err(serde_json::from_slice(&read(path, 2048)?))?, profile)
}
fn owner(
    path: &Path,
    id: &ContentId,
    owner_id: u64,
) -> Result<(ArtifactOwner, Hash32), StoreStatus> {
    let bytes = read(path, 512)?;
    let record: ArtifactOwner = err(serde_json::from_slice(&bytes))?;
    if record.content_id != *id
        || ContentId::parse(record.content_id.as_str()).is_err()
        || record.domain_id != owner_id
        || record.is_global
    {
        return Err(StoreStatus::Unknown);
    }
    Ok((record, hash(&bytes)))
}
fn retained_manifest(
    j: &EditorSelectionJournalV0,
    id: &ContentId,
    manifest: &Manifest,
) -> Result<(), StoreStatus> {
    if j.initial.content_id().ok().as_ref() == Some(id)
        && hash(&err(serde_json::to_vec(manifest))?) != j.initial.manifest_hash
    {
        return Err(StoreStatus::Unknown);
    }
    Ok(())
}
fn allowed(
    j: &EditorSelectionJournalV0,
    id: &ContentId,
    len: u64,
    runtime: &[EditorCommitPermitV0],
) -> bool {
    (j.initial.content_id().ok().as_ref() == Some(id) && u64::from(j.initial.byte_len) == len)
        || j.operations
            .as_slice()
            .iter()
            .filter_map(|r| r.permit.as_ref())
            .chain(runtime.iter())
            .any(|p| {
                format!("sha256:{}", hex::encode(p.binding.source.content_hash)) == id.as_str()
                    && u64::from(p.binding.source.byte_len) == len
            })
}
pub(super) fn inventory(root: &Path) -> Result<(u32, u64), StoreStatus> {
    let mut count = 0u32;
    let mut bytes = 0u64;
    let meta = err(fs::symlink_metadata(root))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(StoreStatus::Unknown);
    }
    for entry in err(fs::read_dir(root))?.take(65) {
        let entry = err(entry)?;
        let meta = err(fs::symlink_metadata(entry.path()))?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(StoreStatus::Unknown);
        }
        count += 1;
        if count > 64 {
            return Err(StoreStatus::Exhausted);
        }
        bytes = bytes
            .checked_add(meta.len())
            .ok_or(StoreStatus::Exhausted)?;
        if bytes > 2 * 1024 * 1024 {
            return Err(StoreStatus::Exhausted);
        }
    }
    Ok((count, bytes))
}
/// Validate EVERY entry before any constructor with auto-recovery side effects.
pub(super) fn preflight(
    root: &Path,
    profile: &FixtureProfile,
    j: &EditorSelectionJournalV0,
    runtime: &[EditorCommitPermitV0],
) -> Result<(), StoreStatus> {
    inventory(root)?;
    let mut blobs = 0;
    for entry in err(fs::read_dir(root))?.take(65) {
        let entry = err(entry)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| StoreStatus::Unknown)?;
        let (stem, kind) = if let Some(v) = name.strip_suffix(".blob") {
            blobs += 1;
            (v, 0)
        } else if let Some(v) = name.strip_suffix(".manifest.json") {
            (v, 1)
        } else if let Some(v) = name.strip_suffix(".ownership.json") {
            (v, 2)
        } else if let Some(v) = name.strip_suffix(".publication.json") {
            (v, 3)
        } else {
            return Err(StoreStatus::Unknown);
        };
        if blobs > 17 {
            return Err(StoreStatus::Exhausted);
        }
        let id = ContentId::parse(&format!("sha256:{stem}")).map_err(|_| StoreStatus::Unknown)?;
        let blob = root.join(format!("{stem}.blob"));
        let meta = root.join(format!("{stem}.manifest.json"));
        let own = root.join(format!("{stem}.ownership.json"));
        let intent = root.join(format!("{stem}.publication.json"));
        match kind {
            3 => {
                let raw: StrictIntent = err(serde_json::from_slice(&read(&intent, 4096)?))?;
                let m = verify_manifest(raw.manifest, profile)?;
                retained_manifest(j, &id, &m)?;
                if raw.owner.content_id != id
                    || raw.owner.domain_id != j.owner_id
                    || raw.owner.is_global
                    || m.content_id != id.as_str()
                    || !allowed(j, &id, m.size_bytes, runtime)
                {
                    return Err(StoreStatus::Unknown);
                }
                if blob.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    let b = read(&blob, 4096)?;
                    if hash(&b) != parse_hash(&id) || b.len() as u64 != m.size_bytes {
                        return Err(StoreStatus::Unknown);
                    }
                }
                if meta.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    let existing = manifest(&meta, profile)?;
                    if err(serde_json::to_vec(&existing))? != err(serde_json::to_vec(&m))? {
                        return Err(StoreStatus::Unknown);
                    }
                }
                if own.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    owner(&own, &id, j.owner_id)?;
                }
            }
            _ => {
                let m = if meta.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    manifest(&meta, profile)?
                } else if intent.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    let raw: StrictIntent = err(serde_json::from_slice(&read(&intent, 4096)?))?;
                    verify_manifest(raw.manifest, profile)?
                } else {
                    return Err(StoreStatus::Unknown);
                };
                retained_manifest(j, &id, &m)?;
                if m.content_id != id.as_str() || !allowed(j, &id, m.size_bytes, runtime) {
                    return Err(StoreStatus::Unknown);
                }
                let b = read(&blob, 4096)?;
                if hash(&b) != parse_hash(&id) || b.len() as u64 != m.size_bytes {
                    return Err(StoreStatus::Unknown);
                }
                if own.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    owner(&own, &id, j.owner_id)?;
                } else if !intent.try_exists().map_err(|_| StoreStatus::Unknown)? {
                    return Err(StoreStatus::Unknown);
                }
            }
        }
    }
    Ok(())
}
fn parse_hash(id: &ContentId) -> Hash32 {
    hex::decode(id.hash_hex()).unwrap().try_into().unwrap()
}
pub(super) struct Consumed {
    pub bytes: Vec<u8>,
    pub manifest_hash: Hash32,
    pub owner_hash: Hash32,
}
pub(super) fn consume(
    cas: &Path,
    profile: &FixtureProfile,
    selected: &SelectedArtifactV0,
) -> Result<Consumed, StoreStatus> {
    let id = selected.content_id().map_err(|_| StoreStatus::Unknown)?;
    let manifest = manifest(
        &cas.join(format!("{}.manifest.json", id.hash_hex())),
        profile,
    )?;
    let bytes = read(&cas.join(format!("{}.blob", id.hash_hex())), 4096)?;
    let (_, owner_hash) = owner(
        &cas.join(format!("{}.ownership.json", id.hash_hex())),
        &id,
        selected.owner_id,
    )?;
    let manifest_hash = hash(&err(serde_json::to_vec(&manifest))?);
    if manifest.content_id != id.as_str()
        || bytes.len() as u64 != manifest.size_bytes
        || hash(&bytes) != selected.content_hash
        || bytes.len() != selected.byte_len as usize
        || manifest_hash != selected.manifest_hash
    {
        return Err(StoreStatus::Unknown);
    }
    EditorTextHeaderV0::try_new(selected.selected_object_id, selected.revision, &bytes)
        .map_err(|_| StoreStatus::Unknown)?;
    Ok(Consumed {
        bytes,
        manifest_hash,
        owner_hash,
    })
}
pub(super) fn publish(
    cas: &Path,
    profile: &FixtureProfile,
    journal: &EditorSelectionJournalV0,
    bytes: &[u8],
    profile_hash: Hash32,
    provisioning: Option<ProvisioningBinding>,
    observer: &impl Observer,
) -> Result<Consumed, StoreStatus> {
    let candidate = make_manifest(profile, bytes)?;
    if let Some(binding) = provisioning {
        binding.check(&journal.initial, profile_hash, bytes, &candidate)?;
    } else if !journal.operations.as_slice().iter().any(|row| {
        row.permit.as_ref().is_some_and(|permit| {
            permit.binding.allocation.actor.owner_id == journal.owner_id
                && permit.binding.allocation.selected_object_id == journal.selected_object_id
                && permit.binding.allocation.selected_generation == journal.selected_generation
                && permit.binding.source.content_hash == hash(bytes)
                && permit.binding.source.byte_len as usize == bytes.len()
        })
    }) {
        return Err(StoreStatus::Denied);
    }
    let id = ContentId::parse(&candidate.content_id).map_err(|_| StoreStatus::Unknown)?;
    preflight(cas, profile, journal, &[])?;
    let mut registry = err(DomainArtifactRegistry::new(cas))?;
    err(registry.check_publication(cas, &id, journal.owner_id, false))?;
    let manifest_path = cas.join(format!("{}.manifest.json", id.hash_hex()));
    let blob_path = cas.join(format!("{}.blob", id.hash_hex()));
    let manifest_hash = hash(&err(serde_json::to_vec(&candidate))?);
    let selected = SelectedArtifactV0::try_new(
        journal.owner_id,
        journal.selected_object_id,
        journal.selected_generation,
        journal.current.revision,
        hash(bytes),
        bytes.len() as u32,
        manifest_hash,
    )
    .map_err(|_| StoreStatus::Unknown)?;
    if blob_path.try_exists().map_err(|_| StoreStatus::Unknown)? {
        let existing = manifest(&manifest_path, profile)?;
        let existing_selected = SelectedArtifactV0 {
            manifest_hash: hash(&err(serde_json::to_vec(&existing))?),
            ..selected
        };
        return consume(cas, profile, &existing_selected);
    }
    let intent_path = cas.join(format!("{}.publication.json", id.hash_hex()));
    let intent = Intent {
        owner: ArtifactOwner {
            content_id: id.clone(),
            domain_id: journal.owner_id,
            is_global: false,
            ingested_at: 0,
        },
        manifest: candidate.clone(),
    };
    io(
        observer,
        IoPoint::CandidateIntent,
        Some(manifest_hash),
        || atomic(&intent_path, &err(serde_json::to_vec(&intent))?),
    )?;
    let tmp = blob_path.with_extension("tmp");
    let file = io(
        observer,
        IoPoint::CandidateBlobWrite,
        Some(hash(bytes)),
        || write_file(&tmp, bytes),
    )?;
    io(
        observer,
        IoPoint::CandidateBlobSync,
        Some(hash(bytes)),
        || err(file.sync_all()),
    )?;
    drop(file);
    err(fs::rename(&tmp, &blob_path))?;
    io(
        observer,
        IoPoint::CandidateManifest,
        Some(manifest_hash),
        || {
            err(artifact_store_core::write_manifest_atomic(
                &manifest_path,
                &candidate,
            ))
        },
    )?;
    io(observer, IoPoint::CandidateOwner, None, || {
        err(registry.register_artifact(&id, journal.owner_id, false))
    })?;
    err(fs::remove_file(&intent_path))?;
    io(
        observer,
        IoPoint::CandidateDirectorySync,
        Some(manifest_hash),
        || sync_dir(cas),
    )?;
    io(
        observer,
        IoPoint::CandidateReadback,
        Some(hash(bytes)),
        || consume(cas, profile, &selected),
    )
}
pub(super) fn publish_journal(
    root: &Path,
    journal: &EditorSelectionJournalV0,
    observer: &impl Observer,
    selection: bool,
) -> Result<Hash32, StoreStatus> {
    let bytes = journal
        .canonical_bytes()
        .map_err(|_| StoreStatus::Internal)?;
    observer.journal_prepared(journal)?;
    let digest = hash(&bytes);
    let next = root.join("selection.next");
    let file = io(observer, IoPoint::JournalWrite, Some(digest), || {
        write_file(&next, &bytes)
    })?;
    io(observer, IoPoint::JournalSync, Some(digest), || {
        err(file.sync_all())
    })?;
    drop(file);
    if selection {
        observer.pause(PausePoint::BeforeSelectionRename)?;
    }
    io(observer, IoPoint::JournalRename, Some(digest), || {
        err(fs::rename(&next, root.join("selection.json")))
    })?;
    if selection {
        observer.pause(PausePoint::AfterSelectionRename)?;
    }
    io(
        observer,
        IoPoint::JournalDirectorySync,
        Some(digest),
        || sync_dir(root),
    )?;
    io(observer, IoPoint::JournalReadback, Some(digest), || {
        let actual = read(&root.join("selection.json"), 131072)?;
        let parsed = EditorSelectionJournalV0::decode_canonical(&actual)
            .map_err(|_| StoreStatus::Unknown)?;
        if parsed.digest().map_err(|_| StoreStatus::Unknown)? != digest || actual != bytes {
            return Err(StoreStatus::Unknown);
        }
        Ok(digest)
    })
}
pub(super) fn read_journal(root: &Path) -> Result<EditorSelectionJournalV0, StoreStatus> {
    EditorSelectionJournalV0::decode_canonical(&read(&root.join("selection.json"), 131072)?)
        .map_err(|_| StoreStatus::Unknown)
}
pub(super) fn fixed_root(root: &Path, id: u64) -> PathBuf {
    root.join(format!("object-{id}"))
}

// Diagnostics count links as links, without following them or accepting their contents.
// Admission still uses strict inventory/preflight above.
pub(super) fn diagnostic_inventory(root: &Path) -> Result<(u32, u64), StoreStatus> {
    let mut count = 0u32;
    let mut bytes = 0u64;
    let metadata = err(fs::symlink_metadata(root))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StoreStatus::Unknown);
    }
    for entry in err(fs::read_dir(root))?.take(65) {
        let entry = err(entry)?;
        let metadata = err(fs::symlink_metadata(entry.path()))?;
        count += 1;
        if count > 64 {
            return Err(StoreStatus::Exhausted);
        }
        bytes = bytes
            .checked_add(metadata.len())
            .ok_or(StoreStatus::Exhausted)?;
        if bytes > 2 * 1024 * 1024 {
            return Err(StoreStatus::Exhausted);
        }
    }
    Ok((count, bytes))
}
