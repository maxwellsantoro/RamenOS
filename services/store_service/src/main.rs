// V-007 Phase 2: Store Service Binary
//
// Unix domain socket server providing artifact storage operations.
// Enforces "kernel ≠ services ≠ store" architectural boundary.
//
// V-007 Phase 3: Added audit logging, signature validation (stub), and access control (stub).

use anyhow::{Context, Result};
use artifact_store_core::{hash_blob, verify_blob_matches_manifest};
use artifact_store_schema::{
    ContentId, Manifest,
    signature::{ManifestSignature, SignaturePolicy, SignatureValidationConfig, TrustedKeys},
};
use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use store_service::access_control::{AccessControl, ClientInfo};
use store_service::audit::{
    AuditLogEntry, AuditLogParameters, AuditLogger, Operation, OperationResult, Timer,
    current_timestamp,
};
use store_service::frame::{read_message, write_message};

// Import capability types
use serde::Deserialize;
use store_service::capability::{
    STORE_RIGHT_ALL, STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability,
};
use store_service::domain_visibility::DomainArtifactRegistry;
use store_service::projection_index::{ProjectionIndexStore, ingest_projection_records};
use store_service::status::*;

// Store service configuration
const DEFAULT_SOCKET_PATH: &str = "out/store_service.sock";
const DEFAULT_STORE_ROOT: &str = "out/installed/artifacts";
const DEFAULT_AUDIT_LOG_PATH: &str = "out/store_service_audit.log";

// Message types
const MSG_GET_MANIFEST: u8 = 1;
const MSG_GET_BLOB: u8 = 2;
const MSG_VERIFY_ARTIFACT: u8 = 3;
const MSG_INGEST_ARTIFACT: u8 = 4;
const MSG_QUERY_PROJECTION_BY_PATH: u8 = 5;
const MSG_QUERY_PROJECTION_BY_TAG: u8 = 6;

/// Helper trait for extracting capability from request messages
trait RequestWithCapability {
    fn capability_bytes(&self) -> &[u8];
}

/// Outcome of capability validation with rights check.
///
/// This enum represents the result of validating a capability and checking
/// that it has the required rights. It either contains the validated capability
/// or the serialized error reply bytes ready to send.
#[must_use]
enum CapabilityOutcome {
    /// Capability is valid and has the required rights
    Valid(StoreCapability),
    /// Validation failed - contains the serialized error reply bytes
    Denied(Vec<u8>),
}

/// Validates capability and checks required rights.
///
/// This function combines:
/// 1. Capability extraction and validation (via `extract_and_validate_capability`)
/// 2. Rights checking
/// 3. Error reply creation (via the provided closure)
///
/// This reduces code duplication across handlers by centralizing the common
/// pattern of validating a capability, checking rights, and creating error
/// replies on failure.
///
/// # Type Parameters
/// - `T`: Request type implementing `RequestWithCapability`
/// - `F`: Closure type for creating error reply
///
/// # Arguments
/// * `payload` - The raw request payload bytes
/// * `required_right` - The required rights mask (e.g., `STORE_RIGHT_READ`)
/// * `operation_name` - Name of the operation for error logging
/// * `create_denied_reply` - Closure that creates serialized error reply bytes
///
/// # Returns
/// * `CapabilityOutcome::Valid(cap)` - Capability is valid with required rights
/// * `CapabilityOutcome::Denied(reply_bytes)` - Validation failed, reply ready to send
///
/// # Example
/// ```ignore
/// let cap = match validate_capability::<GetManifestRequest, _>(
///     payload,
///     STORE_RIGHT_READ,
///     "get_manifest",
///     || create_permission_denied_reply(request_id),
/// ) {
///     CapabilityOutcome::Valid(cap) => cap,
///     CapabilityOutcome::Denied(reply_bytes) => {
///         return Ok((reply_bytes, audit_params, OperationResult::PermissionDenied));
///     }
/// };
/// ```
fn validate_capability<T, F>(
    payload: &[u8],
    required_right: u8,
    operation_name: &'static str,
    create_denied_reply: F,
) -> CapabilityOutcome
where
    T: for<'de> Deserialize<'de> + RequestWithCapability,
    F: FnOnce() -> Vec<u8>,
{
    match extract_and_validate_capability::<T>(payload) {
        Ok(Some(cap)) if cap.has_right(required_right) => CapabilityOutcome::Valid(cap),
        Ok(Some(cap)) => {
            eprintln!(
                "store_service: {}: capability missing required right for domain {}",
                operation_name, cap.domain_id
            );
            CapabilityOutcome::Denied(create_denied_reply())
        }
        Ok(None) => {
            eprintln!(
                "store_service: {}: no capability provided in request",
                operation_name
            );
            CapabilityOutcome::Denied(create_denied_reply())
        }
        Err(err) => {
            eprintln!(
                "store_service: {}: capability validation failed: {}",
                operation_name, err
            );
            CapabilityOutcome::Denied(create_denied_reply())
        }
    }
}

/// Extract and validate a capability from request payload
///
/// This function:
/// 1. Deserializes the request to extract capability_bytes
/// 2. Deserializes the capability_bytes into a StoreCapability
/// 3. Validates the capability signature
/// 4. Checks that the capability is not expired
///
/// # Returns
/// - Ok(Some(capability)) if the capability is valid
/// - Ok(None) if no capability was provided (for backward compatibility)
/// - Err if the capability is invalid or malformed
fn extract_and_validate_capability<T: for<'de> Deserialize<'de> + RequestWithCapability>(
    payload: &[u8],
) -> Result<Option<StoreCapability>> {
    let dev_mode = store_service::dev_mode::is_dev_mode_enabled();

    // Deserialize the request to get capability_bytes
    let request: T = bincode::deserialize(payload).context("failed to deserialize request")?;

    let capability_bytes = request.capability_bytes();

    // If no capability bytes provided, allow a synthetic full-rights capability only
    // in explicit dev mode. This preserves fail-closed behavior in production while
    // keeping S0/S1 local Foundry flows operational.
    if capability_bytes.is_empty() {
        if dev_mode {
            eprintln!(
                "store_service: no capability provided; using synthetic dev capability (domain=0, rights=ALL)"
            );
            return Ok(Some(StoreCapability::new(0, STORE_RIGHT_ALL, 0)));
        }
        eprintln!("store_service: no capability provided in request");
        return Ok(None);
    }

    // Deserialize the capability
    let capability: StoreCapability = match bincode::deserialize(capability_bytes) {
        Ok(cap) => cap,
        Err(err) => {
            if dev_mode {
                eprintln!(
                    "store_service: malformed capability in dev mode ({}); using synthetic dev capability",
                    err
                );
                return Ok(Some(StoreCapability::new(0, STORE_RIGHT_ALL, 0)));
            }
            return Err(err).context("failed to deserialize capability");
        }
    };

    // Validate the capability signature
    if !capability.verify_signature() {
        if dev_mode {
            eprintln!(
                "store_service: invalid capability signature in dev mode; using synthetic dev capability"
            );
            return Ok(Some(StoreCapability::new(0, STORE_RIGHT_ALL, 0)));
        }
        anyhow::bail!("capability signature verification failed");
    }

    // Check if the capability is expired
    if capability.is_expired() {
        if dev_mode {
            eprintln!(
                "store_service: expired capability in dev mode; using synthetic dev capability"
            );
            return Ok(Some(StoreCapability::new(0, STORE_RIGHT_ALL, 0)));
        }
        anyhow::bail!("capability has expired");
    }

    eprintln!(
        "store_service: capability validated: domain_id={}, rights={:#04x}, capability_id={}",
        capability.domain_id, capability.rights_mask, capability.capability_id
    );

    Ok(Some(capability))
}

