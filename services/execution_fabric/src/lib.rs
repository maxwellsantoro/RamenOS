//! S10.4 simulation-only execution fabric for contract validation.

use std::collections::HashMap;

use artifact_store_schema::execution_fabric::{
    ComputeNodeManifestV0, ExecutionRequestV0, ExecutionStatusV0, ExecutionTraceEventV0,
    ExecutionTraceV0, NodeLoadSnapshotV0, ResourceLeaseV0, ResourceRequestV0,
};
use artifact_store_schema::semantic_state::{
    ComputeFabricSnapshotV0, ComputeNodeSnapshotV0, DuplicateExecutionGroupV0, ExecutionSummaryV0,
    ResourceLeaseSummaryV0,
};

pub const STATUS_OK: u32 = 0;
pub const STATUS_DENIED: u32 = 1;
pub const STATUS_NOT_FOUND: u32 = 2;

pub const LOCAL_NODE_ID: u64 = 0;
const REMOTE_NODE_ID: u64 = 1;
const HEAVY_CPU_THRESHOLD: u32 = 4;
const HEAVY_MEMORY_MB_THRESHOLD: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingTarget {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub target: RoutingTarget,
    pub node_id: u64,
}

#[derive(Debug, Default)]
pub struct SimulationExecutionFabric {
    nodes: Vec<ComputeNodeManifestV0>,
    leases: HashMap<u64, ResourceLeaseV0>,
    traces: HashMap<u64, ExecutionTraceV0>,
    duplicate_index: HashMap<String, u64>,
    next_lease_id: u64,
    next_execution_id: u64,
}

impl SimulationExecutionFabric {
    pub fn new() -> Self {
        Self {
            nodes: vec![
                ComputeNodeManifestV0 {
                    schema_version: 1,
                    node_id: LOCAL_NODE_ID,
                    name: "local".into(),
                    local: true,
                },
                ComputeNodeManifestV0 {
                    schema_version: 1,
                    node_id: REMOTE_NODE_ID,
                    name: "sim-remote".into(),
                    local: false,
                },
            ],
            leases: HashMap::new(),
            traces: HashMap::new(),
            duplicate_index: HashMap::new(),
            next_lease_id: 1,
            next_execution_id: 1,
        }
    }

    pub fn route_execution(&self, req: &ExecutionRequestV0) -> RoutingDecision {
        let heavy = req.resource_request.as_ref().is_some_and(|r| {
            r.cpu_cores >= HEAVY_CPU_THRESHOLD || r.memory_mb >= HEAVY_MEMORY_MB_THRESHOLD
        });
        if heavy {
            RoutingDecision {
                target: RoutingTarget::Remote,
                node_id: REMOTE_NODE_ID,
            }
        } else {
            RoutingDecision {
                target: RoutingTarget::Local,
                node_id: LOCAL_NODE_ID,
            }
        }
    }

    pub fn request_lease(
        &mut self,
        request: &ResourceRequestV0,
        force_deny: bool,
    ) -> Result<ResourceLeaseV0, u32> {
        if force_deny {
            return Err(STATUS_DENIED);
        }
        let route = self.route_execution(&ExecutionRequestV0 {
            schema_version: 1,
            program_id: "lease.probe".into(),
            artifact_ref: "sha256:lease".into(),
            action: "probe".into(),
            resource_request: Some(request.clone()),
        });
        let lease_id = self.next_lease_id;
        self.next_lease_id += 1;
        let lease = ResourceLeaseV0 {
            schema_version: 1,
            lease_id,
            node_id: route.node_id,
            resource_request: request.clone(),
            granted: true,
        };
        self.leases.insert(lease_id, lease.clone());
        Ok(lease)
    }

    pub fn submit_execution(
        &mut self,
        req: &ExecutionRequestV0,
        lease_id: u64,
    ) -> Result<(u64, ExecutionTraceV0), u32> {
        let duplicate_key = format!("{}:{}", req.program_id, req.artifact_ref);
        if let Some(existing) = self.duplicate_index.get(&duplicate_key).copied() {
            return Ok((existing, self.traces[&existing].clone()));
        }

        let lease = self.leases.get(&lease_id).ok_or(STATUS_NOT_FOUND)?;
        if !lease.granted {
            return Err(STATUS_DENIED);
        }

        let execution_id = self.next_execution_id;
        self.next_execution_id += 1;
        let trace = ExecutionTraceV0 {
            schema_version: 1,
            execution_id,
            node_id: lease.node_id,
            runner: "simulated".into(),
            events: vec![
                ExecutionTraceEventV0 {
                    seq: 1,
                    status: ExecutionStatusV0::Queued,
                    detail: "queued".into(),
                },
                ExecutionTraceEventV0 {
                    seq: 2,
                    status: ExecutionStatusV0::Running,
                    detail: "running".into(),
                },
            ],
        };
        self.traces.insert(execution_id, trace.clone());
        self.duplicate_index.insert(duplicate_key, execution_id);
        Ok((execution_id, trace))
    }

