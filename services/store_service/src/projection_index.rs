//! S10.3 projection index support for the store service.
//!
//! Path/tag lookups and durable CAS-backed persistence for `ProjectionIndexV0`.

use artifact_store_core::{hash_bytes, write_blob_bytes_atomic, write_manifest_atomic};
use artifact_store_schema::projection_storage::{
    PathProjectionV0, ProjectionIndexV0, SemanticIndexEntryV0, validate_path_projection,
    validate_projection_index, validate_semantic_index_entry,
};
use artifact_store_schema::{ContentId, Manifest};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const STATUS_OK: u32 = 0;
pub const STATUS_NOT_FOUND: u32 = 1;
pub const STATUS_INVALID_INDEX: u32 = 2;

pub const PROJECTION_INDEX_KIND: &str = "projection_index_v0";
pub const PROJECTION_INDEX_FILENAME: &str = "projection_index.json";
pub const INGEST_PROJECTION_ROOT: &str = "/store";

#[derive(Debug, thiserror::Error)]
pub enum ProjectionIndexError {
    #[error("failed to read projection index: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse projection index: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("projection index validation failed: {0}")]
    Validation(String),

    #[error("projection index has no durable path configured")]
    NoPath,

    #[error("projection index path is outside store root (read-only override)")]
    ReadOnly,
}

/// In-memory projection index with optional durable backing path.
#[derive(Debug, Clone, Default)]
pub struct ProjectionIndexStore {
    path: Option<PathBuf>,
    index: ProjectionIndexV0,
}

impl ProjectionIndexStore {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn default_path(store_root: &Path) -> PathBuf {
        store_root.join(PROJECTION_INDEX_FILENAME)
    }

    pub fn load_from_path(path: impl Into<PathBuf>) -> Result<Self, ProjectionIndexError> {
        let path = path.into();
        let raw = fs::read_to_string(&path)?;
        let index: ProjectionIndexV0 = serde_json::from_str(&raw)?;
        validate_projection_index(&index).map_err(|err| ProjectionIndexError::Validation(err.0))?;
        Ok(Self {
            path: Some(path),
            index,
        })
    }

    /// Load an existing index or start with an empty valid index at `path`.
    pub fn load_or_empty(path: impl Into<PathBuf>) -> Result<Self, ProjectionIndexError> {
        let path = path.into();
        if path.exists() {
            Self::load_from_path(path)
        } else {
            Ok(Self {
                path: Some(path),
                index: ProjectionIndexV0::new(),
            })
        }
    }

    pub fn try_load_optional(path: &Path) -> Result<Self, ProjectionIndexError> {
        if path.exists() {
            Self::load_from_path(path)
        } else {
            Ok(Self::empty())
        }
    }

    pub fn index_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_loaded(&self) -> bool {
        self.path.is_some() && self.path.as_ref().is_some_and(|p| p.exists())
    }

    pub fn index(&self) -> &ProjectionIndexV0 {
        &self.index
    }

    /// True when the index path lives under `store_root` and may be mutated.
    pub fn allows_mutation(&self, store_root: &Path) -> bool {
        self.path
            .as_ref()
            .is_some_and(|p| path_is_under_store_root(p, store_root))
    }

    pub fn upsert_entry(
        &mut self,
        entry: SemanticIndexEntryV0,
    ) -> Result<(), ProjectionIndexError> {
        validate_semantic_index_entry(&entry)
            .map_err(|err| ProjectionIndexError::Validation(err.0))?;
        if let Some(pos) = self
            .index
            .entries
            .iter()
            .position(|e| e.content_id == entry.content_id)
        {
            self.index.entries[pos] = entry;
        } else {
            self.index.entries.push(entry);
        }
        Ok(())
    }

    pub fn upsert_path_projection(
        &mut self,
        projection: PathProjectionV0,
    ) -> Result<(), ProjectionIndexError> {
        validate_path_projection(&projection)
            .map_err(|err| ProjectionIndexError::Validation(err.0))?;
        if let Some(pos) = self
            .index
            .path_projections
            .iter()
            .position(|p| p.virtual_path == projection.virtual_path)
        {
            self.index.path_projections[pos] = projection;
        } else {
            self.index.path_projections.push(projection);
        }
        Ok(())
    }