/// Convert StoreCapability rights_mask to AccessRights
// Implement RequestWithCapability for each request type
impl RequestWithCapability for store_service::client::GetManifestRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

impl RequestWithCapability for store_service::client::GetBlobRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

impl RequestWithCapability for store_service::client::VerifyArtifactRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

impl RequestWithCapability for store_service::client::IngestArtifactRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

impl RequestWithCapability for store_service::client::QueryProjectionByPathRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

impl RequestWithCapability for store_service::client::QueryProjectionByTagRequest {
    fn capability_bytes(&self) -> &[u8] {
        &self.capability_bytes
    }
}

fn main() -> Result<()> {
    println!("store_service: starting (V-007 Phase 3: Audit logging enabled)");

    // Get configuration from environment
    let socket_path =
        std::env::var("RAMEN_STORE_SOCKET").unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());
    let store_root = PathBuf::from(
        std::env::var("RAMEN_STORE_ROOT").unwrap_or_else(|_| DEFAULT_STORE_ROOT.to_string()),
    );
    let audit_log_path = std::env::var("RAMEN_STORE_AUDIT_LOG")
        .unwrap_or_else(|_| DEFAULT_AUDIT_LOG_PATH.to_string());

    // Ensure store root exists
    fs::create_dir_all(&store_root).context("failed to create store root")?;

    // Initialize audit logger
    let audit_logger =
        AuditLogger::new(&audit_log_path).context("failed to initialize audit logger")?;
    println!("store_service: audit log at {}", audit_logger.log_path());

    // S7 Security Hardening: Fail-Closed Store Signature Policy
    //
    // Security Fix: Signature validation now defaults to RequireSignature with empty keys,
    // which rejects ALL unsigned artifacts. To use AllowUnsigned, explicitly set
    // RAMEN_STORE_DEV_MODE=1 with prominent warnings.
    //
    // Environment Variables:
    // - RAMEN_STORE_TRUSTED_KEYS: Path to file containing trusted Ed25519 public keys (REQUIRED in production)
    // - RAMEN_STORE_DEV_MODE: Set to "1" to allow unsigned artifacts (DEVELOPMENT ONLY)

    let dev_mode = store_service::dev_mode::is_dev_mode_enabled();

    if dev_mode {
        eprintln!();
        eprintln!("╔════════════════════════════════════════════════════════════════════════════╗");
        eprintln!("║ WARNING: RAMEN_STORE_DEV_MODE IS ENABLED                                   ║");
        eprintln!("║                                                                            ║");
        eprintln!("║ The store service will ACCEPT UNSIGNED ARTIFACTS.                          ║");
        eprintln!(
            "║ This is a SECURITY RISK and should NEVER be used in production.             ║"
        );
        eprintln!("║                                                                            ║");
        eprintln!("║ To disable dev mode: unset RAMEN_STORE_DEV_MODE                            ║");
        eprintln!("╚════════════════════════════════════════════════════════════════════════════╝");
        eprintln!();
    }

    let trusted_keys_path = std::env::var("RAMEN_STORE_TRUSTED_KEYS")
        .ok()
        .map(PathBuf::from);

    let (sig_config, keys_info) = if let Some(keys_path) = trusted_keys_path {
        // Load trusted keys from file
        match TrustedKeys::load_from_file(&keys_path) {
            Ok(keys) => {
                let num_keys = keys.len();
                println!(
                    "store_service: loaded {} trusted keys from {}",
                    num_keys,
                    keys_path.display()
                );
                (
                    SignatureValidationConfig {
                        policy: SignaturePolicy::RequireSignature,
                        trusted_keys: keys,
                        ..Default::default()
                    },
                    format!("RequireSignature with {} keys", num_keys),
                )
            }
            Err(e) => {
                eprintln!(
                    "store_service: SECURITY ERROR: failed to load trusted keys from {}: {}",
                    keys_path.display(),
                    e
                );
                eprintln!(
                    "store_service: ABORTING: Signature validation cannot be properly configured"
                );
                eprintln!(
                    "store_service: Fix: Ensure RAMEN_STORE_TRUSTED_KEYS points to a valid file with Ed25519 public keys"
                );
                eprintln!(
                    "store_service:       Or set RAMEN_STORE_DEV_MODE=1 for development (NOT RECOMMENDED)"
                );
                return Err(anyhow::anyhow!(
                    "signature validation configuration failed: {}",
                    e
                ));
            }
        }
    } else if dev_mode {
        // Dev mode with no trusted keys: AllowUnsigned with warnings
        println!(
            "store_service: RAMEN_STORE_TRUSTED_KEYS not set, using AllowUnsigned policy (DEV MODE)"
        );
        println!("store_service: WARNING: Unsigned artifacts will be accepted - SECURITY RISK");
        (
            SignatureValidationConfig {
                policy: SignaturePolicy::AllowUnsigned,
                trusted_keys: TrustedKeys::new(),
                ..Default::default()
            },
            "AllowUnsigned (dev mode)".to_string(),
        )
    } else {
        // Production mode without trusted keys: Fail-closed
        eprintln!("store_service: SECURITY ERROR: RAMEN_STORE_TRUSTED_KEYS not set");
        eprintln!(
            "store_service: ABORTING: Signature validation requires trusted keys in production mode"
        );
        eprintln!(
            "store_service: Fix: Set RAMEN_STORE_TRUSTED_KEYS to point to a file with trusted Ed25519 public keys"
        );
        eprintln!(
            "store_service:       Or set RAMEN_STORE_DEV_MODE=1 for development (NOT RECOMMENDED)"
        );
        return Err(anyhow::anyhow!(
            "signature validation configuration failed: RAMEN_STORE_TRUSTED_KEYS not set"
        ));
    };

    println!("store_service: signature validation policy: {}", keys_info);

    // S7 Security Hardening: Fail-Closed Access Control Default
    //
    // Security Fix: Access control now defaults to RequireCredentials.
    // To change policy, set RAMEN_STORE_ACCESS_POLICY environment variable.
    //
    // Environment Variables:
    // - RAMEN_STORE_ACCESS_POLICY: Access control policy (AllowAll, RequireCredentials, RequireKnownService, Whitelist)
    //                               Default: RequireCredentials (fail-closed)

    let access_policy_str = std::env::var("RAMEN_STORE_ACCESS_POLICY")
        .ok()
        .map(|s| s.to_lowercase());

    let access_policy = match access_policy_str.as_deref() {
        Some("allowall") => {
            eprintln!(
                "store_service: WARNING: RAMEN_STORE_ACCESS_POLICY=AllowAll - NO ACCESS CONTROL"
            );
            eprintln!(
                "store_service: This is a SECURITY RISK and should NEVER be used in production."
            );
            store_service::access_control::AccessPolicy::AllowAll
        }
        Some("requireknownservice") => {
            store_service::access_control::AccessPolicy::RequireKnownService
        }
        Some("whitelist") => store_service::access_control::AccessPolicy::Whitelist,
        Some("requirecredentials") | None => {
            // Default to RequireCredentials (fail-closed)
            store_service::access_control::AccessPolicy::RequireCredentials
        }
        Some(invalid) => {
            eprintln!(
                "store_service: WARNING: Invalid RAMEN_STORE_ACCESS_POLICY value: {}",
                invalid
            );
            eprintln!("store_service: Using default: RequireCredentials");
            store_service::access_control::AccessPolicy::RequireCredentials
        }
    };

    let access_control = AccessControl::with_policy(access_policy);
    println!("store_service: access control policy: {:?}", access_policy);

    // Initialize domain artifact registry (V-007 Phase 5)
    let domain_registry =
        DomainArtifactRegistry::new(&store_root).context("failed to initialize domain registry")?;
    println!(
        "store_service: domain registry initialized with {} artifacts",
        domain_registry.list_domain_artifacts(0).len()
    ); // Global artifacts

    let projection_index_path = std::env::var("RAMEN_STORE_PROJECTION_INDEX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ProjectionIndexStore::default_path(&store_root));
    let projection_index = match ProjectionIndexStore::load_or_empty(&projection_index_path) {
        Ok(store) => {
            if store.is_loaded() {
                println!(
                    "store_service: projection index loaded from {}",
                    projection_index_path.display()
                );
            } else {
                println!(
                    "store_service: projection index not found at {} (starting empty durable index)",
                    projection_index_path.display()
                );
            }
            store
        }
        Err(err) => {
            eprintln!(
                "store_service: failed to load projection index from {}: {}",
                projection_index_path.display(),
                err
            );
            return Err(anyhow::anyhow!("projection index load failed: {}", err));
        }
    };

    // Remove stale socket file if it exists
    if Path::new(&socket_path).exists() {
        fs::remove_file(&socket_path).context("failed to remove stale socket file")?;
    }

    // Bind socket
    let listener = UnixListener::bind(&socket_path).context("failed to bind socket")?;

    // Set socket permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&socket_path)
            .context("failed to get socket metadata")?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&socket_path, permissions)
            .context("failed to set socket permissions")?;
    }

    println!("store_service: listening on {}", socket_path);

    // Wrap resources for multi-threaded access
    let store_root = Arc::new(store_root);
    let audit_logger = Arc::new(audit_logger);
    let sig_config = Arc::new(sig_config);
    let access_control = Arc::new(access_control);
    let domain_registry = Arc::new(Mutex::new(domain_registry));
    let projection_index = Arc::new(Mutex::new(projection_index));

    // Accept and handle connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store_root = Arc::clone(&store_root);
                let audit_logger = Arc::clone(&audit_logger);
                let sig_config = Arc::clone(&sig_config);
                let access_control = Arc::clone(&access_control);
                let domain_registry = Arc::clone(&domain_registry);
                let projection_index = Arc::clone(&projection_index);

                std::thread::spawn(move || {
                    if let Err(err) = handle_client(
                        stream,
                        &store_root,
                        &audit_logger,
                        &sig_config,
                        &access_control,
                        domain_registry,
                        projection_index,
                    ) {
                        eprintln!("store_service: client thread error: {}", err);
                    }
                });
            }
            Err(err) => {
                eprintln!("store_service: accept error: {}", err);
            }
        }
    }

    Ok(())
}

