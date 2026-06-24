//! Minimal Policy v0 — proposed capability policy for graduation.
//!
//! Generated from observed capabilities, this proposes the minimal set of
//! capabilities needed for an application to function at a given target level.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use crate::queue_item::TargetLevel;

const MINIMAL_POLICY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct MinimalPolicyValidationError(pub String);

impl core::fmt::Display for MinimalPolicyValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MinimalPolicyValidationError {}

/// Scope for a capability grant.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityScope {
    /// Never grant.
    None,
    /// Prompt user each time.
    Prompt,
    /// Grant on first use, remember.
    OncePrompt,
    /// Always grant without prompting.
    Always,
}

/// A single capability in the policy.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicyCapability {
    /// Capability name (e.g., "portal.file_picker.ro").
    pub cap: String,

    /// Proposed scope.
    pub scope: CapabilityScope,

    /// Why this capability is needed (from observation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Evidence: how many times observed used.
    #[serde(default)]
    pub observed_use_count: u32,

    /// Whether this capability is required (vs optional).
    #[serde(default)]
    pub required: bool,
}

/// Minimal policy artifact.
#[derive(Debug, Serialize, Deserialize)]
pub struct MinimalPolicyV0 {
    pub schema_version: u32,

    /// Program identifier.
    pub program_id: String,

    /// Target level this policy is for.
    pub target_level: TargetLevel,

    /// Observed caps artifact this was derived from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_caps_ref: Option<String>,

    /// Proposed capabilities (sorted by cap name for deterministic output).
    pub capabilities: Vec<PolicyCapability>,

    /// Capabilities that were observed but excluded (sorted by cap name).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<ExcludedCapability>,

    /// Overall policy strictness score (0-100, higher = stricter).
    #[serde(default)]
    pub strictness_score: u32,

    /// Version of the policy proposal algorithm that produced this artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposer_version: Option<String>,

    /// Human-readable policy summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A capability that was excluded from the minimal policy.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExcludedCapability {
    pub cap: String,
    pub reason: String,
}

impl MinimalPolicyV0 {
    /// Create a minimal policy from observed capabilities.
    pub fn from_observed(
        program_id: &str,
        target_level: TargetLevel,
        observed_caps_ref: Option<String>,
    ) -> Self {
        Self {
            schema_version: MINIMAL_POLICY_SCHEMA_VERSION,
            program_id: program_id.into(),
            target_level,
            observed_caps_ref,
            capabilities: vec![],
            excluded: vec![],
            strictness_score: 0,
            proposer_version: None,
            summary: None,
        }
    }

    /// Add a capability to the policy.
    pub fn add_capability(&mut self, cap: PolicyCapability) {
        self.capabilities.push(cap);
        self.update_strictness();
    }

    /// Exclude a capability with a reason.
    pub fn exclude_capability(&mut self, cap: &str, reason: &str) {
        self.excluded.push(ExcludedCapability {
            cap: cap.into(),
            reason: reason.into(),
        });
    }

    /// Calculate strictness score.
    fn update_strictness(&mut self) {
        if self.capabilities.is_empty() {
            self.strictness_score = 100; // No caps = maximally strict
            return;
        }

        let mut score = 0u32;
        for cap in &self.capabilities {
            score += match cap.scope {
                CapabilityScope::None => 100,
                CapabilityScope::Prompt => 75,
                CapabilityScope::OncePrompt => 50,
                CapabilityScope::Always => 25,
            };
        }
        self.strictness_score = score / self.capabilities.len() as u32;
    }

    /// Generate human-readable summary.
    pub fn generate_summary(&mut self) {
        let required_count = self.capabilities.iter().filter(|c| c.required).count();
        let optional_count = self.capabilities.len() - required_count;
        let excluded_count = self.excluded.len();

        let mut lines = vec![format!(
            "Minimal policy for {} at {} level:",
            self.program_id,
            self.target_level.as_str()
        )];

        lines.push(format!(
            "  {} required capabilities, {} optional, {} excluded",
            required_count, optional_count, excluded_count
        ));

        lines.push(format!("  Strictness score: {}/100", self.strictness_score));

        if !self.capabilities.is_empty() {
            lines.push("  Required:".into());
            for cap in self.capabilities.iter().filter(|c| c.required) {
                lines.push(format!("    - {} ({:?})", cap.cap, cap.scope));
            }
        }

        self.summary = Some(lines.join("\n"));
    }
}

/// Validate a minimal policy artifact.
pub fn validate_minimal_policy(
    policy: &MinimalPolicyV0,
) -> Result<(), MinimalPolicyValidationError> {
    if policy.schema_version != MINIMAL_POLICY_SCHEMA_VERSION {
        return Err(MinimalPolicyValidationError(format!(
            "minimal_policy schema_version unsupported: {}",
            policy.schema_version
        )));
    }

    if policy.program_id.trim().is_empty() {
        return Err(MinimalPolicyValidationError("program_id required".into()));
    }

    if let Some(ref id) = policy.observed_caps_ref {
        validate_content_id(id)?;
    }

    for cap in &policy.capabilities {
        if cap.cap.trim().is_empty() {
            return Err(MinimalPolicyValidationError(
                "capability name required".into(),
            ));
        }
    }

    Ok(())
}

fn validate_content_id(id: &str) -> Result<(), MinimalPolicyValidationError> {
    if !id.starts_with("sha256:") {
        return Err(MinimalPolicyValidationError(format!(
            "content id must be sha256: {id}"
        )));
    }
    Ok(())
}