    /// Atomically persist the working copy and snapshot the same bytes into CAS.
    pub fn persist_atomic(&mut self, store_root: &Path) -> Result<String, ProjectionIndexError> {
        let path = self.path.as_ref().ok_or(ProjectionIndexError::NoPath)?;
        if !self.allows_mutation(store_root) {
            return Err(ProjectionIndexError::ReadOnly);
        }

        validate_projection_index(&self.index)
            .map_err(|err| ProjectionIndexError::Validation(err.0))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_vec_pretty(&self.index)?;
        write_json_atomic(path, &json)?;

        let content_id = hash_bytes(&json);
        let id = ContentId::parse(&content_id)
            .map_err(|_| ProjectionIndexError::Validation("invalid content_id".into()))?;

        let blob_dst = store_root.join(format!("{}.blob", id.hash_hex()));
        write_blob_bytes_atomic(&blob_dst, &json)?;

        let manifest = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes: json.len() as u64,
            kind: PROJECTION_INDEX_KIND.to_string(),
            channels: vec!["stable".to_string()],
            signatures: vec![],
        };
        let manifest_dst = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        write_manifest_atomic(&manifest_dst, &manifest)?;

        Ok(content_id)
    }

    pub fn query_by_path(&self, virtual_path: &str) -> Result<String, u32> {
        self.index
            .resolve_path(virtual_path)
            .map(str::to_string)
            .ok_or(STATUS_NOT_FOUND)
    }

    pub fn query_by_tag(&self, tag: &str) -> Result<Vec<String>, u32> {
        let matches: Vec<String> = self
            .index
            .entries_with_tag(tag)
            .into_iter()
            .map(|entry| entry.content_id.clone())
            .collect();
        if matches.is_empty() {
            return Err(STATUS_NOT_FOUND);
        }
        Ok(matches)
    }
}

pub fn ingest_projection_records(
    content_id: &str,
    kind: &str,
    channel: &str,
    src_path: &Path,
    domain_id: u64,
) -> (SemanticIndexEntryV0, PathProjectionV0) {
    let virtual_path = ingest_virtual_path(content_id, kind, channel, src_path);
    let mut entry = SemanticIndexEntryV0::new(content_id);
    entry.tags = dedupe_tags([kind, channel]);
    entry.path_alias = Some(virtual_path.clone());
    entry.domain_id = domain_id;

    let mut projection = PathProjectionV0::new(virtual_path, content_id);
    projection.domain_id = domain_id;

    (entry, projection)
}

pub fn ingest_virtual_path(content_id: &str, kind: &str, channel: &str, src_path: &Path) -> String {
    let filename = src_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(sanitize_path_segment)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| sanitize_path_segment(content_id));

    format!(
        "{}/{}/{}/{}",
        INGEST_PROJECTION_ROOT,
        sanitize_path_segment(kind),
        sanitize_path_segment(channel),
        filename
    )
}