fn handle_client(
    mut stream: UnixStream,
    store_root: &Path,
    audit_logger: &AuditLogger,
    sig_config: &SignatureValidationConfig,
    access_control: &AccessControl,
    domain_registry: Arc<Mutex<DomainArtifactRegistry>>,
    projection_index: Arc<Mutex<ProjectionIndexStore>>,
) -> Result<()> {
    // Get client info (V-007 Phase 3: Stub, will be enhanced in Phase 4)
    let client_info = ClientInfo::from_stream(&stream);
    eprintln!(
        "store_service: client connected (pid: {:?}, uid: {:?}, gid: {:?})",
        client_info.pid, client_info.uid, client_info.gid
    );

    // Handle requests until connection closes
    loop {
        let msg = match read_message(&mut stream) {
            Ok(msg) => msg,
            Err(err) => {
                // Connection closed or error
                eprintln!("store_service: read error: {}", err);
                return Ok(()); // Don't propagate connection errors
            }
        };

        if msg.is_empty() {
            break;
        }

        // Extract message type
        let msg_type = msg[0];

        // Process request with audit logging
        let timer = Timer::new();
        let (reply, operation, parameters, result) = match msg_type {
            MSG_GET_MANIFEST => {
                let registry = domain_registry
                    .lock()
                    .map_err(|_| anyhow::anyhow!("domain registry mutex poisoned"))?;
                let (reply, params, result) = handle_get_manifest(
                    &msg[1..],
                    store_root,
                    sig_config,
                    access_control,
                    &client_info,
                    &registry,
                )?;
                (reply, Operation::GetManifest, params, result)
            }
            MSG_GET_BLOB => {
                let registry = domain_registry
                    .lock()
                    .map_err(|_| anyhow::anyhow!("domain registry mutex poisoned"))?;
                let (reply, params, result) = handle_get_blob(
                    &msg[1..],
                    store_root,
                    sig_config,
                    access_control,
                    &client_info,
                    &registry,
                )?;
                (reply, Operation::GetBlob, params, result)
            }
            MSG_VERIFY_ARTIFACT => {
                let registry = domain_registry
                    .lock()
                    .map_err(|_| anyhow::anyhow!("domain registry mutex poisoned"))?;
                let (reply, params, result) = handle_verify_artifact(
                    &msg[1..],
                    store_root,
                    sig_config,
                    access_control,
                    &client_info,
                    &registry,
                )?;
                (reply, Operation::VerifyArtifact, params, result)
            }
            MSG_INGEST_ARTIFACT => {
                let mut registry = domain_registry
                    .lock()
                    .map_err(|_| anyhow::anyhow!("domain registry mutex poisoned"))?;
                let mut index = projection_index
                    .lock()
                    .map_err(|_| anyhow::anyhow!("projection index mutex poisoned"))?;
                let (reply, params, result) = handle_ingest_artifact(
                    &msg[1..],
                    store_root,
                    access_control,
                    &client_info,
                    &mut registry,
                    &mut index,
                )?;
                (reply, Operation::IngestArtifact, params, result)
            }
            MSG_QUERY_PROJECTION_BY_PATH => {
                let index = projection_index
                    .lock()
                    .map_err(|_| anyhow::anyhow!("projection index mutex poisoned"))?;
                let (reply, params, result) = handle_query_projection_by_path(
                    &msg[1..],
                    access_control,
                    &client_info,
                    &index,
                )?;
                (reply, Operation::QueryProjectionByPath, params, result)
            }
            MSG_QUERY_PROJECTION_BY_TAG => {
                let index = projection_index
                    .lock()
                    .map_err(|_| anyhow::anyhow!("projection index mutex poisoned"))?;
                let (reply, params, result) = handle_query_projection_by_tag(
                    &msg[1..],
                    access_control,
                    &client_info,
                    &index,
                )?;
                (reply, Operation::QueryProjectionByTag, params, result)
            }
            _ => {
                eprintln!("store_service: unknown message type: {}", msg_type);
                continue; // Don't send reply for unknown messages
            }
        };

        // Send reply
        write_message(&mut stream, &reply)?;

        // Write audit log
        let duration_ms = timer.elapsed_ms();
        let audit_entry = AuditLogEntry {
            timestamp: current_timestamp(),
            client_pid: client_info.pid,
            operation,
            parameters,
            result,
            duration_ms,
        };

        if let Err(err) = audit_logger.log(&audit_entry) {
            eprintln!("store_service: failed to write audit log: {}", err);
        }
    }

    Ok(())
}

