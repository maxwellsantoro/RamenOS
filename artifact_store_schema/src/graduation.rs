//! Graduation Evidence v0 — tracks progression through target levels.
//!
//! Records attempts to graduate software from compat → posix → wasi → native,
//! including successful runs, crashes, and policy validations.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use crate::crash_context::RunnerIdentityV0;
use crate::queue_item::TargetLevel;

const GRADUATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct GraduationValidationError(pub String);

impl core::fmt::Display for GraduationValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GraduationValidationError {}

/// Result of a graduation attempt.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptResult {
    /// Attempt succeeded.
    Success,
    /// Attempt failed with crash.
    Crashed,
    /// Attempt failed due to missing prereqs.
    Blocked,
    /// Attempt failed validation/gates.
    ValidationFailed,
    /// Attempt is in progress.
    InProgress,
}

/// A single graduation attempt at a target level.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraduationAttempt {
    /// Target level attempted.
    pub target_level: TargetLevel,

    /// Attempt number (1-indexed).
    pub attempt_number: u32,

    /// ISO 8601 timestamp.
    pub timestamp: String,

    /// Result of the attempt.
    pub result: AttemptResult,

    /// Scenario trace content ID from this attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_trace_ref: Option<String>,

    /// Observed caps content ID from this attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_caps_ref: Option<String>,

    /// Crash context content ID (if crashed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crash_context_ref: Option<String>,

    /// Policy bundle content ID used for this attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_bundle_ref: Option<String>,

    /// Notes about the attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Runner that executed this attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<RunnerIdentityV0>,
}

/// Summary of graduation status at a target level.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelStatus {
    pub level: TargetLevel,

    /// Number of attempts at this level.
    pub attempt_count: u32,

    /// Number of successful runs.
    pub success_count: u32,

    /// Number of crashes.
    pub crash_count: u32,

    /// Has ever succeeded at this level.
    pub achieved: bool,

    /// Is currently the active/best level.
    pub current: bool,
}

/// Graduation evidence artifact.
#[derive(Debug, Serialize, Deserialize)]
pub struct GraduationV0 {
    pub schema_version: u32,

    /// Program identifier.
    pub program_id: String,

    /// Current best achieved level.
    pub current_level: TargetLevel,

    /// Target level being worked towards.
    pub target_level: TargetLevel,

    /// History of graduation attempts.
    pub attempts: Vec<GraduationAttempt>,

    /// Summary status per level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_status: Vec<LevelStatus>,

    /// Queue item reference (if in the voting queue).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_item_ref: Option<String>,
}

impl GraduationV0 {
    /// Create a new graduation record starting at compat level.
    pub fn new(program_id: &str, target_level: TargetLevel) -> Self {
        Self {
            schema_version: GRADUATION_SCHEMA_VERSION,
            program_id: program_id.into(),
            current_level: TargetLevel::Compat,
            target_level,
            attempts: vec![],
            level_status: vec![],
            queue_item_ref: None,
        }
    }

    /// Add an attempt.
    pub fn add_attempt(&mut self, attempt: GraduationAttempt) {
        self.attempts.push(attempt);
        self.update_status();
    }

    /// Update level status summary from attempts.
    pub fn update_status(&mut self) {
        let statuses = Self::compute_level_statuses(&self.attempts);
        // Find best achieved from computed statuses
        let best_achieved = statuses
            .iter()
            .filter(|s| s.current)
            .map(|s| s.level)
            .next()
            .unwrap_or(TargetLevel::Compat);
        self.current_level = best_achieved;
        self.level_status = statuses;
    }

