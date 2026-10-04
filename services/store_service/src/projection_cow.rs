//! S10.3.4 copy-on-write projection writes — scratch/commit without mutating CAS history.
//!
//! v0: single-shot `commit_projection_write` with in-memory replacement bytes.
//! The S10.3.3 read-only 9p export is unchanged; compat writes reach this path later.

use crate::projection_index::ProjectionIndexStore;
use artifact_store_core::{hash_bytes, write_blob_bytes_atomic};
use artifact_store_schema::projection_storage::{PathProjectionV0, SemanticIndexEntryV0};
use artifact_store_schema::{ContentId, Manifest};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ProjectionCowError {
    #[error("projection index error: {0}")]
    Index(#[from] crate::projection_index::ProjectionIndexError),

    #[error("virtual path not projected: {0}")]
    PathNotProjected(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid content id: {0}")]
    InvalidContentId(String),

    #[error("artifact ownership error: {0}")]
    Ownership(#[from] anyhow::Error),
}

/// Replacement payload for a single CoW commit (v0 scratch buffer).
pub struct ProjectionWriteCommit<'a> {
    pub virtual_path: &'a str,
    pub replacement_bytes: &'a [u8],
    pub kind: &'a str,
    pub channel: &'a str,
    pub domain_id: u64,
}

/// Outcome of a successful CoW commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionWriteCommitResult {
    pub virtual_path: String,
    pub prior_content_id: String,
    pub new_content_id: String,
}

/// Commit replacement bytes for a projected virtual path.
///
/// Ingests a fresh CAS blob, repoints the path projection, and leaves the prior blob intact.
/// Callers serialize all writers to this store and share the live ownership registry.
/// A domain may overlay its own or a globally readable source with owned content.
pub fn commit_projection_write(
    store_root: &Path,
    projection_index: &mut ProjectionIndexStore,
    domain_registry: &mut crate::DomainArtifactRegistry,
    commit: &ProjectionWriteCommit<'_>,
) -> Result<ProjectionWriteCommitResult, ProjectionCowError> {
    if !projection_index.allows_mutation(store_root) {
        return Err(crate::projection_index::ProjectionIndexError::ReadOnly.into());
    }

    let prior_content_id = projection_index
        .query_by_path_for_domain(commit.virtual_path, commit.domain_id, domain_registry)
        .map_err(|_| ProjectionCowError::PathNotProjected(commit.virtual_path.to_string()))?;

    let new_content_id = hash_bytes(commit.replacement_bytes);
    ContentId::parse(&new_content_id)
        .map_err(|_| ProjectionCowError::InvalidContentId(new_content_id.clone()))?;

    let manifest = Manifest {
        schema_version: 1,
        content_id: new_content_id.clone(),
        size_bytes: commit.replacement_bytes.len() as u64,
        kind: commit.kind.to_string(),
        channels: vec![commit.channel.to_string()],
        signatures: vec![],
    };
    domain_registry.publish_owned(
        store_root,
        &manifest,
        commit.domain_id,
        commit.domain_id == 0,
        |blob| write_blob_bytes_atomic(blob, commit.replacement_bytes),
    )?;

    let mut entry = SemanticIndexEntryV0::new(&new_content_id);
    entry.tags = dedupe_tags([commit.kind, commit.channel]);
    entry.path_alias = Some(commit.virtual_path.to_string());
    entry.domain_id = commit.domain_id;
    let mut candidate = projection_index.clone();
    candidate.upsert_entry(entry)?;

    let mut projection = PathProjectionV0::new(commit.virtual_path, &new_content_id);
    projection.domain_id = commit.domain_id;
    candidate.upsert_path_projection(projection)?;
    candidate.persist_atomic(store_root)?;
    *projection_index = candidate;

    Ok(ProjectionWriteCommitResult {
        virtual_path: commit.virtual_path.to_string(),
        prior_content_id,
        new_content_id,
    })
}

fn dedupe_tags<'a>(tags: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        if !tag.is_empty() && !out.iter().any(|existing| existing == tag) {
            out.push(tag.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection_index::{
        PROJECTION_INDEX_FILENAME, ProjectionIndexStore, ingest_projection_records,
    };
    use crate::projection_vfs::{materialize_read_only, read_projected_file};
    use artifact_store_core::{hash_bytes, write_blob_bytes_atomic, write_manifest_atomic};
    use artifact_store_schema::Manifest;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    const OLD_BYTES: &[u8] = b"projection cow original\n";
    const NEW_BYTES: &[u8] = b"projection cow replacement\n";

    fn seed_projected_artifact(store_root: &Path) -> (String, String) {
        let content_id = hash_bytes(OLD_BYTES);
        let hash_hex = content_id.strip_prefix("sha256:").expect("hash hex");
        let blob_path = store_root.join(format!("{hash_hex}.blob"));
        write_blob_bytes_atomic(&blob_path, OLD_BYTES).expect("write blob");

        let manifest = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes: OLD_BYTES.len() as u64,
            kind: "test_kind".into(),
            channels: vec!["beta".into()],
            signatures: vec![],
        };
        write_manifest_atomic(
            &store_root.join(format!("{hash_hex}.manifest.json")),
            &manifest,
        )
        .expect("write manifest");
        crate::DomainArtifactRegistry::new(store_root)
            .unwrap()
            .register_artifact(&ContentId::parse(&content_id).unwrap(), 0, true)
            .unwrap();

        let src = store_root.join("source.txt");
        fs::File::create(&src)
            .and_then(|mut f| f.write_all(OLD_BYTES))
            .expect("write source");

        let (entry, projection) =
            ingest_projection_records(&content_id, "test_kind", "beta", &src, 0);
        let virtual_path = projection.virtual_path.clone();

        let mut index =
            ProjectionIndexStore::load_or_empty(ProjectionIndexStore::default_path(store_root))
                .expect("load index");
        index.upsert_entry(entry).expect("upsert entry");
        index
            .upsert_path_projection(projection)
            .expect("upsert projection");
        index.persist_atomic(store_root).expect("persist");

        (virtual_path, content_id)
    }

    fn read_blob(store_root: &Path, content_id: &str) -> Vec<u8> {
        let hash_hex = content_id.strip_prefix("sha256:").expect("hash hex");
        fs::read(store_root.join(format!("{hash_hex}.blob"))).expect("read blob")
    }

    #[test]
    fn projection_cow_commit_repoints_path_preserves_prior_blob() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_root = temp_dir.path().join("store");
        let mount_root = temp_dir.path().join("mount");
        fs::create_dir_all(&store_root).expect("create store root");

        let (virtual_path, prior_content_id) = seed_projected_artifact(&store_root);

        let mut index =
            ProjectionIndexStore::load_or_empty(store_root.join(PROJECTION_INDEX_FILENAME))
                .expect("reload index");
        assert_eq!(
            index.query_by_path(&virtual_path).expect("path before cow"),
            prior_content_id
        );

        materialize_read_only(&store_root, &mount_root).expect("materialize before");
        assert_eq!(
            read_projected_file(&mount_root, &virtual_path).expect("read before"),
            OLD_BYTES
        );

        let commit = ProjectionWriteCommit {
            virtual_path: &virtual_path,
            replacement_bytes: NEW_BYTES,
            kind: "test_kind",
            channel: "beta",
            domain_id: 0,
        };
        let result = commit_projection_write(
            &store_root,
            &mut index,
            &mut crate::DomainArtifactRegistry::new(&store_root).unwrap(),
            &commit,
        )
        .expect("commit projection write");

        assert_eq!(result.virtual_path, virtual_path);
        assert_eq!(result.prior_content_id, prior_content_id);
        assert_ne!(result.new_content_id, prior_content_id);

        assert_eq!(
            index.query_by_path(&virtual_path).expect("path after cow"),
            result.new_content_id
        );

        materialize_read_only(&store_root, &mount_root).expect("materialize after");
        assert_eq!(
            read_projected_file(&mount_root, &virtual_path).expect("read after"),
            NEW_BYTES
        );

        assert_eq!(
            read_blob(&store_root, &prior_content_id),
            OLD_BYTES,
            "prior CAS blob must remain unchanged"
        );
        assert_eq!(
            read_blob(&store_root, &result.new_content_id),
            NEW_BYTES,
            "new CAS blob must contain committed bytes"
        );
    }

    #[test]
    fn review_projection_cow_owner_can_retrieve_after_restart() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let (path, _) = seed_projected_artifact(root);
        let mut index =
            ProjectionIndexStore::load_from_path(ProjectionIndexStore::default_path(root)).unwrap();
        let mut live_registry = crate::DomainArtifactRegistry::new(root).unwrap();
        let result = commit_projection_write(
            root,
            &mut index,
            &mut live_registry,
            &ProjectionWriteCommit {
                virtual_path: &path,
                replacement_bytes: NEW_BYTES,
                kind: "config",
                channel: "stable",
                domain_id: 7,
            },
        )
        .unwrap();
        let registry = crate::DomainArtifactRegistry::new(root).unwrap();
        let id = ContentId::parse(&result.new_content_id).unwrap();
        assert!(live_registry.can_access(&id, 7));
        assert_eq!(
            index
                .query_by_path_for_domain(&path, 7, &live_registry)
                .unwrap(),
            result.new_content_id
        );
        assert!(registry.can_access(&id, 7));
        assert!(!registry.can_access(&id, 99));
        let index =
            ProjectionIndexStore::load_from_path(ProjectionIndexStore::default_path(root)).unwrap();
        assert_eq!(
            index.query_by_path_for_domain(&path, 7, &registry).unwrap(),
            result.new_content_id
        );
        assert!(
            index.query_by_path_for_domain(&path, 99, &registry).is_ok(),
            "global source stays readable"
        );
    }

    fn assert_existing_manifest_preserved(owner: u64, succeeds: bool) {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let (path, _) = seed_projected_artifact(root);
        let id = ContentId::parse(&hash_bytes(NEW_BYTES)).unwrap();
        write_blob_bytes_atomic(&root.join(format!("{}.blob", id.hash_hex())), NEW_BYTES).unwrap();
        let manifest_path = root.join(format!("{}.manifest.json", id.hash_hex()));
        write_manifest_atomic(
            &manifest_path,
            &Manifest {
                schema_version: 1,
                content_id: id.as_str().to_owned(),
                size_bytes: NEW_BYTES.len() as u64,
                kind: "signed_validator".into(),
                channels: vec!["trusted".into()],
                signatures: vec!["signature-sentinel".into()],
            },
        )
        .unwrap();
        crate::DomainArtifactRegistry::new(root)
            .unwrap()
            .register_artifact(&id, owner, false)
            .unwrap();
        let before = fs::read(&manifest_path).unwrap();
        let index_path = ProjectionIndexStore::default_path(root);
        let prior_index = fs::read(&index_path).unwrap();
        let mut index = ProjectionIndexStore::load_from_path(&index_path).unwrap();
        let result = commit_projection_write(
            root,
            &mut index,
            &mut crate::DomainArtifactRegistry::new(root).unwrap(),
            &ProjectionWriteCommit {
                virtual_path: &path,
                replacement_bytes: NEW_BYTES,
                kind: "config",
                channel: "stable",
                domain_id: 7,
            },
        );
        assert_eq!(result.is_ok(), succeeds);
        assert_eq!(fs::read(&manifest_path).unwrap(), before);
        assert_eq!(
            fs::read(root.join(format!("{}.blob", id.hash_hex()))).unwrap(),
            NEW_BYTES
        );
        assert_eq!(
            crate::DomainArtifactRegistry::new(root)
                .unwrap()
                .get_owner(&id)
                .unwrap()
                .domain_id,
            owner
        );
        if !succeeds {
            assert_eq!(fs::read(index_path).unwrap(), prior_index);
            assert!(
                !index
                    .index()
                    .path_projections
                    .iter()
                    .any(|p| p.domain_id == 7)
            );
        }
    }

    #[test]
    fn review_projection_cow_denies_foreign_content_collision() {
        assert_existing_manifest_preserved(99, false);
    }

    #[test]
    fn review_projection_cow_reuses_own_signed_manifest() {
        assert_existing_manifest_preserved(7, true);
    }

    #[test]
    fn review_projection_cow_denies_corrupt_ownership() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let (path, _) = seed_projected_artifact(root);
        let id = ContentId::parse(&hash_bytes(NEW_BYTES)).unwrap();
        fs::write(
            root.join(format!("{}.ownership.json", id.hash_hex())),
            b"invalid",
        )
        .unwrap();
        let mut index =
            ProjectionIndexStore::load_from_path(ProjectionIndexStore::default_path(root)).unwrap();
        let before = fs::read(ProjectionIndexStore::default_path(root)).unwrap();
        assert!(
            commit_projection_write(
                root,
                &mut index,
                &mut crate::DomainArtifactRegistry::new(root).unwrap(),
                &ProjectionWriteCommit {
                    virtual_path: &path,
                    replacement_bytes: NEW_BYTES,
                    kind: "config",
                    channel: "stable",
                    domain_id: 7,
                }
            )
            .is_err()
        );
        assert!(!root.join(format!("{}.blob", id.hash_hex())).exists());
        assert_eq!(
            fs::read(ProjectionIndexStore::default_path(root)).unwrap(),
            before
        );
    }

    #[test]
    fn review_projection_cow_denies_private_source_and_unattributed_destination() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let (path, old) = seed_projected_artifact(root);
        let mut registry = crate::DomainArtifactRegistry::new(root).unwrap();
        let mut index =
            ProjectionIndexStore::load_from_path(ProjectionIndexStore::default_path(root)).unwrap();
        let private_path = "/private/config";
        let mut projection = PathProjectionV0::new(private_path, &old);
        projection.domain_id = 99;
        index.upsert_path_projection(projection).unwrap();
        index.persist_atomic(root).unwrap();
        let before = fs::read(ProjectionIndexStore::default_path(root)).unwrap();
        let commit = ProjectionWriteCommit {
            virtual_path: private_path,
            replacement_bytes: NEW_BYTES,
            kind: "config",
            channel: "stable",
            domain_id: 7,
        };
        assert!(matches!(
            commit_projection_write(root, &mut index, &mut registry, &commit),
            Err(ProjectionCowError::PathNotProjected(_))
        ));
        let id = ContentId::parse(&hash_bytes(NEW_BYTES)).unwrap();
        assert!(!root.join(format!("{}.blob", id.hash_hex())).exists());
        // Presence alone grants no right to repair or claim legacy content.
        fs::write(root.join(format!("{}.blob", id.hash_hex())), NEW_BYTES).unwrap();
        let commit = ProjectionWriteCommit {
            virtual_path: &path,
            ..commit
        };
        assert!(commit_projection_write(root, &mut index, &mut registry, &commit).is_err());
        assert_eq!(
            fs::read(ProjectionIndexStore::default_path(root)).unwrap(),
            before
        );
        assert!(
            !root
                .join(format!("{}.ownership.json", id.hash_hex()))
                .exists()
        );
    }
}