fn path_is_under_store_root(path: &Path, store_root: &Path) -> bool {
    let path = lexical_normalize(path);
    let store_root = lexical_normalize(store_root);
    path == store_root.join(PROJECTION_INDEX_FILENAME) || path.starts_with(store_root)
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

fn sanitize_path_segment(segment: &str) -> String {
    let sanitized = segment
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "artifact".to_string()
    } else {
        sanitized
    }
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

fn write_json_atomic(path: &Path, json: &[u8]) -> Result<(), ProjectionIndexError> {
    let tmp_path = path.with_extension("tmp");
    {
        let mut tmp = fs::File::create(&tmp_path)?;
        tmp.write_all(json)?;
        tmp.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    const SAMPLE_CONTENT_ID: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const SAMPLE_PATH: &str = "/home/user/docs/receipt.pdf";

    fn sample_entry() -> SemanticIndexEntryV0 {
        let mut entry = SemanticIndexEntryV0::new(SAMPLE_CONTENT_ID);
        entry.tags.push("receipt".into());
        entry.path_alias = Some(SAMPLE_PATH.into());
        entry
    }

    fn sample_projection() -> PathProjectionV0 {
        PathProjectionV0::new(SAMPLE_PATH, SAMPLE_CONTENT_ID)
    }

    fn write_sample_index_file() -> (NamedTempFile, ProjectionIndexV0) {
        let mut index = ProjectionIndexV0::new();
        index.path_projections.push(sample_projection());
        index.entries.push(sample_entry());

        let mut file = NamedTempFile::new().expect("temp file");
        serde_json::to_writer(&mut file, &index).expect("write index");
        file.flush().expect("flush");
        (file, index)
    }

    #[test]
    fn query_by_path_resolves_projection() {
        let (file, _) = write_sample_index_file();
        let store = ProjectionIndexStore::load_from_path(file.path()).expect("load");
        let content_id = store.query_by_path(SAMPLE_PATH).expect("resolve");
        assert_eq!(content_id, SAMPLE_CONTENT_ID);
    }

    #[test]
    fn query_by_tag_returns_matching_content_ids() {
        let (file, _) = write_sample_index_file();
        let store = ProjectionIndexStore::load_from_path(file.path()).expect("load");
        let ids = store.query_by_tag("receipt").expect("tag query");
        assert_eq!(ids, vec![SAMPLE_CONTENT_ID]);
    }

    #[test]
    fn missing_path_returns_not_found() {
        let store = ProjectionIndexStore::empty();
        assert_eq!(store.query_by_path("/missing"), Err(STATUS_NOT_FOUND));
    }

    #[test]
    fn durable_index_roundtrip() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_root = temp_dir.path();
        let index_path = ProjectionIndexStore::default_path(store_root);

        assert!(!index_path.exists());

        let mut store = ProjectionIndexStore::load_or_empty(&index_path).expect("load_or_empty");
        store.upsert_entry(sample_entry()).expect("upsert entry");
        store
            .upsert_path_projection(sample_projection())
            .expect("upsert projection");

        let content_id = store.persist_atomic(store_root).expect("persist");
        assert!(content_id.starts_with("sha256:"));
        assert!(index_path.exists());

        artifact_store_schema::projection_storage::validate_projection_index(
            &serde_json::from_str::<ProjectionIndexV0>(
                &fs::read_to_string(&index_path).expect("read index"),
            )
            .expect("parse index"),
        )
        .expect("validate working copy");

        let reloaded = ProjectionIndexStore::load_or_empty(&index_path).expect("reload");
        assert_eq!(
            reloaded.query_by_path(SAMPLE_PATH).expect("path query"),
            SAMPLE_CONTENT_ID
        );
        assert_eq!(
            reloaded.query_by_tag("receipt").expect("tag query"),
            vec![SAMPLE_CONTENT_ID]
        );

        let id = ContentId::parse(&content_id).expect("parse content id");
        let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
        assert!(manifest_path.exists());
        assert!(blob_path.exists());

        let manifest: Manifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
                .expect("parse manifest");
        assert_eq!(manifest.kind, PROJECTION_INDEX_KIND);
        assert_eq!(manifest.content_id, content_id);
    }

    #[test]
    fn corrupt_index_fail_closed() {
        let temp_dir = TempDir::new().expect("temp dir");
        let index_path = ProjectionIndexStore::default_path(temp_dir.path());
        fs::write(&index_path, b"{not valid json").expect("write corrupt");

        let result = ProjectionIndexStore::load_or_empty(&index_path);
        assert!(result.is_err());

        fs::write(
            &index_path,
            br#"{"schema_version":99,"entries":[],"path_projections":[]}"#,
        )
        .expect("write bad schema");
        let result = ProjectionIndexStore::load_from_path(&index_path);
        assert!(matches!(result, Err(ProjectionIndexError::Validation(_))));
    }

    #[test]
    fn read_only_override_rejects_mutation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_root = temp_dir.path().join("store");
        let override_path = temp_dir.path().join("external_index.json");
        fs::create_dir_all(&store_root).expect("create store root");

        let mut store = ProjectionIndexStore::load_or_empty(&override_path).expect("load_or_empty");
        store.upsert_entry(sample_entry()).expect("upsert entry");
        let err = store
            .persist_atomic(&store_root)
            .expect_err("persist should fail");
        assert!(matches!(err, ProjectionIndexError::ReadOnly));
    }

    #[test]
    fn parent_dir_override_rejects_mutation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_root = temp_dir.path().join("store");
        let override_path = store_root.join("../external_index.json");
        fs::create_dir_all(&store_root).expect("create store root");

        let mut store = ProjectionIndexStore::load_or_empty(&override_path).expect("load_or_empty");
        store.upsert_entry(sample_entry()).expect("upsert entry");
        let err = store
            .persist_atomic(&store_root)
            .expect_err("persist should fail");
        assert!(matches!(err, ProjectionIndexError::ReadOnly));
    }

    #[test]
    fn ingest_projection_records_derive_path_and_tags() {
        let (entry, projection) = ingest_projection_records(
            SAMPLE_CONTENT_ID,
            "native wasm",
            "beta/channel",
            Path::new("/tmp/source file.txt"),
            42,
        );

        assert_eq!(entry.content_id, SAMPLE_CONTENT_ID);
        assert_eq!(entry.tags, vec!["native wasm", "beta/channel"]);
        assert_eq!(
            entry.path_alias.as_deref(),
            Some("/store/native_wasm/beta_channel/source_file.txt")
        );
        assert_eq!(
            projection.virtual_path,
            "/store/native_wasm/beta_channel/source_file.txt"
        );
        assert_eq!(projection.content_id, SAMPLE_CONTENT_ID);
        assert_eq!(projection.domain_id, 42);
    }
}
