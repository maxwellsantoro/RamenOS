use serde::{Deserialize, Serialize};

use crate::prelude::*;

const TRACE_SCHEMA_VERSION: u32 = 1;
const MAX_EVENT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub struct TraceValidationError(pub String);

impl core::fmt::Display for TraceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TraceValidationError {}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TraceType {
    ProtocolTrace,
    ScenarioTrace,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceArtifactV0 {
    pub schema_version: u32,
    pub trace_type: TraceType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_trace: Option<ProtocolTrace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_trace: Option<ScenarioTrace>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolTrace {
    pub metadata: ProtocolTraceMetadata,
    pub events: Vec<ProtocolTraceEvent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolTraceMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule_image: Option<String>,
    pub harness_name: String,
    pub harness_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_bundle_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TraceDir {
    Request,
    Response,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolTraceEvent {
    pub seq: u64,
    pub dir: TraceDir,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    pub bytes_hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioTrace {
    pub metadata: ScenarioTraceMetadata,
    pub events: Vec<ScenarioTraceEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioTraceMetadata {
    pub scenario_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScenarioTraceEvent {
    pub seq: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

pub fn validate_trace_artifact(trace: &TraceArtifactV0) -> Result<(), TraceValidationError> {
    if trace.schema_version != TRACE_SCHEMA_VERSION {
        return Err(TraceValidationError(format!(
            "trace schema_version unsupported: {}",
            trace.schema_version
        )));
    }

    match trace.trace_type {
        TraceType::ProtocolTrace => {
            if trace.protocol_trace.is_none() {
                return Err(TraceValidationError(
                    "protocol_trace missing for protocol_trace type".into(),
                ));
            }
            if trace.scenario_trace.is_some() {
                return Err(TraceValidationError(
                    "scenario_trace must be absent for protocol_trace type".into(),
                ));
            }
            validate_protocol_trace(trace.protocol_trace.as_ref().unwrap())
        }
        TraceType::ScenarioTrace => {
            if trace.scenario_trace.is_none() {
                return Err(TraceValidationError(
                    "scenario_trace missing for scenario_trace type".into(),
                ));
            }
            if trace.protocol_trace.is_some() {
                return Err(TraceValidationError(
                    "protocol_trace must be absent for scenario_trace type".into(),
                ));
            }
            validate_scenario_trace(trace.scenario_trace.as_ref().unwrap())
        }
    }
}

fn validate_protocol_trace(trace: &ProtocolTrace) -> Result<(), TraceValidationError> {
    if trace.metadata.harness_name.trim().is_empty() {
        return Err(TraceValidationError(
            "protocol_trace.metadata.harness_name required".into(),
        ));
    }
    if trace.events.is_empty() {
        return Err(TraceValidationError("protocol_trace.events empty".into()));
    }
    let mut last_seq: Option<u64> = None;
    for (idx, event) in trace.events.iter().enumerate() {
        if let Some(prev) = last_seq {
            if event.seq <= prev {
                return Err(TraceValidationError(format!(
                    "protocol_trace.events[{}] sequence not monotonic",
                    idx
                )));
            }
        }
        last_seq = Some(event.seq);
        if event.bytes_hex.len() % 2 != 0 {
            return Err(TraceValidationError(format!(
                "protocol_trace.events[{}] bytes_hex has odd length",
                idx
            )));
        }

        let decoded = hex::decode(&event.bytes_hex).map_err(|_| {
            TraceValidationError(format!(
                "protocol_trace.events[{}] bytes_hex invalid hex",
                idx
            ))
        })?;
        if decoded.len() > MAX_EVENT_BYTES {
            return Err(TraceValidationError(format!(
                "protocol_trace.events[{}] bytes_hex too large",
                idx
            )));
        }
    }
    Ok(())
}

fn validate_scenario_trace(trace: &ScenarioTrace) -> Result<(), TraceValidationError> {
    if trace.metadata.scenario_id.trim().is_empty() {
        return Err(TraceValidationError(
            "scenario_trace.metadata.scenario_id required".into(),
        ));
    }
    if trace.events.is_empty() {
        return Err(TraceValidationError("scenario_trace.events empty".into()));
    }
    let mut last_seq: Option<u64> = None;
    for (idx, event) in trace.events.iter().enumerate() {
        if let Some(prev) = last_seq {
            if event.seq <= prev {
                return Err(TraceValidationError(format!(
                    "scenario_trace.events[{}] sequence not monotonic",
                    idx
                )));
            }
        }
        last_seq = Some(event.seq);
        if event.name.trim().is_empty() {
            return Err(TraceValidationError(format!(
                "scenario_trace.events[{}] name required",
                idx
            )));
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn validate_protocol_trace_ok() {
        let trace = TraceArtifactV0 {
            schema_version: 1,
            trace_type: TraceType::ProtocolTrace,
            protocol_trace: Some(ProtocolTrace {
                metadata: ProtocolTraceMetadata {
                    trace_id: None,
                    timestamp_start: None,
                    timestamp_end: None,
                    capsule_id: Some("demo".into()),
                    capsule_image: None,
                    harness_name: "ping_harness".into(),
                    harness_version: 0,
                    policy_bundle_id: None,
                },
                events: vec![
                    ProtocolTraceEvent {
                        seq: 1,
                        dir: TraceDir::Request,
                        op: Some("ping".into()),
                        bytes_hex: "70696e67".into(),
                        result: Some("ok".into()),
                        notes: None,
                    },
                    ProtocolTraceEvent {
                        seq: 2,
                        dir: TraceDir::Response,
                        op: Some("pong".into()),
                        bytes_hex: "706f6e67".into(),
                        result: Some("ok".into()),
                        notes: None,
                    },
                ],
            }),
            scenario_trace: None,
        };
        validate_trace_artifact(&trace).unwrap();
    }

    #[test]
    fn validate_protocol_trace_missing_harness_name() {
        let trace = TraceArtifactV0 {
            schema_version: 1,
            trace_type: TraceType::ProtocolTrace,
            protocol_trace: Some(ProtocolTrace {
                metadata: ProtocolTraceMetadata {
                    trace_id: None,
                    timestamp_start: None,
                    timestamp_end: None,
                    capsule_id: None,
                    capsule_image: None,
                    harness_name: "".into(),
                    harness_version: 0,
                    policy_bundle_id: None,
                },
                events: vec![ProtocolTraceEvent {
                    seq: 1,
                    dir: TraceDir::Request,
                    op: None,
                    bytes_hex: "00".into(),
                    result: None,
                    notes: None,
                }],
            }),
            scenario_trace: None,
        };
        assert!(validate_trace_artifact(&trace).is_err());
    }

    #[test]
    fn validate_scenario_trace_ok() {
        let trace = TraceArtifactV0 {
            schema_version: 1,
            trace_type: TraceType::ScenarioTrace,
            protocol_trace: None,
            scenario_trace: Some(ScenarioTrace {
                metadata: ScenarioTraceMetadata {
                    scenario_id: "portal.file_picker.ro".into(),
                    timestamp_start: None,
                    timestamp_end: None,
                },
                events: vec![ScenarioTraceEvent {
                    seq: 1,
                    name: "protocol_trace_ref".into(),
                    payload: Some(serde_json::json!({"content_id": "sha256:abc"})),
                }],
            }),
        };
        validate_trace_artifact(&trace).unwrap();
    }
}