fn handle_get_manifest(
    payload: &[u8],
    store_root: &Path,
    sig_config: &SignatureValidationConfig,
    access_control: &AccessControl,
    client_info: &ClientInfo,
    domain_registry: &DomainArtifactRegistry,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    let request: GetManifestRequest =
        bincode::deserialize(payload).context("failed to deserialize GetManifestRequest")?;

    let content_id = request.content_id.clone();
    let request_id = request.request_id;

    // V-007 Phase 6: Extract and validate capability from request message
    let cap = match validate_capability::<GetManifestRequest, _>(
        payload,
        STORE_RIGHT_READ,
        "get_manifest",
        || {
            bincode::serialize(&GetManifestReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                ..Default::default()
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::GetManifest { content_id },
                OperationResult::PermissionDenied,
            ));
        }
    };

    // V-007 Phase 5: Check domain-scoped visibility
    let id =
        ContentId::parse(&content_id).map_err(|_| anyhow::anyhow!("invalid content_id format"))?;

    if !domain_registry.can_access(&id, cap.domain_id) {
        eprintln!(
            "store_service: get_manifest: domain {} cannot access artifact {}",
            cap.domain_id, content_id
        );
        let error_reply = GetManifestReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            ..Default::default()
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::GetManifest { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    // V-007 Phase 3: Check access control (stub)
    let access_decision = access_control.can_read(client_info, &content_id);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = GetManifestReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            ..Default::default()
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::GetManifest { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    let result = (|| -> Result<GetManifestReply> {
        // id is already parsed above in domain visibility check

        let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        if !manifest_path.exists() {
            return Ok(GetManifestReply {
                request_id,
                status: STATUS_NOT_FOUND,
                ..Default::default()
            });
        }

        let manifest_json =
            fs::read_to_string(&manifest_path).context("failed to read manifest")?;
        let manifest: Manifest =
            serde_json::from_str(&manifest_json).context("failed to parse manifest")?;

        // S7 Security Hardening: Validate signatures with detailed logging
        let sig_result = artifact_store_schema::signature::validate_manifest_signatures(
            &manifest.signatures,
            manifest_json.as_bytes(),
            sig_config,
        );

        match sig_result {
            artifact_store_schema::signature::SignatureValidationResult::Valid => {
                // Signature valid or unsigned allowed
                if manifest.signatures.is_empty() {
                    eprintln!(
                        "store_service: get_manifest: artifact {} accepted (unsigned allowed by policy)",
                        content_id
                    );
                } else {
                    eprintln!(
                        "store_service: get_manifest: artifact {} accepted (signature valid)",
                        content_id
                    );
                }
            }
            artifact_store_schema::signature::SignatureValidationResult::Invalid => {
                eprintln!(
                    "store_service: SECURITY ALERT: Signature validation FAILED for artifact {}",
                    content_id
                );
                eprintln!("store_service:   - Content ID: {}", content_id);
                eprintln!("store_service:   - Client PID: {:?}", client_info.pid);
                eprintln!("store_service:   - Client UID: {:?}", client_info.uid);
                eprintln!("store_service:   - Client Domain: {}", cap.domain_id);
                eprintln!(
                    "store_service:   - Number of signatures: {}",
                    manifest.signatures.len()
                );
                eprintln!(
                    "store_service:   - Validation Policy: {:?}",
                    sig_config.policy
                );
                eprintln!(
                    "store_service:   - Trusted Keys Count: {}",
                    sig_config.trusted_keys.len()
                );
                for (i, sig) in manifest.signatures.iter().enumerate() {
                    match serde_json::from_str::<ManifestSignature>(sig) {
                        Ok(parsed) => eprintln!(
                            "store_service:   - Signature {}: algorithm={:?}, key_id={}",
                            i, parsed.algorithm, parsed.key_id
                        ),
                        Err(_) => eprintln!(
                            "store_service:   - Signature {}: malformed entry: {}",
                            i, sig
                        ),
                    }
                }
                return Ok(GetManifestReply {
                    request_id,
                    status: STATUS_VALIDATION_FAILED,
                    ..Default::default()
                });
            }
            artifact_store_schema::signature::SignatureValidationResult::Malformed => {
                eprintln!(
                    "store_service: SECURITY ALERT: Malformed signature for artifact {}",
                    content_id
                );
                eprintln!("store_service:   - Content ID: {}", content_id);
                eprintln!("store_service:   - Client PID: {:?}", client_info.pid);
                eprintln!("store_service:   - Client Domain: {}", cap.domain_id);
                return Ok(GetManifestReply {
                    request_id,
                    status: STATUS_VALIDATION_FAILED,
                    ..Default::default()
                });
            }
            artifact_store_schema::signature::SignatureValidationResult::UnknownKey => {
                eprintln!(
                    "store_service: SECURITY ALERT: Unknown signing key for artifact {}",
                    content_id
                );
                eprintln!("store_service:   - Content ID: {}", content_id);
                eprintln!("store_service:   - Client PID: {:?}", client_info.pid);
                eprintln!("store_service:   - Client Domain: {}", cap.domain_id);
                for sig in &manifest.signatures {
                    match serde_json::from_str::<ManifestSignature>(sig) {
                        Ok(parsed) => {
                            eprintln!("store_service:   - Unknown key_id: {}", parsed.key_id)
                        }
                        Err(_) => {
                            eprintln!("store_service:   - Unknown key_id in malformed entry")
                        }
                    }
                }
                return Ok(GetManifestReply {
                    request_id,
                    status: STATUS_VALIDATION_FAILED,
                    ..Default::default()
                });
            }
            artifact_store_schema::signature::SignatureValidationResult::Expired => {
                eprintln!(
                    "store_service: SECURITY ALERT: Expired signature for artifact {}",
                    content_id
                );
                eprintln!("store_service:   - Content ID: {}", content_id);
                eprintln!("store_service:   - Client PID: {:?}", client_info.pid);
                eprintln!("store_service:   - Client Domain: {}", cap.domain_id);
                return Ok(GetManifestReply {
                    request_id,
                    status: STATUS_VALIDATION_FAILED,
                    ..Default::default()
                });
            }
            artifact_store_schema::signature::SignatureValidationResult::NoSignatures => {
                eprintln!(
                    "store_service: SECURITY ALERT: No signatures for artifact {} (RequireSignature policy)",
                    content_id
                );
                eprintln!("store_service:   - Content ID: {}", content_id);
                eprintln!("store_service:   - Client PID: {:?}", client_info.pid);
                eprintln!("store_service:   - Client Domain: {}", cap.domain_id);
                eprintln!("store_service:   - Validation Policy: RequireSignature");
                return Ok(GetManifestReply {
                    request_id,
                    status: STATUS_VALIDATION_FAILED,
                    ..Default::default()
                });
            }
        }

        Ok(GetManifestReply {
            request_id,
            status: STATUS_OK,
            schema_version: manifest.schema_version,
            content_id: manifest.content_id,
            size_bytes: manifest.size_bytes,
            kind: manifest.kind,
            channels: manifest.channels.join(","),
            signatures: serde_json::to_string(&manifest.signatures).unwrap_or_default(),
        })
    })();

    match result {
        Ok(reply) => {
            let status = OperationResult::from_status_code(reply.status);
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::GetManifest { content_id },
                status,
            ))
        }
        Err(err) => {
            eprintln!("store_service: get_manifest error: {}", err);
            let error_reply = GetManifestReply {
                request_id,
                status: STATUS_IO_ERROR,
                ..Default::default()
            };
            let reply_bytes =
                bincode::serialize(&error_reply).context("failed to serialize error reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::GetManifest { content_id },
                OperationResult::IoError,
            ))
        }
    }
}

