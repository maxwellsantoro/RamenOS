//! S10.3.4 copy-on-write projection writes — scratch/commit without mutating CAS history.
//!
//! v0: single-shot `commit_projection_write` with in-memory replacement bytes.
//! The S10.3.3 read-only 9p export is unchanged; compat writes reach this path later.

use crate::projection_index::ProjectionIndexStore;
use artifact_store_core::{hash_bytes, write_blob_bytes_atomic, write_manifest_atomic};
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
pub fn commit_projection_write(
    store_root: &Path,
    projection_index: &mut ProjectionIndexStore,
    commit: &ProjectionWriteCommit<'_>,
) -> Result<ProjectionWriteCommitResult, ProjectionCowError> {
    if !projection_index.allows_mutation(store_root) {
        return Err(crate::projection_index::ProjectionIndexError::ReadOnly.into());
    }

    let prior_content_id = projection_index
        .query_by_path(commit.virtual_path)
        .map_err(|_| ProjectionCowError::PathNotProjected(commit.virtual_path.to_string()))?;

    let new_content_id = hash_bytes(commit.replacement_bytes);
    let id = ContentId::parse(&new_content_id)
        .map_err(|_| ProjectionCowError::InvalidContentId(new_content_id.clone()))?;

    let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
    write_blob_bytes_atomic(&blob_path, commit.replacement_bytes)?;

    let manifest = Manifest {
        schema_version: 1,
        content_id: new_content_id.clone(),
        size_bytes: commit.replacement_bytes.len() as u64,
        kind: commit.kind.to_string(),
        channels: vec![commit.channel.to_string()],
        signatures: vec![],
    };
    let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));
    write_manifest_atomic(&manifest_path, &manifest)?;

    let mut entry = SemanticIndexEntryV0::new(&new_content_id);
    entry.tags = dedupe_tags([commit.kind, commit.channel]);
    entry.path_alias = Some(commit.virtual_path.to_string());
    entry.domain_id = commit.domain_id;
    projection_index.upsert_entry(entry)?;

    let mut projection = PathProjectionV0::new(commit.virtual_path, &new_content_id);
    projection.domain_id = commit.domain_id;
    projection_index.upsert_path_projection(projection)?;
    projection_index.persist_atomic(store_root)?;

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
        let result = commit_projection_write(&store_root, &mut index, &commit)
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
}
