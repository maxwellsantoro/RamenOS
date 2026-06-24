//! Capability broker for S10.1 Native Runner Production Integration.
//!
//! The broker is the policy decision point that evaluates manifests against
//! policy and grants capability handles transactionally.
//!
//! Key design principles:
//! - **Broker owns policy:** Runner is executor-only, never decides policy
//! - **Transactional grants:** All-or-nothing semantics with kernel-side revocation on failure
//! - **Fail-closed:** Unknown interfaces denied, not ignored
//! - **Channel trust model:** Broker derives policy from manifest's declared channels
//! - **Full audit trail:** All decisions logged with complete context

use std::collections::{HashMap, HashSet};

use crate::audit::{AuditReasonCode, BrokerAuditEvent, SharedAuditSink, stderr_sink};

use artifact_store_schema::native_wasm::{NativeWasmManifestV0, RequiredCapability};

#[cfg(test)]
use artifact_store_schema::native_wasm::NativeWasmV0;

// ============================================================================
// Error Types
// ============================================================================

/// Errors returned by the capability broker.
#[derive(Debug)]
#[allow(dead_code)] // Variants are part of public API, used in Task 5+ and future implementations
pub enum BrokerError {
    /// Manifest failed validation
    ManifestInvalid(String),
    /// Interface is not in the known interface registry
    InterfaceUnknown { interface: String },
    /// Capability was denied by policy
    CapabilityDenied {
        export_name: String,
        reason: DenialReason,
    },
    /// Kernel failed to grant capability handle
    KernelGrantFailed { interface: String, status: u32 },
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestInvalid(msg) => write!(f, "manifest invalid: {}", msg),
            Self::InterfaceUnknown { interface } => {
                write!(f, "interface unknown: {}", interface)
            }
            Self::CapabilityDenied {
                export_name,
                reason,
            } => {
                write!(f, "capability denied for {}: {}", export_name, reason)
            }
            Self::KernelGrantFailed { interface, status } => {
                write!(
                    f,
                    "kernel grant failed for {}: status {}",
                    interface, status
                )
            }
        }
    }
}

impl std::error::Error for BrokerError {}

/// Reasons why a capability was denied by policy.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants are part of public API, used in future policy implementations
pub enum DenialReason {
    /// Interface is not allowed for the artifact's channel
    InterfaceNotAllowedForChannel { channel: String },
    /// Requested rights exceed maximum allowed
    RightsExceedMaximum { requested: u64, maximum: u64 },
    /// Explicit deny in policy
    PolicyExplicitDeny,
}

impl std::fmt::Display for DenialReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InterfaceNotAllowedForChannel { channel } => {
                write!(f, "interface not allowed for channel: {}", channel)
            }
            Self::RightsExceedMaximum { requested, maximum } => {
                write!(f, "rights {} exceed maximum {}", requested, maximum)
            }
            Self::PolicyExplicitDeny => write!(f, "explicit policy deny"),
        }
    }
}

// ============================================================================
// Policy Engine Trait
// ============================================================================

/// Trait for policy evaluation engines.
///
/// Implementations define what capabilities are allowed for a given
/// channel and interface combination.
pub trait PolicyEngine {
    /// Evaluate whether a capability should be granted.
    ///
    /// Returns `Ok(())` if the capability is allowed, `Err(DenialReason)` if denied.
    fn evaluate(
        &self,
        artifact_channels: &[String],
        cap: &RequiredCapability,
    ) -> Result<(), DenialReason>;
}

// ============================================================================
// Channel Allowlist Policy (S10.1)
// ============================================================================

/// Per-channel interface allowlist policy for S10.1.
///
/// This policy uses a simple allowlist per channel to determine which
/// interfaces are permitted. Future versions will use policy files.
pub struct ChannelAllowlistPolicy {
    /// Map from channel name to set of allowed interfaces
    allowlist: HashMap<String, HashSet<String>>,
    /// Maximum rights allowed per interface (for simplicity, same for all)
    max_rights: u64,
}

impl ChannelAllowlistPolicy {
    /// Create a new policy with the given allowlist.
    #[allow(dead_code)] // Used in future implementations with policy files
    pub fn new(allowlist: HashMap<String, HashSet<String>>, max_rights: u64) -> Self {
        Self {
            allowlist,
            max_rights,
        }
    }

    /// Create a test policy with known interfaces.
    ///
    /// Test policy configuration:
    /// - Experimental channel: allows `harness.echo_v0`, `harness.echo_v1`,
    ///   `harness.trace_v0`, `harness.trace_v1`
    /// - Stable channel: no harness capabilities
    pub fn new_test() -> Self {
        let mut experimental = HashSet::new();
        experimental.insert("harness.echo_v0".to_string());
        experimental.insert("harness.echo_v1".to_string());
        experimental.insert("harness.trace_v0".to_string());
        experimental.insert("harness.trace_v1".to_string());

        let mut allowlist = HashMap::new();
        allowlist.insert("Experimental".to_string(), experimental);
        allowlist.insert("Stable".to_string(), HashSet::new());

        Self {
            allowlist,
            max_rights: 0xFFFF,
        }
    }

    /// Create the narrow S10.5.1 semantic harness policy.
    #[allow(dead_code)] // Used by semantic harness tests and future bridge entrypoints.
    pub fn new_semantic_harness() -> Self {
        let mut semantic = HashSet::new();
        semantic.insert("shared_memory.control_v1".to_string());
        semantic.insert("services.semantic_state_v1".to_string());

        let mut allowlist = HashMap::new();
        allowlist.insert("SemanticHarness".to_string(), semantic);

        Self {
            allowlist,
            max_rights: 0xFFFF,
        }
    }
}