fn handle_get_blob(
    payload: &[u8],
    store_root: &Path,
    sig_config: &SignatureValidationConfig,
    access_control: &AccessControl,
    client_info: &ClientInfo,
    domain_registry: &DomainArtifactRegistry,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    let request: GetBlobRequest =
        bincode::deserialize(payload).context("failed to deserialize GetBlobRequest")?;

    let content_id = request.content_id.clone();
    let request_id = request.request_id;

    // V-007 Phase 6: Extract and validate capability from request message
    let cap = match validate_capability::<GetBlobRequest, _>(
        payload,
        STORE_RIGHT_READ,
        "get_blob",
        || {
            bincode::serialize(&GetBlobReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                blob_path: String::new(),
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::GetBlob { content_id },
                OperationResult::PermissionDenied,
            ));
        }
    };

    // V-007 Phase 5: Check domain-scoped visibility
    let id =
        ContentId::parse(&content_id).map_err(|_| anyhow::anyhow!("invalid content_id format"))?;

    if !domain_registry.can_access(&id, cap.domain_id) {
        eprintln!(
            "store_service: get_blob: domain {} cannot access artifact {}",
            cap.domain_id, content_id
        );
        let error_reply = GetBlobReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            blob_path: String::new(),
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::GetBlob { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    // V-007 Phase 3: Check access control (stub)
    let access_decision = access_control.can_read(client_info, &content_id);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = GetBlobReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            blob_path: String::new(),
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::GetBlob { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    let result = (|| -> Result<GetBlobReply> {
        // id is already parsed above in domain visibility check

        let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
        let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        if !blob_path.exists() || !manifest_path.exists() {
            return Ok(GetBlobReply {
                request_id,
                status: STATUS_NOT_FOUND,
                blob_path: String::new(),
            });
        }

        let manifest_json =
            fs::read_to_string(&manifest_path).context("failed to read manifest")?;
        let manifest: Manifest =
            serde_json::from_str(&manifest_json).context("failed to parse manifest")?;

        let sig_result = artifact_store_schema::signature::validate_manifest_signatures(
            &manifest.signatures,
            manifest_json.as_bytes(),
            sig_config,
        );

        if sig_result != artifact_store_schema::signature::SignatureValidationResult::Valid {
            eprintln!(
                "store_service: SECURITY ALERT: get_blob signature policy rejected artifact {}",
                content_id
            );
            eprintln!("store_service:   - Validation result: {:?}", sig_result);
            return Ok(GetBlobReply {
                request_id,
                status: STATUS_VALIDATION_FAILED,
                blob_path: String::new(),
            });
        }

        Ok(GetBlobReply {
            request_id,
            status: STATUS_OK,
            blob_path: blob_path.to_string_lossy().to_string(),
        })
    })();

    match result {
        Ok(reply) => {
            let status = OperationResult::from_status_code(reply.status);
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::GetBlob { content_id },
                status,
            ))
        }
        Err(err) => {
            eprintln!("store_service: get_blob error: {}", err);
            let error_reply = GetBlobReply {
                request_id,
                status: STATUS_IO_ERROR,
                blob_path: String::new(),
            };
            let reply_bytes =
                bincode::serialize(&error_reply).context("failed to serialize error reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::GetBlob { content_id },
                OperationResult::IoError,
            ))
        }
    }
}

fn handle_verify_artifact(
    payload: &[u8],
    store_root: &Path,
    sig_config: &SignatureValidationConfig,
    access_control: &AccessControl,
    client_info: &ClientInfo,
    domain_registry: &DomainArtifactRegistry,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    let request: VerifyArtifactRequest =
        bincode::deserialize(payload).context("failed to deserialize VerifyArtifactRequest")?;

    let content_id = request.content_id.clone();
    let request_id = request.request_id;

    // V-007 Phase 6: Extract and validate capability from request message
    let cap = match validate_capability::<VerifyArtifactRequest, _>(
        payload,
        STORE_RIGHT_READ,
        "verify_artifact",
        || {
            bincode::serialize(&VerifyArtifactReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                valid: 0,
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::VerifyArtifact { content_id },
                OperationResult::PermissionDenied,
            ));
        }
    };

    // V-007 Phase 5: Check domain-scoped visibility
    let id =
        ContentId::parse(&content_id).map_err(|_| anyhow::anyhow!("invalid content_id format"))?;

    if !domain_registry.can_access(&id, cap.domain_id) {
        eprintln!(
            "store_service: verify_artifact: domain {} cannot access artifact {}",
            cap.domain_id, content_id
        );
        let error_reply = VerifyArtifactReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            valid: 0,
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::VerifyArtifact { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    // V-007 Phase 3: Check access control (stub)
    let access_decision = access_control.can_read(client_info, &content_id);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = VerifyArtifactReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            valid: 0,
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::VerifyArtifact { content_id },
            OperationResult::PermissionDenied,
        ));
    }

    let result = (|| -> Result<VerifyArtifactReply> {
        // id is already parsed above in domain visibility check

        let blob_path = store_root.join(format!("{}.blob", id.hash_hex()));
        let manifest_path = store_root.join(format!("{}.manifest.json", id.hash_hex()));

        if !blob_path.exists() || !manifest_path.exists() {
            return Ok(VerifyArtifactReply {
                request_id,
                status: STATUS_NOT_FOUND,
                valid: 0,
            });
        }

        let manifest_json =
            fs::read_to_string(&manifest_path).context("failed to read manifest")?;
        let manifest: Manifest =
            serde_json::from_str(&manifest_json).context("failed to parse manifest")?;

        let sig_result = artifact_store_schema::signature::validate_manifest_signatures(
            &manifest.signatures,
            manifest_json.as_bytes(),
            sig_config,
        );

        if sig_result != artifact_store_schema::signature::SignatureValidationResult::Valid {
            eprintln!(
                "store_service: SECURITY ALERT: verify_artifact signature policy rejected artifact {}",
                content_id
            );
            eprintln!("store_service:   - Validation result: {:?}", sig_result);
            return Ok(VerifyArtifactReply {
                request_id,
                status: STATUS_VALIDATION_FAILED,
                valid: 0,
            });
        }

        match verify_blob_matches_manifest(&blob_path, &manifest_path) {
            Ok(()) => Ok(VerifyArtifactReply {
                request_id,
                status: STATUS_OK,
                valid: 1,
            }),
            Err(_) => Ok(VerifyArtifactReply {
                request_id,
                status: STATUS_OK,
                valid: 0,
            }),
        }
    })();

    match result {
        Ok(reply) => {
            let status = OperationResult::from_status_code(reply.status);
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::VerifyArtifact { content_id },
                status,
            ))
        }
        Err(err) => {
            eprintln!("store_service: verify_artifact error: {}", err);
            let error_reply = VerifyArtifactReply {
                request_id,
                status: STATUS_IO_ERROR,
                valid: 0,
            };
            let reply_bytes =
                bincode::serialize(&error_reply).context("failed to serialize error reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::VerifyArtifact { content_id },
                OperationResult::IoError,
            ))
        }
    }
}