    /// Get progression summary as human-readable string.
    pub fn progression_summary(&self) -> String {
        // If level_status is empty but we have attempts, compute inline
        let statuses: Vec<LevelStatus>;
        let source = if self.level_status.is_empty() && !self.attempts.is_empty() {
            statuses = Self::compute_level_statuses(&self.attempts);
            &statuses
        } else {
            &self.level_status
        };

        let mut summary = String::new();
        for (i, s) in source.iter().enumerate() {
            if i > 0 {
                summary.push_str(" → ");
            }
            let mark = if s.current {
                "[*]"
            } else if s.achieved {
                "[✓]"
            } else if s.attempt_count > 0 {
                "[x]"
            } else {
                "[ ]"
            };
            summary.push_str(&format!("{} {}", mark, s.level.as_str()));
        }
        summary
    }

    /// Compute level statuses from attempts (pure function, no mutation).
    fn compute_level_statuses(attempts: &[GraduationAttempt]) -> Vec<LevelStatus> {
        let levels = [
            TargetLevel::Compat,
            TargetLevel::Posix,
            TargetLevel::Wasi,
            TargetLevel::Native,
        ];

        let mut statuses = Vec::new();
        let mut best_achieved = TargetLevel::Compat;

        for level in levels {
            let attempts_at_level: Vec<_> = attempts
                .iter()
                .filter(|a| a.target_level == level)
                .collect();

            let success_count = attempts_at_level
                .iter()
                .filter(|a| a.result == AttemptResult::Success)
                .count() as u32;

            let crash_count = attempts_at_level
                .iter()
                .filter(|a| a.result == AttemptResult::Crashed)
                .count() as u32;

            let achieved = success_count > 0;
            if achieved {
                best_achieved = level;
            }

            statuses.push(LevelStatus {
                level,
                attempt_count: attempts_at_level.len() as u32,
                success_count,
                crash_count,
                achieved,
                current: false,
            });
        }

        for status in &mut statuses {
            status.current = status.level == best_achieved;
        }

        statuses
    }
}

/// Validate a graduation artifact.
pub fn validate_graduation(grad: &GraduationV0) -> Result<(), GraduationValidationError> {
    if grad.schema_version != GRADUATION_SCHEMA_VERSION {
        return Err(GraduationValidationError(format!(
            "graduation schema_version unsupported: {}",
            grad.schema_version
        )));
    }

    if grad.program_id.trim().is_empty() {
        return Err(GraduationValidationError("program_id required".into()));
    }

    // Validate all content refs and runner identities
    for attempt in &grad.attempts {
        if let Some(ref id) = attempt.scenario_trace_ref {
            validate_content_id(id)?;
        }
        if let Some(ref id) = attempt.observed_caps_ref {
            validate_content_id(id)?;
        }
        if let Some(ref id) = attempt.crash_context_ref {
            validate_content_id(id)?;
        }
        if let Some(ref id) = attempt.policy_bundle_ref {
            validate_content_id(id)?;
        }
        if let Some(ref runner) = attempt.runner {
            if runner.kind.trim().is_empty() {
                return Err(GraduationValidationError(
                    "attempt runner.kind required when runner is present".into(),
                ));
            }
        }
    }

    if let Some(ref id) = grad.queue_item_ref {
        validate_content_id(id)?;
    }

    Ok(())
}