impl PolicyEngine for ChannelAllowlistPolicy {
    fn evaluate(
        &self,
        artifact_channels: &[String],
        cap: &RequiredCapability,
    ) -> Result<(), DenialReason> {
        // Check rights don't exceed maximum
        if cap.rights > self.max_rights {
            return Err(DenialReason::RightsExceedMaximum {
                requested: cap.rights,
                maximum: self.max_rights,
            });
        }

        // Check if interface is allowed for any of the artifact's channels
        for channel in artifact_channels {
            if let Some(allowed) = self.allowlist.get(channel) {
                if allowed.contains(&cap.interface) {
                    return Ok(());
                }
            }
        }

        // No channel allowed this interface
        let channel = artifact_channels.first().cloned().unwrap_or_default();
        Err(DenialReason::InterfaceNotAllowedForChannel { channel })
    }
}

// ============================================================================
// Kernel Grant Operations Trait
// ============================================================================

/// Trait for kernel capability grant/revoke operations.
///
/// This abstraction allows the broker to be tested with simulated or
/// intentionally-failing kernel implementations while keeping the
/// transaction orchestration logic pure.
///
/// S10.1: Used for torture testing rollback behavior.
/// S10.2+: Will connect to real kernel IPC.
pub trait KernelGrantOps {
    /// Grant a capability handle for the given interface.
    ///
    /// Returns the granted handle on success, or a kernel status code on failure.
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32>;

    /// Revoke a previously granted capability handle.
    ///
    /// Returns Ok(()) on success, or a kernel status code on failure.
    fn revoke(&mut self, handle: u64) -> Result<(), u32>;
}

pub const SEMANTIC_HARNESS_SHMEM_INTERFACE: &str = "shared_memory.control_v1";
pub const SEMANTIC_HARNESS_STATE_INTERFACE: &str = "services.semantic_state_v1";
pub const SEMANTIC_HARNESS_EXPORT_SHMEM: &str = "RAMEN_CAP_SHMEM_CONTROL";
pub const SEMANTIC_HARNESS_EXPORT_STATE: &str = "RAMEN_CAP_SEMANTIC_STATE";
pub const SEMANTIC_HARNESS_SHMEM_EXPORT_ID: u16 = 1;
pub const SEMANTIC_HARNESS_STATE_EXPORT_ID: u16 = 2;
const STATUS_INVALID_INTERFACE: u32 = 3;
const SEMANTIC_HANDLE_DOMAIN_SHIFT: u64 = 16;
const SEMANTIC_HANDLE_DOMAIN_MASK: u64 = 0x0000_0000_ffff_0000;
const SEMANTIC_HANDLE_MAX_DOMAIN_ID: u64 =
    SEMANTIC_HANDLE_DOMAIN_MASK >> SEMANTIC_HANDLE_DOMAIN_SHIFT;
const SEMANTIC_HANDLE_SHMEM_KIND: u64 = 0x5308_0000_0000_0000;
const SEMANTIC_HANDLE_STATE_KIND: u64 = 0x5310_0000_0000_0000;

/// S10.5.1 host-side grant ops for the semantic harness bridge.
///
/// This is intentionally narrower than `SimulatedKernelOps`: it grants only
/// `shared_memory.control_v1` and `services.semantic_state_v1`, records the
/// handle binding, and fails closed for every other interface.
#[derive(Debug, Default)]
pub struct SemanticHarnessGrantOps {
    registry: HashMap<u64, SemanticHarnessGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticHarnessGrant {
    pub interface: String,
    pub rights: u64,
    pub domain_id: u64,
}

impl SemanticHarnessGrantOps {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)] // Exposed for grant-audit tests and bridge diagnostics.
    pub fn registry(&self) -> &HashMap<u64, SemanticHarnessGrant> {
        &self.registry
    }

    pub fn deterministic_handle(interface: &str, domain_id: u64) -> Option<u64> {
        if domain_id > SEMANTIC_HANDLE_MAX_DOMAIN_ID {
            return None;
        }
        let domain_bits = (domain_id << SEMANTIC_HANDLE_DOMAIN_SHIFT) & SEMANTIC_HANDLE_DOMAIN_MASK;
        match interface {
            SEMANTIC_HARNESS_SHMEM_INTERFACE => Some(
                SEMANTIC_HANDLE_SHMEM_KIND
                    | domain_bits
                    | u64::from(SEMANTIC_HARNESS_SHMEM_EXPORT_ID),
            ),
            SEMANTIC_HARNESS_STATE_INTERFACE => Some(
                SEMANTIC_HANDLE_STATE_KIND
                    | domain_bits
                    | u64::from(SEMANTIC_HARNESS_STATE_EXPORT_ID),
            ),
            _ => None,
        }
    }
}

impl KernelGrantOps for SemanticHarnessGrantOps {
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32> {
        let Some(handle) = Self::deterministic_handle(interface, domain_id) else {
            return Err(STATUS_INVALID_INTERFACE);
        };
        self.registry.insert(
            handle,
            SemanticHarnessGrant {
                interface: interface.to_string(),
                rights,
                domain_id,
            },
        );
        Ok(handle)
    }

    fn revoke(&mut self, handle: u64) -> Result<(), u32> {
        if self.registry.remove(&handle).is_some() {
            Ok(())
        } else {
            Err(STATUS_INVALID_INTERFACE)
        }
    }
}

/// Selects kernel grant backend from environment (S10.5.1 semantic harness profile).
#[derive(Debug)]
pub enum KernelOpsBackend {
    Simulated(SimulatedKernelOps),
    SemanticHarness(SemanticHarnessGrantOps),
}

impl KernelOpsBackend {
    pub fn from_env() -> Self {
        match std::env::var("RAMEN_SEMANTIC_HARNESS_BRIDGE")
            .ok()
            .as_deref()
        {
            Some("1") | Some("true") | Some("yes") | Some("on") => {
                Self::SemanticHarness(SemanticHarnessGrantOps::new())
            }
            _ => Self::Simulated(SimulatedKernelOps::new()),
        }
    }
}

impl KernelGrantOps for KernelOpsBackend {
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32> {
        match self {
            Self::Simulated(ops) => ops.grant(interface, rights, domain_id),
            Self::SemanticHarness(ops) => ops.grant(interface, rights, domain_id),
        }
    }

    fn revoke(&mut self, handle: u64) -> Result<(), u32> {
        match self {
            Self::Simulated(ops) => ops.revoke(handle),
            Self::SemanticHarness(ops) => ops.revoke(handle),
        }
    }
}