    pub fn load_snapshot(&self, node_id: u64) -> NodeLoadSnapshotV0 {
        NodeLoadSnapshotV0 {
            schema_version: 1,
            node_id,
            cpu_util_pct: if node_id == LOCAL_NODE_ID { 12.5 } else { 55.0 },
            memory_used_mb: if node_id == LOCAL_NODE_ID { 256 } else { 4096 },
            queued_executions: self
                .traces
                .values()
                .filter(|t| t.node_id == node_id)
                .count() as u32,
        }
    }

    pub fn semantic_snapshot(&self) -> ComputeFabricSnapshotV0 {
        let duplicate_groups = if self.duplicate_index.is_empty() {
            vec![]
        } else {
            vec![DuplicateExecutionGroupV0 {
                duplicate_key: self
                    .duplicate_index
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_default(),
                execution_ids: self.duplicate_index.values().copied().collect(),
            }]
        };

        ComputeFabricSnapshotV0 {
            nodes: self
                .nodes
                .iter()
                .map(|n| ComputeNodeSnapshotV0 {
                    node_id: n.node_id,
                    name: n.name.clone(),
                    local: n.local,
                    cpu_util_pct: self.load_snapshot(n.node_id).cpu_util_pct,
                    queued_executions: self.load_snapshot(n.node_id).queued_executions,
                })
                .collect(),
            active_leases: self
                .leases
                .values()
                .map(|l| ResourceLeaseSummaryV0 {
                    lease_id: l.lease_id,
                    node_id: l.node_id,
                    granted: l.granted,
                })
                .collect(),
            running_executions: self
                .traces
                .values()
                .filter(|t| t.events.last().map(|e| e.status) == Some(ExecutionStatusV0::Running))
                .map(|t| ExecutionSummaryV0 {
                    execution_id: t.execution_id,
                    program_id: "sim".into(),
                    status: "running".into(),
                    node_id: t.node_id,
                })
                .collect(),
            queued_executions: vec![],
            duplicate_groups,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheap_command_routes_local() {
        let fabric = SimulationExecutionFabric::new();
        let req = ExecutionRequestV0 {
            schema_version: 1,
            program_id: "demo".into(),
            artifact_ref: "sha256:abc".into(),
            action: "run".into(),
            resource_request: Some(ResourceRequestV0 {
                schema_version: 1,
                cpu_cores: 1,
                memory_mb: 512,
                gpu_profile: None,
            }),
        };
        assert_eq!(fabric.route_execution(&req).node_id, LOCAL_NODE_ID);
    }

    #[test]
    fn heavy_command_routes_remote() {
        let fabric = SimulationExecutionFabric::new();
        let req = ExecutionRequestV0 {
            schema_version: 1,
            program_id: "demo".into(),
            artifact_ref: "sha256:abc".into(),
            action: "run".into(),
            resource_request: Some(ResourceRequestV0 {
                schema_version: 1,
                cpu_cores: 8,
                memory_mb: 4096,
                gpu_profile: None,
            }),
        };
        assert_eq!(fabric.route_execution(&req).node_id, REMOTE_NODE_ID);
    }

    #[test]
    fn duplicate_execution_attaches_existing_id() {
        let mut fabric = SimulationExecutionFabric::new();
        let req = ExecutionRequestV0 {
            schema_version: 1,
            program_id: "demo".into(),
            artifact_ref: "sha256:abc".into(),
            action: "run".into(),
            resource_request: None,
        };
        let lease = fabric
            .request_lease(
                &ResourceRequestV0 {
                    schema_version: 1,
                    cpu_cores: 1,
                    memory_mb: 512,
                    gpu_profile: None,
                },
                false,
            )
            .unwrap();
        let (first, _) = fabric.submit_execution(&req, lease.lease_id).unwrap();
        let (second, _) = fabric.submit_execution(&req, lease.lease_id).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn denied_lease_fails_closed() {
        let mut fabric = SimulationExecutionFabric::new();
        let err = fabric.request_lease(
            &ResourceRequestV0 {
                schema_version: 1,
                cpu_cores: 1,
                memory_mb: 512,
                gpu_profile: None,
            },
            true,
        );
        assert_eq!(err, Err(STATUS_DENIED));
    }

    #[test]
    fn execution_trace_is_monotonic_after_submit() {
        let mut fabric = SimulationExecutionFabric::new();
        let req = ExecutionRequestV0 {
            schema_version: 1,
            program_id: "demo".into(),
            artifact_ref: "sha256:abc".into(),
            action: "run".into(),
            resource_request: None,
        };
        let lease = fabric
            .request_lease(
                &ResourceRequestV0 {
                    schema_version: 1,
                    cpu_cores: 1,
                    memory_mb: 512,
                    gpu_profile: None,
                },
                false,
            )
            .unwrap();
        let (_, trace) = fabric.submit_execution(&req, lease.lease_id).unwrap();
        artifact_store_schema::execution_fabric::validate_execution_trace(&trace)
            .expect("monotonic trace");
    }
}