fn validate_content_id(id: &str) -> Result<(), GraduationValidationError> {
    if !id.starts_with("sha256:") {
        return Err(GraduationValidationError(format!(
            "content id must be sha256: {id}"
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn new_graduation() {
        let grad = GraduationV0::new("org.example.app", TargetLevel::Native);
        assert_eq!(grad.current_level, TargetLevel::Compat);
        assert_eq!(grad.target_level, TargetLevel::Native);
        validate_graduation(&grad).unwrap();
    }

    #[test]
    fn add_attempts_and_progress() {
        let mut grad = GraduationV0::new("org.example.app", TargetLevel::Native);

        // Succeed at compat
        grad.add_attempt(GraduationAttempt {
            target_level: TargetLevel::Compat,
            attempt_number: 1,
            timestamp: "2026-02-05T12:00:00Z".into(),
            result: AttemptResult::Success,
            scenario_trace_ref: Some("sha256:aaa".into()),
            observed_caps_ref: None,
            crash_context_ref: None,
            policy_bundle_ref: None,
            notes: None,
            runner: None,
        });

        assert_eq!(grad.current_level, TargetLevel::Compat);

        // Fail at posix
        grad.add_attempt(GraduationAttempt {
            target_level: TargetLevel::Posix,
            attempt_number: 1,
            timestamp: "2026-02-05T13:00:00Z".into(),
            result: AttemptResult::Crashed,
            scenario_trace_ref: None,
            observed_caps_ref: None,
            crash_context_ref: Some("sha256:bbb".into()),
            policy_bundle_ref: None,
            notes: Some("SIGSEGV in libc shim".into()),
            runner: None,
        });

        assert_eq!(grad.current_level, TargetLevel::Compat); // Still at compat

        // Succeed at posix
        grad.add_attempt(GraduationAttempt {
            target_level: TargetLevel::Posix,
            attempt_number: 2,
            timestamp: "2026-02-05T14:00:00Z".into(),
            result: AttemptResult::Success,
            scenario_trace_ref: Some("sha256:ccc".into()),
            observed_caps_ref: Some("sha256:ddd".into()),
            crash_context_ref: None,
            policy_bundle_ref: None,
            notes: None,
            runner: Some(RunnerIdentityV0 {
                kind: "posix_runner".into(),
                version: Some("0.1.0".into()),
                build_hash: None,
            }),
        });

        assert_eq!(grad.current_level, TargetLevel::Posix); // Now at posix

        let summary = grad.progression_summary();
        assert!(summary.contains("[✓] compat"));
        assert!(summary.contains("[*] posix"));

        validate_graduation(&grad).unwrap();
    }

    #[test]
    fn validate_bad_ref() {
        let mut grad = GraduationV0::new("app", TargetLevel::Native);
        grad.queue_item_ref = Some("bad-ref".into());
        assert!(validate_graduation(&grad).is_err());
    }

    #[test]
    fn json_load_progression_summary() {
        // Simulate JSON deserialization where level_status is empty
        // but attempts are populated (the bug this fixes)
        let json = r#"{
            "schema_version": 1,
            "program_id": "org.test.app",
            "current_level": "compat",
            "target_level": "native",
            "attempts": [
                {
                    "target_level": "compat",
                    "attempt_number": 1,
                    "timestamp": "2026-02-05T10:00:00Z",
                    "result": "success"
                },
                {
                    "target_level": "posix",
                    "attempt_number": 1,
                    "timestamp": "2026-02-05T11:00:00Z",
                    "result": "crashed"
                },
                {
                    "target_level": "posix",
                    "attempt_number": 2,
                    "timestamp": "2026-02-05T12:00:00Z",
                    "result": "success"
                }
            ],
            "level_status": []
        }"#;

        let grad: GraduationV0 = serde_json::from_str(json).unwrap();
        validate_graduation(&grad).unwrap();

        // This was the bug: progression_summary returned empty string
        let summary = grad.progression_summary();
        assert!(
            summary.contains("compat"),
            "should contain level names, got: {}",
            summary
        );
        assert!(summary.contains("[✓] compat"), "compat should be achieved");
        assert!(summary.contains("[*] posix"), "posix should be current");
        assert!(summary.contains("[ ] wasi"), "wasi should be unattempted");
    }

    #[test]
    fn backward_compat_old_json_without_runner() {
        let json = r#"{
            "schema_version": 1,
            "program_id": "app",
            "current_level": "compat",
            "target_level": "native",
            "attempts": [
                {
                    "target_level": "compat",
                    "attempt_number": 1,
                    "timestamp": "2026-02-05T10:00:00Z",
                    "result": "success"
                }
            ]
        }"#;
        let grad: GraduationV0 = serde_json::from_str(json).unwrap();
        assert!(grad.attempts[0].runner.is_none());
        validate_graduation(&grad).unwrap();
    }
}