fn handle_ingest_artifact(
    payload: &[u8],
    store_root: &Path,
    access_control: &AccessControl,
    client_info: &ClientInfo,
    domain_registry: &mut DomainArtifactRegistry,
    projection_index: &mut ProjectionIndexStore,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    let request: IngestArtifactRequest =
        bincode::deserialize(payload).context("failed to deserialize IngestArtifactRequest")?;

    let kind = request.kind.clone();
    let channel = request.channel.clone();
    let src_path = request.src_path.clone();
    let request_id = request.request_id;

    // V-007 Phase 6: Extract and validate capability from request message
    let cap = match validate_capability::<IngestArtifactRequest, _>(
        payload,
        STORE_RIGHT_WRITE,
        "ingest_artifact",
        || {
            bincode::serialize(&IngestArtifactReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                content_id: String::new(),
                size_bytes: 0,
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::IngestArtifact {
                    kind,
                    channel,
                    src_path,
                    content_id: None,
                },
                OperationResult::PermissionDenied,
            ));
        }
    };

    // V-007 Phase 3: Check access control (stub)
    let access_decision = access_control.can_write(client_info);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = IngestArtifactReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            content_id: String::new(),
            size_bytes: 0,
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::IngestArtifact {
                kind,
                channel,
                src_path,
                content_id: None,
            },
            OperationResult::PermissionDenied,
        ));
    }

    let result = (|| -> Result<IngestArtifactReply> {
        let src = Path::new(&src_path);
        if !src.exists() {
            return Ok(IngestArtifactReply {
                request_id,
                status: STATUS_NOT_FOUND,
                content_id: String::new(),
                size_bytes: 0,
            });
        }

        // Compute content hash
        let content_id = hash_blob(src).context("failed to hash blob")?;
        let id = ContentId::parse(&content_id)
            .map_err(|_| anyhow::anyhow!("invalid content_id format"))?;

        // Write blob
        let blob_dst = store_root.join(format!("{}.blob", id.hash_hex()));
        write_blob_atomic(&blob_dst, src).context("failed to write blob")?;

        // Get size
        let size_bytes = fs::metadata(&blob_dst)?.len();

        // Create and write manifest
        let manifest = Manifest {
            schema_version: 1,
            content_id: content_id.clone(),
            size_bytes,
            kind: kind.clone(),
            channels: vec![channel.clone()],
            signatures: vec![], // V-007 Phase 3: No signatures yet
        };

        let manifest_dst = store_root.join(format!("{}.manifest.json", id.hash_hex()));
        write_manifest_atomic(&manifest_dst, &manifest).context("failed to write manifest")?;

        // V-007 Phase 5: Register artifact ownership
        let is_global = cap.domain_id == 0; // Kernel artifacts are global
        if let Err(err) = domain_registry.register_artifact(&id, cap.domain_id, is_global) {
            eprintln!(
                "store_service: failed to register artifact ownership: {}",
                err
            );
            // Non-fatal: log error but continue
        }

        update_projection_index_after_ingest(
            projection_index,
            store_root,
            &content_id,
            &kind,
            &channel,
            src,
            cap.domain_id,
        )
        .context("failed to update projection index after ingest")?;

        Ok(IngestArtifactReply {
            request_id,
            status: STATUS_OK,
            content_id: content_id.clone(),
            size_bytes,
        })
    })();

    match result {
        Ok(reply) => {
            let status = OperationResult::from_status_code(reply.status);
            let content_id = if reply.status == STATUS_OK {
                Some(reply.content_id.clone())
            } else {
                None
            };
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::IngestArtifact {
                    kind,
                    channel,
                    src_path,
                    content_id,
                },
                status,
            ))
        }
        Err(err) => {
            eprintln!("store_service: ingest_artifact error: {}", err);
            let error_reply = IngestArtifactReply {
                request_id,
                status: STATUS_IO_ERROR,
                content_id: String::new(),
                size_bytes: 0,
            };
            let reply_bytes =
                bincode::serialize(&error_reply).context("failed to serialize error reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::IngestArtifact {
                    kind,
                    channel,
                    src_path,
                    content_id: None,
                },
                OperationResult::IoError,
            ))
        }
    }
}

fn update_projection_index_after_ingest(
    projection_index: &mut ProjectionIndexStore,
    store_root: &Path,
    content_id: &str,
    kind: &str,
    channel: &str,
    src_path: &Path,
    domain_id: u64,
) -> Result<()> {
    if !projection_index.allows_mutation(store_root) {
        eprintln!(
            "store_service: projection index mutation skipped because configured index is read-only"
        );
        return Ok(());
    }

    let (entry, projection) =
        ingest_projection_records(content_id, kind, channel, src_path, domain_id);
    projection_index
        .upsert_entry(entry)
        .context("failed to upsert projection index entry")?;
    projection_index
        .upsert_path_projection(projection)
        .context("failed to upsert path projection")?;
    projection_index
        .persist_atomic(store_root)
        .context("failed to persist projection index")?;
    Ok(())
}

fn handle_query_projection_by_path(
    payload: &[u8],
    access_control: &AccessControl,
    client_info: &ClientInfo,
    projection_index: &ProjectionIndexStore,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    use store_service::client::{QueryProjectionByPathReply, QueryProjectionByPathRequest};

    let request: QueryProjectionByPathRequest = bincode::deserialize(payload)
        .context("failed to deserialize QueryProjectionByPathRequest")?;
    let request_id = request.request_id;
    let path = request.path.clone();

    let _cap = match validate_capability::<QueryProjectionByPathRequest, _>(
        payload,
        STORE_RIGHT_READ,
        "query_projection_by_path",
        || {
            bincode::serialize(&QueryProjectionByPathReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                ..Default::default()
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByPath { path },
                OperationResult::PermissionDenied,
            ));
        }
    };

    let access_decision = access_control.can_read(client_info, &path);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = QueryProjectionByPathReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            ..Default::default()
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::QueryProjectionByPath { path },
            OperationResult::PermissionDenied,
        ));
    }

    match projection_index.query_by_path(&path) {
        Ok(content_id) => {
            let reply = QueryProjectionByPathReply {
                request_id,
                status: STATUS_OK,
                content_id,
            };
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByPath { path },
                OperationResult::Success,
            ))
        }
        Err(status) => {
            let reply = QueryProjectionByPathReply {
                request_id,
                status,
                ..Default::default()
            };
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByPath { path },
                OperationResult::from_status_code(status),
            ))
        }
    }
}

fn handle_query_projection_by_tag(
    payload: &[u8],
    access_control: &AccessControl,
    client_info: &ClientInfo,
    projection_index: &ProjectionIndexStore,
) -> Result<(Vec<u8>, AuditLogParameters, OperationResult)> {
    use store_service::client::{QueryProjectionByTagReply, QueryProjectionByTagRequest};

    let request: QueryProjectionByTagRequest = bincode::deserialize(payload)
        .context("failed to deserialize QueryProjectionByTagRequest")?;
    let request_id = request.request_id;
    let tag = request.tag.clone();

    let _cap = match validate_capability::<QueryProjectionByTagRequest, _>(
        payload,
        STORE_RIGHT_READ,
        "query_projection_by_tag",
        || {
            bincode::serialize(&QueryProjectionByTagReply {
                request_id,
                status: STATUS_PERMISSION_DENIED,
                ..Default::default()
            })
            .context("failed to serialize error reply")
            .unwrap()
        },
    ) {
        CapabilityOutcome::Valid(cap) => cap,
        CapabilityOutcome::Denied(reply_bytes) => {
            return Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByTag { tag },
                OperationResult::PermissionDenied,
            ));
        }
    };

    let access_decision = access_control.can_read(client_info, &tag);
    if access_decision != store_service::access_control::AccessDecision::Allowed {
        let error_reply = QueryProjectionByTagReply {
            request_id,
            status: STATUS_PERMISSION_DENIED,
            ..Default::default()
        };
        let reply_bytes =
            bincode::serialize(&error_reply).context("failed to serialize error reply")?;
        return Ok((
            reply_bytes,
            AuditLogParameters::QueryProjectionByTag { tag },
            OperationResult::PermissionDenied,
        ));
    }

    match projection_index.query_by_tag(&tag) {
        Ok(content_ids) => {
            let reply = QueryProjectionByTagReply {
                request_id,
                status: STATUS_OK,
                content_ids: content_ids.join(","),
            };
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByTag { tag },
                OperationResult::Success,
            ))
        }
        Err(status) => {
            let reply = QueryProjectionByTagReply {
                request_id,
                status,
                ..Default::default()
            };
            let reply_bytes = bincode::serialize(&reply).context("failed to serialize reply")?;
            Ok((
                reply_bytes,
                AuditLogParameters::QueryProjectionByTag { tag },
                OperationResult::from_status_code(status),
            ))
        }
    }
}

// Request/Reply types (re-exported from client module for binary use)
pub use store_service::client::{
    GetBlobReply, GetBlobRequest, GetManifestReply, GetManifestRequest, IngestArtifactReply,
    IngestArtifactRequest, QueryProjectionByPathReply, QueryProjectionByPathRequest,
    QueryProjectionByTagReply, QueryProjectionByTagRequest, VerifyArtifactReply,
    VerifyArtifactRequest,
};

// Helper functions
fn write_blob_atomic(dst: &Path, src: &Path) -> Result<()> {
    use std::io::Write;

    // Create parent directory
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temporary file
    let tmp_path = dst.with_extension("tmp");
    let mut tmp = fs::File::create(&tmp_path)?;
    let data = fs::read(src)?;
    tmp.write_all(&data)?;

    // Atomic rename
    fs::rename(&tmp_path, dst)?;

    Ok(())
}

