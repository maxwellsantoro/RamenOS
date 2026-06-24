// V-007 Phase 4: Manifest Signature Validation
//
// This module provides types and validation for manifest signatures
// with real cryptographic verification for Ed25519.
//
// Note: ECDSA and RSA verification are stubbed pending API stabilization.
// The framework is in place to add them once the exact API patterns are confirmed.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

// Ed25519 imports
#[cfg(feature = "std")]
use base64::{Engine as _, engine::general_purpose};
#[cfg(feature = "std")]
use ed25519_dalek::{Signature as EdSignature, Verifier, VerifyingKey as EdVerifyingKey};

/// Signature algorithm supported for manifest signing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignatureAlgorithm {
    /// Ed25519 (recommended for production use)
    Ed25519,

    /// ECDSA with P-256 curve
    EcdsaP256,

    /// RSA with PKCS#1 v1.5 padding (legacy, not recommended)
    RsaPkcs1,

    /// RSA with PSS padding
    RsaPss,
}

/// A single signature on a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSignature {
    /// Signature algorithm used
    pub algorithm: SignatureAlgorithm,

    /// Base64-encoded signature data
    pub signature_data: String,

    /// Key identifier (e.g., key fingerprint or key ID)
    pub key_id: String,

    /// Optional signing timestamp (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,

    /// Optional signer identity (e.g., "build-system@ramenos")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,
}

/// Result of signature validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureValidationResult {
    /// Signature is valid
    Valid,

    /// Signature is invalid (cryptographic verification failed)
    Invalid,

    /// Signature format is malformed
    Malformed,

    /// Unknown key (key not in trusted set)
    UnknownKey,

    /// Signature expired (if timestamp-based expiration is enabled)
    Expired,

    /// No signatures present
    NoSignatures,
}

/// Signature validation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePolicy {
    /// All manifests must be signed by at least one trusted key
    RequireSignature,

    /// Manifests may be unsigned (development mode)
    AllowUnsigned,

    /// Manifests must be signed by specific key IDs
    RequireSpecificKeyIds,
}

/// Trusted public key storage
#[derive(Debug, Clone)]
pub struct TrustedKeys {
    /// Ed25519 public keys mapped by key_id
    #[cfg(feature = "std")]
    ed25519_keys: Vec<(String, EdVerifyingKey)>,
    #[cfg(not(feature = "std"))]
    ed25519_keys: Vec<(String, ())>, // Stub for no_std
}

impl Default for TrustedKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustedKeys {
    /// Create a new empty trusted keys store
    pub fn new() -> Self {
        Self {
            ed25519_keys: Vec::new(),
        }
    }

    /// Add an Ed25519 public key
    #[cfg(feature = "std")]
    pub fn add_ed25519_key(
        &mut self,
        key_id: String,
        public_key_bytes: &[u8],
    ) -> Result<(), String> {
        if public_key_bytes.len() != 32 {
            return Err(format!(
                "Ed25519 public key must be 32 bytes, got {}",
                public_key_bytes.len()
            ));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(public_key_bytes);
        let public_key = EdVerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| format!("invalid Ed25519 public key: {}", e))?;
        self.ed25519_keys.push((key_id, public_key));
        Ok(())
    }

