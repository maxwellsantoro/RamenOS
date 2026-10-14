//! Launch plan parsing for legacy and canonical execution-fabric schemas.

use artifact_store_schema::execution_fabric::{
    ExecutionLaunchPlanV0, ExecutionRunnerConfigPayloadV0, ResourceRequestV0,
};
use serde::Deserialize;

use crate::compat_runner::CompatCapsuleConfig;
use crate::gpu_runner::GpuQuarantineConfig;
use crate::native_wasm_runner::NativeWasmConfig;

pub const LOCAL_FABRIC_NODE_ID: u64 = 0;

#[derive(Debug)]
pub struct ResolvedLaunchPlan {
    pub program_id: String,
    pub runner: String,
    pub artifact_ref: String,
    pub notes: Option<String>,
    pub compat_capsule: Option<CompatCapsuleConfig>,
    pub gpu_quarantine: Option<GpuQuarantineConfig>,
    pub expected_display_cap_token_high: Option<u64>,
    pub expected_display_cap_token_low: Option<u64>,
    pub native_wasm: Option<NativeWasmConfig>,
    pub resource_request: Option<ResourceRequestV0>,
    pub fabric_execution_id: Option<u64>,
    pub fabric_node_id: u64,
}

#[derive(Debug, Deserialize)]
struct LegacyLaunchPlan {
    program_id: String,
    runner: String,
    artifact_ref: String,
    notes: Option<String>,
    #[serde(default)]
    compat_capsule: Option<CompatCapsuleConfig>,
    #[serde(default)]
    gpu_quarantine: Option<GpuQuarantineConfig>,
    #[serde(default)]
    expected_display_cap_token_high: Option<u64>,
    #[serde(default)]
    expected_display_cap_token_low: Option<u64>,
    #[serde(default)]
    native_wasm: Option<NativeWasmConfig>,
}

pub fn parse_launch_plan(raw: &str) -> Result<ResolvedLaunchPlan, String> {
    if let Ok(canonical) = serde_json::from_str::<ExecutionLaunchPlanV0>(raw) {
        artifact_store_schema::execution_fabric::validate_execution_launch_plan(&canonical)
            .map_err(|err| err.0)?;
        return resolve_canonical(canonical);
    }

    let legacy: LegacyLaunchPlan =
        serde_json::from_str(raw).map_err(|err| format!("invalid launch plan JSON: {err}"))?;
    Ok(ResolvedLaunchPlan {
        program_id: legacy.program_id,
        runner: legacy.runner,
        artifact_ref: legacy.artifact_ref,
        notes: legacy.notes,
        compat_capsule: legacy.compat_capsule,
        gpu_quarantine: legacy.gpu_quarantine,
        expected_display_cap_token_high: legacy.expected_display_cap_token_high,
        expected_display_cap_token_low: legacy.expected_display_cap_token_low,
        native_wasm: legacy.native_wasm,
        resource_request: None,
        fabric_execution_id: None,
        fabric_node_id: LOCAL_FABRIC_NODE_ID,
    })
}