// ============================================================================
// Simulated Kernel Grant Operations (for S10.1 main.rs)
// ============================================================================

/// Simulated kernel operations for S10.1.
///
/// This is a **non-production** implementation used by `main.rs` until
/// S10.2+ connects to real kernel IPC via Unix domain sockets.
///
/// **DO NOT USE IN PRODUCTION** - This always succeeds and doesn't actually
/// communicate with the kernel. Use `KernelIpcOps` (S10.2+) for real grants.
///
/// The real IPC implementation will live in a separate module and implement
/// the same `KernelGrantOps` trait.
#[derive(Debug)]
pub struct SimulatedKernelOps {
    next_handle: u64,
}

impl SimulatedKernelOps {
    /// Create a new simulated kernel ops instance.
    pub fn new() -> Self {
        Self {
            next_handle: 0x1000,
        }
    }
}

impl Default for SimulatedKernelOps {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelGrantOps for SimulatedKernelOps {
    fn grant(&mut self, _interface: &str, _rights: u64, _domain_id: u64) -> Result<u64, u32> {
        // S10.1: Simulated grant - S10.2+ will use real kernel IPC
        let handle = self.next_handle;
        self.next_handle += 1;
        Ok(handle)
    }

    fn revoke(&mut self, _handle: u64) -> Result<(), u32> {
        // S10.1: Simulated revoke - S10.2+ will use real kernel IPC
        Ok(())
    }
}

// ============================================================================
// Simulated Kernel Grant Operations (for tests)
// ============================================================================

/// Simulated kernel that grants handles sequentially and tracks all calls.
///
/// Used for testing broker transaction behavior without real kernel IPC.
#[cfg(test)]
pub struct SimKernelGrantOps {
    next_handle: u64,
    /// Record of all grant calls: (interface, rights, domain_id, result)
    pub grant_calls: Vec<(String, u64, u64, Result<u64, u32>)>,
    /// Record of all revoke calls: (handle, result)
    pub revoke_calls: Vec<(u64, Result<(), u32>)>,
}

#[cfg(test)]
impl SimKernelGrantOps {
    pub fn new() -> Self {
        Self {
            next_handle: 0x1000,
            grant_calls: Vec::new(),
            revoke_calls: Vec::new(),
        }
    }
}

#[cfg(test)]
impl Default for SimKernelGrantOps {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl KernelGrantOps for SimKernelGrantOps {
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32> {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.grant_calls
            .push((interface.to_string(), rights, domain_id, Ok(handle)));
        Ok(handle)
    }

    fn revoke(&mut self, handle: u64) -> Result<(), u32> {
        self.revoke_calls.push((handle, Ok(())));
        Ok(())
    }
}

// ============================================================================
// Failing Kernel Grant Operations (for torture tests)
// ============================================================================

/// Kernel that fails on the Nth grant call.
///
/// Used to test transactional rollback: if the broker requests N grants
/// and this kernel fails on the Mth, the broker must revoke all M-1
/// previously granted handles.
#[cfg(test)]
pub struct FailingKernelGrantOps {
    /// Grant call number that will fail (1-indexed)
    fail_on_grant: usize,
    /// Current grant call count
    grant_count: usize,
    next_handle: u64,
    /// Record of all grant calls
    pub grant_calls: Vec<(String, u64, u64, Result<u64, u32>)>,
    /// Record of all revoke calls
    pub revoke_calls: Vec<(u64, Result<(), u32>)>,
}

#[cfg(test)]
impl FailingKernelGrantOps {
    /// Create a kernel that will fail on the Nth grant call.
    pub fn fail_on_grant(n: usize) -> Self {
        Self {
            fail_on_grant: n,
            grant_count: 0,
            next_handle: 0x1000,
            grant_calls: Vec::new(),
            revoke_calls: Vec::new(),
        }
    }

    pub fn revoke_calls_len(&self) -> usize {
        self.revoke_calls.len()
    }

    /// Get the handles that were successfully granted before failure.
    pub fn granted_handles(&self) -> Vec<u64> {
        self.grant_calls
            .iter()
            .filter_map(|(_, _, _, result)| result.ok())
            .collect()
    }
}

#[cfg(test)]
impl KernelGrantOps for FailingKernelGrantOps {
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32> {
        self.grant_count += 1;

        if self.grant_count == self.fail_on_grant {
            // Fail with a deterministic status code
            const STATUS_KERNEL_ERROR: u32 = 0x1001;
            self.grant_calls.push((
                interface.to_string(),
                rights,
                domain_id,
                Err(STATUS_KERNEL_ERROR),
            ));
            Err(STATUS_KERNEL_ERROR)
        } else {
            let handle = self.next_handle;
            self.next_handle += 1;
            self.grant_calls
                .push((interface.to_string(), rights, domain_id, Ok(handle)));
            Ok(handle)
        }
    }

    fn revoke(&mut self, handle: u64) -> Result<(), u32> {
        self.revoke_calls.push((handle, Ok(())));
        Ok(())
    }
}

// ============================================================================
// Revoke-Failing Kernel Grant Operations (for audit tests)
// ============================================================================

/// Kernel that grants successfully but fails revoke operations.
///
/// Used to test that failed revokes are properly audited and counted.
#[cfg(test)]
pub struct RevokeFailingKernelOps {
    next_handle: u64,
    /// Which handle should fail to revoke
    fail_revoke_handle: u64,
    /// Record of all grant calls
    pub grant_calls: Vec<(String, u64, u64, Result<u64, u32>)>,
    /// Record of all revoke calls
    pub revoke_calls: Vec<(u64, Result<(), u32>)>,
}

#[cfg(test)]
impl RevokeFailingKernelOps {
    /// Create a kernel that will fail when revoking the given handle.
    pub fn fail_on_revoke(handle: u64) -> Self {
        Self {
            next_handle: 0x1000,
            fail_revoke_handle: handle,
            grant_calls: Vec::new(),
            revoke_calls: Vec::new(),
        }
    }
}

#[cfg(test)]
impl KernelGrantOps for RevokeFailingKernelOps {
    fn grant(&mut self, interface: &str, rights: u64, domain_id: u64) -> Result<u64, u32> {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.grant_calls
            .push((interface.to_string(), rights, domain_id, Ok(handle)));
        Ok(handle)
    }

