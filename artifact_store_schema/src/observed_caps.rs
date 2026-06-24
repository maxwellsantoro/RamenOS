use serde::{Deserialize, Serialize};

use crate::prelude::*;

const OBSERVED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct ObservedCapsValidationError(pub String);

impl core::fmt::Display for ObservedCapsValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ObservedCapsValidationError {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObservedCapsV0 {
    pub schema_version: u32,
    pub program_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_plan_id: Option<String>,
    pub capabilities: Vec<ObservedCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObservedCapability {
    pub cap: String,
    pub scope: ObservedCapScope,
    pub counts: ObservedCapCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObservedCapScope {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObservedCapCounts {
    pub granted: u32,
    pub used: u32,
}

pub fn validate_observed_caps(obs: &ObservedCapsV0) -> Result<(), ObservedCapsValidationError> {
    if obs.schema_version != OBSERVED_SCHEMA_VERSION {
        return Err(ObservedCapsValidationError(format!(
            "observed_caps schema_version unsupported: {}",
            obs.schema_version
        )));
    }
    if obs.program_id.trim().is_empty() {
        return Err(ObservedCapsValidationError("program_id required".into()));
    }
    if obs.run_id.trim().is_empty() {
        return Err(ObservedCapsValidationError("run_id required".into()));
    }
    if let Some(plan_id) = &obs.launch_plan_id {
        if !plan_id.is_empty() {
            validate_content_id(plan_id)?;
        }
    }
    if obs.capabilities.is_empty() {
        return Err(ObservedCapsValidationError(
            "capabilities must not be empty".into(),
        ));
    }
    for cap in &obs.capabilities {
        if cap.cap.trim().is_empty() {
            return Err(ObservedCapsValidationError("cap name required".into()));
        }
        if cap.counts.used > cap.counts.granted {
            return Err(ObservedCapsValidationError(format!(
                "cap {} used exceeds granted",
                cap.cap
            )));
        }
        for id in &cap.scope.artifact_ids {
            validate_content_id(id)?;
        }
        for id in &cap.evidence {
            validate_content_id(id)?;
        }
    }
    for id in &obs.evidence {
        validate_content_id(id)?;
    }
    Ok(())
}

fn validate_content_id(id: &str) -> Result<(), ObservedCapsValidationError> {
    if !id.starts_with("sha256:") {
        return Err(ObservedCapsValidationError(format!(
            "content id must be sha256: {id}"
        )));
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn validate_ok() {
        let obs = ObservedCapsV0 {
            schema_version: 1,
            program_id: "demo".into(),
            run_id: "run-1".into(),
            launch_plan_id: None,
            capabilities: vec![ObservedCapability {
                cap: "portal.file_picker.ro".into(),
                scope: ObservedCapScope {
                    artifact_ids: vec!["sha256:abc".into()],
                },
                counts: ObservedCapCounts {
                    granted: 1,
                    used: 1,
                },
                evidence: vec!["sha256:def".into()],
            }],
            evidence: vec![],
        };
        validate_observed_caps(&obs).unwrap();
    }
}
