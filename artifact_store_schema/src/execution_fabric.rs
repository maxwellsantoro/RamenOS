//! S10.4 Execution Fabric schemas — canonical launch plans and execution traces.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

const EXECUTION_FABRIC_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct ExecutionFabricValidationError(pub String);

impl core::fmt::Display for ExecutionFabricValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExecutionFabricValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatusV0 {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceRequestV0 {
    pub schema_version: u32,
    pub cpu_cores: u32,
    pub memory_mb: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_profile: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceLeaseV0 {
    pub schema_version: u32,
    pub lease_id: u64,
    pub node_id: u64,
    pub resource_request: ResourceRequestV0,
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionRequestV0 {
    pub schema_version: u32,
    pub program_id: String,
    pub artifact_ref: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_request: Option<ResourceRequestV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeNodeManifestV0 {
    pub schema_version: u32,
    pub node_id: u64,
    pub name: String,
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeLoadSnapshotV0 {
    pub schema_version: u32,
    pub node_id: u64,
    pub cpu_util_pct: f32,
    pub memory_used_mb: u64,
    pub queued_executions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerSelectorV0 {
    pub runner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeSelectorV0 {
    pub node_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputMountV0 {
    pub mount_path: String,
    pub content_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputContractV0 {
    pub stdout_ref: Option<String>,
    pub stderr_ref: Option<String>,
    pub result_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RunnerConfigV0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
}

/// Runner-specific payload carried inside `RunnerConfigV0.config_json`.
///
/// Keeps the canonical `ExecutionLaunchPlanV0` shape stable while preserving
/// legacy runner fields (compat capsule, GPU quarantine, native wasm).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ExecutionRunnerConfigPayloadV0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat_capsule: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_quarantine: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_wasm: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_display_cap_token_high: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_display_cap_token_low: Option<u64>,
}

impl ExecutionRunnerConfigPayloadV0 {
    pub fn to_runner_config(&self) -> Result<RunnerConfigV0, serde_json::Error> {
        Ok(RunnerConfigV0 {
            config_json: Some(serde_json::to_string(self)?),
        })
    }

    pub fn from_runner_config(config: &RunnerConfigV0) -> Result<Option<Self>, serde_json::Error> {
        match config.config_json.as_deref() {
            Some(json) if !json.is_empty() => Ok(Some(serde_json::from_str(json)?)),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionLaunchPlanV0 {
    pub schema_version: u32,
    pub program_id: String,
    pub artifact_ref: String,
    pub runner: RunnerSelectorV0,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_request: Option<ResourceRequestV0>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<NodeSelectorV0>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_policy_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_mounts: Vec<InputMountV0>,
    pub output_contract: OutputContractV0,
    #[serde(default)]
    pub runner_config: RunnerConfigV0,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionTraceEventV0 {
    pub seq: u32,
    pub status: ExecutionStatusV0,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionTraceV0 {
    pub schema_version: u32,
    pub execution_id: u64,
    pub node_id: u64,
    pub runner: String,
    pub events: Vec<ExecutionTraceEventV0>,
}

pub fn validate_execution_request(
    req: &ExecutionRequestV0,
) -> Result<(), ExecutionFabricValidationError> {
    if req.schema_version != EXECUTION_FABRIC_SCHEMA_VERSION {
        return Err(ExecutionFabricValidationError(format!(
            "unsupported execution request schema version: {}",
            req.schema_version
        )));
    }
    if req.program_id.is_empty() || req.artifact_ref.is_empty() {
        return Err(ExecutionFabricValidationError(
            "execution request requires program_id and artifact_ref".into(),
        ));
    }
    Ok(())
}

pub fn validate_resource_lease(
    lease: &ResourceLeaseV0,
) -> Result<(), ExecutionFabricValidationError> {
    if lease.schema_version != EXECUTION_FABRIC_SCHEMA_VERSION {
        return Err(ExecutionFabricValidationError(format!(
            "unsupported resource lease schema version: {}",
            lease.schema_version
        )));
    }
    Ok(())
}

pub fn validate_execution_launch_plan(
    plan: &ExecutionLaunchPlanV0,
) -> Result<(), ExecutionFabricValidationError> {
    if plan.schema_version != EXECUTION_FABRIC_SCHEMA_VERSION {
        return Err(ExecutionFabricValidationError(format!(
            "unsupported launch plan schema version: {}",
            plan.schema_version
        )));
    }
    if plan.program_id.is_empty() || plan.artifact_ref.is_empty() || plan.runner.runner.is_empty() {
        return Err(ExecutionFabricValidationError(
            "launch plan requires program_id, artifact_ref, and runner".into(),
        ));
    }
    Ok(())
}

pub fn validate_execution_trace(
    trace: &ExecutionTraceV0,
) -> Result<(), ExecutionFabricValidationError> {
    if trace.schema_version != EXECUTION_FABRIC_SCHEMA_VERSION {
        return Err(ExecutionFabricValidationError(format!(
            "unsupported execution trace schema version: {}",
            trace.schema_version
        )));
    }
    for window in trace.events.windows(2) {
        if window[1].seq <= window[0].seq {
            return Err(ExecutionFabricValidationError(
                "execution trace events must be monotonic by seq".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan() -> ExecutionLaunchPlanV0 {
        ExecutionLaunchPlanV0 {
            schema_version: EXECUTION_FABRIC_SCHEMA_VERSION,
            program_id: "demo.app".into(),
            artifact_ref: "sha256:abc".into(),
            runner: RunnerSelectorV0 {
                runner: "native_wasm_v0".into(),
            },
            resource_request: None,
            node_selector: None,
            capability_policy_ref: None,
            input_mounts: vec![],
            output_contract: OutputContractV0 {
                stdout_ref: None,
                stderr_ref: None,
                result_ref: None,
            },
            runner_config: RunnerConfigV0::default(),
        }
    }

    #[test]
    fn launch_plan_validates() {
        validate_execution_launch_plan(&sample_plan()).expect("valid plan");
    }

    #[test]
    fn trace_requires_monotonic_seq() {
        let trace = ExecutionTraceV0 {
            schema_version: EXECUTION_FABRIC_SCHEMA_VERSION,
            execution_id: 1,
            node_id: 0,
            runner: "native_wasm_v0".into(),
            events: vec![
                ExecutionTraceEventV0 {
                    seq: 2,
                    status: ExecutionStatusV0::Running,
                    detail: "start".into(),
                },
                ExecutionTraceEventV0 {
                    seq: 1,
                    status: ExecutionStatusV0::Completed,
                    detail: "done".into(),
                },
            ],
        };
        assert!(validate_execution_trace(&trace).is_err());
    }
}