    fn revoke(&mut self, handle: u64) -> Result<(), u32> {
        const STATUS_REVOKE_FAILED: u32 = 0x2001;
        let result = if handle == self.fail_revoke_handle {
            Err(STATUS_REVOKE_FAILED)
        } else {
            Ok(())
        };
        self.revoke_calls.push((handle, result));
        result
    }
}

// ============================================================================
// Interface Registry
// ============================================================================

/// Registry of known interfaces that can be granted.
///
/// This is a simple in-memory registry for S10.1. Future versions
/// may load this from IDL files or configuration.
pub struct InterfaceRegistry {
    known: HashSet<String>,
}

impl InterfaceRegistry {
    /// Create a registry with default known interfaces.
    pub fn new() -> Self {
        let mut known = HashSet::new();
        // S10.1: Known harness interfaces
        known.insert("harness.echo_v0".to_string());
        known.insert("harness.echo_v1".to_string());
        known.insert("harness.trace_v0".to_string());
        known.insert("harness.trace_v1".to_string());
        known.insert("harness.trace_v2".to_string());
        known.insert(SEMANTIC_HARNESS_SHMEM_INTERFACE.to_string());
        known.insert(SEMANTIC_HARNESS_STATE_INTERFACE.to_string());
        Self { known }
    }

    /// Check if an interface is known.
    pub fn is_known(&self, interface: &str) -> bool {
        self.known.contains(interface)
    }
}

impl Default for InterfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Grant Result
// ============================================================================

/// Result of a successful capability grant.
#[derive(Debug)]
pub struct GrantResult {
    /// Map from export name to granted handle
    pub granted_handles: HashMap<String, u64>,
}

// ============================================================================
// Capability Broker (Test-Focused Implementation)
// ============================================================================

/// Capability broker that evaluates policy and grants capability handles.
///
/// The broker orchestrates capability grants transactionally:
/// - Evaluates each capability against policy
/// - Requests kernel grants via the `KernelGrantOps` trait
/// - On any failure, revokes all previously granted handles
/// - Emits structured audit events for all decisions
///
/// This design separates policy decisions (broker) from kernel enforcement
/// (KernelGrantOps), enabling both test doubles and real IPC implementations.
pub struct CapabilityBroker<P: PolicyEngine, K: KernelGrantOps> {
    policy: P,
    registry: InterfaceRegistry,
    kernel_ops: K,
    /// Active grants: domain_id -> (export_name, handle)
    active_grants: HashMap<u64, HashMap<String, u64>>,
    /// Count of revokes that failed during rollback (for audit/diagnostics)
    failed_revokes: u32,
    /// Audit sink for structured event emission
    audit: SharedAuditSink,
    /// Current grant context (content_id, channel) for audit events
    /// Audit context for current grant operation (content_id, channel).
    #[allow(dead_code)] // Wired when store-side manifest refs flow through broker grants
    audit_context: Option<(String, String)>,
}

impl<P: PolicyEngine, K: KernelGrantOps> CapabilityBroker<P, K> {
    /// Create a new broker with the given policy engine and kernel ops.
    ///
    /// Uses `StdErrAuditSink` by default. Use `new_with_audit` for a custom sink.
    pub fn new(policy: P, kernel_ops: K) -> Self {
        Self {
            policy,
            registry: InterfaceRegistry::new(),
            kernel_ops,
            active_grants: HashMap::new(),
            failed_revokes: 0,
            audit: stderr_sink(),
            audit_context: None,
        }
    }

    /// Create a new broker with a custom audit sink.
    ///
    /// Use this for testing with `VecAuditSink` or for future
    /// integration with trace ring buffers.
    /// Create a broker with a custom audit sink.
    #[allow(dead_code)] // Used by broker audit tests in this module
    pub fn new_with_audit(policy: P, kernel_ops: K, audit: SharedAuditSink) -> Self {
        Self {
            policy,
            registry: InterfaceRegistry::new(),
            kernel_ops,
            active_grants: HashMap::new(),
            failed_revokes: 0,
            audit,
            audit_context: None,
        }
    }

    /// Set the audit context for the current grant operation.
    ///
    /// This should be called before `grant_capabilities` to provide
    /// content_id and channel for audit events.
    #[allow(dead_code)] // Used when grant paths attach manifest metadata to audit events
    pub fn set_audit_context(&mut self, content_id: &str, channel: &str) {
        self.audit_context = Some((content_id.to_string(), channel.to_string()));
    }

    /// Clear the audit context after a grant operation completes.
    #[allow(dead_code)]
    pub fn clear_audit_context(&mut self) {
        self.audit_context = None;
    }

