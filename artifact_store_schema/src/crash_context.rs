//! Crash Context v0 — structured crash bundles for Semantic State v2.
//!
//! Captures crash information to enable debugging failed graduation attempts
//! and provides structured context for humans and agents.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

const CRASH_CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct CrashContextValidationError(pub String);

impl core::fmt::Display for CrashContextValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CrashContextValidationError {}

/// Identity of the runner that executed the attempt.
///
/// Captures software provenance so "same program, different runner build"
/// discrepancies are instantly diagnosable.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RunnerIdentityV0 {
    /// Runner kind (e.g., "compat_vm", "posix_runner", "wasi_runner", "native").
    pub kind: String,

    /// Runner version string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Build hash — git SHA or artifact content ID of the runner binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_hash: Option<String>,
}

/// A named content reference for the evidence extras bag.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NamedRef {
    pub key: String,
    pub ref_id: String,
}

/// Typed evidence bundle with conventional fields.
///
/// New writers should populate both `evidence_bundle` and the legacy
/// `evidence: Vec<String>` during the transition window.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceBundleV0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_tail_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_tail_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_log_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_dump_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_snapshot_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocol_traces: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_trace_ref: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_caps_ref: Option<String>,

    /// Additional named references not covered by conventional fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extras: Vec<NamedRef>,
}

/// Exit metrics — resource budget vs actual usage at crash time.
///
/// Kept separate from ExitReason to avoid breaking enum deserialization.
/// Provides deterministic "OOM vs timeout vs signal" classification context.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExitMetricsV0 {
    /// Wall-clock time budget (milliseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_budget_ms: Option<u64>,

    /// Wall-clock time actually elapsed (milliseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_elapsed_ms: Option<u64>,

    /// CPU time budget (microseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_budget_us: Option<u64>,

    /// CPU time actually used (microseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_time_us: Option<u64>,

    /// Memory budget (bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_budget_bytes: Option<u64>,

    /// Peak memory usage (bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_peak_bytes: Option<u64>,

    /// Content ID for captured cgroup memory.events snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oom_events_ref: Option<String>,
}

/// Exit reason for a component.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    /// Normal exit with status code.
    Exited { code: i32 },
    /// Killed by signal.
    Signal { signal: i32, name: Option<String> },
    /// Out of memory.
    Oom,
    /// Timeout exceeded.
    Timeout,
    /// Unknown/other failure.
    Unknown,
}

/// Semantic state snapshot at crash time.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticStateSnapshot {
    /// Active capabilities at crash time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_capabilities: Vec<String>,

    /// Portal calls in flight.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_portal_calls: Vec<String>,

    /// Last harness operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_harness_op: Option<String>,

    /// Memory usage at crash (bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,

    /// CPU time used (microseconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_time_us: Option<u64>,
}

/// Crash context artifact.
#[derive(Debug, Serialize, Deserialize)]
pub struct CrashContextV0 {
    pub schema_version: u32,

    /// Component that crashed.
    pub component_id: String,

    /// Run ID (correlates with observed_caps and scenario_trace).
    pub run_id: String,

    /// ISO 8601 timestamp of crash.
    pub crash_timestamp: String,

    /// Exit reason.
    pub exit_reason: ExitReason,

    /// Stack trace content ID (optional, points to stored stack trace artifact).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace_ref: Option<String>,

    /// Semantic state snapshot at crash time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_state: Option<SemanticStateSnapshot>,

    /// Target level being attempted when crash occurred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_level: Option<String>,

    /// Evidence content IDs (scenario traces, protocol traces, etc.).
    /// Legacy field — prefer evidence_bundle for new writers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,

    /// Typed evidence bundle with conventional fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_bundle: Option<EvidenceBundleV0>,

    /// Runner that executed this attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<RunnerIdentityV0>,

    /// Resource budget vs actual usage at crash time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_metrics: Option<ExitMetricsV0>,

