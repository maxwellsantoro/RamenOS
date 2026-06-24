// V-007 Phase 5: Capability-based access control for store operations
//
// Capabilities are unforgeable tokens that grant specific rights to specific domains.
// Inspired by seL4/KeyKOS capabilities: authority is explicitly conferred, not inferred.

use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature as EdSignature, Verifier, VerifyingKey as EdVerifyingKey};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

/// Store capability version (for future key rotation)
pub const CAPABILITY_VERSION: u32 = 1;

/// Store capability rights (bitmask)
pub const STORE_RIGHT_READ: u8 = 0x01; // Can read artifacts
pub const STORE_RIGHT_WRITE: u8 = 0x02; // Can ingest artifacts
pub const STORE_RIGHT_DELETE: u8 = 0x04; // Can delete artifacts
pub const STORE_RIGHT_ADMIN: u8 = 0x08; // Can manage capabilities
pub const STORE_RIGHT_ALL: u8 = 0x0F; // All rights

/// Store capability structure
///
/// Capabilities are granted by the kernel (or a trusted authority) and presented
/// to services like store_service. Each capability is bound to a specific domain
/// and grants specific rights.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreCapability {
    /// Capability version (for key rotation)
    pub version: u32,

    /// Domain ID this capability is bound to
    pub domain_id: u64,

    /// Rights bitmask (STORE_RIGHT_* constants)
    pub rights_mask: u8,

    /// Capability ID (unique identifier)
    pub capability_id: u64,

    /// Issue timestamp (seconds since UNIX epoch)
    pub issued_at: u64,

    /// Expiration timestamp (seconds since UNIX epoch, 0 = never expires)
    pub expires_at: u64,

    /// Creator's signature (Ed25519)
    /// Signature over: (version || domain_id || rights_mask || capability_id || issued_at || expires_at)
    pub signature: Vec<u8>,
}

fn parse_base64_trusted_keys(keys_str: &str) -> Vec<EdVerifyingKey> {
    let mut keys = Vec::new();

    for key_b64 in keys_str.split(',') {
        let key_b64 = key_b64.trim();
        if key_b64.is_empty() {
            continue;
        }

        match general_purpose::STANDARD.decode(key_b64) {
            Ok(key_bytes) => {
                if key_bytes.len() != 32 {
                    // Invalid key length, skip
                    continue;
                }

                let mut key_array = [0u8; 32];
                key_array.copy_from_slice(&key_bytes);

                match EdVerifyingKey::from_bytes(&key_array) {
                    Ok(key) => keys.push(key),
                    Err(_) => continue,
                }
            }
            Err(_) => continue,
        }
    }

    keys
}

fn load_trusted_keys_from_file(path: &std::path::Path) -> Vec<EdVerifyingKey> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut keys = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match general_purpose::STANDARD.decode(line) {
            Ok(key_bytes) if key_bytes.len() == 32 => {
                let mut key_array = [0u8; 32];
                key_array.copy_from_slice(&key_bytes);
                if let Ok(key) = EdVerifyingKey::from_bytes(&key_array) {
                    keys.push(key);
                }
            }
            _ => continue,
        }
    }

    keys
}

/// Load trusted Ed25519 public keys for capability signature verification.
///
/// Resolution order:
/// 1. `RAMEN_STORE_CAP_TRUSTED_KEYS` (comma-separated base64 Ed25519 keys)
/// 2. `RAMEN_STORE_TRUSTED_KEYS` (preferred: path to key file; fallback: base64 list)
/// 3. Development fallback key (only when compiled with `--features dev_insecure`)
/// 4. In test builds, always provides a fallback key for unit tests
/// 5. In production builds without `dev_insecure`, returns empty vector (fail-closed)
///
/// # Security
/// The `dev_insecure` feature flag MUST NOT be enabled in production builds.
/// It exists solely for development convenience and provides NO security.
///
/// # Returns
/// A vector of Ed25519 public keys for signature verification.
pub fn load_trusted_public_keys() -> Vec<EdVerifyingKey> {
    if let Ok(keys_str) = env::var("RAMEN_STORE_CAP_TRUSTED_KEYS") {
        return parse_base64_trusted_keys(&keys_str);
    }

    if let Ok(keys_cfg) = env::var("RAMEN_STORE_TRUSTED_KEYS") {
        let path = std::path::Path::new(&keys_cfg);
        if path.exists() {
            return load_trusted_keys_from_file(path);
        }

        // Backward compatibility: accept inline base64 list if value is not a path.
        return parse_base64_trusted_keys(&keys_cfg);
    }

    // Test mode: always provide fallback key for unit tests
    #[cfg(test)]
    {
        let default_key_bytes = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];

        EdVerifyingKey::from_bytes(&default_key_bytes)
            .map(|key| vec![key])
            .unwrap_or_default()
    }

    // Check for dev_insecure feature flag (compile-time gate)
    #[cfg(all(feature = "dev_insecure", not(test)))]
    {
        // Development mode fallback key (RFC 8032 test vector).
        // WARNING: This is only compiled in when --features dev_insecure is used.
        let default_key_bytes = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];

        return EdVerifyingKey::from_bytes(&default_key_bytes)
            .map(|key| vec![key])
            .unwrap_or_default();
    }

    // Production path: no fallback keys
    #[cfg(not(any(test, feature = "dev_insecure")))]
    Vec::new()
}

