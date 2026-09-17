// V-007 Phase 5: Domain-scoped artifact visibility
//
// Artifacts are scoped to domains for isolation. A domain can only see:
// 1. Artifacts it created
// 2. Kernel artifacts (domain_id = 0)
// 3. Artifacts explicitly shared with it (future)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use artifact_store_schema::ContentId;

/// Artifact ownership record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactOwner {
    /// Content ID of the artifact
    pub content_id: ContentId,

    /// Domain ID that owns this artifact
    pub domain_id: u64,

    /// Whether this is a global (kernel) artifact
    pub is_global: bool,

    /// Ingestion timestamp
    pub ingested_at: u64,
}

/// Domain-scoped artifact registry
///
/// This registry tracks which domain owns each artifact.
/// On startup, we scan existing artifacts to build the registry.
/// Ownership is durably committed in per-artifact sidecars before it is exposed.
/// Missing or invalid ownership is never inferred to be global.
///
/// # Thread Safety
///
/// This struct is NOT thread-safe. For concurrent access, wrap in a Mutex or RwLock.
/// The internal HashMap is not safe for concurrent reads/writes.
///
/// # Hash Storage Format
///
/// Hashes are stored WITHOUT the "sha256:" prefix for efficiency.
/// Always use ContentId::hash_hex() to extract the hash part before storage.
#[derive(Clone)]
pub struct DomainArtifactRegistry {
    /// Map from content_id -> owner record
    /// Key format: 64-character hex string (no "sha256:" prefix)
    artifacts: HashMap<String, ArtifactOwner>,

    /// Store root path (for scanning)
    store_root: PathBuf,
}

impl DomainArtifactRegistry {
    /// Create a new registry and scan existing artifacts
    pub fn new(store_root: &Path) -> Result<Self> {
        let mut registry = Self {
            artifacts: HashMap::new(),
            store_root: store_root.to_path_buf(),
        };

        // Scan existing artifacts to populate registry
        registry.scan_existing_artifacts()?;

        Ok(registry)
    }

    /// Scan existing artifacts in store to build ownership map
    ///
    /// S7 Security Hardening: Read ownership from manifest metadata and directory structure.
    /// - Artifacts under `store_root/global/` are marked as global (kernel-owned)
    /// - Artifacts under `store_root/domains/{domain_id}/` are owned by that domain
    /// - Ownership is also read from manifest metadata when available
    fn scan_existing_artifacts(&mut self) -> Result<()> {
        use std::fs;

        // P1 fix: Scan for *.blob files directly in store_root
        // The ingest path writes to store_root/<hash>.blob, not store_root/blobs/sha256/<hash>
        let store_root = &self.store_root;

        if !store_root.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(store_root)?;
        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy().to_string();

            // Only process .blob files (match ingest write path)
            if !filename_str.ends_with(".blob") {
                continue;
            }

            // Extract hash from filename (e.g., "abc123.blob" -> "abc123")
            let content_id_str = filename_str.trim_end_matches(".blob").to_string();
            if let Ok(content_id) = ContentId::parse(&format!("sha256:{}", content_id_str)) {
                // Try to read manifest to determine ownership
                let manifest_path = self
                    .store_root
                    .join(format!("{}.manifest.json", content_id.hash_hex()));

                let ownership_path = self.ownership_path(&content_id);
                let owner = if ownership_path.exists() {
                    // A corrupt sidecar must not fall back to a more permissive legacy record.
                    match std::fs::read(&ownership_path)
                        .ok()
                        .and_then(|raw| serde_json::from_slice::<ArtifactOwner>(&raw).ok())
                    {
                        Some(owner)
                            if owner.content_id == content_id
                                && (!owner.is_global || owner.domain_id == 0) =>
                        {
                            owner
                        }
                        _ => continue,
                    }
                } else {
                    // Legacy stores require explicit ownership; never infer it from presence.
                    let ownership = if manifest_path.exists() {
                        self.read_ownership_from_manifest(&manifest_path)
                    } else {
                        self.determine_ownership_from_directory(&content_id)
                    };
                    let Ok((domain_id, is_global)) = ownership else {
                        continue;
                    };
                    ArtifactOwner {
                        content_id: content_id.clone(),
                        domain_id,
                        is_global,
                        ingested_at: 0,
                    }
                };

                self.artifacts.insert(content_id_str, owner);
            }
        }

