//! S10.4.1 execution-fabric policy hook for supervisor dispatch.
//!
//! v0 policy is always-local: the simulation fabric records leases/traces, but
//! dispatch remains on the local node regardless of routing hints.

use artifact_store_schema::execution_fabric::{ExecutionRequestV0, ResourceRequestV0};
use execution_fabric::LOCAL_NODE_ID;
use execution_fabric::SimulationExecutionFabric;

use crate::launch_plan::ResolvedLaunchPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricDispatchDecision {
    pub node_id: u64,
    pub execution_id: Option<u64>,
    pub routed_remote: bool,
    pub dispatch_locally: bool,
}

/// Consult the simulation fabric and apply the always-local dispatch policy.
pub fn consult_always_local(plan: &ResolvedLaunchPlan) -> Result<FabricDispatchDecision, u32> {
    let mut fabric = SimulationExecutionFabric::new();
    let resource_request = plan.resource_request.clone().unwrap_or(ResourceRequestV0 {
        schema_version: 1,
        cpu_cores: 1,
        memory_mb: 512,
        gpu_profile: None,
    });

    let lease = fabric.request_lease(&resource_request, false)?;
    let routed_remote = fabric
        .route_execution(&ExecutionRequestV0 {
            schema_version: 1,
            program_id: plan.program_id.clone(),
            artifact_ref: plan.artifact_ref.clone(),
            action: "dispatch".into(),
            resource_request: Some(resource_request),
        })
        .node_id
        != LOCAL_NODE_ID;

    let (execution_id, _) = fabric.submit_execution(
        &ExecutionRequestV0 {
            schema_version: 1,
            program_id: plan.program_id.clone(),
            artifact_ref: plan.artifact_ref.clone(),
            action: "dispatch".into(),
            resource_request: plan.resource_request.clone(),
        },
        lease.lease_id,
    )?;

    Ok(FabricDispatchDecision {
        node_id: LOCAL_NODE_ID,
        execution_id: Some(execution_id),
        routed_remote,
        dispatch_locally: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use artifact_store_schema::execution_fabric::ResourceRequestV0;

    #[test]
    fn fabric_policy_always_local_despite_heavy_request() {
        let plan = ResolvedLaunchPlan {
            program_id: "heavy.demo".into(),
            runner: "native_stub".into(),
            artifact_ref: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .into(),
            notes: None,
            compat_capsule: None,
            gpu_quarantine: None,
            expected_display_cap_token_high: None,
            expected_display_cap_token_low: None,
            native_wasm: None,
            resource_request: Some(ResourceRequestV0 {
                schema_version: 1,
                cpu_cores: 8,
                memory_mb: 4096,
                gpu_profile: None,
            }),
            fabric_execution_id: None,
            fabric_node_id: LOCAL_NODE_ID,
        };

        let decision = consult_always_local(&plan).expect("fabric consult");
        assert!(decision.dispatch_locally);
        assert_eq!(decision.node_id, LOCAL_NODE_ID);
        assert!(decision.routed_remote);
        assert!(decision.execution_id.is_some());
    }
}