impl StoreCapability {
    /// Create a new capability (for testing/trusted authority use)
    pub fn new(domain_id: u64, rights_mask: u8, capability_id: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: CAPABILITY_VERSION,
            domain_id,
            rights_mask,
            capability_id,
            issued_at: now,
            expires_at: 0,     // Never expires for now
            signature: vec![], // Unsigned for now (Phase 5 stub)
        }
    }

    /// Check if this capability grants a specific right
    ///
    /// # Arguments
    /// * `right` - One of the STORE_RIGHT_* constants
    pub fn has_right(&self, right: u8) -> bool {
        (self.rights_mask & right) != 0
    }

    /// Check if this capability has expired
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now > self.expires_at
    }

    /// Check if this capability is bound to a specific domain
    pub fn is_for_domain(&self, domain_id: u64) -> bool {
        self.domain_id == domain_id
    }

    /// Get the data that should be signed
    pub fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(37); // 4 + 8 + 1 + 8 + 8 + 8 = 37 bytes

        data.extend_from_slice(&self.version.to_le_bytes());
        data.extend_from_slice(&self.domain_id.to_le_bytes());
        data.push(self.rights_mask);
        data.extend_from_slice(&self.capability_id.to_le_bytes());
        data.extend_from_slice(&self.issued_at.to_le_bytes());
        data.extend_from_slice(&self.expires_at.to_le_bytes());

        data
    }

    /// Verify the capability signature against trusted public keys.
    ///
    /// This method loads trusted public keys from the RAMEN_STORE_TRUSTED_KEYS
    /// environment variable and verifies that the signature was created by
    /// one of the trusted keys.
    ///
    /// # Returns
    /// `true` if the signature is valid, `false` otherwise.
    pub fn verify_signature(&self) -> bool {
        let trusted_keys = load_trusted_public_keys();
        self.verify_signature_with_keys(&trusted_keys)
    }

    /// Verify the capability signature against a specific set of public keys.
    ///
    /// This method is provided for testability and allows verification against
    /// a specific set of keys rather than loading from the environment.
    ///
    /// # Arguments
    /// * `keys` - A slice of Ed25519 public keys to verify against
    ///
    /// # Returns
    /// `true` if the signature is valid for any of the provided keys, `false` otherwise.
    pub fn verify_signature_with_keys(&self, keys: &[EdVerifyingKey]) -> bool {
        // Check if signature is present and has correct length
        if self.signature.len() != 64 {
            return false;
        }

        // Parse the signature
        let signature = match EdSignature::from_slice(&self.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        // Get the signing data
        let signing_data = self.signing_data();

        // Try to verify against each trusted key
        for public_key in keys {
            if public_key.verify(&signing_data, &signature).is_ok() {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey as EdSigningKey};
    use rand::RngCore;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Helper function to create a test Ed25519 key pair
    fn create_test_keypair() -> (EdSigningKey, EdVerifyingKey) {
        let mut rng = rand::rngs::OsRng;
        let mut secret_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut secret_key_bytes);
        let signing_key = EdSigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    #[test]
    fn capability_has_right_checks_correctly() {
        let cap = StoreCapability::new(1, STORE_RIGHT_READ | STORE_RIGHT_WRITE, 100);

        assert!(cap.has_right(STORE_RIGHT_READ));
        assert!(cap.has_right(STORE_RIGHT_WRITE));
        assert!(!cap.has_right(STORE_RIGHT_DELETE));
        assert!(!cap.has_right(STORE_RIGHT_ADMIN));
    }

    #[test]
    fn capability_is_for_domain_checks_correctly() {
        let cap = StoreCapability::new(5, STORE_RIGHT_READ, 100);

        assert!(cap.is_for_domain(5));
        assert!(!cap.is_for_domain(1));
        assert!(!cap.is_for_domain(0));
    }

    #[test]
    fn capability_never_expires_when_expires_at_is_zero() {
        let cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        assert!(!cap.is_expired());
    }

    #[test]
    fn capability_expires_when_past_expiration_time() {
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        cap.expires_at = 1; // Epoch + 1 second (definitely expired)
        assert!(cap.is_expired());
    }

    #[test]
    fn capability_signing_data_is_deterministic() {
        let cap = StoreCapability::new(42, STORE_RIGHT_READ, 999);

        let data1 = cap.signing_data();
        let data2 = cap.signing_data();

        assert_eq!(data1, data2);
    }

    #[test]
    fn capability_signing_data_contains_all_fields() {
        let cap = StoreCapability::new(123, STORE_RIGHT_ALL, 456);

        let data = cap.signing_data();

        // Should be 37 bytes: version(4) + domain_id(8) + rights(1) + cap_id(8) + issued(8) + expires(8)
        assert_eq!(data.len(), 37);
    }

    #[test]
    fn verify_signature_with_keys_valid_signature_passes() {
        let (signing_key, verifying_key) = create_test_keypair();

        // Create a capability
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        // Sign the capability
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data);
        cap.signature = signature.to_bytes().to_vec();

        // Verify with the correct key
        assert!(cap.verify_signature_with_keys(&[verifying_key]));
    }

    #[test]
    fn verify_signature_with_keys_invalid_signature_fails() {
        let (_signing_key, verifying_key) = create_test_keypair();

        // Create a capability with an invalid signature
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        cap.signature = vec![0u8; 64]; // Invalid signature

        // Verify should fail
        assert!(!cap.verify_signature_with_keys(&[verifying_key]));
    }

    #[test]
    fn verify_signature_with_keys_wrong_key_fails() {
        let (signing_key, _verifying_key) = create_test_keypair();
        let (_, wrong_verifying_key) = create_test_keypair();

        // Create a capability
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        // Sign the capability
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data);
        cap.signature = signature.to_bytes().to_vec();

        // Verify with a different key should fail
        assert!(!cap.verify_signature_with_keys(&[wrong_verifying_key]));
    }

    #[test]
    fn verify_signature_with_keys_no_keys_fails() {
        let (signing_key, _) = create_test_keypair();

        // Create a capability
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        // Sign the capability
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data);
        cap.signature = signature.to_bytes().to_vec();

        // Verify with no keys should fail
        assert!(!cap.verify_signature_with_keys(&[]));
    }

    #[test]
    fn verify_signature_with_keys_multiple_keys_valid() {
        let (signing_key, verifying_key) = create_test_keypair();
        let (_, extra_key1) = create_test_keypair();
        let (_, extra_key2) = create_test_keypair();

        // Create a capability
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        // Sign the capability
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data);
        cap.signature = signature.to_bytes().to_vec();

        // Verify with multiple keys, one of which is correct
        assert!(cap.verify_signature_with_keys(&[extra_key1, verifying_key, extra_key2]));
    }

    #[test]
    fn verify_signature_with_keys_invalid_signature_length_fails() {
        let (_, verifying_key) = create_test_keypair();

        // Create a capability with wrong signature length
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        cap.signature = vec![0u8; 32]; // Wrong length

        // Verify should fail
        assert!(!cap.verify_signature_with_keys(&[verifying_key]));
    }

    #[test]
    fn load_trusted_public_keys_from_env_variable() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let (signing_key, verifying_key) = create_test_keypair();
        let key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());

        // Set the environment variable
        env::set_var("RAMEN_STORE_TRUSTED_KEYS", &key_b64);

        // Load keys
        let keys = load_trusted_public_keys();

        // Clean up
        env::remove_var("RAMEN_STORE_TRUSTED_KEYS");

        // Should have loaded one key
        assert_eq!(keys.len(), 1);
        // Verify it's the correct key by signing and verifying
        let signing_data = b"test data";
        let signature = signing_key.sign(signing_data);
        assert!(keys[0].verify(signing_data, &signature).is_ok());
    }

    #[test]
    fn load_trusted_public_keys_multiple_keys() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let (_, verifying_key1) = create_test_keypair();
        let (_, verifying_key2) = create_test_keypair();

        let key1_b64 = general_purpose::STANDARD.encode(verifying_key1.as_bytes());
        let key2_b64 = general_purpose::STANDARD.encode(verifying_key2.as_bytes());

        // Set the environment variable with multiple keys
        env::set_var(
            "RAMEN_STORE_TRUSTED_KEYS",
            format!("{},{}", key1_b64, key2_b64),
        );

        // Load keys
        let keys = load_trusted_public_keys();

        // Clean up
        env::remove_var("RAMEN_STORE_TRUSTED_KEYS");

        // Should have loaded two keys
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn load_trusted_public_keys_invalid_key_skipped() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let (_, verifying_key) = create_test_keypair();

        let valid_key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());
        let invalid_key_b64 = "invalid_base64!!!";

        // Set the environment variable with one valid and one invalid key
        env::set_var(
            "RAMEN_STORE_TRUSTED_KEYS",
            format!("{},{}", valid_key_b64, invalid_key_b64),
        );

        // Load keys
        let keys = load_trusted_public_keys();

        // Clean up
        env::remove_var("RAMEN_STORE_TRUSTED_KEYS");

        // Should have loaded only the valid key
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn load_trusted_public_keys_no_env_var_uses_default() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // Ensure the environment variable is not set
        env::remove_var("RAMEN_STORE_TRUSTED_KEYS");

        // Load keys
        let keys = load_trusted_public_keys();

        // Should have loaded the default development key
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn verify_signature_integration() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let (signing_key, verifying_key) = create_test_keypair();

        // Create a capability
        let mut cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);

        // Sign the capability
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data);
        cap.signature = signature.to_bytes().to_vec();

        // Set the environment variable with the verifying key
        let key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());
        env::set_var("RAMEN_STORE_TRUSTED_KEYS", &key_b64);

        // Verify
        let result = cap.verify_signature();

        // Clean up
        env::remove_var("RAMEN_STORE_TRUSTED_KEYS");

        // Should succeed
        assert!(result);
    }
}
