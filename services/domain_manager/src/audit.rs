//! Structured audit events for the capability broker.
//!
//! This module provides a structured audit trail for all broker decisions:
//! - Successful grants
//! - Policy denials
//! - Kernel failures
//! - Rollback events
//!
//! The audit system uses the sink pattern, allowing different backends:
//! - S10.1: StdErrAuditSink (eprintln! for visibility)
//! - S10.2+: Trace ring buffer sink or file sink

#![allow(dead_code)] // Scaffold API exercised partially by broker tests; full sink wiring is S10.2+

use std::sync::Arc;

// ============================================================================
// Audit Decision Types
// ============================================================================

/// The outcome of a capability request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditDecision {
    /// Capability was granted successfully
    Granted,
    /// Capability was denied by policy or validation
    Denied,
    /// Previously granted capabilities were revoked (rollback)
    RolledBack,
    /// Revoke operation failed during rollback (potential leak)
    RevokeFailed,
}

/// Machine-readable reason codes for denials and failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditReasonCode {
    // Denial reasons
    /// Interface is not in the known registry
    InterfaceUnknown,
    /// Interface not allowed for artifact's channel
    ChannelNotAllowed,
    /// Requested rights exceed maximum allowed
    RightsExceeded,

    // Failure reasons
    /// Kernel returned error on grant
    KernelGrantFailed,
    /// Kernel returned error on revoke
    KernelRevokeFailed,

    // Other
    /// Manifest failed validation
    ManifestInvalid,
}

// ============================================================================
// Audit Event
// ============================================================================

/// A structured audit event for broker operations.
///
/// Contains full context for security analysis and debugging:
/// - Who requested (domain_id, content_id, channel)
/// - What was requested (export_name, interface, rights)
/// - What was allowed (max_rights for comparison)
/// - Outcome (decision, reason_code, detail)
#[derive(Debug, Clone)]
pub struct BrokerAuditEvent {
    // Requester identity
    /// Domain requesting the capability
    pub domain_id: u64,
    /// Content ID of the artifact (e.g., "sha256:...")
    pub content_id: String,
    /// Channel the artifact was launched with
    pub channel: String,

    // Request details
    /// Export name (e.g., "RAMEN_CAP_ECHO_V0")
    pub export_name: Option<String>,
    /// Interface being requested (e.g., "harness.echo_v0")
    pub interface: Option<String>,
    /// Rights bits requested
    pub requested_rights: Option<u64>,
    /// Maximum rights allowed by policy
    pub max_rights: Option<u64>,

    // Outcome
    /// The decision made
    pub decision: AuditDecision,
    /// Machine-readable reason code (for denials/failures)
    pub reason_code: Option<AuditReasonCode>,
    /// Human-readable detail message
    pub detail: Option<String>,
    /// Kernel status code (for kernel failures)
    pub kernel_status: Option<u32>,

    // For rollback events
    /// Handle that failed to revoke (for RevokeFailed)
    pub failed_handle: Option<u64>,
}

impl BrokerAuditEvent {
    /// Create an event for a successful grant.
    pub fn granted(
        domain_id: u64,
        content_id: &str,
        channel: &str,
        export_name: &str,
        interface: &str,
        rights: u64,
    ) -> Self {
        Self {
            domain_id,
            content_id: content_id.to_string(),
            channel: channel.to_string(),
            export_name: Some(export_name.to_string()),
            interface: Some(interface.to_string()),
            requested_rights: Some(rights),
            max_rights: None,
            decision: AuditDecision::Granted,
            reason_code: None,
            detail: None,
            kernel_status: None,
            failed_handle: None,
        }
    }

    /// Create an event for a denial.
    #[allow(clippy::too_many_arguments)]
    pub fn denied(
        domain_id: u64,
        content_id: &str,
        channel: &str,
        export_name: Option<&str>,
        interface: Option<&str>,
        requested_rights: Option<u64>,
        max_rights: Option<u64>,
        reason_code: AuditReasonCode,
        detail: &str,
    ) -> Self {
        Self {
            domain_id,
            content_id: content_id.to_string(),
            channel: channel.to_string(),
            export_name: export_name.map(|s| s.to_string()),
            interface: interface.map(|s| s.to_string()),
            requested_rights,
            max_rights,
            decision: AuditDecision::Denied,
            reason_code: Some(reason_code),
            detail: Some(detail.to_string()),
            kernel_status: None,
            failed_handle: None,
        }
    }

    /// Create an event for a kernel grant failure.
    pub fn kernel_grant_failed(
        domain_id: u64,
        content_id: &str,
        channel: &str,
        interface: &str,
        status: u32,
    ) -> Self {
        Self {
            domain_id,
            content_id: content_id.to_string(),
            channel: channel.to_string(),
            export_name: None,
            interface: Some(interface.to_string()),
            requested_rights: None,
            max_rights: None,
            decision: AuditDecision::Denied,
            reason_code: Some(AuditReasonCode::KernelGrantFailed),
            detail: Some(format!("kernel grant failed with status 0x{:x}", status)),
            kernel_status: Some(status),
            failed_handle: None,
        }
    }