    /// Grant capabilities for a domain based on the manifest.
    ///
    /// This is a **transactional** operation:
    /// - All capabilities must be valid and pass policy
    /// - If any capability fails, all previously granted handles are revoked
    /// - Returns error with no partial capability leaks
    ///
    /// # Arguments
    /// * `manifest` - The native WASM manifest
    /// * `channel` - The channel to use for policy evaluation (derived from manifest)
    /// * `domain_id` - The domain to grant capabilities to
    pub fn grant_capabilities(
        &mut self,
        manifest: &NativeWasmManifestV0,
        channel: &str,
        domain_id: u64,
    ) -> Result<GrantResult, BrokerError> {
        let caps = &manifest.native_wasm.required_capabilities;
        let channels = vec![channel.to_string()];
        let content_id = &manifest.manifest.content_id;

        // Early return for capless manifests
        if caps.is_empty() {
            return Ok(GrantResult {
                granted_handles: HashMap::new(),
            });
        }

        let mut granted = HashMap::new();

        for cap in caps {
            // Step 1: Check interface is known
            if !self.registry.is_known(&cap.interface) {
                // Emit audit event for interface unknown
                self.audit.emit(&BrokerAuditEvent::denied(
                    domain_id,
                    content_id,
                    channel,
                    Some(&cap.export_name),
                    Some(&cap.interface),
                    None,
                    None,
                    AuditReasonCode::InterfaceUnknown,
                    &format!("interface '{}' is not registered", cap.interface),
                ));
                // Revoke any already-granted handles
                self.revoke_granted_handles(domain_id, &granted);
                return Err(BrokerError::InterfaceUnknown {
                    interface: cap.interface.clone(),
                });
            }

            // Step 2: Evaluate policy
            if let Err(reason) = self.policy.evaluate(&channels, cap) {
                // Map denial reason to audit reason code
                let reason_code = match &reason {
                    DenialReason::InterfaceNotAllowedForChannel { .. } => {
                        AuditReasonCode::ChannelNotAllowed
                    }
                    DenialReason::RightsExceedMaximum { .. } => AuditReasonCode::RightsExceeded,
                    DenialReason::PolicyExplicitDeny => AuditReasonCode::ChannelNotAllowed,
                };
                // Emit audit event for policy denial
                self.audit.emit(&BrokerAuditEvent::denied(
                    domain_id,
                    content_id,
                    channel,
                    Some(&cap.export_name),
                    Some(&cap.interface),
                    Some(cap.rights),
                    None,
                    reason_code,
                    &reason.to_string(),
                ));
                // Revoke any already-granted handles
                self.revoke_granted_handles(domain_id, &granted);
                return Err(BrokerError::CapabilityDenied {
                    export_name: cap.export_name.clone(),
                    reason,
                });
            }

            // Step 3: Grant the capability via kernel ops
            let handle = match self.kernel_ops.grant(&cap.interface, cap.rights, domain_id) {
                Ok(h) => h,
                Err(status) => {
                    // Emit audit event for kernel grant failure
                    self.audit.emit(&BrokerAuditEvent::kernel_grant_failed(
                        domain_id,
                        content_id,
                        channel,
                        &cap.interface,
                        status,
                    ));
                    // Kernel grant failed - revoke all previously granted and return error
                    self.revoke_granted_handles(domain_id, &granted);
                    return Err(BrokerError::KernelGrantFailed {
                        interface: cap.interface.clone(),
                        status,
                    });
                }
            };
            granted.insert(cap.export_name.clone(), handle);
        }

        // Store active grants
        self.active_grants.insert(domain_id, granted.clone());

        Ok(GrantResult {
            granted_handles: granted,
        })
    }

    /// Revoke all capabilities for a domain.
    ///
    /// Returns the number of handles revoked.
    pub fn revoke_domain(&mut self, domain_id: u64) -> u32 {
        if let Some(grants) = self.active_grants.remove(&domain_id) {
            let count = grants.len() as u32;
            // Revoke each handle via kernel ops
            for (_export_name, handle) in grants {
                // Best-effort revocation; log but don't fail on errors
                let _ = self.kernel_ops.revoke(handle);
            }
            count
        } else {
            0
        }
    }

    /// Get the active grants for a domain (for testing).
    pub fn active_grants_for_domain(&self, domain_id: u64) -> HashMap<String, u64> {
        self.active_grants
            .get(&domain_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get a reference to the kernel ops (for testing).
    #[cfg(test)]
    pub fn kernel_ops(&self) -> &K {
        &self.kernel_ops
    }

    /// Get the count of failed revokes during rollbacks (for audit).
    #[allow(dead_code)] // Used by broker rollback audit tests in this module
    pub fn failed_revokes(&self) -> u32 {
        self.failed_revokes
    }

    /// Revoke a set of granted handles (internal, for rollback).
    ///
    /// This is best-effort: we continue unwinding even if individual revokes fail.
    /// Failed revokes are counted and emitted to audit for diagnostics.
    fn revoke_granted_handles(&mut self, domain_id: u64, grants: &HashMap<String, u64>) {
        for handle in grants.values() {
            match self.kernel_ops.revoke(*handle) {
                Ok(()) => {}
                Err(status) => {
                    self.failed_revokes += 1;
                    // Emit audit event for potential capability leak
                    self.audit
                        .emit(&BrokerAuditEvent::revoke_failed(domain_id, *handle, status));
                }
            }
        }
    }
}

// ============================================================================
// Test Constructors (for unit tests only)
// ============================================================================

#[cfg(test)]
impl CapabilityBroker<ChannelAllowlistPolicy, SimKernelGrantOps> {
    /// Create a test broker with the default test policy and simulated kernel.
    pub fn new_test(policy: ChannelAllowlistPolicy) -> Self {
        Self::new(policy, SimKernelGrantOps::new())
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test manifest with a single echo capability.
#[cfg(test)]
fn test_manifest_with_echo_cap() -> NativeWasmManifestV0 {
    NativeWasmManifestV0 {
        manifest: artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: "sha256:test123".to_string(),
            size_bytes: 1024,
            kind: "native_wasm_v0".to_string(),
            channels: vec!["Experimental".to_string()],
            signatures: vec![],
        },
        native_wasm: NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![RequiredCapability {
                export_name: "RAMEN_CAP_ECHO_V0".to_string(),
                interface: "harness.echo_v0".to_string(),
                rights: 1,
                purpose: "Send echo requests".to_string(),
            }],
            declares_no_capabilities: false,
        },
    }
}

/// Create a test manifest with an unknown interface.
#[cfg(test)]
fn test_manifest_with_unknown_interface() -> NativeWasmManifestV0 {
    NativeWasmManifestV0 {
        manifest: artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: "sha256:unknown".to_string(),
            size_bytes: 1024,
            kind: "native_wasm_v0".to_string(),
            channels: vec!["Experimental".to_string()],
            signatures: vec![],
        },
        native_wasm: NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![RequiredCapability {
                export_name: "RAMEN_CAP_UNKNOWN".to_string(),
                interface: "harness.unknown_v99".to_string(),
                rights: 1,
                purpose: "Unknown interface".to_string(),
            }],
            declares_no_capabilities: false,
        },
    }
}

/// Create a test manifest with mixed valid and invalid capabilities.
#[cfg(test)]
fn test_manifest_with_mixed_caps() -> NativeWasmManifestV0 {
    NativeWasmManifestV0 {
        manifest: artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: "sha256:mixed".to_string(),
            size_bytes: 1024,
            kind: "native_wasm_v0".to_string(),
            channels: vec!["Experimental".to_string()],
            signatures: vec![],
        },
        native_wasm: NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![
                // First cap is valid
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO_V0".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 1,
                    purpose: "Send echo requests".to_string(),
                },
                // Second cap has unknown interface (will fail)
                RequiredCapability {
                    export_name: "RAMEN_CAP_UNKNOWN".to_string(),
                    interface: "harness.unknown_v99".to_string(),
                    rights: 1,
                    purpose: "Unknown interface".to_string(),
                },
            ],
            declares_no_capabilities: false,
        },
    }
}

