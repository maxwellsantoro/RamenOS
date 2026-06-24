//! S10.3.3 read-only VFS projection — materialize index paths as a symlink tree.

use crate::projection_index::{PROJECTION_INDEX_FILENAME, ProjectionIndexStore};
use artifact_store_schema::ContentId;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ProjectionVfsError {
    #[error("failed to read projection index: {0}")]
    Index(#[from] crate::projection_index::ProjectionIndexError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid virtual path: {0}")]
    InvalidPath(String),

    #[error("missing blob for content id {content_id}: {path}")]
    MissingBlob { content_id: String, path: PathBuf },
}

/// Materialize all `path_projections` as symlinks under `mount_root`.
///
/// Each `virtual_path` like `/store/kind/channel/file.txt` becomes
/// `{mount_root}/store/kind/channel/file.txt` → `{store_root}/{hash}.blob`.
pub fn materialize_read_only(
    store_root: &Path,
    mount_root: &Path,
) -> Result<usize, ProjectionVfsError> {
    let index_path = store_root.join(PROJECTION_INDEX_FILENAME);
    let store = ProjectionIndexStore::load_from_path(&index_path)?;

    if mount_root.exists() {
        fs::remove_dir_all(mount_root)?;
    }
    fs::create_dir_all(mount_root)?;

    let mut count = 0usize;
    for projection in &store.index().path_projections {
        let dest = map_virtual_path(mount_root, &projection.virtual_path)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let id = ContentId::parse(&projection.content_id).map_err(|_| {
            ProjectionVfsError::InvalidPath(format!(
                "invalid content_id in projection: {}",
                projection.content_id
            ))
        })?;
        let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
        if !blob_path.is_file() {
            return Err(ProjectionVfsError::MissingBlob {
                content_id: projection.content_id.clone(),
                path: blob_path,
            });
        }

        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&blob_path, &dest)?;
        }
        #[cfg(not(unix))]
        {
            fs::copy(&blob_path, &dest)?;
        }
        count += 1;
    }

    Ok(count)
}

/// Read file bytes through a materialized projected path (host-side VFS read).
pub fn read_projected_file(
    mount_root: &Path,
    virtual_path: &str,
) -> Result<Vec<u8>, ProjectionVfsError> {
    let path = map_virtual_path(mount_root, virtual_path)?;
    let mut file = fs::File::open(&path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn map_virtual_path(mount_root: &Path, virtual_path: &str) -> Result<PathBuf, ProjectionVfsError> {
    if virtual_path.is_empty() || !virtual_path.starts_with('/') {
        return Err(ProjectionVfsError::InvalidPath(format!(
            "virtual path must be absolute: {virtual_path}"
        )));
    }

    let relative = virtual_path.trim_start_matches('/');
    if relative.is_empty() {
        return Err(ProjectionVfsError::InvalidPath(
            "virtual path must not be root".into(),
        ));
    }

    let mut joined = mount_root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(segment) => joined.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ProjectionVfsError::InvalidPath(format!(
                    "virtual path contains forbidden component: {virtual_path}"
                )));
            }
        }
    }

    let mount_root = lexical_normalize(mount_root);
    let joined = lexical_normalize(&joined);
    if !joined.starts_with(&mount_root) {
        return Err(ProjectionVfsError::InvalidPath(format!(
            "virtual path escapes mount root: {virtual_path}"
        )));
    }

    Ok(joined)
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection_index::{
        PROJECTION_INDEX_KIND, ProjectionIndexStore, ingest_projection_records,
    };
    use artifact_store_core::{hash_bytes, write_blob_bytes_atomic, write_manifest_atomic};
    use artifact_store_schema::Manifest;
    use std::io::Write;
    use tempfile::TempDir;

    const SAMPLE_CONTENT: &[u8] = b"projection vfs payload\n";

    fn seed_store_with_projection(store_root: &Path) -> String {
        let content_id = hash_bytes(SAMPLE_CONTENT);
        let id = ContentId::parse(&content_id).expect("parse content id");
        let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
        write_blob_bytes_atomic(&blob_path, SAMPLE_CONTENT).expect("write blob");

        let manifest = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes: SAMPLE_CONTENT.len() as u64,
            kind: "test".into(),
            channels: vec!["beta".into()],
            signatures: vec![],
        };
        let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        write_manifest_atomic(&manifest_path, &manifest).expect("write manifest");

        let src = store_root.join("source.txt");
        fs::File::create(&src)
            .and_then(|mut f| f.write_all(SAMPLE_CONTENT))
            .expect("write source");

        let (entry, projection) =
            ingest_projection_records(&content_id, "test_kind", "beta", &src, 0);
        let mut index =
            ProjectionIndexStore::load_or_empty(ProjectionIndexStore::default_path(store_root))
                .expect("load index");
        index.upsert_entry(entry).expect("upsert entry");
        index
            .upsert_path_projection(projection)
            .expect("upsert projection");
        index.persist_atomic(store_root).expect("persist");

        content_id
    }

    #[test]
    fn read_only_vfs_projection() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_root = temp_dir.path().join("store");
        let mount_root = temp_dir.path().join("mount");
        fs::create_dir_all(&store_root).expect("create store root");

        let content_id = seed_store_with_projection(&store_root);
        let virtual_path = "/store/test_kind/beta/source.txt";

        let count = materialize_read_only(&store_root, &mount_root).expect("materialize");
        assert_eq!(count, 1);

        let bytes = read_projected_file(&mount_root, virtual_path).expect("read projected");
        assert_eq!(bytes, SAMPLE_CONTENT);

        let symlink_path = mount_root.join("store/test_kind/beta/source.txt");
        assert!(symlink_path.is_symlink() || symlink_path.is_file());

        let index =
            ProjectionIndexStore::load_or_empty(ProjectionIndexStore::default_path(&store_root))
                .expect("reload index");
        assert_eq!(
            index.query_by_path(virtual_path).expect("query"),
            content_id
        );

        let snapshot_kind = fs::read_dir(&store_root)
            .expect("read store")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    return None;
                }
                let name = path.file_name()?.to_str()?;
                if !name.ends_with(".manifest.json") {
                    return None;
                }
                let raw = fs::read_to_string(&path).ok()?;
                let manifest: Manifest = serde_json::from_str(&raw).ok()?;
                (manifest.kind == PROJECTION_INDEX_KIND).then_some(manifest.kind)
            })
            .next();
        assert_eq!(snapshot_kind.as_deref(), Some(PROJECTION_INDEX_KIND));
    }

    #[test]
    fn materialize_rejects_parent_dir_escape() {
        let temp_dir = TempDir::new().expect("temp dir");
        let err = map_virtual_path(temp_dir.path(), "/store/../secret")
            .expect_err("should reject parent dir");
        assert!(matches!(err, ProjectionVfsError::InvalidPath(_)));
    }
}