    /// Get Ed25519 public key by key_id
    #[cfg(feature = "std")]
    pub fn get_ed25519_key(&self, key_id: &str) -> Option<&EdVerifyingKey> {
        self.ed25519_keys
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, key)| key)
    }

    /// Check if a key_id is trusted for Ed25519
    pub fn has_ed25519_key(&self, key_id: &str) -> bool {
        self.ed25519_keys.iter().any(|(id, _)| id == key_id)
    }

    /// V-007 Phase 5: Load trusted keys from a file
    #[cfg(feature = "std")]
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufRead;
        use std::io::BufReader;
        use std::os::unix::fs::PermissionsExt;

        let file = File::open(path)?;

        // V-007 Phase 5: Verify file permissions are not overly permissive
        let metadata = file.metadata()?;
        let perms = metadata.permissions().mode();
        if perms & 0o022 != 0 {
            return Err("trusted keys file must not be world-writable or group-writable".into());
        }

        let reader = BufReader::new(file);

        let mut keys = Self::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse base64-encoded public key (32 bytes)
            let key_bytes = base64::engine::general_purpose::STANDARD
                .decode(line)
                .map_err(|e| format!("invalid base64: {}", e))?;

            if key_bytes.len() != 32 {
                return Err(format!("public key must be 32 bytes, got {}", key_bytes.len()).into());
            }

            let mut array = [0u8; 32];
            array.copy_from_slice(&key_bytes);

            let vk = EdVerifyingKey::from_bytes(&array)
                .map_err(|e| format!("invalid verifying key: {}", e))?;

            // Use key fingerprint as identifier
            let key_id = format!("sha256:{}", hex::encode(array));

            keys.ed25519_keys.push((key_id, vk));
        }

        if keys.ed25519_keys.is_empty() {
            return Err("no keys found in file".into());
        }

        Ok(keys)
    }

    /// V-007 Phase 5: Get the number of trusted keys
    pub fn len(&self) -> usize {
        self.ed25519_keys.len()
    }

    /// V-007 Phase 5: Check if there are no trusted keys
    pub fn is_empty(&self) -> bool {
        self.ed25519_keys.is_empty()
    }
}

/// Configuration for signature validation
#[derive(Debug, Clone)]
pub struct SignatureValidationConfig {
    /// Validation policy
    pub policy: SignaturePolicy,

    /// Trusted key IDs (if policy is RequireSpecificKeyIds)
    pub trusted_key_ids: Vec<String>,

    /// Maximum signature age in seconds (None = no expiration)
    pub max_signature_age_secs: Option<u64>,

    /// Trusted public keys
    pub trusted_keys: TrustedKeys,
}

impl Default for SignatureValidationConfig {
    fn default() -> Self {
        Self {
            policy: SignaturePolicy::AllowUnsigned, // V-007 Phase 4: Default to allow unsigned
            trusted_key_ids: Vec::new(),
            max_signature_age_secs: None,
            trusted_keys: TrustedKeys::new(),
        }
    }
}

/// Signature verification errors
#[derive(Debug)]
enum SignatureError {
    InvalidBase64,
    InvalidSignatureFormat,
    VerificationFailed,
    UnsupportedAlgorithm,
}

/// Verify Ed25519 signature (fully implemented)
#[cfg(feature = "std")]
fn verify_ed25519_signature(
    signature_data: &str,
    message: &[u8],
    public_key: &EdVerifyingKey,
) -> Result<(), SignatureError> {
    let signature_bytes = general_purpose::STANDARD
        .decode(signature_data)
        .map_err(|_| SignatureError::InvalidBase64)?;

    let signature = EdSignature::from_slice(&signature_bytes)
        .map_err(|_| SignatureError::InvalidSignatureFormat)?;

    public_key
        .verify(message, &signature)
        .map_err(|_| SignatureError::VerificationFailed)?;

    Ok(())
}

/// Verify a single manifest signature
fn _verify_single_signature(
    sig: &ManifestSignature,
    _message: &[u8],
    _trusted_keys: &TrustedKeys,
) -> Result<(), SignatureError> {
    #[cfg(feature = "std")]
    {
        match sig.algorithm {
            SignatureAlgorithm::Ed25519 => {
                let public_key = _trusted_keys
                    .get_ed25519_key(&sig.key_id)
                    .ok_or(SignatureError::VerificationFailed)?;
                verify_ed25519_signature(&sig.signature_data, _message, public_key)
            }
            _ => Err(SignatureError::UnsupportedAlgorithm),
        }
    }
    #[cfg(not(feature = "std"))]
    {
        Err(SignatureError::UnsupportedAlgorithm)
    }
}