/// Create a test manifest with three valid capabilities (for torture test).
#[cfg(test)]
fn test_manifest_with_three_caps_all_allowed() -> NativeWasmManifestV0 {
    NativeWasmManifestV0 {
        manifest: artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: "sha256:three_caps".to_string(),
            size_bytes: 1024,
            kind: "native_wasm_v0".to_string(),
            channels: vec!["Experimental".to_string()],
            signatures: vec![],
        },
        native_wasm: NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO_V0".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 1,
                    purpose: "Echo capability".to_string(),
                },
                RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO_V1".to_string(),
                    interface: "harness.echo_v1".to_string(),
                    rights: 1,
                    purpose: "Echo v1 capability".to_string(),
                },
                RequiredCapability {
                    export_name: "RAMEN_CAP_TRACE_V0".to_string(),
                    interface: "harness.trace_v0".to_string(),
                    rights: 1,
                    purpose: "Trace capability".to_string(),
                },
            ],
            declares_no_capabilities: false,
        },
    }
}

/// Create a test manifest for the S10.5.1 semantic harness bridge.
#[cfg(test)]
fn test_manifest_with_semantic_harness_caps() -> NativeWasmManifestV0 {
    NativeWasmManifestV0 {
        manifest: artifact_store_schema::Manifest {
            schema_version: 1,
            content_id: "sha256:semantic_harness".to_string(),
            size_bytes: 1024,
            kind: "native_wasm_v0".to_string(),
            channels: vec!["SemanticHarness".to_string()],
            signatures: vec![],
        },
        native_wasm: NativeWasmV0 {
            entrypoint: "_start".to_string(),
            required_capabilities: vec![
                RequiredCapability {
                    export_name: SEMANTIC_HARNESS_EXPORT_SHMEM.to_string(),
                    interface: SEMANTIC_HARNESS_SHMEM_INTERFACE.to_string(),
                    rights: 1,
                    purpose: "Create and read snapshot shmem".to_string(),
                },
                RequiredCapability {
                    export_name: SEMANTIC_HARNESS_EXPORT_STATE.to_string(),
                    interface: SEMANTIC_HARNESS_STATE_INTERFACE.to_string(),
                    rights: 1,
                    purpose: "Request semantic snapshots".to_string(),
                },
            ],
            declares_no_capabilities: false,
        },
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditDecision;

    #[test]
    fn broker_grants_valid_capability() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_echo_cap();
        let result = broker
            .grant_capabilities(&manifest, "Experimental", 1)
            .unwrap();

        assert!(result.granted_handles.contains_key("RAMEN_CAP_ECHO_V0"));
    }

    #[test]
    fn broker_denies_unknown_interface() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_unknown_interface();
        let result = broker.grant_capabilities(&manifest, "Experimental", 1);

        assert!(matches!(result, Err(BrokerError::InterfaceUnknown { .. })));
    }

    #[test]
    fn broker_denies_channel_policy_violation() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_echo_cap();
        // Stable channel doesn't allow harness.echo_v0 in test policy
        let result = broker.grant_capabilities(&manifest, "Stable", 1);

        assert!(matches!(result, Err(BrokerError::CapabilityDenied { .. })));
    }

    #[test]
    fn broker_revokes_on_partial_failure() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        // Manifest with one valid + one invalid capability
        let manifest = test_manifest_with_mixed_caps();
        let result = broker.grant_capabilities(&manifest, "Experimental", 1);

        // Should fail and have no active grants
        assert!(result.is_err());
        assert!(broker.active_grants_for_domain(1).is_empty());
    }

    #[test]
    fn broker_revoke_domain_removes_all_grants() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = test_manifest_with_echo_cap();
        let result = broker
            .grant_capabilities(&manifest, "Experimental", 1)
            .unwrap();

        assert!(!result.granted_handles.is_empty());
        assert!(!broker.active_grants_for_domain(1).is_empty());

        let revoked = broker.revoke_domain(1);
        assert_eq!(revoked, 1);
        assert!(broker.active_grants_for_domain(1).is_empty());
    }

    #[test]
    fn broker_handles_capless_manifest() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = NativeWasmManifestV0 {
            manifest: artifact_store_schema::Manifest {
                schema_version: 1,
                content_id: "sha256:capless".to_string(),
                size_bytes: 1024,
                kind: "native_wasm_v0".to_string(),
                channels: vec!["Experimental".to_string()],
                signatures: vec![],
            },
            native_wasm: NativeWasmV0 {
                entrypoint: "_start".to_string(),
                required_capabilities: vec![],
                declares_no_capabilities: true,
            },
        };

        let result = broker
            .grant_capabilities(&manifest, "Experimental", 1)
            .unwrap();
        assert!(result.granted_handles.is_empty());
    }

    #[test]
    fn semantic_harness_grant_ops_grants_allowlisted_interfaces() {
        let policy = ChannelAllowlistPolicy::new_semantic_harness();
        let kernel = SemanticHarnessGrantOps::new();
        let mut broker = CapabilityBroker::new(policy, kernel);
        let domain_id = 42;

        let result = broker
            .grant_capabilities(
                &test_manifest_with_semantic_harness_caps(),
                "SemanticHarness",
                domain_id,
            )
            .expect("semantic harness grants");

        let shmem_handle = result
            .granted_handles
            .get(SEMANTIC_HARNESS_EXPORT_SHMEM)
            .copied()
            .expect("shmem handle");
        let semantic_handle = result
            .granted_handles
            .get(SEMANTIC_HARNESS_EXPORT_STATE)
            .copied()
            .expect("semantic handle");

        assert_eq!(
            shmem_handle,
            SemanticHarnessGrantOps::deterministic_handle(
                SEMANTIC_HARNESS_SHMEM_INTERFACE,
                domain_id
            )
            .unwrap()
        );
        assert_eq!(
            semantic_handle,
            SemanticHarnessGrantOps::deterministic_handle(
                SEMANTIC_HARNESS_STATE_INTERFACE,
                domain_id
            )
            .unwrap()
        );
        assert_eq!(broker.kernel_ops().registry().len(), 2);
        assert_eq!(broker.active_grants_for_domain(domain_id).len(), 2);
    }

    #[test]
    fn semantic_harness_grant_ops_denies_non_allowlisted_interface() {
        let mut kernel = SemanticHarnessGrantOps::new();
        let denied = kernel.grant("harness.echo_v1", 1, 42);

        assert!(matches!(denied, Err(STATUS_INVALID_INTERFACE)));
        assert!(kernel.registry().is_empty());
    }

    #[test]
    fn semantic_harness_handles_do_not_alias_domain_low_bits() {
        let domain_ids = [0, 1, 2, 42, SEMANTIC_HANDLE_MAX_DOMAIN_ID];
        let mut seen = std::collections::HashSet::new();

        for domain_id in domain_ids {
            for interface in [
                SEMANTIC_HARNESS_SHMEM_INTERFACE,
                SEMANTIC_HARNESS_STATE_INTERFACE,
            ] {
                let handle = SemanticHarnessGrantOps::deterministic_handle(interface, domain_id)
                    .expect("valid domain id");
                assert!(
                    seen.insert(handle),
                    "duplicate handle {handle:#x} for domain {domain_id}"
                );
            }
        }
    }

    #[test]
    fn semantic_harness_handles_reject_out_of_range_domains() {
        assert_eq!(
            SemanticHarnessGrantOps::deterministic_handle(
                SEMANTIC_HARNESS_SHMEM_INTERFACE,
                SEMANTIC_HANDLE_MAX_DOMAIN_ID + 1,
            ),
            None
        );
    }

    #[test]
    fn kernel_ops_backend_semantic_harness_grants_allowlist() {
        let mut backend = KernelOpsBackend::SemanticHarness(SemanticHarnessGrantOps::new());
        let handle = backend
            .grant(SEMANTIC_HARNESS_STATE_INTERFACE, 1, 7)
            .expect("semantic harness grant");
        assert_eq!(
            handle,
            SemanticHarnessGrantOps::deterministic_handle(SEMANTIC_HARNESS_STATE_INTERFACE, 7)
                .expect("deterministic handle")
        );
    }

    #[test]
    fn policy_denies_excessive_rights() {
        let policy = ChannelAllowlistPolicy::new_test();
        let mut broker = CapabilityBroker::new_test(policy);

        let manifest = NativeWasmManifestV0 {
            manifest: artifact_store_schema::Manifest {
                schema_version: 1,
                content_id: "sha256:excessive".to_string(),
                size_bytes: 1024,
                kind: "native_wasm_v0".to_string(),
                channels: vec!["Experimental".to_string()],
                signatures: vec![],
            },
            native_wasm: NativeWasmV0 {
                entrypoint: "_start".to_string(),
                required_capabilities: vec![RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO_V0".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 0x1_0000, // Exceeds max 0xFFFF
                    purpose: "Too many rights".to_string(),
                }],
                declares_no_capabilities: false,
            },
        };

        let result = broker.grant_capabilities(&manifest, "Experimental", 1);
        assert!(matches!(
            result,
            Err(BrokerError::CapabilityDenied {
                reason: DenialReason::RightsExceedMaximum { .. },
                ..
            })
        ));
    }

    #[test]
    fn interface_registry_known_interfaces() {
        let registry = InterfaceRegistry::new();

        // Should know standard interfaces
        assert!(registry.is_known("harness.echo_v0"));
        assert!(registry.is_known("harness.echo_v1"));
        assert!(registry.is_known("harness.trace_v0"));
        assert!(registry.is_known("harness.trace_v1"));
        assert!(registry.is_known("harness.trace_v2"));

        // Should not know unknown interfaces
        assert!(!registry.is_known("harness.unknown_v99"));
        assert!(!registry.is_known("random.interface_v1"));
    }

    // ========================================================================
    // Transactional Torture Test
    // ========================================================================

    #[test]
    fn broker_rolls_back_if_nth_grant_fails() {
        // This test proves the core security invariant:
        // If kernel grant fails on the Nth capability, the broker:
        // 1. Returns an error
        // 2. Revokes all previously granted handles (kernel-side)
        // 3. Leaves active_grants[domain_id] empty
        // 4. Produces an audit trail (via revoke_calls)

        let policy = ChannelAllowlistPolicy::new_test();

        // Create a kernel that will fail on the 3rd grant
        let kernel = FailingKernelGrantOps::fail_on_grant(3);

        let mut broker = CapabilityBroker::new(policy, kernel);

        // Manifest requests 3 capabilities, all allowed by policy
        let manifest = test_manifest_with_three_caps_all_allowed();
        let domain_id = 42u64;

        let result = broker.grant_capabilities(&manifest, "Experimental", domain_id);

        // Assertion 1: Operation must fail
        assert!(result.is_err(), "expected error when kernel grant fails");
        assert!(
            matches!(result, Err(BrokerError::KernelGrantFailed { ref interface, status: 0x1001 }) if interface == "harness.trace_v0"),
            "expected KernelGrantFailed for harness.trace_v0 with status 0x1001, got {:?}",
            result
        );

        // Assertion 2: active_grants must be empty (no partial capability leaks)
        assert!(
            broker.active_grants_for_domain(domain_id).is_empty(),
            "active_grants must be empty after rollback, but got {:?}",
            broker.active_grants_for_domain(domain_id)
        );

        // Assertion 3: Kernel must have received revoke calls for the first 2 handles
        let kernel = broker.kernel_ops();
        assert_eq!(
            kernel.revoke_calls_len(),
            2,
            "expected 2 revoke calls (for grants 1 and 2), got {}",
            kernel.revoke_calls_len()
        );

        // Assertion 4: Verify the exact handles that were revoked
        let granted_handles: std::collections::HashSet<u64> =
            kernel.granted_handles().into_iter().collect();
        assert_eq!(
            granted_handles.len(),
            2,
            "expected 2 successful grants before failure"
        );

        let revoked_handles: std::collections::HashSet<u64> =
            kernel.revoke_calls.iter().map(|(h, _)| *h).collect();
        assert_eq!(
            granted_handles, revoked_handles,
            "revoked handles must match the handles that were successfully granted"
        );
    }

    #[test]
    fn broker_rollback_preserves_prior_grants() {
        // Verify that a failed grant doesn't disturb grants from other domains

        let policy = ChannelAllowlistPolicy::new_test();
        let kernel = FailingKernelGrantOps::fail_on_grant(2);
        let mut broker = CapabilityBroker::new(policy, kernel);

        // First, successfully grant to domain 1
        let manifest1 = test_manifest_with_echo_cap();
        let result1 = broker.grant_capabilities(&manifest1, "Experimental", 1);
        assert!(result1.is_ok(), "first grant should succeed");
        assert!(!broker.active_grants_for_domain(1).is_empty());

        // Now attempt to grant to domain 2 (will fail on 2nd cap)
        let manifest2 = test_manifest_with_three_caps_all_allowed();
        let result2 = broker.grant_capabilities(&manifest2, "Experimental", 2);
        assert!(result2.is_err(), "second grant should fail");

        // Domain 1's grants must still be intact
        assert!(
            !broker.active_grants_for_domain(1).is_empty(),
            "domain 1's grants must not be affected by domain 2's failure"
        );

        // Domain 2 must have no grants
        assert!(
            broker.active_grants_for_domain(2).is_empty(),
            "domain 2 must have no grants after failure"
        );
    }

    // ========================================================================
    // Audit Trail Tests
    // ========================================================================

    #[test]
    fn audit_denial_has_full_context() {
        use crate::audit::vec_sink;

        let audit = vec_sink();
        let policy = ChannelAllowlistPolicy::new_test();
        let kernel = SimKernelGrantOps::new();
        let mut broker = CapabilityBroker::new_with_audit(policy, kernel, audit.clone());

        // Request a capability that's denied by channel policy
        let manifest = test_manifest_with_echo_cap();
        let result = broker.grant_capabilities(&manifest, "Stable", 42);

        assert!(result.is_err());

        // Verify audit event has full context
        let denied_events = audit.events_with_decision(AuditDecision::Denied);
        assert_eq!(denied_events.len(), 1, "should have one denial event");

        let event = &denied_events[0];
        assert_eq!(event.domain_id, 42);
        assert_eq!(event.channel, "Stable");
        assert_eq!(event.export_name, Some("RAMEN_CAP_ECHO_V0".to_string()));
        assert_eq!(event.interface, Some("harness.echo_v0".to_string()));
        assert_eq!(event.reason_code, Some(AuditReasonCode::ChannelNotAllowed));
    }

    #[test]
    fn audit_interface_unknown_has_context() {
        use crate::audit::vec_sink;

        let audit = vec_sink();
        let policy = ChannelAllowlistPolicy::new_test();
        let kernel = SimKernelGrantOps::new();
        let mut broker = CapabilityBroker::new_with_audit(policy, kernel, audit.clone());

        // Request an unknown interface
        let manifest = test_manifest_with_unknown_interface();
        let result = broker.grant_capabilities(&manifest, "Experimental", 99);

        assert!(result.is_err());

        // Verify audit event for interface unknown
        let denied_events = audit.events_with_decision(AuditDecision::Denied);
        assert_eq!(denied_events.len(), 1);

        let event = &denied_events[0];
        assert_eq!(event.domain_id, 99);
        assert_eq!(event.interface, Some("harness.unknown_v99".to_string()));
        assert_eq!(event.reason_code, Some(AuditReasonCode::InterfaceUnknown));
    }

    #[test]
    fn revoke_failure_is_audited_and_counted() {
        use crate::audit::vec_sink;

        let audit = vec_sink();
        let policy = ChannelAllowlistPolicy::new_test();

        // Kernel that will fail on revoke for handle 0x1000 (the first granted handle)
        let kernel = RevokeFailingKernelOps::fail_on_revoke(0x1000);

        let mut broker = CapabilityBroker::new_with_audit(policy, kernel, audit.clone());

        // Grant two capabilities, then cause a failure to trigger rollback
        // (Use the kernel that grants successfully but we'll trigger rollback via mixed caps)
        let manifest = test_manifest_with_mixed_caps(); // Has unknown interface on 2nd cap
        let result = broker.grant_capabilities(&manifest, "Experimental", 1);

        // Grant should fail due to unknown interface
        assert!(result.is_err());

        // The first grant succeeded and was then revoked during rollback
        // Since RevokeFailingKernelOps fails revoke for handle 0x1000:
        assert_eq!(broker.failed_revokes(), 1, "should have one failed revoke");

        // Verify audit event for revoke failure
        let revoke_failures = audit.events_with_decision(AuditDecision::RevokeFailed);
        assert_eq!(
            revoke_failures.len(),
            1,
            "should have one revoke failure event"
        );

        let event = &revoke_failures[0];
        assert_eq!(event.domain_id, 1);
        assert_eq!(event.failed_handle, Some(0x1000));
        assert_eq!(event.reason_code, Some(AuditReasonCode::KernelRevokeFailed));
        assert!(
            event
                .detail
                .as_ref()
                .unwrap()
                .contains("POTENTIAL CAPABILITY LEAK")
        );
    }
}