    /// Human-readable crash summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Validate a crash context artifact.
pub fn validate_crash_context(ctx: &CrashContextV0) -> Result<(), CrashContextValidationError> {
    if ctx.schema_version != CRASH_CONTEXT_SCHEMA_VERSION {
        return Err(CrashContextValidationError(format!(
            "crash_context schema_version unsupported: {}",
            ctx.schema_version
        )));
    }

    if ctx.component_id.trim().is_empty() {
        return Err(CrashContextValidationError("component_id required".into()));
    }

    if ctx.run_id.trim().is_empty() {
        return Err(CrashContextValidationError("run_id required".into()));
    }

    if ctx.crash_timestamp.trim().is_empty() {
        return Err(CrashContextValidationError(
            "crash_timestamp required".into(),
        ));
    }

    // Validate stack trace ref if present
    if let Some(ref id) = ctx.stack_trace_ref {
        validate_content_id(id)?;
    }

    // Validate legacy evidence refs
    for id in &ctx.evidence {
        validate_content_id(id)?;
    }

    // Validate evidence bundle refs if present
    if let Some(ref bundle) = ctx.evidence_bundle {
        validate_evidence_bundle(bundle)?;
    }

    // Validate runner identity if present
    if let Some(ref runner) = ctx.runner {
        if runner.kind.trim().is_empty() {
            return Err(CrashContextValidationError(
                "runner.kind required when runner is present".into(),
            ));
        }
    }

    // Validate exit metrics refs if present
    if let Some(ref metrics) = ctx.exit_metrics {
        if let Some(ref id) = metrics.oom_events_ref {
            validate_content_id(id)?;
        }
    }

    Ok(())
}

fn validate_evidence_bundle(bundle: &EvidenceBundleV0) -> Result<(), CrashContextValidationError> {
    if let Some(ref id) = bundle.stdout_tail_ref {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.stderr_tail_ref {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.runner_log_ref {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.core_dump_ref {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.system_snapshot_ref {
        validate_content_id(id)?;
    }
    for id in &bundle.protocol_traces {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.scenario_trace_ref {
        validate_content_id(id)?;
    }
    if let Some(ref id) = bundle.observed_caps_ref {
        validate_content_id(id)?;
    }
    for named in &bundle.extras {
        validate_content_id(&named.ref_id)?;
    }
    Ok(())
}

fn validate_content_id(id: &str) -> Result<(), CrashContextValidationError> {
    if !id.starts_with("sha256:") {
        return Err(CrashContextValidationError(format!(
            "content id must be sha256: {id}"
        )));
    }
    Ok(())
}

/// Create a crash context from basic information.
///
/// The timestamp should be in ISO 8601 format (e.g., "2026-02-05T12:00:00Z").
pub fn create_crash_context(
    component_id: &str,
    run_id: &str,
    crash_timestamp: &str,
    exit_reason: ExitReason,
) -> CrashContextV0 {
    CrashContextV0 {
        schema_version: CRASH_CONTEXT_SCHEMA_VERSION,
        component_id: component_id.into(),
        run_id: run_id.into(),
        crash_timestamp: crash_timestamp.into(),
        exit_reason,
        stack_trace_ref: None,
        semantic_state: None,
        target_level: None,
        evidence: vec![],
        evidence_bundle: None,
        runner: None,
        exit_metrics: None,
        summary: None,
    }
}

/// Generate a human-readable summary of the crash.
pub fn summarize_crash(ctx: &CrashContextV0) -> String {
    let reason = match &ctx.exit_reason {
        ExitReason::Exited { code } => format!("exited with code {}", code),
        ExitReason::Signal { signal, name } => match name {
            Some(n) => format!("killed by signal {} ({})", signal, n),
            None => format!("killed by signal {}", signal),
        },
        ExitReason::Oom => "out of memory".into(),
        ExitReason::Timeout => "timeout exceeded".into(),
        ExitReason::Unknown => "unknown failure".into(),
    };

    let level_info = ctx
        .target_level
        .as_ref()
        .map(|l| format!(" while attempting {} level", l))
        .unwrap_or_default();

    format!(
        "Component {} {}{} at {}",
        ctx.component_id, reason, level_info, ctx.crash_timestamp
    )
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate() {
        let ctx = create_crash_context(
            "org.example.app",
            "run-123",
            "2026-02-05T12:00:00Z",
            ExitReason::Signal {
                signal: 11,
                name: Some("SIGSEGV".into()),
            },
        );
        validate_crash_context(&ctx).unwrap();
    }

    #[test]
    fn summarize() {
        let mut ctx = create_crash_context(
            "org.example.app",
            "run-123",
            "2026-02-05T12:00:00Z",
            ExitReason::Signal {
                signal: 11,
                name: Some("SIGSEGV".into()),
            },
        );
        ctx.target_level = Some("posix".into());
        let summary = summarize_crash(&ctx);
        assert!(summary.contains("SIGSEGV"));
        assert!(summary.contains("posix"));
    }

    #[test]
    fn validate_missing_component_id() {
        let mut ctx =
            create_crash_context("", "run-123", "2026-02-05T12:00:00Z", ExitReason::Unknown);
        ctx.component_id = "".into();
        assert!(validate_crash_context(&ctx).is_err());
    }

    #[test]
    fn validate_bad_evidence_ref() {
        let mut ctx = create_crash_context(
            "app",
            "run-123",
            "2026-02-05T12:00:00Z",
            ExitReason::Exited { code: 1 },
        );
        ctx.evidence.push("bad-ref".into());
        assert!(validate_crash_context(&ctx).is_err());
    }

    #[test]
    fn runner_identity_roundtrip() {
        let mut ctx = create_crash_context("app", "run-1", "2026-02-05T12:00:00Z", ExitReason::Oom);
        ctx.runner = Some(RunnerIdentityV0 {
            kind: "compat_vm".into(),
            version: Some("0.0.12".into()),
            build_hash: Some("sha256:abcdef".into()),
        });
        validate_crash_context(&ctx).unwrap();

        let json = serde_json::to_string(&ctx).unwrap();
        let loaded: CrashContextV0 = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.runner.as_ref().unwrap().kind, "compat_vm");
    }

    #[test]
    fn runner_identity_empty_kind_rejected() {
        let mut ctx =
            create_crash_context("app", "run-1", "2026-02-05T12:00:00Z", ExitReason::Unknown);
        ctx.runner = Some(RunnerIdentityV0 {
            kind: "".into(),
            version: None,
            build_hash: None,
        });
        assert!(validate_crash_context(&ctx).is_err());
    }

    #[test]
    fn evidence_bundle_roundtrip() {
        let mut ctx = create_crash_context(
            "app",
            "run-1",
            "2026-02-05T12:00:00Z",
            ExitReason::Signal {
                signal: 11,
                name: Some("SIGSEGV".into()),
            },
        );
        ctx.evidence_bundle = Some(EvidenceBundleV0 {
            stdout_tail_ref: Some("sha256:stdout111".into()),
            stderr_tail_ref: Some("sha256:stderr222".into()),
            runner_log_ref: Some("sha256:runlog333".into()),
            core_dump_ref: None,
            system_snapshot_ref: None,
            protocol_traces: vec![],
            scenario_trace_ref: Some("sha256:trace444".into()),
            observed_caps_ref: None,
            extras: vec![NamedRef {
                key: "custom_log".into(),
                ref_id: "sha256:custom555".into(),
            }],
        });
        validate_crash_context(&ctx).unwrap();

        let json = serde_json::to_string(&ctx).unwrap();
        let loaded: CrashContextV0 = serde_json::from_str(&json).unwrap();
        let bundle = loaded.evidence_bundle.unwrap();
        assert_eq!(bundle.stdout_tail_ref.unwrap(), "sha256:stdout111");
        assert_eq!(bundle.extras.len(), 1);
    }

    #[test]
    fn evidence_bundle_bad_ref() {
        let mut ctx =
            create_crash_context("app", "run-1", "2026-02-05T12:00:00Z", ExitReason::Unknown);
        ctx.evidence_bundle = Some(EvidenceBundleV0 {
            stdout_tail_ref: Some("bad-ref".into()),
            stderr_tail_ref: None,
            runner_log_ref: None,
            core_dump_ref: None,
            system_snapshot_ref: None,
            protocol_traces: vec![],
            scenario_trace_ref: None,
            observed_caps_ref: None,
            extras: vec![],
        });
        assert!(validate_crash_context(&ctx).is_err());
    }

    #[test]
    fn exit_metrics_roundtrip() {
        let mut ctx = create_crash_context("app", "run-1", "2026-02-05T12:00:00Z", ExitReason::Oom);
        ctx.exit_metrics = Some(ExitMetricsV0 {
            wall_budget_ms: Some(30000),
            wall_elapsed_ms: Some(12500),
            cpu_budget_us: None,
            cpu_time_us: Some(8_000_000),
            memory_budget_bytes: Some(512 * 1024 * 1024),
            memory_peak_bytes: Some(512 * 1024 * 1024),
            oom_events_ref: Some("sha256:oom_events_abc".into()),
        });
        validate_crash_context(&ctx).unwrap();

        let json = serde_json::to_string(&ctx).unwrap();
        let loaded: CrashContextV0 = serde_json::from_str(&json).unwrap();
        let metrics = loaded.exit_metrics.unwrap();
        assert_eq!(metrics.memory_peak_bytes, Some(512 * 1024 * 1024));
    }

    #[test]
    fn backward_compat_old_json_loads() {
        // Old JSON without new fields should still parse fine
        let old_json = r#"{
            "schema_version": 1,
            "component_id": "app",
            "run_id": "run-1",
            "crash_timestamp": "2026-02-05T12:00:00Z",
            "exit_reason": "oom"
        }"#;
        let ctx: CrashContextV0 = serde_json::from_str(old_json).unwrap();
        assert!(ctx.runner.is_none());
        assert!(ctx.evidence_bundle.is_none());
        assert!(ctx.exit_metrics.is_none());
        validate_crash_context(&ctx).unwrap();
    }
}
