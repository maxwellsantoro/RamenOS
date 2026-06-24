//! S10.3 Projection Storage schemas — semantic index and VFS path projections.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

const PROJECTION_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct ProjectionValidationError(pub String);

impl core::fmt::Display for ProjectionValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProjectionValidationError {}

/// A single semantic index entry mapping metadata to a CAS blob.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticIndexEntryV0 {
    pub schema_version: u32,
    pub content_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_alias: Option<String>,
    #[serde(default)]
    pub domain_id: u64,
}

/// A virtual path projection over the semantic index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathProjectionV0 {
    pub schema_version: u32,
    pub virtual_path: String,
    pub content_id: String,
    #[serde(default)]
    pub domain_id: u64,
}

/// Aggregate semantic index for a domain or global scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionIndexV0 {
    pub schema_version: u32,
    pub entries: Vec<SemanticIndexEntryV0>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_projections: Vec<PathProjectionV0>,
}

impl SemanticIndexEntryV0 {
    pub fn new(content_id: impl Into<String>) -> Self {
        Self {
            schema_version: PROJECTION_INDEX_SCHEMA_VERSION,
            content_id: content_id.into(),
            tags: Vec::new(),
            path_alias: None,
            domain_id: 0,
        }
    }
}

impl PathProjectionV0 {
    pub fn new(virtual_path: impl Into<String>, content_id: impl Into<String>) -> Self {
        Self {
            schema_version: PROJECTION_INDEX_SCHEMA_VERSION,
            virtual_path: virtual_path.into(),
            content_id: content_id.into(),
            domain_id: 0,
        }
    }
}

impl ProjectionIndexV0 {
    pub fn new() -> Self {
        Self {
            schema_version: PROJECTION_INDEX_SCHEMA_VERSION,
            entries: Vec::new(),
            path_projections: Vec::new(),
        }
    }

    /// Resolve a virtual path to a content id via path projections.
    pub fn resolve_path(&self, virtual_path: &str) -> Option<&str> {
        self.path_projections
            .iter()
            .find(|p| p.virtual_path == virtual_path)
            .map(|p| p.content_id.as_str())
    }

    /// Find entries matching a tag.
    pub fn entries_with_tag(&self, tag: &str) -> Vec<&SemanticIndexEntryV0> {
        self.entries
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }
}

impl Default for ProjectionIndexV0 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_semantic_index_entry(
    entry: &SemanticIndexEntryV0,
) -> Result<(), ProjectionValidationError> {
    if entry.schema_version != PROJECTION_INDEX_SCHEMA_VERSION {
        return Err(ProjectionValidationError(format!(
            "unsupported semantic index entry schema version: {}",
            entry.schema_version
        )));
    }
    if entry.content_id.is_empty() {
        return Err(ProjectionValidationError(
            "semantic index entry content_id must not be empty".into(),
        ));
    }
    Ok(())
}

pub fn validate_path_projection(
    projection: &PathProjectionV0,
) -> Result<(), ProjectionValidationError> {
    if projection.schema_version != PROJECTION_INDEX_SCHEMA_VERSION {
        return Err(ProjectionValidationError(format!(
            "unsupported path projection schema version: {}",
            projection.schema_version
        )));
    }
    if projection.virtual_path.is_empty() {
        return Err(ProjectionValidationError(
            "path projection virtual_path must not be empty".into(),
        ));
    }
    if projection.content_id.is_empty() {
        return Err(ProjectionValidationError(
            "path projection content_id must not be empty".into(),
        ));
    }
    Ok(())
}

pub fn validate_projection_index(
    index: &ProjectionIndexV0,
) -> Result<(), ProjectionValidationError> {
    if index.schema_version != PROJECTION_INDEX_SCHEMA_VERSION {
        return Err(ProjectionValidationError(format!(
            "unsupported projection index schema version: {}",
            index.schema_version
        )));
    }
    for entry in &index.entries {
        validate_semantic_index_entry(entry)?;
    }
    for projection in &index.path_projections {
        validate_path_projection(projection)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_returns_content_id() {
        let mut index = ProjectionIndexV0::new();
        index.path_projections.push(PathProjectionV0::new(
            "/home/user/docs/plan.txt",
            "sha256:abc",
        ));
        assert_eq!(
            index.resolve_path("/home/user/docs/plan.txt"),
            Some("sha256:abc")
        );
    }

    #[test]
    fn entries_with_tag_filters() {
        let mut index = ProjectionIndexV0::new();
        let mut entry = SemanticIndexEntryV0::new("sha256:deadbeef");
        entry.tags.push("receipt".into());
        index.entries.push(entry);
        assert_eq!(index.entries_with_tag("receipt").len(), 1);
        assert_eq!(index.entries_with_tag("missing").len(), 0);
    }

    #[test]
    fn validate_rejects_empty_path() {
        let projection = PathProjectionV0 {
            schema_version: PROJECTION_INDEX_SCHEMA_VERSION,
            virtual_path: String::new(),
            content_id: "sha256:abc".into(),
            domain_id: 0,
        };
        assert!(validate_path_projection(&projection).is_err());
    }
}