/// Current version of the policy proposal algorithm.
const PROPOSER_VERSION: &str = "0.1.0";

/// Propose a minimal policy from observed capabilities.
///
/// This is the core algorithm for the wizard's "propose minimal policy" step.
/// Output is deterministic: capabilities and excluded lists are sorted by cap
/// name, so identical inputs always produce identical artifact bytes.
pub fn propose_minimal_policy(
    program_id: &str,
    target_level: TargetLevel,
    observed_caps_ref: Option<String>,
    observed_capabilities: &[(String, u32, bool)], // (cap_name, use_count, was_granted)
) -> MinimalPolicyV0 {
    let mut policy = MinimalPolicyV0::from_observed(program_id, target_level, observed_caps_ref);
    policy.proposer_version = Some(PROPOSER_VERSION.into());

    for (cap, use_count, was_granted) in observed_capabilities {
        if !was_granted {
            // Was requested but denied — exclude
            policy.exclude_capability(cap, "denied during observation run");
            continue;
        }

        if *use_count == 0 {
            // Granted but never used — exclude
            policy.exclude_capability(cap, "granted but never used");
            continue;
        }

        // Determine scope based on capability type and usage
        let scope = if cap.contains("file_picker") || cap.contains("clipboard") {
            // User-mediated portals should prompt
            CapabilityScope::Prompt
        } else if cap.contains("network") || cap.contains("device") {
            // Sensitive capabilities should prompt once
            CapabilityScope::OncePrompt
        } else {
            // Standard capabilities can be always granted
            CapabilityScope::Always
        };

        let required = *use_count > 1; // Used multiple times = likely required

        policy.add_capability(PolicyCapability {
            cap: cap.clone(),
            scope,
            reason: Some(format!("observed {} uses", use_count)),
            observed_use_count: *use_count,
            required,
        });
    }

    // Sort for deterministic output (same inputs → identical artifact bytes)
    policy.capabilities.sort_by(|a, b| a.cap.cmp(&b.cap));
    policy.excluded.sort_by(|a, b| a.cap.cmp(&b.cap));

    policy.generate_summary();
    policy
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn propose_policy() {
        let caps = vec![
            ("portal.file_picker.ro".into(), 5, true),
            ("portal.clipboard".into(), 2, true),
            ("unused_cap".into(), 0, true),
            ("denied_cap".into(), 1, false),
        ];

        let policy = propose_minimal_policy("org.example.app", TargetLevel::Posix, None, &caps);

        assert_eq!(policy.capabilities.len(), 2);
        assert_eq!(policy.excluded.len(), 2);

        let file_picker = policy
            .capabilities
            .iter()
            .find(|c| c.cap.contains("file_picker"))
            .unwrap();
        assert_eq!(file_picker.scope, CapabilityScope::Prompt);
        assert!(file_picker.required);

        validate_minimal_policy(&policy).unwrap();
    }

    #[test]
    fn strictness_calculation() {
        let mut policy = MinimalPolicyV0::from_observed("app", TargetLevel::Native, None);

        // All prompt = high strictness
        policy.add_capability(PolicyCapability {
            cap: "cap1".into(),
            scope: CapabilityScope::Prompt,
            reason: None,
            observed_use_count: 1,
            required: true,
        });
        assert_eq!(policy.strictness_score, 75);

        // Add always = lower strictness
        policy.add_capability(PolicyCapability {
            cap: "cap2".into(),
            scope: CapabilityScope::Always,
            reason: None,
            observed_use_count: 1,
            required: false,
        });
        assert_eq!(policy.strictness_score, 50); // (75 + 25) / 2
    }

    #[test]
    fn deterministic_output() {
        // Same inputs in different order should produce identical JSON
        let caps_order_a = vec![
            ("portal.clipboard".into(), 2, true),
            ("portal.file_picker.ro".into(), 5, true),
            ("network.tcp".into(), 3, true),
            ("unused_cap".into(), 0, true),
        ];

        let caps_order_b = vec![
            ("network.tcp".into(), 3, true),
            ("unused_cap".into(), 0, true),
            ("portal.file_picker.ro".into(), 5, true),
            ("portal.clipboard".into(), 2, true),
        ];

        let policy_a = propose_minimal_policy("app", TargetLevel::Posix, None, &caps_order_a);
        let policy_b = propose_minimal_policy("app", TargetLevel::Posix, None, &caps_order_b);

        let json_a = serde_json::to_string_pretty(&policy_a).unwrap();
        let json_b = serde_json::to_string_pretty(&policy_b).unwrap();

        assert_eq!(
            json_a, json_b,
            "same inputs in different order should produce identical JSON"
        );
    }

    #[test]
    fn proposer_version_set() {
        let caps = vec![("portal.file_picker.ro".into(), 5, true)];
        let policy = propose_minimal_policy("app", TargetLevel::Posix, None, &caps);
        assert!(policy.proposer_version.is_some());
        assert_eq!(policy.proposer_version.unwrap(), PROPOSER_VERSION);
    }

    #[test]
    fn backward_compat_old_json_without_proposer_version() {
        let json = r#"{
            "schema_version": 1,
            "program_id": "app",
            "target_level": "posix",
            "capabilities": [],
            "strictness_score": 100
        }"#;
        let policy: MinimalPolicyV0 = serde_json::from_str(json).unwrap();
        assert!(policy.proposer_version.is_none());
        validate_minimal_policy(&policy).unwrap();
    }
}
