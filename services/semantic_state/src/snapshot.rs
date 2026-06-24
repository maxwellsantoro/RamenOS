//! Snapshot construction for the Semantic State Service.

use artifact_store_schema::semantic_state::{
    CapInfoV0, DomainInventoryEntry, DomainStatusV0, HarnessInfoV0, OBSERVER_INTERFACE,
    PlatformSnapshotV0, SystemInfoV0, domain_status_from_manager_state,
};

#[cfg(target_arch = "wasm32")]
use alloc::string::ToString;

/// Semantic snapshot format selector (matches `semantic_state_v1` IDL).
pub const FORMAT_JSON: u32 = 0;
pub const FORMAT_MARKDOWN: u32 = 1;

/// Service status codes for snapshot delivery.
pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_FORMAT: u32 = 1;
pub const STATUS_SHMEM_UNAVAILABLE: u32 = 2;

/// Build a platform snapshot from domain-manager-style inventory.
pub fn build_platform_snapshot(
    arch: &str,
    boot_id: &str,
    uptime_seconds: u64,
    domains: Vec<DomainInventoryEntry>,
) -> PlatformSnapshotV0 {
    PlatformSnapshotV0::from_inventory(
        current_timestamp_stub(),
        SystemInfoV0 {
            arch: arch.to_string(),
            boot_id: boot_id.to_string(),
            uptime_seconds,
        },
        domains,
        default_harness_registry(),
    )
}

/// Build the default platform snapshot for the v0 substrate.
pub fn build_default_snapshot() -> PlatformSnapshotV0 {
    build_platform_snapshot(
        "x86_64",
        "ramen-boot-001",
        5678,
        vec![DomainInventoryEntry {
            id: 0,
            name: "kernel".to_string(),
            status: DomainStatusV0::Active,
            capabilities: vec![],
            metrics: None,
        }],
    )
}

/// Map a `domain_manager_v1` domain record into snapshot inventory.
pub fn domain_inventory_from_manager(
    domain_id: u64,
    name: &str,
    manager_state: u32,
    capabilities: Vec<CapInfoV0>,
) -> DomainInventoryEntry {
    DomainInventoryEntry {
        id: domain_id,
        name: name.to_string(),
        status: domain_status_from_manager_state(manager_state),
        capabilities,
        metrics: None,
    }
}

fn default_harness_registry() -> Vec<HarnessInfoV0> {
    vec![
        HarnessInfoV0 {
            interface: OBSERVER_INTERFACE.to_string(),
            providers: vec![0],
        },
        HarnessInfoV0 {
            interface: "harness.shmem_control".to_string(),
            providers: vec![0],
        },
    ]
}

/// Placeholder until kernel uptime clock is wired into the WASM service.
fn current_timestamp_stub() -> String {
    "2026-02-21T10:00:00Z".to_string()
}

/// Serialize a snapshot for the requested wire format.
pub fn serialize_snapshot(snapshot: &PlatformSnapshotV0, format: u32) -> Result<Vec<u8>, u32> {
    match format {
        FORMAT_JSON => serde_json::to_vec(snapshot).map_err(|_| STATUS_INVALID_FORMAT),
        FORMAT_MARKDOWN => Ok(snapshot.to_markdown().into_bytes()),
        _ => Err(STATUS_INVALID_FORMAT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_has_kernel_domain() {
        let snap = build_default_snapshot();
        assert_eq!(snap.domains.len(), 1);
        assert_eq!(snap.domains[0].name, "kernel");
    }

    #[test]
    fn markdown_snapshot_contains_platform_header() {
        let snap = build_default_snapshot();
        let md = snap.to_markdown();
        assert!(md.contains("# RamenOS Platform Snapshot"));
        assert!(md.contains("kernel"));
    }

    #[test]
    fn json_roundtrip_preserves_arch() {
        let snap = build_default_snapshot();
        let bytes = serialize_snapshot(&snap, FORMAT_JSON).expect("json serialize");
        let decoded: PlatformSnapshotV0 = serde_json::from_slice(&bytes).expect("json deserialize");
        assert_eq!(decoded.system.arch, "x86_64");
    }

    #[test]
    fn invalid_format_is_rejected() {
        let snap = build_default_snapshot();
        assert_eq!(serialize_snapshot(&snap, 99), Err(STATUS_INVALID_FORMAT));
    }

    #[test]
    fn domain_inventory_maps_manager_running_state() {
        let entry = domain_inventory_from_manager(7, "demo", 1, vec![]);
        assert_eq!(entry.id, 7);
        assert_eq!(entry.status, DomainStatusV0::Active);
    }

    #[test]
    fn domain_inventory_maps_manager_restarting_state() {
        let entry = domain_inventory_from_manager(3, "gpu", 2, vec![]);
        assert_eq!(entry.status, DomainStatusV0::Restarting);
    }

    #[test]
    fn platform_snapshot_includes_multiple_domains() {
        let snap = build_platform_snapshot(
            "aarch64",
            "boot-42",
            120,
            vec![
                domain_inventory_from_manager(0, "kernel", 1, vec![]),
                domain_inventory_from_manager(
                    1,
                    "demo",
                    1,
                    vec![CapInfoV0 {
                        handle: 0x1000,
                        interface: "harness.echo".to_string(),
                        rights: vec!["read".to_string(), "write".to_string()],
                    }],
                ),
            ],
        );
        assert_eq!(snap.domains.len(), 2);
        assert_eq!(snap.system.arch, "aarch64");
        assert_eq!(snap.domains[1].capabilities.len(), 1);
    }
}