    /// Create an event for a rollback (successful revoke during unwind).
    pub fn rolled_back(
        domain_id: u64,
        content_id: &str,
        channel: &str,
        export_name: &str,
        interface: &str,
        handle: u64,
    ) -> Self {
        Self {
            domain_id,
            content_id: content_id.to_string(),
            channel: channel.to_string(),
            export_name: Some(export_name.to_string()),
            interface: Some(interface.to_string()),
            requested_rights: None,
            max_rights: None,
            decision: AuditDecision::RolledBack,
            reason_code: None,
            detail: Some(format!("revoked handle 0x{:x} during rollback", handle)),
            kernel_status: None,
            failed_handle: None,
        }
    }

    /// Create an event for a failed revoke during rollback.
    pub fn revoke_failed(domain_id: u64, handle: u64, status: u32) -> Self {
        Self {
            domain_id,
            content_id: String::new(),
            channel: String::new(),
            export_name: None,
            interface: None,
            requested_rights: None,
            max_rights: None,
            decision: AuditDecision::RevokeFailed,
            reason_code: Some(AuditReasonCode::KernelRevokeFailed),
            detail: Some(format!(
                "revoke(handle=0x{:x}) failed with status 0x{:x} - POTENTIAL CAPABILITY LEAK",
                handle, status
            )),
            kernel_status: Some(status),
            failed_handle: Some(handle),
        }
    }
}

// ============================================================================
// Audit Sink Trait
// ============================================================================

/// Trait for audit event destinations.
///
/// Implementations determine where audit events go:
/// - `StdErrAuditSink`: Prints to stderr (S10.1 default)
/// - `VecAuditSink`: Collects in memory (for tests)
/// - Future: `TraceRingSink`, `FileSink`, etc.
pub trait AuditSink: Send + Sync {
    /// Emit an audit event.
    fn emit(&self, event: &BrokerAuditEvent);
}

// ============================================================================
// Standard Error Sink (S10.1 Default)
// ============================================================================

/// Audit sink that prints events to stderr.
///
/// This is the default sink for S10.1, providing visibility
/// without requiring a tracing infrastructure.
pub struct StdErrAuditSink;

impl AuditSink for StdErrAuditSink {
    fn emit(&self, event: &BrokerAuditEvent) {
        let decision = match event.decision {
            AuditDecision::Granted => "GRANT",
            AuditDecision::Denied => "DENY",
            AuditDecision::RolledBack => "ROLLBACK",
            AuditDecision::RevokeFailed => "REVOKE_FAIL",
        };

        let reason = event.reason_code.map(|r| match r {
            AuditReasonCode::InterfaceUnknown => "interface_unknown",
            AuditReasonCode::ChannelNotAllowed => "channel_not_allowed",
            AuditReasonCode::RightsExceeded => "rights_exceeded",
            AuditReasonCode::KernelGrantFailed => "kernel_grant_failed",
            AuditReasonCode::KernelRevokeFailed => "kernel_revoke_failed",
            AuditReasonCode::ManifestInvalid => "manifest_invalid",
        });

        eprintln!(
            "audit: domain={} content={} channel={} decision={} export={} iface={} rights={:?} reason={} detail={}",
            event.domain_id,
            event.content_id,
            event.channel,
            decision,
            event.export_name.as_deref().unwrap_or("-"),
            event.interface.as_deref().unwrap_or("-"),
            event.requested_rights,
            reason.unwrap_or("-"),
            event.detail.as_deref().unwrap_or("-"),
        );
    }
}

impl Default for StdErrAuditSink {
    fn default() -> Self {
        Self
    }
}

// ============================================================================
// Vector Sink (for testing)
// ============================================================================

/// Audit sink that collects events in a vector.
///
/// Used in tests to assert on audit output.
#[cfg(test)]
pub struct VecAuditSink {
    events: std::sync::Mutex<Vec<BrokerAuditEvent>>,
}

#[cfg(test)]
impl VecAuditSink {
    /// Create a new vector sink.
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Get all collected events.
    pub fn events(&self) -> Vec<BrokerAuditEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Get events matching a decision type.
    pub fn events_with_decision(&self, decision: AuditDecision) -> Vec<BrokerAuditEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.decision == decision)
            .cloned()
            .collect()
    }

    /// Check if any event matches the decision and reason.
    pub fn has_event(&self, decision: AuditDecision, reason_code: Option<AuditReasonCode>) -> bool {
        self.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.decision == decision && e.reason_code == reason_code)
    }
}

#[cfg(test)]
impl Default for VecAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl AuditSink for VecAuditSink {
    fn emit(&self, event: &BrokerAuditEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

// ============================================================================
// Shared Sink Type
// ============================================================================

/// Type alias for a shared audit sink.
pub type SharedAuditSink = Arc<dyn AuditSink>;

/// Create a default stderr sink.
pub fn stderr_sink() -> SharedAuditSink {
    Arc::new(StdErrAuditSink)
}

#[cfg(test)]
/// Create a vector sink for testing.
pub fn vec_sink() -> Arc<VecAuditSink> {
    Arc::new(VecAuditSink::new())
}
