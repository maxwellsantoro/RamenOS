//! Queue Item v0 — voting queue entries with evidence and scoring.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

const QUEUE_ITEM_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct QueueItemValidationError(pub String);

impl core::fmt::Display for QueueItemValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for QueueItemValidationError {}

/// Target graduation level for porting.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TargetLevel {
    /// Linux Domain / Flatpak compatibility
    Compat,
    /// POSIX Personality / rebuild
    Posix,
    /// WASI sandbox
    Wasi,
    /// Native portal/harness-first
    Native,
}

impl TargetLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetLevel::Compat => "compat",
            TargetLevel::Posix => "posix",
            TargetLevel::Wasi => "wasi",
            TargetLevel::Native => "native",
        }
    }
}

/// Evidence references for queue item.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueItemEvidence {
    /// Scenario trace content IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenario_traces: Vec<String>,

    /// Observed capabilities content ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_caps: Option<String>,

    /// Protocol trace content IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocol_traces: Vec<String>,
}

/// Prerequisite blocking this queue item.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prereq {
    /// Kind of prereq: "portal", "harness", "service", "driver"
    pub kind: String,

    /// Name of the missing component.
    pub name: String,

    /// Optional notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Scoring inputs for priority calculation.
///
/// Formula: `priority = (vote_weight * leverage * reuse) / (effort * risk)`
/// All values are 1..5.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ScoringInputs {
    /// How many votes / how important (1-5).
    pub vote_weight: u32,

    /// Leverage score: how much does porting this enable? (1-5)
    pub leverage: u32,

    /// Reuse score: how reusable is the work? (1-5)
    pub reuse: u32,

    /// Effort score: how hard is it to port? (1-5, higher = harder)
    pub effort: u32,

    /// Risk score: how risky is the port? (1-5, higher = riskier)
    pub risk: u32,
}

impl ScoringInputs {
    /// Compute priority from scoring inputs.
    pub fn compute_priority(&self) -> f64 {
        let numerator = (self.vote_weight * self.leverage * self.reuse) as f64;
        let denominator = (self.effort * self.risk) as f64;
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Validate scoring inputs are in range 1..5.
    pub fn validate(&self) -> Result<(), QueueItemValidationError> {
        for (name, val) in [
            ("vote_weight", self.vote_weight),
            ("leverage", self.leverage),
            ("reuse", self.reuse),
            ("effort", self.effort),
            ("risk", self.risk),
        ] {
            if !(1..=5).contains(&val) {
                return Err(QueueItemValidationError(format!(
                    "scoring.{} must be 1-5, got {}",
                    name, val
                )));
            }
        }
        Ok(())
    }
}

/// Queue item artifact.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueueItemV0 {
    pub schema_version: u32,

    /// Program identifier.
    pub program_id: String,

    /// Target graduation level.
    pub target_level: TargetLevel,

    /// Evidence references.
    pub evidence: QueueItemEvidence,

    /// Prerequisites blocking this item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prereqs: Vec<Prereq>,

    /// Scoring inputs.
    pub scoring: ScoringInputs,

    /// Computed priority (should match scoring formula).
    pub priority: f64,

    /// Explanation strings for the priority score.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explanation: Vec<String>,
}

/// Validate a queue item artifact.
pub fn validate_queue_item(item: &QueueItemV0) -> Result<(), QueueItemValidationError> {
    if item.schema_version != QUEUE_ITEM_SCHEMA_VERSION {
        return Err(QueueItemValidationError(format!(
            "queue_item schema_version unsupported: {}",
            item.schema_version
        )));
    }

    if item.program_id.trim().is_empty() {
        return Err(QueueItemValidationError("program_id required".into()));
    }

    // Validate evidence content IDs
    for id in &item.evidence.scenario_traces {
        validate_content_id(id)?;
    }
    if let Some(id) = &item.evidence.observed_caps {
        validate_content_id(id)?;
    }
    for id in &item.evidence.protocol_traces {
        validate_content_id(id)?;
    }

    // Must have at least one evidence artifact
    if item.evidence.scenario_traces.is_empty()
        && item.evidence.observed_caps.is_none()
        && item.evidence.protocol_traces.is_empty()
    {
        return Err(QueueItemValidationError(
            "at least one evidence artifact required".into(),
        ));
    }

    // Validate prereqs
    for (idx, prereq) in item.prereqs.iter().enumerate() {
        if prereq.kind.trim().is_empty() {
            return Err(QueueItemValidationError(format!(
                "prereqs[{}].kind required",
                idx
            )));
        }
        if prereq.name.trim().is_empty() {
            return Err(QueueItemValidationError(format!(
                "prereqs[{}].name required",
                idx
            )));
        }
    }

    // Validate scoring
    item.scoring.validate()?;

    // Verify computed priority matches formula
    let expected_priority = item.scoring.compute_priority();
    let tolerance = 0.001;
    if (item.priority - expected_priority).abs() > tolerance {
        return Err(QueueItemValidationError(format!(
            "priority mismatch: stored {} but computed {}",
            item.priority, expected_priority
        )));
    }

    Ok(())
}