fn write_manifest_atomic(dst: &Path, manifest: &Manifest) -> Result<()> {
    use std::io::Write;

    // Create parent directory
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temporary file
    let tmp_path = dst.with_extension("tmp");
    let mut tmp = fs::File::create(&tmp_path)?;
    let data = serde_json::to_vec_pretty(manifest)?;
    tmp.write_all(&data)?;

    // Atomic rename
    fs::rename(&tmp_path, dst)?;

    Ok(())
}

// ============================================================================
// Tests: Capability Verification Integration (V-007 Phase 5, Task 3)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use store_service::capability::{STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability};
    use store_service::domain_visibility::DomainArtifactRegistry;
    use tempfile::TempDir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: String) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn set_test_capability_trusted_key_env() -> EnvVarGuard {
        use base64::{Engine as _, engine::general_purpose};

        let default_key_bytes = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];
        let key_b64 = general_purpose::STANDARD.encode(default_key_bytes);
        EnvVarGuard::set("RAMEN_STORE_CAP_TRUSTED_KEYS", key_b64)
    }

    /// Helper: Create a test store with a fake artifact
    fn setup_test_store() -> (TempDir, ContentId) {
        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        // Create a fake artifact
        let content_id = ContentId::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        let blob_path = store_root.join(format!("{}.blob", content_id.hash_hex()));
        fs::write(&blob_path, b"test artifact data").unwrap();

        let manifest = artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: content_id.as_str().to_string(),
            size_bytes: 16,
            kind: "test".to_string(),
            channels: vec!["test".to_string()],
            signatures: vec![],
        };
        let manifest_path = store_root.join(format!("{}.manifest.json", content_id.hash_hex()));
        fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        (temp_dir, content_id)
    }

    fn writable_projection_index(store_root: &Path) -> ProjectionIndexStore {
        ProjectionIndexStore::load_or_empty(ProjectionIndexStore::default_path(store_root))
            .expect("load projection index")
    }

    /// Build a capability signed by the default development test key so signature
    /// verification succeeds in integration tests.
    fn signed_test_capability(
        domain_id: u64,
        rights_mask: u8,
        capability_id: u64,
    ) -> StoreCapability {
        use ed25519_dalek::{Signer, SigningKey};

        // RFC 8032 test vector private key; matches the default verifying key in capability.rs.
        let secret_key_bytes = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);

        let mut cap = StoreCapability::new(domain_id, rights_mask, capability_id);
        let signature = signing_key.sign(&cap.signing_data());
        cap.signature = signature.to_bytes().to_vec();
        cap
    }

    #[test]
    fn get_manifest_denied_without_capability() {
        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        let request = GetManifestRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: vec![],
        };
        let payload = bincode::serialize(&request).unwrap();

        let sig_config = SignatureValidationConfig::default();
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let domain_registry = DomainArtifactRegistry::new(store_root).unwrap();

        // No capability presented
        let result = handle_get_manifest(
            &payload,
            store_root,
            &sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: GetManifestReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn get_manifest_denied_with_expired_capability() {
        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        // Expired capability
        let mut expired_cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        expired_cap.expires_at = 1; // Epoch + 1 second (definitely expired)

        let request = GetManifestRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: bincode::serialize(&expired_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let sig_config = SignatureValidationConfig::default();
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let domain_registry = DomainArtifactRegistry::new(store_root).unwrap();

        let result = handle_get_manifest(
            &payload,
            store_root,
            &sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: GetManifestReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn get_manifest_denied_cross_domain_access() {
        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        // Try to access with domain 1 capability
        let wrong_domain_cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        let request = GetManifestRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: bincode::serialize(&wrong_domain_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let sig_config = SignatureValidationConfig::default();
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();

        // Register artifact as owned by domain 5
        domain_registry
            .register_artifact(&content_id, 5, false)
            .unwrap();

        let result = handle_get_manifest(
            &payload,
            store_root,
            &sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: GetManifestReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn get_blob_denied_without_read_right() {
        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        // Capability with only WRITE right (no READ)
        let write_only_cap = StoreCapability::new(1, STORE_RIGHT_WRITE, 100);
        let request = GetBlobRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: bincode::serialize(&write_only_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let _sig_config = SignatureValidationConfig::default();
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let domain_registry = DomainArtifactRegistry::new(store_root).unwrap();

        let result = handle_get_blob(
            &payload,
            store_root,
            &_sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: GetBlobReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn verify_artifact_denied_without_capability() {
        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        let request = VerifyArtifactRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: vec![],
        };
        let payload = bincode::serialize(&request).unwrap();

        let _sig_config = SignatureValidationConfig::default();
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let domain_registry = DomainArtifactRegistry::new(store_root).unwrap();

        // No capability
        let result = handle_verify_artifact(
            &payload,
            store_root,
            &_sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: VerifyArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn get_blob_validation_failed_when_signature_required_and_manifest_unsigned() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let _cap_key_guard = set_test_capability_trusted_key_env();

        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        let read_cap = signed_test_capability(1, STORE_RIGHT_READ, 101);
        let request = GetBlobRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: bincode::serialize(&read_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let sig_config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            trusted_keys: TrustedKeys::new(),
            ..Default::default()
        };
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        domain_registry
            .register_artifact(&content_id, 1, false)
            .unwrap();

        let result = handle_get_blob(
            &payload,
            store_root,
            &sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: GetBlobReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_VALIDATION_FAILED);
        assert_eq!(result_status, OperationResult::ValidationFailed);
        assert_eq!(reply.blob_path, "");
    }

    #[test]
    fn verify_artifact_validation_failed_when_signature_required_and_manifest_unsigned() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let _cap_key_guard = set_test_capability_trusted_key_env();

        let (_temp_dir, content_id) = setup_test_store();
        let store_root = _temp_dir.path();

        let read_cap = signed_test_capability(1, STORE_RIGHT_READ, 102);
        let request = VerifyArtifactRequest {
            request_id: 1,
            content_id: content_id.as_str().to_string(),
            capability_bytes: bincode::serialize(&read_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let sig_config = SignatureValidationConfig {
            policy: SignaturePolicy::RequireSignature,
            trusted_keys: TrustedKeys::new(),
            ..Default::default()
        };
        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        domain_registry
            .register_artifact(&content_id, 1, false)
            .unwrap();

        let result = handle_verify_artifact(
            &payload,
            store_root,
            &sig_config,
            &access_control,
            &client_info,
            &domain_registry,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: VerifyArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_VALIDATION_FAILED);
        assert_eq!(result_status, OperationResult::ValidationFailed);
        assert_eq!(reply.valid, 0);
    }

    #[test]
    fn ingest_artifact_denied_without_write_right() {
        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        // Create a source file to ingest
        let src_file = temp_dir.path().join("source.txt");
        fs::write(&src_file, b"test artifact").unwrap();

        // Capability with only READ right (no WRITE)
        let read_only_cap = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        let request = IngestArtifactRequest {
            request_id: 1,
            kind: "test".to_string(),
            channel: "test".to_string(),
            src_path: src_file.to_string_lossy().to_string(),
            capability_bytes: bincode::serialize(&read_only_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        let mut projection_index = writable_projection_index(store_root);

        let result = handle_ingest_artifact(
            &payload,
            store_root,
            &access_control,
            &client_info,
            &mut domain_registry,
            &mut projection_index,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_PERMISSION_DENIED);
        assert_eq!(result_status, OperationResult::PermissionDenied);
    }

    #[test]
    fn ingest_artifact_registers_ownership_correctly() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let _cap_key_guard = set_test_capability_trusted_key_env();

        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        // Create a source file to ingest
        let src_file = temp_dir.path().join("source.txt");
        fs::write(&src_file, b"test artifact").unwrap();

        // Capability with WRITE right for domain 5
        let write_cap = signed_test_capability(5, STORE_RIGHT_WRITE, 100);
        let request = IngestArtifactRequest {
            request_id: 1,
            kind: "test".to_string(),
            channel: "test".to_string(),
            src_path: src_file.to_string_lossy().to_string(),
            capability_bytes: bincode::serialize(&write_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        let mut projection_index = writable_projection_index(store_root);

        let result = handle_ingest_artifact(
            &payload,
            store_root,
            &access_control,
            &client_info,
            &mut domain_registry,
            &mut projection_index,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_OK);
        assert_eq!(result_status, OperationResult::Success);

        // Verify ownership was registered
        let content_id = ContentId::parse(&reply.content_id).unwrap();
        assert!(domain_registry.can_access(&content_id, 5));
        assert!(!domain_registry.can_access(&content_id, 1)); // Other domains can't access

        let owner = domain_registry.get_owner(&content_id).unwrap();
        assert_eq!(owner.domain_id, 5);
        assert!(!owner.is_global); // Not kernel domain
    }

    #[test]
    fn ingest_artifact_updates_projection_index() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let _cap_key_guard = set_test_capability_trusted_key_env();

        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        let src_file = temp_dir.path().join("source.txt");
        fs::write(&src_file, b"test artifact").unwrap();

        let write_cap = signed_test_capability(5, STORE_RIGHT_WRITE, 101);
        let request = IngestArtifactRequest {
            request_id: 1,
            kind: "test_kind".to_string(),
            channel: "beta".to_string(),
            src_path: src_file.to_string_lossy().to_string(),
            capability_bytes: bincode::serialize(&write_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        let mut projection_index = writable_projection_index(store_root);

        let result = handle_ingest_artifact(
            &payload,
            store_root,
            &access_control,
            &client_info,
            &mut domain_registry,
            &mut projection_index,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_OK);
        assert_eq!(result_status, OperationResult::Success);
        assert_eq!(
            projection_index
                .query_by_path("/store/test_kind/beta/source.txt")
                .expect("query path"),
            reply.content_id
        );
        assert_eq!(
            projection_index
                .query_by_tag("test_kind")
                .expect("query kind tag"),
            vec![reply.content_id.clone()]
        );
        assert_eq!(
            projection_index
                .query_by_tag("beta")
                .expect("query channel tag"),
            vec![reply.content_id.clone()]
        );

        let reloaded =
            ProjectionIndexStore::load_or_empty(ProjectionIndexStore::default_path(store_root))
                .expect("reload projection index");
        assert_eq!(
            reloaded
                .query_by_path("/store/test_kind/beta/source.txt")
                .expect("reloaded query path"),
            reply.content_id
        );
    }

    #[test]
    fn kernel_artifacts_marked_as_global() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let _cap_key_guard = set_test_capability_trusted_key_env();

        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path();

        // Create a source file to ingest
        let src_file = temp_dir.path().join("source.txt");
        fs::write(&src_file, b"test artifact").unwrap();

        // Kernel capability (domain 0)
        let kernel_cap = signed_test_capability(0, STORE_RIGHT_WRITE, 0);
        let request = IngestArtifactRequest {
            request_id: 1,
            kind: "kernel".to_string(),
            channel: "stable".to_string(),
            src_path: src_file.to_string_lossy().to_string(),
            capability_bytes: bincode::serialize(&kernel_cap).unwrap(),
        };
        let payload = bincode::serialize(&request).unwrap();

        let access_control = AccessControl::new();
        let client_info = ClientInfo {
            pid: Some(1234),
            uid: Some(0),
            gid: Some(0),
            domain_id: Some(1),
            rights: store_service::access_control::AccessRights::read_write(),
            exe_path: None,
            cmdline: None,
        };
        let mut domain_registry = DomainArtifactRegistry::new(store_root).unwrap();
        let mut projection_index = writable_projection_index(store_root);

        let result = handle_ingest_artifact(
            &payload,
            store_root,
            &access_control,
            &client_info,
            &mut domain_registry,
            &mut projection_index,
        );

        assert!(result.is_ok());
        let (reply_bytes, _params, result_status) = result.unwrap();
        let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes).unwrap();

        assert_eq!(reply.status, STATUS_OK);
        assert_eq!(result_status, OperationResult::Success);

        // Verify ownership was registered as global
        let content_id = ContentId::parse(&reply.content_id).unwrap();
        assert!(domain_registry.can_access(&content_id, 0)); // Kernel can access
        assert!(domain_registry.can_access(&content_id, 1)); // Anyone can access (global)
        assert!(domain_registry.can_access(&content_id, 99)); // Anyone can access (global)

        let owner = domain_registry.get_owner(&content_id).unwrap();
        assert_eq!(owner.domain_id, 0);
        assert!(owner.is_global); // Should be marked as global
    }

    // V-007 Phase 6: Capability Extraction Tests
    // ============================================================================

    #[test]
    fn extract_and_validate_capability_with_valid_capability() {
        let capability = StoreCapability::new(1, STORE_RIGHT_READ, 100);
        let capability_bytes = bincode::serialize(&capability).unwrap();

        let request = GetManifestRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes,
        };
        let payload = bincode::serialize(&request).unwrap();

        let result = extract_and_validate_capability::<GetManifestRequest>(&payload);

        // Note: This will fail because the capability is not signed
        // In a real scenario, the capability would be signed by a trusted authority
        assert!(result.is_err() || result.unwrap().is_none());
    }

    #[test]
    fn extract_and_validate_capability_with_empty_capability() {
        let request = GetManifestRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![],
        };
        let payload = bincode::serialize(&request).unwrap();

        let result = extract_and_validate_capability::<GetManifestRequest>(&payload);

        // Empty capability should return Ok(None)
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn extract_and_validate_capability_with_malformed_capability() {
        let request = GetManifestRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![0x01, 0x02, 0x03], // Invalid capability bytes
        };
        let payload = bincode::serialize(&request).unwrap();

        let result = extract_and_validate_capability::<GetManifestRequest>(&payload);

        // Malformed capability should return an error
        assert!(result.is_err());
    }

    #[test]
    fn get_manifest_request_with_capability_bytes_field() {
        let request = GetManifestRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![],
        };

        // Verify the capability_bytes field exists and is accessible
        assert_eq!(request.capability_bytes, Vec::<u8>::new());
    }

    #[test]
    fn get_blob_request_with_capability_bytes_field() {
        let request = GetBlobRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![],
        };

        // Verify the capability_bytes field exists and is accessible
        assert_eq!(request.capability_bytes, Vec::<u8>::new());
    }

    #[test]
    fn verify_artifact_request_with_capability_bytes_field() {
        let request = VerifyArtifactRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![],
        };

        // Verify the capability_bytes field exists and is accessible
        assert_eq!(request.capability_bytes, Vec::<u8>::new());
    }

    #[test]
    fn ingest_artifact_request_with_capability_bytes_field() {
        let request = IngestArtifactRequest {
            request_id: 1,
            kind: "test".to_string(),
            channel: "test".to_string(),
            src_path: "/tmp/test".to_string(),
            capability_bytes: vec![],
        };

        // Verify the capability_bytes field exists and is accessible
        assert_eq!(request.capability_bytes, Vec::<u8>::new());
    }

    #[test]
    fn request_with_capability_trait_implementation() {
        let request = GetManifestRequest {
            request_id: 1,
            content_id: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            capability_bytes: vec![0x01, 0x02, 0x03],
        };

        // Verify the RequestWithCapability trait is implemented
        let capability_bytes = request.capability_bytes();
        assert_eq!(capability_bytes, vec![0x01, 0x02, 0x03]);
    }
}
