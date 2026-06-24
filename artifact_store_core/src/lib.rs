// artifact_store_core: IO-only crate with schema re-exports for backward compatibility.
//
// SC-09 boundary split:
// - Schema types and validation are in artifact_store_schema (no IO)
// - IO functions (hash_blob, write_manifest_atomic, etc.) remain here
// - Services should depend on artifact_store_schema for types/validation only
// - Store writes happen only through store-owned paths (store_cli, artifact_store service)

use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// Re-export all schema types for backward compatibility
pub use artifact_store_schema::{
    // Constants
    CONTENT_ID_PREFIX,
    // Core types
    ContentId,
    ContentIdError,
    Manifest,
    // Schema modules
    claim,
    crash_context,
    evidence_policy,
    graduation,
    minimal_policy,
    observed_caps,
    prereq_graph,
    queue_item,
    trace,
};

pub fn hash_blob(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(format!("sha256:{}", hex::encode(digest)))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(digest))
}

pub fn write_manifest_atomic(path: &Path, manifest: &Manifest) -> Result<(), std::io::Error> {
    let tmp = temp_path(path);
    let json = serde_json::to_string_pretty(manifest)?;
    {
        let mut file = File::create(&tmp)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn write_blob_atomic(dst: &Path, src: &Path) -> Result<(), std::io::Error> {
    let tmp = temp_path(dst);
    {
        let mut input = File::open(src)?;
        let mut output = File::create(&tmp)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
    }
    fs::rename(&tmp, dst)?;
    Ok(())
}

pub fn write_blob_bytes_atomic(dst: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let tmp = temp_path(dst);
    {
        let mut output = File::create(&tmp)?;
        output.write_all(bytes)?;
        output.sync_all()?;
    }
    fs::rename(&tmp, dst)?;
    Ok(())
}

pub fn verify_blob_matches_manifest(blob: &Path, manifest: &Path) -> Result<(), std::io::Error> {
    let raw = fs::read_to_string(manifest)?;
    let meta: Manifest = serde_json::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if meta.schema_version != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "schema_version unsupported",
        ));
    }
    let actual_size = fs::metadata(blob)?.len();
    if meta.size_bytes != actual_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "size_bytes mismatch",
        ));
    }
    let expected = hash_blob(blob)?;
    if meta.content_id != expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "content_id mismatch",
        ));
    }
    Ok(())
}

pub fn manifest_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    root.join(format!("{}.manifest.json", content_id.hash_hex()))
}

pub fn blob_path_for(root: &Path, content_id: &ContentId) -> PathBuf {
    root.join(format!("{}.blob", content_id.hash_hex()))
}

pub fn manifest_path(root: &Path, content_id: &str) -> PathBuf {
    let name = content_id
        .strip_prefix(CONTENT_ID_PREFIX)
        .unwrap_or(content_id);
    root.join(format!("{}.manifest.json", name))
}

pub fn blob_path(root: &Path, content_id: &str) -> PathBuf {
    let name = content_id
        .strip_prefix(CONTENT_ID_PREFIX)
        .unwrap_or(content_id);
    root.join(format!("{}.blob", name))
}

fn temp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let mut dir = std::env::temp_dir();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        dir.push(format!("ramenos_artifact_test_{}", n));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn hash_blob_formats_sha256() {
        let dir = temp_dir();
        let path = dir.join("blob.bin");
        fs::write(&path, b"ramen").unwrap();
        let id = hash_blob(&path).unwrap();
        assert!(id.starts_with("sha256:"));
        assert_eq!(id.len(), "sha256:".len() + 64);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn manifest_paths_strip_prefix() {
        let dir = PathBuf::from("root");
        let blob = blob_path(&dir, "sha256:abc");
        let manifest = manifest_path(&dir, "sha256:abc");
        assert_eq!(blob, PathBuf::from("root/abc.blob"));
        assert_eq!(manifest, PathBuf::from("root/abc.manifest.json"));
    }

    #[test]
    fn content_id_accepts_valid_sha256_lower_hex() {
        let id = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let parsed = ContentId::parse(id).unwrap();
        assert_eq!(parsed.as_str(), id);
        assert_eq!(parsed.hash_hex().len(), 64);
    }

    #[test]
    fn content_id_rejects_traversal_and_malformed_values() {
        let bad = [
            "sha256:../x",
            "sha256:../../etc/passwd",
            "sha256:abc/def",
            "sha256:abc\\def",
            "sha256:ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef01234567",
            "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            "sha256:0123",
            "sha257:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ];
        for id in bad {
            assert!(ContentId::parse(id).is_err(), "expected invalid id: {id}");
        }
    }

    #[test]
    fn strict_paths_use_validated_content_id() {
        let dir = PathBuf::from("root");
        let id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let blob = blob_path_for(&dir, &id);
        let manifest = manifest_path_for(&dir, &id);
        assert_eq!(
            blob,
            PathBuf::from(
                "root/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.blob"
            )
        );
        assert_eq!(
            manifest,
            PathBuf::from(
                "root/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.manifest.json"
            )
        );
    }

    #[test]
    fn verify_blob_matches_manifest_ok_and_fail() {
        let dir = temp_dir();
        let blob = dir.join("blob.bin");
        fs::write(&blob, b"ramen_s1_demo_blob_v0\n").unwrap();
        let content_id = hash_blob(&blob).unwrap();

        let base_kind = "component".to_string();
        let base_channels = vec!["Experimental".to_string()];

        let manifest = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes: fs::metadata(&blob).unwrap().len(),
            kind: base_kind.clone(),
            channels: base_channels.clone(),
            signatures: vec![],
        };
        let manifest_path = dir.join("blob.manifest.json");
        write_manifest_atomic(&manifest_path, &manifest).unwrap();

        verify_blob_matches_manifest(&blob, &manifest_path).unwrap();

        let bad = Manifest {
            schema_version: 1,
            content_id: "sha256:deadbeef".to_string(),
            size_bytes: manifest.size_bytes,
            kind: base_kind.clone(),
            channels: base_channels.clone(),
            signatures: vec![],
        };
        let bad_path = dir.join("bad.manifest.json");
        write_manifest_atomic(&bad_path, &bad).unwrap();
        assert!(verify_blob_matches_manifest(&blob, &bad_path).is_err());

        let bad_size = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes: manifest.size_bytes + 1,
            kind: base_kind.clone(),
            channels: base_channels.clone(),
            signatures: vec![],
        };
        let bad_size_path = dir.join("bad_size.manifest.json");
        write_manifest_atomic(&bad_size_path, &bad_size).unwrap();
        assert!(verify_blob_matches_manifest(&blob, &bad_size_path).is_err());

        let bad_schema = Manifest {
            schema_version: 2,
            content_id,
            size_bytes: manifest.size_bytes,
            kind: base_kind,
            channels: base_channels,
            signatures: vec![],
        };
        let bad_schema_path = dir.join("bad_schema.manifest.json");
        write_manifest_atomic(&bad_schema_path, &bad_schema).unwrap();
        assert!(verify_blob_matches_manifest(&blob, &bad_schema_path).is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