/// Validate manifest signatures
pub fn validate_manifest_signatures(
    signatures: &[String],
    manifest_bytes: &[u8],
    config: &SignatureValidationConfig,
) -> SignatureValidationResult {
    match config.policy {
        SignaturePolicy::AllowUnsigned => {
            // V-007 Phase 4: Allow unsigned manifests for development
            if signatures.is_empty() {
                return SignatureValidationResult::Valid;
            }

            // If signatures are present, validate them
            for sig_str in signatures {
                let sig = match serde_json::from_str::<ManifestSignature>(sig_str) {
                    Ok(s) => s,
                    Err(_) => return SignatureValidationResult::Malformed,
                };

                // Check if key is trusted
                if !config.trusted_keys.has_ed25519_key(&sig.key_id) {
                    continue;
                }

                // Verify signature
                match _verify_single_signature(&sig, manifest_bytes, &config.trusted_keys) {
                    Ok(()) => {}
                    Err(_) => {
                        // For AllowUnsigned, verification failure doesn't invalidate
                    }
                }
            }

            SignatureValidationResult::Valid
        }

        SignaturePolicy::RequireSignature => {
            if signatures.is_empty() {
                return SignatureValidationResult::NoSignatures;
            }

            let mut valid_count = 0;
            for sig_str in signatures {
                let sig = match serde_json::from_str::<ManifestSignature>(sig_str) {
                    Ok(s) => s,
                    Err(_) => return SignatureValidationResult::Malformed,
                };

                // Check if key is trusted
                if !config.trusted_keys.has_ed25519_key(&sig.key_id) {
                    return SignatureValidationResult::UnknownKey;
                }

                // Verify signature
                match _verify_single_signature(&sig, manifest_bytes, &config.trusted_keys) {
                    Ok(()) => valid_count += 1,
                    Err(_) => return SignatureValidationResult::Invalid,
                }
            }

            // At least one valid signature is required
            if valid_count > 0 {
                SignatureValidationResult::Valid
            } else {
                SignatureValidationResult::Invalid
            }
        }

        SignaturePolicy::RequireSpecificKeyIds => {
            if signatures.is_empty() {
                return SignatureValidationResult::NoSignatures;
            }

            let mut valid_count = 0;
            for sig_str in signatures {
                let sig = match serde_json::from_str::<ManifestSignature>(sig_str) {
                    Ok(s) => s,
                    Err(_) => return SignatureValidationResult::Malformed,
                };

                // Check if key is in trusted list
                if !config.trusted_key_ids.contains(&sig.key_id) {
                    return SignatureValidationResult::UnknownKey;
                }

                // Check if key exists in trusted_keys
                if !config.trusted_keys.has_ed25519_key(&sig.key_id) {
                    return SignatureValidationResult::UnknownKey;
                }

                // Verify signature
                match _verify_single_signature(&sig, manifest_bytes, &config.trusted_keys) {
                    Ok(()) => valid_count += 1,
                    Err(_) => return SignatureValidationResult::Invalid,
                }
            }

            // At least one valid signature from trusted key IDs is required
            if valid_count > 0 {
                SignatureValidationResult::Valid
            } else {
                SignatureValidationResult::Invalid
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey as EdSigningKey};

    // Helper function to create a test Ed25519 key pair
    fn create_ed25519_test_keypair() -> (EdSigningKey, String, [u8; 32]) {
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        let mut secret_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_key_bytes);
        let signing_key = EdSigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();
        let key_bytes = verifying_key.to_bytes();
        let key_id = "test-ed25519-key".to_string();
        (signing_key, key_id, key_bytes)
    }

    #[test]
    fn allow_unsigned_policy_accepts_no_signatures() {
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::AllowUnsigned,
            ..Default::default()
        };

        let result = validate_manifest_signatures(&[], &[], &config);
        assert_eq!(result, SignatureValidationResult::Valid);
    }

    #[test]
    fn verify_ed25519_valid_signature() {
        // Create test key pair
        let (signing_key, key_id, key_bytes) = create_ed25519_test_keypair();

        // Create signature
        let message = b"test message";
        let signature = signing_key.sign(message);
        let signature_data = general_purpose::STANDARD.encode(signature.to_bytes());

        // Setup trusted keys
        let mut trusted_keys = TrustedKeys::new();
        trusted_keys
            .add_ed25519_key(key_id.clone(), &key_bytes)
            .unwrap();

        // Create config
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            trusted_keys,
            ..Default::default()
        };

        // Create manifest signature
        let sig = ManifestSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature_data,
            key_id,
            timestamp: None,
            signer: None,
        };

        let signatures = vec![serde_json::to_string(&sig).unwrap()];
        let result = validate_manifest_signatures(&signatures, message, &config);
        assert_eq!(result, SignatureValidationResult::Valid);
    }

    #[test]
    fn verify_ed25519_invalid_signature() {
        // Create test key pair
        let (_signing_key, key_id, key_bytes) = create_ed25519_test_keypair();

        // Use wrong signature
        let message = b"test message";
        let signature_data = general_purpose::STANDARD.encode([0u8; 64]);

        // Setup trusted keys
        let mut trusted_keys = TrustedKeys::new();
        trusted_keys
            .add_ed25519_key(key_id.clone(), &key_bytes)
            .unwrap();

        // Create config
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            trusted_keys,
            ..Default::default()
        };

        // Create manifest signature
        let sig = ManifestSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature_data,
            key_id,
            timestamp: None,
            signer: None,
        };

        let signatures = vec![serde_json::to_string(&sig).unwrap()];
        let result = validate_manifest_signatures(&signatures, message, &config);
        assert_eq!(result, SignatureValidationResult::Invalid);
    }

    #[test]
    fn require_signature_policy_rejects_no_signatures() {
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            ..Default::default()
        };

        let result = validate_manifest_signatures(&[], &[], &config);
        assert_eq!(result, SignatureValidationResult::NoSignatures);
    }

    #[test]
    fn require_specific_key_ids_rejects_unknown_keys() {
        // Create test key pair
        let (_signing_key, key_id, key_bytes) = create_ed25519_test_keypair();

        // Create signature
        let message = b"test message";
        let signature = _signing_key.sign(message);
        let signature_data = general_purpose::STANDARD.encode(signature.to_bytes());

        // Setup trusted keys
        let mut trusted_keys = TrustedKeys::new();
        trusted_keys
            .add_ed25519_key(key_id.clone(), &key_bytes)
            .unwrap();

        // Create config with different trusted key ID
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSpecificKeyIds,
            trusted_key_ids: vec!["different-key".to_string()],
            trusted_keys,
            ..Default::default()
        };

        // Create manifest signature
        let sig = ManifestSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature_data,
            key_id,
            timestamp: None,
            signer: None,
        };

        let signatures = vec![serde_json::to_string(&sig).unwrap()];
        let result = validate_manifest_signatures(&signatures, message, &config);
        assert_eq!(result, SignatureValidationResult::UnknownKey);
    }

    #[test]
    fn signature_algorithm_serialization() {
        let sig = ManifestSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature_data: "dGVzdA==".to_string(),
            key_id: "key".to_string(),
            timestamp: None,
            signer: None,
        };

        let json = serde_json::to_string(&sig).unwrap();
        assert!(json.contains("\"ed25519\""));
    }

    #[test]
    fn trusted_keys_add_retrieve_ed25519() {
        let mut keys = TrustedKeys::new();
        let key_id = "test-key".to_string();
        let key_bytes = [1u8; 32];

        keys.add_ed25519_key(key_id.clone(), &key_bytes).unwrap();
        assert!(keys.has_ed25519_key(&key_id));
        assert!(keys.get_ed25519_key(&key_id).is_some());
    }

    #[test]
    fn trusted_keys_rejects_invalid_ed25519_length() {
        let mut keys = TrustedKeys::new();
        let key_id = "test-key".to_string();
        let key_bytes = [1u8; 33]; // Wrong length

        let result = keys.add_ed25519_key(key_id, &key_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn allow_unsigned_policy_rejects_malformed_signatures() {
        let config = SignatureValidationConfig {
            policy: SignaturePolicy::AllowUnsigned,
            ..Default::default()
        };

        let signatures = vec!["invalid json".to_string()];
        let result = validate_manifest_signatures(&signatures, &[], &config);
        assert_eq!(result, SignatureValidationResult::Malformed);
    }
}