        Ok(())
    }

    /// Read ownership information from manifest metadata
    ///
    /// S7 Security Hardening: Extracts domain ownership from manifest.
    /// Returns (domain_id, is_global) tuple.
    fn read_ownership_from_manifest(&self, manifest_path: &Path) -> Result<(u64, bool)> {
        use serde_json::Value;
        use std::fs;

        let manifest_json = fs::read_to_string(manifest_path).context("failed to read manifest")?;

        let manifest: Value =
            serde_json::from_str(&manifest_json).context("failed to parse manifest")?;

        // Try to extract ownership from metadata
        if let Some(metadata) = manifest.get("metadata") {
            if let Some(domain_id) = metadata.get("domain_id").and_then(|v| v.as_u64()) {
                let is_global = metadata
                    .get("is_global")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(domain_id == 0);

                anyhow::ensure!(!is_global || domain_id == 0, "non-kernel global ownership");
                return Ok((domain_id, is_global));
            }
        }

        anyhow::bail!("explicit artifact ownership missing")
    }

    /// Determine ownership from directory structure
    ///
    /// S7 Security Hardening: Uses directory structure to determine ownership.
    /// - Artifacts in `store_root/global/` are global
    /// - Artifacts in `store_root/domains/{domain_id}/` are owned by that domain
    fn determine_ownership_from_directory(&self, content_id: &ContentId) -> Result<(u64, bool)> {
        use std::fs;

        // Check if this is a global artifact
        let global_dir = self.store_root.join("global");
        if global_dir.exists() {
            let global_manifest =
                global_dir.join(format!("{}.manifest.json", content_id.hash_hex()));
            if global_manifest.exists() {
                return Ok((0, true));
            }
        }

        // Check if this is a domain-specific artifact
        let domains_dir = self.store_root.join("domains");
        if domains_dir.exists() {
            let entries = fs::read_dir(&domains_dir)?;
            for entry in entries {
                let entry = entry?;
                let dirname = entry.file_name();
                if let Ok(domain_id) = dirname.to_string_lossy().parse::<u64>() {
                    let domain_manifest = entry
                        .path()
                        .join(format!("{}.manifest.json", content_id.hash_hex()));
                    if domain_manifest.exists() {
                        eprintln!(
                            "store_service: domain registry: artifact {:?} owned by domain {} (from directory)",
                            content_id, domain_id
                        );
                        return Ok((domain_id, false));
                    }
                }
            }
        }

        anyhow::bail!("explicit artifact ownership missing")
    }

    /// Register a new artifact ownership
    ///
    /// S7 Security Hardening: Logs all domain ownership changes for audit trail.
    pub fn register_artifact(
        &mut self,
        content_id: &ContentId,
        domain_id: u64,
        is_global: bool,
    ) -> Result<()> {
        self.check_registration(content_id, domain_id, is_global)?;
        let owner = ArtifactOwner {
            content_id: content_id.clone(),
            domain_id,
            is_global,
            ingested_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Extract just the hash part for the key (without "sha256:" prefix)
        let hash = content_id.hash_hex();

        // Log ownership change
        eprintln!("store_service: DOMAIN OWNERSHIP REGISTERED");
        eprintln!("store_service:   - Content ID: {:?}", content_id);
        eprintln!("store_service:   - Owner Domain: {}", domain_id);
        eprintln!("store_service:   - Is Global: {}", is_global);
        eprintln!("store_service:   - Ingested At: {}", owner.ingested_at);

        std::fs::create_dir_all(&self.store_root)?;
        let path = self.ownership_path(content_id);
        let tmp = path.with_extension("tmp");
        {
            let mut file = std::fs::File::create(&tmp)?;
            file.write_all(&serde_json::to_vec(&owner)?)?;
            file.sync_all()?;
        }
        std::fs::rename(&tmp, &path)?;
        std::fs::File::open(&self.store_root)?.sync_all()?;
        self.artifacts.insert(hash.to_string(), owner);

        Ok(())
    }

    fn ownership_path(&self, content_id: &ContentId) -> PathBuf {
        self.store_root
            .join(format!("{}.ownership.json", content_id.hash_hex()))
    }

    /// Reject ownership changes before any existing CAS metadata is overwritten.
    pub fn check_registration(
        &self,
        id: &ContentId,
        domain_id: u64,
        is_global: bool,
    ) -> Result<()> {
        anyhow::ensure!(
            !is_global || domain_id == 0,
            "only kernel artifacts may be global"
        );
        if let Some(owner) = self.get_owner(id) {
            anyhow::ensure!(
                owner.domain_id == domain_id && owner.is_global == is_global,
                "artifact already belongs to another domain"
            );
        } else {
            // A sidecar that failed validation cannot be silently repaired by an ingest.
            anyhow::ensure!(
                !self.ownership_path(id).exists(),
                "invalid persisted ownership"
            );
        }
        Ok(())
    }

    /// Check if a domain can access an artifact
    ///
    /// S7 Security Hardening: Logs all access denials for forensic analysis.
    /// Returns true if the domain can access the artifact, false otherwise.
    pub fn can_access(&self, content_id: &ContentId, domain_id: u64) -> bool {
        let hash = content_id.hash_hex();

        match self.artifacts.get(hash) {
            Some(owner) => {
                // Can access if: owned by this domain OR is global (kernel-owned)
                if owner.domain_id == domain_id || owner.is_global {
                    true
                } else {
                    eprintln!("store_service: DOMAIN ACCESS DENIED");
                    eprintln!("store_service:   - Content ID: {:?}", content_id);
                    eprintln!("store_service:   - Requesting Domain: {}", domain_id);
                    eprintln!("store_service:   - Owner Domain: {}", owner.domain_id);
                    eprintln!("store_service:   - Is Global: {}", owner.is_global);
                    eprintln!(
                        "store_service:   - Reason: Domain does not own artifact and it is not global"
                    );
                    false
                }
            }
            None => {
                // Unknown artifact: deny access (fail-closed)
                eprintln!("store_service: DOMAIN ACCESS DENIED - Unknown artifact");
                eprintln!("store_service:   - Content ID: {:?}", content_id);
                eprintln!("store_service:   - Requesting Domain: {}", domain_id);
                eprintln!("store_service:   - Reason: Artifact not found in registry");
                false
            }
        }
    }

    /// Get the owner of an artifact
    pub fn get_owner(&self, content_id: &ContentId) -> Option<&ArtifactOwner> {
        let hash = content_id.hash_hex();
        self.artifacts.get(hash)
    }

    /// List all artifacts owned by a domain
    pub fn list_domain_artifacts(&self, domain_id: u64) -> Vec<ContentId> {
        self.artifacts
            .values()
            .filter(|owner| owner.domain_id == domain_id)
            .map(|owner| owner.content_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn missing_or_corrupt_ownership_never_becomes_global() {
        let root = TempDir::new().unwrap();
        let hash = "a".repeat(64);
        let id = ContentId::parse(&format!("sha256:{hash}")).unwrap();
        fs::write(root.path().join(format!("{hash}.blob")), b"private").unwrap();
        for manifest in [
            "{}",
            "{broken",
            r#"{"metadata":{"domain_id":42,"is_global":true}}"#,
        ] {
            fs::write(root.path().join(format!("{hash}.manifest.json")), manifest).unwrap();
            let registry = DomainArtifactRegistry::new(root.path()).unwrap();
            assert!(!registry.can_access(&id, 99));
        }
    }

    #[test]
    fn durable_ownership_rejects_reassignment_and_corrupt_sidecars() {
        let root = TempDir::new().unwrap();
        let id = ContentId::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        std::fs::write(
            root.path().join(format!("{}.blob", id.hash_hex())),
            b"private",
        )
        .unwrap();
        let mut registry = DomainArtifactRegistry::new(root.path()).unwrap();
        registry.register_artifact(&id, 42, false).unwrap();
        registry.register_artifact(&id, 42, false).unwrap();
        assert!(registry.register_artifact(&id, 99, false).is_err());
        assert!(registry.register_artifact(&id, 0, true).is_err());
        let reloaded = DomainArtifactRegistry::new(root.path()).unwrap();
        assert!(reloaded.can_access(&id, 42));
        assert!(!reloaded.can_access(&id, 99));
        std::fs::write(registry.ownership_path(&id), b"{corrupt").unwrap();
        let mut reloaded = DomainArtifactRegistry::new(root.path()).unwrap();
        assert!(!reloaded.can_access(&id, 42));
        assert!(!reloaded.can_access(&id, 99));
        assert!(reloaded.register_artifact(&id, 99, false).is_err());
    }

    #[test]
    fn registry_scans_existing_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        // P1 fix: Create .blob files directly in store_root to match ingest path
        // The scan path now looks for *.blob files in store_root, not blobs/sha256/
        let hash_a = "a".repeat(64);
        let hash_b = "b".repeat(64);

        // Create some artifact blob files
        fs::write(store_root.join(format!("{}.blob", hash_a)), "artifact1").unwrap();
        fs::write(store_root.join(format!("{}.blob", hash_b)), "artifact2").unwrap();

        // Create registry
        let registry = DomainArtifactRegistry::new(store_root).unwrap();

        // Unattributed blobs are not globally visible.
        assert!(registry.artifacts.is_empty());
    }

    #[test]
    fn registry_allows_domain_to_access_own_artifacts() {
        let root = TempDir::new().unwrap();
        let mut registry = DomainArtifactRegistry::new(root.path()).unwrap();

        let content_id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        // Register artifact for domain 5
        registry.register_artifact(&content_id, 5, false).unwrap();

        // Domain 5 should be able to access
        assert!(registry.can_access(&content_id, 5));

        // Domain 1 should NOT be able to access
        assert!(!registry.can_access(&content_id, 1));

        // Domain 0 (kernel) should NOT be able to access (not global)
        assert!(!registry.can_access(&content_id, 0));
    }

    #[test]
    fn registry_allows_kernel_to_access_global_artifacts() {
        let root = TempDir::new().unwrap();
        let mut registry = DomainArtifactRegistry::new(root.path()).unwrap();

        let content_id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        // Register global artifact (kernel-owned)
        registry.register_artifact(&content_id, 0, true).unwrap();

        // Domain 0 (kernel) should be able to access
        assert!(registry.can_access(&content_id, 0));

        // Any domain should be able to access global artifacts
        assert!(registry.can_access(&content_id, 1));
        assert!(registry.can_access(&content_id, 5));
        assert!(registry.can_access(&content_id, 99));
    }

    #[test]
    fn registry_denies_access_to_unknown_artifacts() {
        let root = TempDir::new().unwrap();
        let registry = DomainArtifactRegistry::new(root.path()).unwrap();

        let content_id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        // Unknown artifact should deny access (fail-closed)
        assert!(!registry.can_access(&content_id, 0));
        assert!(!registry.can_access(&content_id, 1));
    }

    #[test]
    fn registry_lists_domain_artifacts() {
        let root = TempDir::new().unwrap();
        let mut registry = DomainArtifactRegistry::new(root.path()).unwrap();

        let content_id1 = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let content_id2 = ContentId::parse(
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .unwrap();
        let content_id3 = ContentId::parse(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        // Register artifacts: 1 and 2 for domain 5, 3 for domain 1
        registry.register_artifact(&content_id1, 5, false).unwrap();
        registry.register_artifact(&content_id2, 5, false).unwrap();
        registry.register_artifact(&content_id3, 1, false).unwrap();

        // List artifacts for domain 5
        let domain5_artifacts = registry.list_domain_artifacts(5);
        assert_eq!(domain5_artifacts.len(), 2);
        assert!(domain5_artifacts.contains(&content_id1));
        assert!(domain5_artifacts.contains(&content_id2));
        assert!(!domain5_artifacts.contains(&content_id3));

        // List artifacts for domain 1
        let domain1_artifacts = registry.list_domain_artifacts(1);
        assert_eq!(domain1_artifacts.len(), 1);
        assert!(domain1_artifacts.contains(&content_id3));
    }

    #[test]
    fn domain_2_cannot_access_domain_1_artifact_even_if_hash_known() {
        let root = TempDir::new().unwrap();
        let mut registry = DomainArtifactRegistry::new(root.path()).unwrap();

        // Domain 1 creates an artifact
        let domain1_artifact = ContentId::parse(
            "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        )
        .unwrap();
        registry
            .register_artifact(&domain1_artifact, 1, false)
            .unwrap();

        // Domain 2 tries to access the same artifact by hash
        // Even though domain 2 knows the exact hash, access should be denied
        assert!(!registry.can_access(&domain1_artifact, 2));

        // Domain 1 can still access it
        assert!(registry.can_access(&domain1_artifact, 1));

        // Verify ownership record
        let owner = registry.get_owner(&domain1_artifact).unwrap();
        assert_eq!(owner.domain_id, 1);
        assert!(!owner.is_global);
    }
}