fn resolve_canonical(canonical: ExecutionLaunchPlanV0) -> Result<ResolvedLaunchPlan, String> {
    let runner = canonical.runner.runner.clone();
    let payload = ExecutionRunnerConfigPayloadV0::from_runner_config(&canonical.runner_config)
        .map_err(|err| format!("invalid runner_config payload: {err}"))?;

    let (
        notes,
        compat_capsule,
        gpu_quarantine,
        expected_display_cap_token_high,
        expected_display_cap_token_low,
        native_wasm,
    ) = if let Some(payload) = payload {
        let compat_capsule = payload
            .compat_capsule
            .as_ref()
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|err| format!("invalid compat_capsule config: {err}"))?;
        let gpu_quarantine = payload
            .gpu_quarantine
            .as_ref()
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|err| format!("invalid gpu_quarantine config: {err}"))?;
        let native_wasm = payload
            .native_wasm
            .as_ref()
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|err| format!("invalid native_wasm config: {err}"))?
            .or_else(|| {
                if runner == "native_wasm_v0" {
                    Some(NativeWasmConfig {
                        kernel_ipc: "/tmp/kernel_ipc".into(),
                        kernel_ipc_transport: Default::default(),
                        domain_manager_ipc: None,
                        domain_id: 1,
                        timeout_ms: 30_000,
                    })
                } else {
                    None
                }
            });
        (
            payload.notes,
            compat_capsule,
            gpu_quarantine,
            payload.expected_display_cap_token_high,
            payload.expected_display_cap_token_low,
            native_wasm,
        )
    } else {
        (
            None,
            None,
            None,
            None,
            None,
            if runner == "native_wasm_v0" {
                Some(NativeWasmConfig {
                    kernel_ipc: "/tmp/kernel_ipc".into(),
                    kernel_ipc_transport: Default::default(),
                    domain_manager_ipc: None,
                    domain_id: 1,
                    timeout_ms: 30_000,
                })
            } else {
                None
            },
        )
    };

    Ok(ResolvedLaunchPlan {
        program_id: canonical.program_id,
        runner,
        artifact_ref: canonical.artifact_ref,
        notes,
        compat_capsule,
        gpu_quarantine,
        expected_display_cap_token_high,
        expected_display_cap_token_low,
        native_wasm,
        resource_request: canonical.resource_request,
        fabric_execution_id: None,
        fabric_node_id: canonical
            .node_selector
            .as_ref()
            .map(|node| node.node_id)
            .unwrap_or(LOCAL_FABRIC_NODE_ID),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_execution_launch_plan() {
        let raw = r#"{
            "schema_version": 1,
            "program_id": "demo.app",
            "artifact_ref": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "runner": { "runner": "native_wasm_v0" },
            "output_contract": {},
            "runner_config": {}
        }"#;
        let plan = parse_launch_plan(raw).expect("canonical plan");
        assert_eq!(plan.runner, "native_wasm_v0");
        assert_eq!(plan.program_id, "demo.app");
    }

    #[test]
    fn parses_canonical_with_runner_config_payload() {
        let raw = r#"{
            "schema_version": 1,
            "program_id": "ramen.compat.hello",
            "artifact_ref": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "runner": { "runner": "linux_vm_v0" },
            "resource_request": { "schema_version": 1, "cpu_cores": 1, "memory_mb": 512 },
            "node_selector": { "node_id": 0 },
            "output_contract": {},
            "runner_config": {
                "config_json": "{\"notes\":\"S2 compat\",\"compat_capsule\":{\"kernel_content_id\":\"sha256:aaaa\",\"initrd_content_id\":\"sha256:bbbb\",\"artifact_disks\":[{\"content_id\":\"sha256:cccc\",\"mount_policy\":\"read_only\",\"device_type\":\"virtio_blk\"}],\"cmdline\":\"console=ttyS0\",\"resources\":{\"memory_mb\":512,\"cpus\":1}}}"
            }
        }"#;
        let plan = parse_launch_plan(raw).expect("canonical compat plan");
        assert_eq!(plan.runner, "linux_vm_v0");
        assert!(plan.compat_capsule.is_some());
        assert_eq!(plan.notes.as_deref(), Some("S2 compat"));
        assert_eq!(
            plan.resource_request.as_ref().map(|r| r.memory_mb),
            Some(512)
        );
    }

    #[test]
    fn parses_legacy_launch_plan() {
        let raw = r#"{
            "program_id": "demo",
            "runner": "linux_vm_v0",
            "artifact_ref": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        }"#;
        let plan = parse_launch_plan(raw).expect("legacy plan");
        assert_eq!(plan.runner, "linux_vm_v0");
    }
}

#[cfg(test)]
mod review_configuration {
    use super::*;
    fn plan(payload: serde_json::Value, node: u64) -> String {
        serde_json::json!({"schema_version":1,"program_id":"review","artifact_ref":format!("sha256:{}", "0".repeat(64)),"runner":{"runner":"native_wasm_v0"},"node_selector":{"node_id":node},"output_contract":{},"runner_config":{"config_json":payload.to_string()}}).to_string()
    }
    #[test]
    fn review_launch_invalid_explicit_config_denies_then_valid_config_preserves_identity() {
        for field in ["native_wasm", "compat_capsule", "gpu_quarantine"] {
            let payload = serde_json::json!({field: {"domain_id":"INVALID"}});
            assert!(
                parse_launch_plan(&plan(payload, 0)).is_err(),
                "malformed {field} must deny"
            );
        }
        let payload = serde_json::json!({"native_wasm":{"kernel_ipc":"/review/kernel","domain_manager_ipc":"/review/broker","domain_id":7,"timeout_ms":100}});
        let p = parse_launch_plan(&plan(payload, 0)).unwrap();
        let config = p.native_wasm.unwrap();
        assert_eq!(config.domain_id, 7);
        assert_eq!(config.kernel_ipc, "/review/kernel");
        assert_eq!(config.domain_manager_ipc.as_deref(), Some("/review/broker"));
        assert_eq!(config.timeout_ms, 100);
    }
    #[test]
    fn review_launch_node_selection_never_supplies_domain_identity() {
        for node in [0, 8, u64::MAX] {
            for payload in [
                serde_json::json!({}),
                serde_json::json!({"notes":"explicit empty wrapper"}),
            ] {
                let p = parse_launch_plan(&plan(payload, node)).unwrap();
                assert_eq!(p.fabric_node_id, node);
                assert_eq!(p.native_wasm.unwrap().domain_id, 1);
            }
        }
        let raw=serde_json::json!({"schema_version":1,"program_id":"review","artifact_ref":"sha256:review","runner":{"runner":"native_wasm_v0"},"node_selector":{"node_id":0},"output_contract":{},"runner_config":{}}).to_string();
        assert_eq!(
            parse_launch_plan(&raw)
                .unwrap()
                .native_wasm
                .unwrap()
                .domain_id,
            1
        );
    }
}
