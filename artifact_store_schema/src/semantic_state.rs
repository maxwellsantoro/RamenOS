//! Semantic State Snapshot schemas for AI-native introspection.
//!
//! These structures define the hierarchical graph of the operating system's
//! state, optimized for LLM ingestion (Markdown) and agentic processing (JSON).

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Root snapshot of the platform state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSnapshotV0 {
    /// UTC timestamp of snapshot capture.
    pub timestamp: String,
    /// System-wide metadata.
    pub system: SystemInfoV0,
    /// Active domains and their status.
    pub domains: Vec<DomainSnapshotV0>,
    /// Global registry of available harness interfaces.
    pub harnesses: Vec<HarnessInfoV0>,
    /// Optional compute-fabric visibility (S10.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_fabric: Option<ComputeFabricSnapshotV0>,
}

/// System-wide metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfoV0 {
    pub arch: String,
    pub boot_id: String,
    pub uptime_seconds: u64,
}

/// Snapshot of a single execution domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSnapshotV0 {
    pub id: u64,
    pub name: String,
    pub status: DomainStatusV0,
    /// Capabilities currently held by this domain.
    pub capabilities: Vec<CapInfoV0>,
    /// Resource usage metrics.
    pub metrics: Option<DomainMetricsV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainStatusV0 {
    Active,
    Quarantined,
    Stalled,
    Restarting,
}

/// Information about a specific capability held by a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapInfoV0 {
    pub handle: u64,
    pub interface: String,
    pub rights: Vec<String>,
}

/// Usage metrics for a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMetricsV0 {
    pub cpu_pct: f32,
    pub mem_mb: u64,
}

/// Information about an available system harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessInfoV0 {
    pub interface: String,
    /// Domain IDs providing this harness.
    pub providers: Vec<u64>,
}

