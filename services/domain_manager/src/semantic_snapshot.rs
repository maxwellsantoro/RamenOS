//! Live semantic snapshot construction from domain-manager inventory.

use artifact_store_schema::semantic_state::{
    CapInfoV0, DomainInventoryEntry, HarnessInfoV0, OBSERVER_INTERFACE, PlatformSnapshotV0,
    SystemInfoV0, domain_status_from_manager_state,
};

/// Minimal domain record for host-side snapshot construction.
#[derive(Debug, Clone)]
pub struct LiveDomainRecord {
    pub id: u64,
    pub name: String,
    pub manager_state: u32,
    pub capabilities: Vec<CapInfoV0>,
}

/// Build a platform snapshot from live domain-manager inventory.
pub fn build_live_platform_snapshot(
    timestamp: impl Into<String>,
    arch: impl Into<String>,
    boot_id: impl Into<String>,
    uptime_seconds: u64,
    records: &[LiveDomainRecord],
) -> PlatformSnapshotV0 {
    let domains: Vec<DomainInventoryEntry> = records
        .iter()
        .map(|record| DomainInventoryEntry {
            id: record.id,
            name: record.name.clone(),
            status: domain_status_from_manager_state(record.manager_state),
            capabilities: record.capabilities.clone(),
            metrics: None,
        })
        .collect();

    PlatformSnapshotV0::from_inventory(
        timestamp,
        SystemInfoV0 {
            arch: arch.into(),
            boot_id: boot_id.into(),
            uptime_seconds,
        },
        domains,
        default_harness_registry(),
    )
}

fn default_harness_registry() -> Vec<HarnessInfoV0> {
    vec![
        HarnessInfoV0 {
            interface: OBSERVER_INTERFACE.into(),
            providers: vec![0],
        },
        HarnessInfoV0 {
            interface: "harness.shmem_control".into(),
            providers: vec![0],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_snapshot_includes_running_domains() {
        let snap = build_live_platform_snapshot(
            "2026-06-17T00:00:00Z",
            "x86_64",
            "boot-live",
            42,
            &[LiveDomainRecord {
                id: 1,
                name: "demo".into(),
                manager_state: 1,
                capabilities: vec![],
            }],
        );
        assert_eq!(snap.domains.len(), 1);
        assert_eq!(snap.domains[0].name, "demo");
    }

    #[test]
    fn inventory_snapshot_publish_enqueues_reactor_event() {
        use kernel_api::generated::semantic_state_v1::Subscribe;
        use semantic_state::{EVENT_MASK_DOMAIN_STATE_CHANGED, SemanticReactor};

        let mut reactor = SemanticReactor::new();
        reactor.handle_subscribe_with_viewer(
            Subscribe {
                request_id: 1,
                event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
            },
            vec!["services.semantic_state_v1".into()],
        );
        let snap = build_live_platform_snapshot(
            "2026-06-17T00:00:00Z",
            "x86_64",
            "boot-live",
            0,
            &[LiveDomainRecord {
                id: 1,
                name: "demo".into(),
                manager_state: 1,
                capabilities: vec![],
            }],
        );
        reactor.publish_domain_inventory_changed(&snap);
        assert_eq!(reactor.pending_count(), 1);
        assert_eq!(reactor.reactor_tick().len(), 1);
    }
}
