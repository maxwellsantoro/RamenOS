// V-007: Read-only path construction helpers for services
//
// These functions provide read-only access to artifact paths without
// requiring direct IO operations. Services use these to construct
// paths for validation and verification, while actual IO operations
// go through the store service (V-007 Phase 2+).
//
// This maintains the architectural boundary: "kernel ≠ services ≠ store"

#[cfg(feature = "std")]
use crate::ContentId;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

/// Construct the blob file path for a content ID.
///
/// This is a read-only path construction function. It does not perform
/// any IO operations. Use this to validate artifact existence or to
/// prepare paths for store service requests.
#[cfg(feature = "std")]
pub fn blob_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    root.join(format!("{}.blob", content_id.hash_hex()))
}

/// Construct the manifest file path for a content ID.
///
/// This is a read-only path construction function. It does not perform
/// any IO operations. Use this to validate manifest existence or to
/// prepare paths for store service requests.
#[cfg(feature = "std")]
pub fn manifest_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    root.join(format!("{}.manifest.json", content_id.hash_hex()))
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn blob_path_for_constructs_correct_path() {
        let id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let root = Path::new("/test/store");
        let path = blob_path_for(root, &id);

        assert_eq!(
            path,
            Path::new(
                "/test/store/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.blob"
            )
        );
    }

    #[test]
    fn manifest_path_for_constructs_correct_path() {
        let id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let root = Path::new("/test/store");
        let path = manifest_path_for(root, &id);

        assert_eq!(
            path,
            Path::new(
                "/test/store/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.manifest.json"
            )
        );
    }

    #[test]
    fn blob_path_uses_hash_not_full_content_id() {
        let id = ContentId::parse(
            "sha256:abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd",
        )
        .unwrap();
        let root = Path::new("/store");
        let path = blob_path_for(root, &id);

        // Should not contain "sha256:" prefix in the filename
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(!filename.contains("sha256:"));
        assert!(filename.ends_with(".blob"));
    }

    #[test]
    fn manifest_path_uses_hash_not_full_content_id() {
        let id = ContentId::parse(
            "sha256:abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd",
        )
        .unwrap();
        let root = Path::new("/store");
        let path = manifest_path_for(root, &id);

        // Should not contain "sha256:" prefix in the filename
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(!filename.contains("sha256:"));
        assert!(filename.ends_with(".manifest.json"));
    }
}