/// S10.4 compute-fabric visibility embedded in platform snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeFabricSnapshotV0 {
    pub nodes: Vec<ComputeNodeSnapshotV0>,
    pub active_leases: Vec<ResourceLeaseSummaryV0>,
    pub running_executions: Vec<ExecutionSummaryV0>,
    pub queued_executions: Vec<ExecutionSummaryV0>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_groups: Vec<DuplicateExecutionGroupV0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeNodeSnapshotV0 {
    pub node_id: u64,
    pub name: String,
    pub local: bool,
    pub cpu_util_pct: f32,
    pub queued_executions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLeaseSummaryV0 {
    pub lease_id: u64,
    pub node_id: u64,
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummaryV0 {
    pub execution_id: u64,
    pub program_id: String,
    pub status: String,
    pub node_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateExecutionGroupV0 {
    pub duplicate_key: String,
    pub execution_ids: Vec<u64>,
}

/// Host-side domain inventory entry for snapshot construction.
///
/// Mirrors `domain_manager_v1` domain state without depending on the service crate.
#[derive(Debug, Clone)]
pub struct DomainInventoryEntry {
    pub id: u64,
    pub name: String,
    pub status: DomainStatusV0,
    pub capabilities: Vec<CapInfoV0>,
    pub metrics: Option<DomainMetricsV0>,
}

/// Map `domain_manager_v1` `state` field values to snapshot status.
pub fn domain_status_from_manager_state(state: u32) -> DomainStatusV0 {
    match state {
        1 => DomainStatusV0::Active,
        2 => DomainStatusV0::Restarting,
        _ => DomainStatusV0::Stalled,
    }
}

#[derive(Debug)]
pub struct SemanticStateValidationError(pub String);

impl core::fmt::Display for SemanticStateValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SemanticStateValidationError {}

pub fn validate_platform_snapshot(
    snapshot: &PlatformSnapshotV0,
) -> Result<(), SemanticStateValidationError> {
    if snapshot.timestamp.is_empty() {
        return Err(SemanticStateValidationError(
            "platform snapshot timestamp must not be empty".into(),
        ));
    }
    if snapshot.system.arch.is_empty() || snapshot.system.boot_id.is_empty() {
        return Err(SemanticStateValidationError(
            "platform snapshot system metadata must include arch and boot_id".into(),
        ));
    }
    Ok(())
}

pub const OBSERVER_INTERFACE: &str = "services.semantic_state_v1";

/// Filter a platform snapshot to the interfaces the viewer is allowed to observe.
///
/// Holders of `services.semantic_state_v1` receive the full snapshot. Other viewers
/// see the kernel domain plus domains/harnesses/capabilities matching their grants.
#[cfg(feature = "std")]
pub fn filter_platform_snapshot_for_viewer(
    snapshot: &PlatformSnapshotV0,
    viewer_interfaces: &[&str],
) -> PlatformSnapshotV0 {
    use std::collections::HashSet;

    if viewer_interfaces.contains(&OBSERVER_INTERFACE) {
        return snapshot.clone();
    }

    let allowed: HashSet<&str> = viewer_interfaces.iter().copied().collect();

    let domains = snapshot
        .domains
        .iter()
        .filter(|domain| domain.id == 0 || domain_visible_to_viewer(domain, &allowed))
        .map(|domain| filter_domain_capabilities(domain.clone(), &allowed, domain.id == 0))
        .collect();

    let harnesses = snapshot
        .harnesses
        .iter()
        .filter(|harness| allowed.contains(harness.interface.as_str()))
        .cloned()
        .collect();

    PlatformSnapshotV0 {
        timestamp: snapshot.timestamp.clone(),
        system: snapshot.system.clone(),
        domains,
        harnesses,
        compute_fabric: None,
    }
}

#[cfg(feature = "std")]
fn domain_visible_to_viewer(
    domain: &DomainSnapshotV0,
    allowed: &std::collections::HashSet<&str>,
) -> bool {
    domain
        .capabilities
        .iter()
        .any(|cap| allowed.contains(cap.interface.as_str()))
}

#[cfg(feature = "std")]
fn filter_domain_capabilities(
    mut domain: DomainSnapshotV0,
    allowed: &std::collections::HashSet<&str>,
    is_kernel: bool,
) -> DomainSnapshotV0 {
    if is_kernel {
        domain.capabilities.retain(|cap| {
            cap.interface == OBSERVER_INTERFACE || allowed.contains(cap.interface.as_str())
        });
    } else {
        domain
            .capabilities
            .retain(|cap| allowed.contains(cap.interface.as_str()));
    }
    domain
}

impl PlatformSnapshotV0 {
    /// Build a snapshot from structured domain inventory (host-side introspection path).
    pub fn from_inventory(
        timestamp: impl Into<String>,
        system: SystemInfoV0,
        domains: Vec<DomainInventoryEntry>,
        harnesses: Vec<HarnessInfoV0>,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            system,
            domains: domains
                .into_iter()
                .map(|entry| DomainSnapshotV0 {
                    id: entry.id,
                    name: entry.name,
                    status: entry.status,
                    capabilities: entry.capabilities,
                    metrics: entry.metrics,
                })
                .collect(),
            harnesses,
            compute_fabric: None,
        }
    }

    /// Default substrate snapshot: kernel domain plus core harness providers.
    pub fn default_substrate(arch: impl Into<String>, boot_id: impl Into<String>) -> Self {
        Self::from_inventory(
            "2026-02-21T10:00:00Z",
            SystemInfoV0 {
                arch: arch.into(),
                boot_id: boot_id.into(),
                uptime_seconds: 0,
            },
            vec![DomainInventoryEntry {
                id: 0,
                name: "kernel".to_string(),
                status: DomainStatusV0::Active,
                capabilities: vec![],
                metrics: None,
            }],
            vec![
                HarnessInfoV0 {
                    interface: OBSERVER_INTERFACE.to_string(),
                    providers: vec![0],
                },
                HarnessInfoV0 {
                    interface: "harness.shmem_control".to_string(),
                    providers: vec![0],
                },
            ],
        )
    }
    /// Generates an LLM-dense Markdown representation of the snapshot.
    pub fn to_markdown(&self) -> String {
        use core::fmt::Write;
        let mut md = String::new();
        let _ = writeln!(md, "# RamenOS Platform Snapshot");
        let _ = writeln!(
            md,
            "**Uptime:** {}s | **Boot ID:** {}",
            self.system.uptime_seconds, self.system.boot_id
        );
        let _ = writeln!(md);

        let _ = writeln!(md, "## Domains");
        for dom in &self.domains {
            let status_emoji = match dom.status {
                DomainStatusV0::Active => "🟢",
                DomainStatusV0::Quarantined => "🟡",
                DomainStatusV0::Stalled => "🔴",
                DomainStatusV0::Restarting => "🔄",
            };
            let _ = writeln!(
                md,
                "- [{}] {} **{}** ({:?})",
                dom.id, status_emoji, dom.name, dom.status
            );
            if let Some(m) = &dom.metrics {
                let _ = writeln!(md, "  - CPU: {:.1}% | MEM: {}MB", m.cpu_pct, m.mem_mb);
            }
            md.push_str("  - Capabilities: ");
            for (i, cap) in dom.capabilities.iter().enumerate() {
                if i > 0 {
                    md.push_str(", ");
                }
                md.push_str(&cap.interface);
            }
            let _ = writeln!(md);
        }

        let _ = writeln!(md);
        let _ = writeln!(md, "## Harnesses");
        for h in &self.harnesses {
            let _ = writeln!(
                md,
                "- **{}**: provided by domains {:?}",
                h.interface, h.providers
            );
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_inventory_roundtrip() {
        let snap = PlatformSnapshotV0::from_inventory(
            "2026-02-21T10:00:00Z",
            SystemInfoV0 {
                arch: "x86_64".to_string(),
                boot_id: "boot".to_string(),
                uptime_seconds: 10,
            },
            vec![DomainInventoryEntry {
                id: 1,
                name: "demo".to_string(),
                status: DomainStatusV0::Active,
                capabilities: vec![],
                metrics: None,
            }],
            vec![],
        );
        assert_eq!(snap.domains[0].name, "demo");
    }

    #[test]
    fn domain_status_maps_manager_states() {
        assert_eq!(domain_status_from_manager_state(1), DomainStatusV0::Active);
        assert_eq!(
            domain_status_from_manager_state(2),
            DomainStatusV0::Restarting
        );
    }

    #[test]
    fn filter_observer_sees_full_snapshot() {
        let snap = PlatformSnapshotV0::from_inventory(
            "2026-06-17T00:00:00Z",
            SystemInfoV0 {
                arch: "x86_64".to_string(),
                boot_id: "boot".to_string(),
                uptime_seconds: 1,
            },
            vec![
                DomainInventoryEntry {
                    id: 0,
                    name: "kernel".to_string(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![],
                    metrics: None,
                },
                DomainInventoryEntry {
                    id: 1,
                    name: "secret".to_string(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![CapInfoV0 {
                        handle: 1,
                        interface: "harness.secret".to_string(),
                        rights: vec!["read".into()],
                    }],
                    metrics: None,
                },
            ],
            vec![HarnessInfoV0 {
                interface: "harness.secret".into(),
                providers: vec![1],
            }],
        );
        let filtered = filter_platform_snapshot_for_viewer(&snap, &[OBSERVER_INTERFACE]);
        assert_eq!(filtered.domains.len(), 2);
        assert_eq!(filtered.harnesses.len(), 1);
    }

    #[test]
    fn filter_restricted_viewer_hides_ungranted_domains() {
        let snap = PlatformSnapshotV0::from_inventory(
            "2026-06-17T00:00:00Z",
            SystemInfoV0 {
                arch: "x86_64".to_string(),
                boot_id: "boot".to_string(),
                uptime_seconds: 1,
            },
            vec![
                DomainInventoryEntry {
                    id: 0,
                    name: "kernel".to_string(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![],
                    metrics: None,
                },
                DomainInventoryEntry {
                    id: 1,
                    name: "echo".to_string(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![CapInfoV0 {
                        handle: 1,
                        interface: "harness.echo".to_string(),
                        rights: vec!["read".into()],
                    }],
                    metrics: None,
                },
                DomainInventoryEntry {
                    id: 2,
                    name: "secret".to_string(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![CapInfoV0 {
                        handle: 2,
                        interface: "harness.secret".to_string(),
                        rights: vec!["read".into()],
                    }],
                    metrics: None,
                },
            ],
            vec![
                HarnessInfoV0 {
                    interface: "harness.echo".into(),
                    providers: vec![1],
                },
                HarnessInfoV0 {
                    interface: "harness.secret".into(),
                    providers: vec![2],
                },
            ],
        );
        let filtered = filter_platform_snapshot_for_viewer(&snap, &["harness.echo"]);
        assert_eq!(filtered.domains.len(), 2);
        assert_eq!(filtered.domains[1].name, "echo");
        assert_eq!(filtered.harnesses.len(), 1);
        assert_eq!(filtered.harnesses[0].interface, "harness.echo");
    }

    #[test]
    fn filter_restricted_viewer_hides_compute_fabric() {
        let mut snap = PlatformSnapshotV0::from_inventory(
            "2026-06-17T00:00:00Z",
            SystemInfoV0 {
                arch: "x86_64".to_string(),
                boot_id: "boot".to_string(),
                uptime_seconds: 1,
            },
            vec![DomainInventoryEntry {
                id: 0,
                name: "kernel".to_string(),
                status: DomainStatusV0::Active,
                capabilities: vec![],
                metrics: None,
            }],
            vec![],
        );
        snap.compute_fabric = Some(ComputeFabricSnapshotV0 {
            nodes: vec![ComputeNodeSnapshotV0 {
                node_id: 1,
                name: "local".into(),
                local: true,
                cpu_util_pct: 42.0,
                queued_executions: 1,
            }],
            active_leases: vec![ResourceLeaseSummaryV0 {
                lease_id: 7,
                node_id: 1,
                granted: true,
            }],
            running_executions: vec![ExecutionSummaryV0 {
                execution_id: 9,
                program_id: "secret.program".into(),
                status: "running".into(),
                node_id: 1,
            }],
            queued_executions: vec![],
            duplicate_groups: vec![DuplicateExecutionGroupV0 {
                duplicate_key: "hidden".into(),
                execution_ids: vec![9],
            }],
        });

        let filtered = filter_platform_snapshot_for_viewer(&snap, &["harness.echo"]);
        assert!(filtered.compute_fabric.is_none());
    }

    #[test]
    fn platform_snapshot_markdown_includes_header() {
        let snap = PlatformSnapshotV0 {
            timestamp: "2026-02-21T10:00:00Z".to_string(),
            system: SystemInfoV0 {
                arch: "x86_64".to_string(),
                boot_id: "boot-1".to_string(),
                uptime_seconds: 1,
            },
            domains: vec![],
            harnesses: vec![],
            compute_fabric: None,
        };
        assert!(snap.to_markdown().contains("# RamenOS Platform Snapshot"));
    }
}