fn validate_content_id(id: &str) -> Result<(), QueueItemValidationError> {
    if !id.starts_with("sha256:") {
        return Err(QueueItemValidationError(format!(
            "content id must be sha256: {id}"
        )));
    }
    Ok(())
}

/// Generate explanation strings for scoring.
pub fn explain_priority(item: &QueueItemV0) -> Vec<String> {
    let mut explanations = Vec::new();

    // Vote weight
    match item.scoring.vote_weight {
        1 => explanations.push("Vote weight is minimal (1/5)".into()),
        2 => explanations.push("Vote weight is low (2/5)".into()),
        3 => explanations.push("Vote weight is moderate (3/5)".into()),
        4 => explanations.push("Vote weight is high (4/5)".into()),
        5 => explanations.push("Vote weight is very high (5/5)".into()),
        _ => {}
    }

    // Leverage
    match item.scoring.leverage {
        1 => explanations.push("Leverage is minimal: standalone app".into()),
        2 => explanations.push("Leverage is low: few dependencies on this".into()),
        3 => explanations.push("Leverage is moderate".into()),
        4 => explanations.push("Leverage is high: enables multiple apps".into()),
        5 => explanations.push("Leverage is very high: foundational component".into()),
        _ => {}
    }

    // Effort - explain what makes it high
    if item.scoring.effort >= 4 {
        let mut reasons = Vec::new();
        let portal_prereqs: Vec<_> = item
            .prereqs
            .iter()
            .filter(|p| p.kind == "portal")
            .map(|p| p.name.as_str())
            .collect();
        if !portal_prereqs.is_empty() {
            reasons.push(format!("missing portals: {}", portal_prereqs.join(", ")));
        }
        let harness_prereqs: Vec<_> = item
            .prereqs
            .iter()
            .filter(|p| p.kind == "harness")
            .map(|p| p.name.as_str())
            .collect();
        if !harness_prereqs.is_empty() {
            reasons.push(format!("missing harnesses: {}", harness_prereqs.join(", ")));
        }
        if reasons.is_empty() {
            explanations.push(format!("Effort is high ({}/5)", item.scoring.effort));
        } else {
            explanations.push(format!(
                "Effort is high ({}/5): {}",
                item.scoring.effort,
                reasons.join("; ")
            ));
        }
    } else {
        explanations.push(format!(
            "Effort is {} ({}/5)",
            if item.scoring.effort <= 2 {
                "low"
            } else {
                "moderate"
            },
            item.scoring.effort
        ));
    }

    // Risk - explain what makes it high
    if item.scoring.risk >= 4 {
        let device_prereqs: Vec<_> = item
            .prereqs
            .iter()
            .filter(|p| p.kind == "driver" || p.kind == "device")
            .map(|p| p.name.as_str())
            .collect();
        if !device_prereqs.is_empty() {
            explanations.push(format!(
                "Risk is high ({}/5): device access: {}",
                item.scoring.risk,
                device_prereqs.join(", ")
            ));
        } else {
            explanations.push(format!("Risk is high ({}/5)", item.scoring.risk));
        }
    } else {
        explanations.push(format!(
            "Risk is {} ({}/5)",
            if item.scoring.risk <= 2 {
                "low"
            } else {
                "moderate"
            },
            item.scoring.risk
        ));
    }

    // Final priority
    explanations.push(format!(
        "Priority: {:.2} = ({} × {} × {}) / ({} × {})",
        item.priority,
        item.scoring.vote_weight,
        item.scoring.leverage,
        item.scoring.reuse,
        item.scoring.effort,
        item.scoring.risk
    ));

    explanations
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    fn sample_item() -> QueueItemV0 {
        QueueItemV0 {
            schema_version: 1,
            program_id: "org.example.app".into(),
            target_level: TargetLevel::Posix,
            evidence: QueueItemEvidence {
                scenario_traces: vec!["sha256:abc123".into()],
                observed_caps: Some("sha256:def456".into()),
                protocol_traces: vec![],
            },
            prereqs: vec![Prereq {
                kind: "portal".into(),
                name: "clipboard".into(),
                notes: None,
            }],
            scoring: ScoringInputs {
                vote_weight: 3,
                leverage: 4,
                reuse: 2,
                effort: 3,
                risk: 2,
            },
            priority: 4.0, // (3*4*2)/(3*2) = 24/6 = 4.0
            explanation: vec![],
        }
    }

    #[test]
    fn validate_ok() {
        let item = sample_item();
        validate_queue_item(&item).unwrap();
    }

    #[test]
    fn validate_priority_mismatch() {
        let mut item = sample_item();
        item.priority = 999.0;
        assert!(validate_queue_item(&item).is_err());
    }

    #[test]
    fn validate_scoring_out_of_range() {
        let mut item = sample_item();
        item.scoring.effort = 0;
        item.priority = item.scoring.compute_priority();
        assert!(validate_queue_item(&item).is_err());
    }

    #[test]
    fn explain_priority_includes_prereqs() {
        let mut item = sample_item();
        // Prereqs only appear in explanations when effort >= 4
        item.scoring.effort = 4;
        item.priority = item.scoring.compute_priority();
        let explanations = explain_priority(&item);
        assert!(
            explanations
                .iter()
                .any(|e| e.contains("clipboard") || e.contains("portal"))
        );
    }
}
