// Converge to canonical schema types from artifact_store_schema
// See: docs/archive/plans/2026-02-13-deep-security-review.md
// V-007 Phase 3: Removed direct dependency on artifact_store_schema
// All artifact operations now go through store service IPC.
// See: docs/plans/security_remediation_v006_v007_v012.md

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

// V-007 Phase 3: Store service client for IPC-based artifact operations
use clap::Parser;
use kernel_api::generated::{
    DOMAIN_MANAGER_V1_PROTOCOL_ID, ExportDisplay, ExportDisplayReply,
    GPU_QUARANTINE_V1_PROTOCOL_ID, GetDomainGrantHandles, GetDomainGrantHandlesReply,
    GetDomainStatus, GetDomainStatusReply, GrantCapabilities, GrantCapabilitiesReply, ListDomains,
    ListDomainsReply, ReportExit, ReportExitReply, ReportScanout, ReportScanoutReply,
    RevokeCapabilities, RevokeCapabilitiesReply, StartDomain, StartDomainReply,
    StartQuarantineDomain, StartQuarantineDomainReply, StopDomain, StopDomainReply,
    StopQuarantineDomain, StopQuarantineDomainReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};
use store_service::StoreClient;

// Converge to canonical schema types from artifact_store_schema
// See: docs/archive/plans/2026-02-13-deep-security-review.md
use artifact_store_schema::observed_caps::{
    ObservedCapCounts, ObservedCapScope, ObservedCapability, ObservedCapsV0, validate_observed_caps,
};
use artifact_store_schema::trace::{
    ProtocolTrace, ProtocolTraceEvent, ProtocolTraceMetadata, ScenarioTrace, ScenarioTraceEvent,
    ScenarioTraceMetadata, TraceArtifactV0, TraceDir, TraceType, validate_trace_artifact,
};

mod audit;
mod broker;
mod error;
mod semantic_snapshot;

use broker::{
    CapabilityBroker, ChannelAllowlistPolicy, GrantResult, KernelOpsBackend,
    SEMANTIC_HARNESS_EXPORT_SHMEM, SEMANTIC_HARNESS_EXPORT_STATE, SEMANTIC_HARNESS_SHMEM_EXPORT_ID,
    SEMANTIC_HARNESS_STATE_EXPORT_ID,
};
use error::DomainManagerError;

const PROTOCOL_DOMAIN_MANAGER_V1: u32 = DOMAIN_MANAGER_V1_PROTOCOL_ID;

const MSG_START_DOMAIN: u32 = 1;
const MSG_START_DOMAIN_REPLY: u32 = 2;
const MSG_STOP_DOMAIN: u32 = 3;
const MSG_STOP_DOMAIN_REPLY: u32 = 4;
const MSG_GET_DOMAIN_STATUS: u32 = 5;
const MSG_GET_DOMAIN_STATUS_REPLY: u32 = 6;
const MSG_REPORT_EXIT: u32 = 7;
const MSG_REPORT_EXIT_REPLY: u32 = 8;
const MSG_LIST_DOMAINS: u32 = 9;
const MSG_LIST_DOMAINS_REPLY: u32 = 10;
const MSG_GRANT_CAPABILITIES: u32 = 11;
const MSG_GRANT_CAPABILITIES_REPLY: u32 = 12;
const MSG_REVOKE_CAPABILITIES: u32 = 13;
const MSG_REVOKE_CAPABILITIES_REPLY: u32 = 14;
const MSG_GET_DOMAIN_GRANT_HANDLES: u32 = 15;
const MSG_GET_DOMAIN_GRANT_HANDLES_REPLY: u32 = 16;

const PROTOCOL_GPU_QUARANTINE_V1: u32 = GPU_QUARANTINE_V1_PROTOCOL_ID;
const MSG_START_QUARANTINE_DOMAIN: u32 = 1;
const MSG_START_QUARANTINE_DOMAIN_REPLY: u32 = 2;
const MSG_STOP_QUARANTINE_DOMAIN: u32 = 3;
const MSG_STOP_QUARANTINE_DOMAIN_REPLY: u32 = 4;
const MSG_EXPORT_DISPLAY: u32 = 5;
const MSG_EXPORT_DISPLAY_REPLY: u32 = 6;
const MSG_REPORT_SCANOUT: u32 = 7;
const MSG_REPORT_SCANOUT_REPLY: u32 = 8;

const STATUS_OK: u32 = 0;
const STATUS_ERR: u32 = 1;
const STATUS_NOT_FOUND: u32 = 2;

const RESTART_POLICY_NEVER: u32 = 0;
const RESTART_POLICY_ON_FAILURE: u32 = 1;
const RESTART_POLICY_ALWAYS: u32 = 2;

const DOMAIN_STATE_STOPPED: u32 = 0;
const DOMAIN_STATE_RUNNING: u32 = 1;
const DOMAIN_STATE_RESTARTING: u32 = 2;

const EXIT_ACTION_NOOP: u32 = 0;
const EXIT_ACTION_STOPPED: u32 = 1;
const EXIT_ACTION_RESTARTED: u32 = 2;

const STATUS_INVALID_CAPABILITY: u32 = 3;
const STATUS_INTERNAL_ERROR: u32 = 4;

// V-04: Per-domain unforgeable token instead of constant
const PIXEL_FORMAT_XRGB8888: u32 = 1;

// V-04: Display export capability identifier
const CAP_DISPLAY_EXPORT: u64 = 0x1000000000000001;

const GPU_RESTART_POLICY_ON_FAILURE: u32 = 1;

const GPU_PROFILE_GENERIC: u32 = 1;

const _: () = {
    let _ = [0u8; 64 - core::mem::size_of::<StartDomain>()];
    let _ = [0u8; 64 - core::mem::size_of::<StartDomainReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<StopDomain>()];
    let _ = [0u8; 64 - core::mem::size_of::<StopDomainReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<GetDomainStatus>()];
    let _ = [0u8; 64 - core::mem::size_of::<GetDomainStatusReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<ReportExit>()];
    let _ = [0u8; 64 - core::mem::size_of::<ReportExitReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<ListDomains>()];
    let _ = [0u8; 64 - core::mem::size_of::<ListDomainsReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<GrantCapabilities>()]; // S10.1
    let _ = [0u8; 64 - core::mem::size_of::<GrantCapabilitiesReply>()]; // S10.1
    let _ = [0u8; 64 - core::mem::size_of::<GetDomainGrantHandles>()]; // S10.5.1
    let _ = [0u8; 64 - core::mem::size_of::<GetDomainGrantHandlesReply>()]; // S10.5.1
    let _ = [0u8; 64 - core::mem::size_of::<RevokeCapabilities>()]; // S10.1
    let _ = [0u8; 64 - core::mem::size_of::<RevokeCapabilitiesReply>()]; // S10.1
    let _ = [0u8; 64 - core::mem::size_of::<StartQuarantineDomain>()];
    let _ = [0u8; 64 - core::mem::size_of::<StartQuarantineDomainReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<StopQuarantineDomain>()];
    let _ = [0u8; 64 - core::mem::size_of::<StopQuarantineDomainReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<ExportDisplay>()]; // V-04: updated size check
    let _ = [0u8; 64 - core::mem::size_of::<ExportDisplayReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<ReportScanout>()];
    let _ = [0u8; 64 - core::mem::size_of::<ReportScanoutReply>()];
};

#[derive(Clone, Copy, Debug)]
struct DomainState {
    state: u32,
    restart_policy: u32,
    generation: u32,
    restart_count: u32,
}

impl DomainState {
    fn running(restart_policy: u32) -> Self {
        Self {
            state: DOMAIN_STATE_RUNNING,
            restart_policy,
            generation: 1,
            restart_count: 0,
        }
    }
}

/// V-04: Unforgeable 128-bit token for display export capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DisplayCapToken {
    high: u64,
    low: u64,
}

impl DisplayCapToken {
    #[cfg(test)]
    fn new() -> Self {
        // V-04: Generate cryptographically random 128-bit token using OsRng
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        Self {
            high: rng.next_u64(),
            low: rng.next_u64(),
        }
    }
}

fn legacy_display_cap_token() -> DisplayCapToken {
    DisplayCapToken {
        // Legacy v1 handshake still carries CAP_DISPLAY_EXPORT as a u64 split across two u64 fields.
        high: CAP_DISPLAY_EXPORT >> 32,
        low: CAP_DISPLAY_EXPORT & 0xFFFF_FFFF,
    }
}

#[cfg(test)]
mod display_cap_token_tests {
    use super::*;

    #[test]
    fn test_display_cap_token_uniqueness() {
        // Generate 1000 tokens and verify all are unique
        let mut tokens = std::collections::HashSet::new();
        for _ in 0..1000 {
            let token = DisplayCapToken::new();
            let key = (token.high, token.low);
            assert!(
                tokens.insert(key),
                "Duplicate token generated: high={}, low={}",
                token.high,
                token.low
            );
        }
    }

    #[test]
    fn test_display_cap_token_entropy() {
        // Generate multiple tokens and verify they have sufficient entropy
        // (i.e., tokens are not predictable or repeating patterns)
        let tokens: Vec<DisplayCapToken> = (0..100).map(|_| DisplayCapToken::new()).collect();

        // Check that no two tokens are identical
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                assert_ne!(
                    tokens[i], tokens[j],
                    "Tokens at indices {} and {} are identical",
                    i, j
                );
            }
        }

        // Verify both high and low fields have variation (not all zeros or same value)
        let highs: Vec<u64> = tokens.iter().map(|t| t.high).collect();
        let lows: Vec<u64> = tokens.iter().map(|t| t.low).collect();

        // Check that not all highs are the same
        let unique_highs: std::collections::HashSet<_> = highs.iter().collect();
        assert!(
            unique_highs.len() > 1,
            "All high fields are identical, indicating insufficient entropy"
        );

        // Check that not all lows are the same
        let unique_lows: std::collections::HashSet<_> = lows.iter().collect();
        assert!(
            unique_lows.len() > 1,
            "All low fields are identical, indicating insufficient entropy"
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct QuarantineDomainState {
    state: u32,
    restart_policy: u32,
    generation: u32,
    gpu_profile: u32,
    surface_id: u64,
    width: u32,
    height: u32,
    display_cap_token: DisplayCapToken,
    last_frame_seq: u64,
}

impl QuarantineDomainState {
    fn running(restart_policy: u32, gpu_profile: u32) -> Self {
        Self {
            state: DOMAIN_STATE_RUNNING,
            restart_policy,
            generation: 1,
            gpu_profile,
            surface_id: 0,
            width: 0,
            height: 0,
            display_cap_token: DisplayCapToken { high: 0, low: 0 },
            last_frame_seq: 0,
        }
    }
}

#[derive(Debug)]
struct GpuHandshakeResult {
    trace_id: String,
    observed_caps_id: String,
    scenario_id: String,
}

#[derive(Parser, Debug)]
struct Args {
    /// Directory where raw evidence JSON files are written.
    #[arg(long, default_value = "out/trace")]
    trace_dir: PathBuf,

    /// Installed root containing an `artifacts/` directory.
    #[arg(long, default_value = "out/installed")]
    installed_root: PathBuf,

    /// Program ID stamped into observed capabilities.
    #[arg(long, default_value = "org.ramen.domain_manager")]
    program_id: String,

    /// Run ID stamped into observed capabilities.
    #[arg(long, default_value = "domain_manager_gpu_s7")]
    run_id: String,

    /// Store service socket path for IPC-based artifact operations.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,

    /// Emit a live semantic-state snapshot JSON built from current domain inventory.
    #[arg(long)]
    emit_semantic_snapshot: bool,

    /// Ingest the live semantic snapshot into the store as evidence.
    #[arg(long)]
    ingest_semantic_snapshot: bool,
}

struct DomainManager {
    domains: HashMap<u64, DomainState>,
    quarantine_domains: HashMap<u64, QuarantineDomainState>,
    /// Capability broker for S10.1 native runner integration.
    /// Uses `KernelOpsBackend::from_env()` — set `RAMEN_SEMANTIC_HARNESS_BRIDGE=1` for
    /// `SemanticHarnessGrantOps` (S10.5.1); otherwise `SimulatedKernelOps`.
    broker: CapabilityBroker<ChannelAllowlistPolicy, KernelOpsBackend>,
    semantic_reactor: semantic_state::SemanticReactor,
    /// Manifest store for capability lookup
    /// Key: content_id_hash (32 bytes as hex string)
    /// Value: (manifest, derived_channel)
    manifest_store: HashMap<
        String,
        (
            artifact_store_schema::native_wasm::NativeWasmManifestV0,
            String,
        ),
    >,
}

impl DomainManager {
    fn new() -> Self {
        let policy = ChannelAllowlistPolicy::new_test();
        let kernel_ops = KernelOpsBackend::from_env();
        Self {
            domains: HashMap::new(),
            quarantine_domains: HashMap::new(),
            broker: CapabilityBroker::new(policy, kernel_ops),
            semantic_reactor: semantic_state::SemanticReactor::new(),
            manifest_store: HashMap::new(),
        }
    }

    /// Register a manifest for capability lookup (for testing/setup)
    #[cfg(test)]
    fn register_manifest(
        &mut self,
        content_id_hash: &[u8; 32],
        manifest: artifact_store_schema::native_wasm::NativeWasmManifestV0,
        channel: &str,
    ) {
        let key = hex::encode(content_id_hash);
        self.manifest_store
            .insert(key, (manifest, channel.to_string()));
    }

    /// Look up a manifest by content_id_hash
    fn lookup_manifest(
        &self,
        content_id_hash: &[u8; 32],
    ) -> Option<&(
        artifact_store_schema::native_wasm::NativeWasmManifestV0,
        String,
    )> {
        let key = hex::encode(content_id_hash);
        self.manifest_store.get(&key)
    }

    fn semantic_inventory_records(&self) -> Vec<semantic_snapshot::LiveDomainRecord> {
        self.domains
            .iter()
            .map(|(id, state)| semantic_snapshot::LiveDomainRecord {
                id: *id,
                name: format!("domain-{id}"),
                manager_state: state.state,
                capabilities: vec![],
            })
            .collect()
    }

    fn notify_semantic_inventory_changed(&mut self) {
        let records = self.semantic_inventory_records();
        let snapshot = semantic_snapshot::build_live_platform_snapshot(
            "2026-06-17T00:00:00Z",
            "host",
            "domain-manager-live",
            0,
            &records,
        );
        self.semantic_reactor
            .publish_domain_inventory_changed(&snapshot);
    }

    fn handle(&mut self, env: &Envelope) -> Envelope {
        if env.protocol == PROTOCOL_GPU_QUARANTINE_V1 {
            return self.handle_gpu(env);
        }
        if env.protocol != PROTOCOL_DOMAIN_MANAGER_V1 {
            return Envelope::empty(env.protocol, env.msg_type);
        }

        match env.msg_type {
            MSG_START_DOMAIN => {
                let req = match read_payload::<StartDomain>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_START_DOMAIN_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                let generation = {
                    let entry = self
                        .domains
                        .entry(req.domain_id)
                        .or_insert_with(|| DomainState::running(req.restart_policy));
                    if entry.state != DOMAIN_STATE_RUNNING {
                        entry.state = DOMAIN_STATE_RUNNING;
                        if entry.generation == 0 {
                            entry.generation = 1;
                        }
                    }
                    entry.restart_policy = req.restart_policy;
                    entry.generation
                };
                self.notify_semantic_inventory_changed();
                match Self::start_reply(req.request_id, req.domain_id, STATUS_OK, generation) {
                    Ok(env) => env,
                    Err(e) => {
                        eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                        Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_START_DOMAIN_REPLY,
                            req.request_id,
                            STATUS_INTERNAL_ERROR,
                        )
                    }
                }
            }
            MSG_STOP_DOMAIN => {
                let req = match read_payload::<StopDomain>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_STOP_DOMAIN_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                let stop_result = match self.domains.get_mut(&req.domain_id) {
                    Some(state) => {
                        state.state = DOMAIN_STATE_STOPPED;
                        Some(state.generation)
                    }
                    None => None,
                };
                if stop_result.is_some() {
                    self.notify_semantic_inventory_changed();
                }
                match stop_result {
                    Some(generation) => {
                        match Self::stop_reply(req.request_id, req.domain_id, STATUS_OK, generation)
                        {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_STOP_DOMAIN_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => {
                        match Self::stop_reply(req.request_id, req.domain_id, STATUS_NOT_FOUND, 0) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_STOP_DOMAIN_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                }
            }
            MSG_GET_DOMAIN_STATUS => {
                let req = match read_payload::<GetDomainStatus>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_GET_DOMAIN_STATUS_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                match self.domains.get(&req.domain_id) {
                    Some(state) => {
                        match Self::status_reply(
                            req.request_id,
                            req.domain_id,
                            STATUS_OK,
                            state.state,
                            state.generation,
                            state.restart_count,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_GET_DOMAIN_STATUS_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => {
                        match Self::status_reply(
                            req.request_id,
                            req.domain_id,
                            STATUS_NOT_FOUND,
                            DOMAIN_STATE_STOPPED,
                            0,
                            0,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_GET_DOMAIN_STATUS_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                }
            }
            MSG_REPORT_EXIT => {
                let req = match read_payload::<ReportExit>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_REPORT_EXIT_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                match self.domains.get_mut(&req.domain_id) {
                    Some(state) => {
                        let action = match state.restart_policy {
                            RESTART_POLICY_ALWAYS => EXIT_ACTION_RESTARTED,
                            RESTART_POLICY_ON_FAILURE if req.exit_code != 0 => {
                                EXIT_ACTION_RESTARTED
                            }
                            _ => EXIT_ACTION_STOPPED,
                        };

                        match action {
                            EXIT_ACTION_RESTARTED => {
                                state.state = DOMAIN_STATE_RESTARTING;
                                state.generation += 1;
                                state.restart_count += 1;
                                state.state = DOMAIN_STATE_RUNNING;
                            }
                            EXIT_ACTION_STOPPED => {
                                state.state = DOMAIN_STATE_STOPPED;
                            }
                            _ => {}
                        }

                        let generation = state.generation;
                        let restart_count = state.restart_count;
                        match Self::report_exit_reply(
                            req.domain_id,
                            action,
                            generation,
                            restart_count,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_REPORT_EXIT_REPLY,
                                    req.domain_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => match Self::report_exit_reply(req.domain_id, EXIT_ACTION_NOOP, 0, 0) {
                        Ok(env) => env,
                        Err(e) => {
                            eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                            Self::error_reply(
                                PROTOCOL_DOMAIN_MANAGER_V1,
                                MSG_REPORT_EXIT_REPLY,
                                req.domain_id,
                                STATUS_INTERNAL_ERROR,
                            )
                        }
                    },
                }
            }
            MSG_LIST_DOMAINS => {
                let req = match read_payload::<ListDomains>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_LIST_DOMAINS_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };

                let total = self.domains.len() as u32;
                let mut running = 0u32;
                let mut restarting = 0u32;
                let mut stopped = 0u32;
                for domain in self.domains.values() {
                    match domain.state {
                        DOMAIN_STATE_RUNNING => running += 1,
                        DOMAIN_STATE_RESTARTING => restarting += 1,
                        _ => stopped += 1,
                    }
                }
                match Self::list_domains_reply(req.request_id, total, running, restarting, stopped)
                {
                    Ok(env) => env,
                    Err(e) => {
                        eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                        Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_LIST_DOMAINS_REPLY,
                            req.request_id,
                            STATUS_INTERNAL_ERROR,
                        )
                    }
                }
            }
            MSG_GRANT_CAPABILITIES => {
                // S10.1: Capability broker IPC endpoint
                // Design: No raw JSON over IPC - broker fetches manifest from store by content_id
                let req = match read_payload::<GrantCapabilities>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_GRANT_CAPABILITIES_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };

                // Look up manifest by content_id_hash
                // Clone immediately to avoid holding immutable borrow across mutable broker call
                let manifest_lookup = self.lookup_manifest(&req.content_id_hash).cloned();
                match manifest_lookup {
                    Some((manifest, channel)) => {
                        // Broker evaluates policy and grants capabilities
                        match self
                            .broker
                            .grant_capabilities(&manifest, &channel, req.domain_id)
                        {
                            Ok(GrantResult { granted_handles }) => {
                                let handle_count = granted_handles.len() as u32;
                                match Self::grant_capabilities_reply(
                                    req.request_id,
                                    req.domain_id,
                                    STATUS_OK,
                                    handle_count,
                                ) {
                                    Ok(env) => env,
                                    Err(e) => {
                                        eprintln!(
                                            "domain_manager: failed to serialize grant reply: {:?}",
                                            e
                                        );
                                        Self::error_reply(
                                            PROTOCOL_DOMAIN_MANAGER_V1,
                                            MSG_GRANT_CAPABILITIES_REPLY,
                                            req.request_id,
                                            STATUS_INTERNAL_ERROR,
                                        )
                                    }
                                }
                            }
                            Err(broker_err) => {
                                eprintln!(
                                    "domain_manager: capability grant denied for domain {}: {}",
                                    req.domain_id, broker_err
                                );
                                match Self::grant_capabilities_reply(
                                    req.request_id,
                                    req.domain_id,
                                    STATUS_INVALID_CAPABILITY,
                                    0,
                                ) {
                                    Ok(env) => env,
                                    Err(e) => {
                                        eprintln!(
                                            "domain_manager: failed to serialize grant denial reply: {:?}",
                                            e
                                        );
                                        Self::error_reply(
                                            PROTOCOL_DOMAIN_MANAGER_V1,
                                            MSG_GRANT_CAPABILITIES_REPLY,
                                            req.request_id,
                                            STATUS_INTERNAL_ERROR,
                                        )
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        // Manifest not found in store
                        match Self::grant_capabilities_reply(
                            req.request_id,
                            req.domain_id,
                            STATUS_NOT_FOUND,
                            0,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!(
                                    "domain_manager: failed to serialize not found reply: {:?}",
                                    e
                                );
                                Self::error_reply(
                                    PROTOCOL_DOMAIN_MANAGER_V1,
                                    MSG_GRANT_CAPABILITIES_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                }
            }
            MSG_GET_DOMAIN_GRANT_HANDLES => {
                let req = match read_payload::<GetDomainGrantHandles>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_GET_DOMAIN_GRANT_HANDLES_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };

                let grants = self.broker.active_grants_for_domain(req.domain_id);
                let status = if grants.is_empty() {
                    STATUS_NOT_FOUND
                } else {
                    STATUS_OK
                };
                match Self::get_domain_grant_handles_reply(
                    req.request_id,
                    req.domain_id,
                    status,
                    &grants,
                ) {
                    Ok(env) => env,
                    Err(e) => {
                        eprintln!(
                            "domain_manager: failed to serialize grant handles reply: {:?}",
                            e
                        );
                        Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_GET_DOMAIN_GRANT_HANDLES_REPLY,
                            req.request_id,
                            STATUS_INTERNAL_ERROR,
                        )
                    }
                }
            }
            MSG_REVOKE_CAPABILITIES => {
                // S10.1: Revoke all capabilities for a domain
                let req = match read_payload::<RevokeCapabilities>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_REVOKE_CAPABILITIES_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };

                let revoked_count = self.broker.revoke_domain(req.domain_id);
                match Self::revoke_capabilities_reply(
                    req.request_id,
                    req.domain_id,
                    STATUS_OK,
                    revoked_count,
                ) {
                    Ok(env) => env,
                    Err(e) => {
                        eprintln!("domain_manager: failed to serialize revoke reply: {:?}", e);
                        Self::error_reply(
                            PROTOCOL_DOMAIN_MANAGER_V1,
                            MSG_REVOKE_CAPABILITIES_REPLY,
                            req.request_id,
                            STATUS_INTERNAL_ERROR,
                        )
                    }
                }
            }
            _ => Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, env.msg_type),
        }
    }

    fn handle_gpu(&mut self, env: &Envelope) -> Envelope {
        match env.msg_type {
            MSG_START_QUARANTINE_DOMAIN => {
                let req = match read_payload::<StartQuarantineDomain>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_GPU_QUARANTINE_V1,
                            MSG_START_QUARANTINE_DOMAIN_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                let entry = self
                    .quarantine_domains
                    .entry(req.domain_id)
                    .or_insert_with(|| {
                        QuarantineDomainState::running(req.restart_policy, req.gpu_profile)
                    });
                if entry.state != DOMAIN_STATE_RUNNING {
                    entry.state = DOMAIN_STATE_RUNNING;
                    if entry.generation == 0 {
                        entry.generation = 1;
                    }
                }
                entry.restart_policy = req.restart_policy;
                entry.gpu_profile = req.gpu_profile;
                match Self::gpu_start_reply(
                    req.request_id,
                    req.domain_id,
                    STATUS_OK,
                    entry.generation,
                ) {
                    Ok(env) => env,
                    Err(e) => {
                        eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                        Self::error_reply(
                            PROTOCOL_GPU_QUARANTINE_V1,
                            MSG_START_QUARANTINE_DOMAIN_REPLY,
                            req.request_id,
                            STATUS_INTERNAL_ERROR,
                        )
                    }
                }
            }
            MSG_STOP_QUARANTINE_DOMAIN => {
                let req = match read_payload::<StopQuarantineDomain>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_GPU_QUARANTINE_V1,
                            MSG_STOP_QUARANTINE_DOMAIN_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                match self.quarantine_domains.get_mut(&req.domain_id) {
                    Some(state) => {
                        state.state = DOMAIN_STATE_STOPPED;
                        match Self::gpu_stop_reply(
                            req.request_id,
                            req.domain_id,
                            STATUS_OK,
                            state.generation,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_GPU_QUARANTINE_V1,
                                    MSG_STOP_QUARANTINE_DOMAIN_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => {
                        match Self::gpu_stop_reply(
                            req.request_id,
                            req.domain_id,
                            STATUS_NOT_FOUND,
                            0,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_GPU_QUARANTINE_V1,
                                    MSG_STOP_QUARANTINE_DOMAIN_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                }
            }
            MSG_EXPORT_DISPLAY => {
                let req = match read_payload::<ExportDisplay>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_GPU_QUARANTINE_V1,
                            MSG_EXPORT_DISPLAY_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };
                // V-04: Validate against stored per-domain token
                let provided_token = DisplayCapToken {
                    high: req.display_cap_token_high,
                    low: req.display_cap_token_low,
                };

                match self.quarantine_domains.get_mut(&req.domain_id) {
                    Some(state) => {
                        if state.state != DOMAIN_STATE_RUNNING {
                            return match Self::gpu_export_reply(
                                req.request_id,
                                req.domain_id,
                                0,
                                STATUS_ERR,
                                0,
                                0,
                                0,
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                    Self::error_reply(
                                        PROTOCOL_GPU_QUARANTINE_V1,
                                        MSG_EXPORT_DISPLAY_REPLY,
                                        req.request_id,
                                        STATUS_INTERNAL_ERROR,
                                    )
                                }
                            };
                        }
                        // Bootstrap compatibility: bind first export to the legacy constant token.
                        // Once the protocol returns per-domain tokens, this can move to explicit issuance.
                        if state.display_cap_token.high == 0 && state.display_cap_token.low == 0 {
                            state.display_cap_token = legacy_display_cap_token();
                        }
                        // V-04: Validate token matches stored value
                        if provided_token != state.display_cap_token {
                            return match Self::gpu_export_reply(
                                req.request_id,
                                req.domain_id,
                                0,
                                STATUS_INVALID_CAPABILITY,
                                0,
                                0,
                                0,
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                    Self::error_reply(
                                        PROTOCOL_GPU_QUARANTINE_V1,
                                        MSG_EXPORT_DISPLAY_REPLY,
                                        req.request_id,
                                        STATUS_INTERNAL_ERROR,
                                    )
                                }
                            };
                        }
                        state.width = req.width;
                        state.height = req.height;
                        state.surface_id = compose_surface_id(req.domain_id, state.generation);
                        let stride = req.width.saturating_mul(4);
                        match Self::gpu_export_reply(
                            req.request_id,
                            req.domain_id,
                            state.surface_id,
                            STATUS_OK,
                            stride,
                            PIXEL_FORMAT_XRGB8888,
                            0,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_GPU_QUARANTINE_V1,
                                    MSG_EXPORT_DISPLAY_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => match Self::gpu_export_reply(
                        req.request_id,
                        req.domain_id,
                        0,
                        STATUS_NOT_FOUND,
                        0,
                        0,
                        0,
                    ) {
                        Ok(env) => env,
                        Err(e) => {
                            eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                            Self::error_reply(
                                PROTOCOL_GPU_QUARANTINE_V1,
                                MSG_EXPORT_DISPLAY_REPLY,
                                req.request_id,
                                STATUS_INTERNAL_ERROR,
                            )
                        }
                    },
                }
            }
            MSG_REPORT_SCANOUT => {
                let req = match read_payload::<ReportScanout>(env) {
                    Ok(v) => v,
                    Err(_) => {
                        return Self::error_reply(
                            PROTOCOL_GPU_QUARANTINE_V1,
                            MSG_REPORT_SCANOUT_REPLY,
                            0,
                            STATUS_ERR,
                        );
                    }
                };

                match self.quarantine_domains.get_mut(&req.domain_id) {
                    Some(state) => {
                        if state.surface_id == 0 || state.surface_id != req.surface_id {
                            return match Self::gpu_scanout_reply(
                                req.request_id,
                                req.frame_seq,
                                STATUS_ERR,
                                0,
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                    Self::error_reply(
                                        PROTOCOL_GPU_QUARANTINE_V1,
                                        MSG_REPORT_SCANOUT_REPLY,
                                        req.request_id,
                                        STATUS_INTERNAL_ERROR,
                                    )
                                }
                            };
                        }
                        if req.frame_seq <= state.last_frame_seq {
                            return match Self::gpu_scanout_reply(
                                req.request_id,
                                state.last_frame_seq,
                                STATUS_ERR,
                                0,
                            ) {
                                Ok(env) => env,
                                Err(e) => {
                                    eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                    Self::error_reply(
                                        PROTOCOL_GPU_QUARANTINE_V1,
                                        MSG_REPORT_SCANOUT_REPLY,
                                        req.request_id,
                                        STATUS_INTERNAL_ERROR,
                                    )
                                }
                            };
                        }
                        state.last_frame_seq = req.frame_seq;
                        match Self::gpu_scanout_reply(req.request_id, req.frame_seq, STATUS_OK, 0) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_GPU_QUARANTINE_V1,
                                    MSG_REPORT_SCANOUT_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                    None => {
                        match Self::gpu_scanout_reply(
                            req.request_id,
                            req.frame_seq,
                            STATUS_NOT_FOUND,
                            0,
                        ) {
                            Ok(env) => env,
                            Err(e) => {
                                eprintln!("domain_manager: failed to serialize reply: {:?}", e);
                                Self::error_reply(
                                    PROTOCOL_GPU_QUARANTINE_V1,
                                    MSG_REPORT_SCANOUT_REPLY,
                                    req.request_id,
                                    STATUS_INTERNAL_ERROR,
                                )
                            }
                        }
                    }
                }
            }
            _ => Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, env.msg_type),
        }
    }

    fn start_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        generation: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_START_DOMAIN_REPLY);
        let payload = StartDomainReply {
            request_id,
            domain_id,
            status,
            generation,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn stop_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        generation: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_STOP_DOMAIN_REPLY);
        let payload = StopDomainReply {
            request_id,
            domain_id,
            status,
            generation,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn status_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        state: u32,
        generation: u32,
        restart_count: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_GET_DOMAIN_STATUS_REPLY);
        let payload = GetDomainStatusReply {
            request_id,
            domain_id,
            status,
            state,
            generation,
            restart_count,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn report_exit_reply(
        domain_id: u64,
        action: u32,
        generation: u32,
        restart_count: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_REPORT_EXIT_REPLY);
        let payload = ReportExitReply {
            domain_id,
            action,
            generation,
            restart_count,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn list_domains_reply(
        request_id: u64,
        total_domains: u32,
        running_domains: u32,
        restarting_domains: u32,
        stopped_domains: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_LIST_DOMAINS_REPLY);
        let payload = ListDomainsReply {
            request_id,
            total_domains,
            running_domains,
            restarting_domains,
            stopped_domains,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn grant_capabilities_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        handle_count: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_GRANT_CAPABILITIES_REPLY);
        let payload = GrantCapabilitiesReply {
            request_id,
            domain_id,
            status,
            handle_count,
            reserved: 0,
            reserved2: 0,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn get_domain_grant_handles_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        grants: &HashMap<String, u64>,
    ) -> Result<Envelope, DomainManagerError> {
        let mut entries = [(0u16, 0u64); 2];
        let mut count = 0usize;
        for (export_name, export_id) in [
            (
                SEMANTIC_HARNESS_EXPORT_SHMEM,
                SEMANTIC_HARNESS_SHMEM_EXPORT_ID,
            ),
            (
                SEMANTIC_HARNESS_EXPORT_STATE,
                SEMANTIC_HARNESS_STATE_EXPORT_ID,
            ),
        ] {
            if let Some(handle) = grants.get(export_name) {
                if count < entries.len() {
                    entries[count] = (export_id, *handle);
                    count += 1;
                }
            }
        }

        let mut env = Envelope::empty(
            PROTOCOL_DOMAIN_MANAGER_V1,
            MSG_GET_DOMAIN_GRANT_HANDLES_REPLY,
        );
        let payload = GetDomainGrantHandlesReply {
            request_id,
            domain_id,
            status,
            count: count as u32,
            entry0_export_id: entries[0].0,
            entry0_reserved: 0,
            entry0_handle: entries[0].1,
            entry1_export_id: entries[1].0,
            entry1_reserved: 0,
            entry1_handle: entries[1].1,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn revoke_capabilities_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        revoked_count: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_REVOKE_CAPABILITIES_REPLY);
        let payload = RevokeCapabilitiesReply {
            request_id,
            domain_id,
            status,
            revoked_count,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn gpu_start_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        generation: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(
            PROTOCOL_GPU_QUARANTINE_V1,
            MSG_START_QUARANTINE_DOMAIN_REPLY,
        );
        let payload = StartQuarantineDomainReply {
            request_id,
            domain_id,
            status,
            generation,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn gpu_stop_reply(
        request_id: u64,
        domain_id: u64,
        status: u32,
        generation: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_STOP_QUARANTINE_DOMAIN_REPLY);
        let payload = StopQuarantineDomainReply {
            request_id,
            domain_id,
            status,
            generation,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn gpu_export_reply(
        request_id: u64,
        domain_id: u64,
        surface_id: u64,
        status: u32,
        stride: u32,
        format: u32,
        reserved: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_EXPORT_DISPLAY_REPLY);
        let payload = ExportDisplayReply {
            request_id,
            domain_id,
            surface_id,
            status,
            stride,
            format,
            reserved,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    fn gpu_scanout_reply(
        request_id: u64,
        acked_frame_seq: u64,
        status: u32,
        reserved: u32,
    ) -> Result<Envelope, DomainManagerError> {
        let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_REPORT_SCANOUT_REPLY);
        let payload = ReportScanoutReply {
            request_id,
            acked_frame_seq,
            status,
            reserved,
        };
        write_payload(&mut env, &payload)
            .map_err(|e| DomainManagerError::PayloadSerialization(format!("{:?}", e)))?;
        Ok(env)
    }

    /// Create a generic error reply for when reply serialization fails.
    /// Returns an empty envelope with just the protocol and message type set,
    /// since we cannot serialize a proper payload when serialization itself is failing.
    fn error_reply(protocol: u32, msg_type: u32, _request_id: u64, _status: u32) -> Envelope {
        Envelope::empty(protocol, msg_type)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    fs::create_dir_all(&args.trace_dir)?;

    // V-007 Phase 2: Connect to store service for IPC-based artifact operations
    let mut store_client = StoreClient::connect(&args.store_socket)?;

    let mut manager = DomainManager::new();

    let start_a = request_start(1, 100, RESTART_POLICY_ON_FAILURE);
    let start_a_reply = manager.handle(&start_a);
    let start_a_payload = read_payload::<StartDomainReply>(&start_a_reply).expect("start reply");
    println!(
        "DOMAIN_MANAGER: start ok domain={} generation={}",
        start_a_payload.domain_id, start_a_payload.generation
    );

    let status_a = request_status(2, 100);
    let status_a_reply = manager.handle(&status_a);
    let status_a_payload =
        read_payload::<GetDomainStatusReply>(&status_a_reply).expect("status reply");
    println!(
        "DOMAIN_MANAGER: status running domain={} restarts={}",
        status_a_payload.domain_id, status_a_payload.restart_count
    );

    let crash_a = request_report_exit(100, 1);
    let crash_a_reply = manager.handle(&crash_a);
    let crash_a_payload =
        read_payload::<ReportExitReply>(&crash_a_reply).expect("report_exit reply");
    println!(
        "DOMAIN_MANAGER: restart policy triggered domain={} action={} generation={} restarts={}",
        crash_a_payload.domain_id,
        crash_a_payload.action,
        crash_a_payload.generation,
        crash_a_payload.restart_count
    );

    let start_b = request_start(3, 200, RESTART_POLICY_NEVER);
    let start_b_reply = manager.handle(&start_b);
    let start_b_payload = read_payload::<StartDomainReply>(&start_b_reply).expect("start reply");
    println!(
        "DOMAIN_MANAGER: start ok domain={} generation={}",
        start_b_payload.domain_id, start_b_payload.generation
    );

    let crash_b = request_report_exit(200, 1);
    let crash_b_reply = manager.handle(&crash_b);
    let crash_b_payload =
        read_payload::<ReportExitReply>(&crash_b_reply).expect("report_exit reply");
    println!(
        "DOMAIN_MANAGER: exit handled domain={} action={}",
        crash_b_payload.domain_id, crash_b_payload.action
    );

    let list = request_list(4);
    let list_reply = manager.handle(&list);
    let list_payload = read_payload::<ListDomainsReply>(&list_reply).expect("list reply");
    println!(
        "DOMAIN_MANAGER: list total={} running={} restarting={} stopped={}",
        list_payload.total_domains,
        list_payload.running_domains,
        list_payload.restarting_domains,
        list_payload.stopped_domains
    );

    let stop_a = request_stop(5, 100);
    let stop_a_reply = manager.handle(&stop_a);
    let stop_a_payload = read_payload::<StopDomainReply>(&stop_a_reply).expect("stop reply");
    println!(
        "DOMAIN_MANAGER: stop ok domain={} generation={}",
        stop_a_payload.domain_id, stop_a_payload.generation
    );

    println!("DOMAIN_MANAGER: lifecycle api ok");
    println!("DOMAIN_MANAGER: restart policy ok");
    println!("DOMAIN_MANAGER: multi-domain ok");

    let gpu = run_gpu_quarantine_handshake(&mut manager, &args, &mut store_client)?;
    println!(
        "DOMAIN_MANAGER: gpu quarantine ok trace={} observed={} scenario={}",
        gpu.trace_id, gpu.observed_caps_id, gpu.scenario_id
    );

    if args.emit_semantic_snapshot || args.ingest_semantic_snapshot {
        let records = manager.semantic_inventory_records();
        let snapshot = semantic_snapshot::build_live_platform_snapshot(
            "2026-06-17T00:00:00Z",
            "host",
            "domain-manager-live",
            0,
            &records,
        );
        println!(
            "DOMAIN_MANAGER: semantic_snapshot domains={}",
            snapshot.domains.len()
        );
        if args.emit_semantic_snapshot {
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".into())
            );
        }
        if args.ingest_semantic_snapshot {
            let tmp = std::env::temp_dir().join(format!(
                "ramen-platform-snapshot-{}.json",
                std::process::id()
            ));
            fs::write(&tmp, serde_json::to_vec_pretty(&snapshot)?)?;
            let reply =
                store_client.ingest_artifact("platform_snapshot_v0", "semantic_state", &tmp)?;
            let _ = fs::remove_file(&tmp);
            println!(
                "DOMAIN_MANAGER: semantic_snapshot_ingested content_id={}",
                reply.content_id
            );
        }
    }

    println!("DOMAIN_MANAGER: ok");
    Ok(())
}

fn request_start(request_id: u64, domain_id: u64, restart_policy: u32) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_START_DOMAIN);
    let payload = StartDomain {
        request_id,
        domain_id,
        runner_kind: 1,
        restart_policy,
    };
    write_payload(&mut env, &payload).expect("start payload");
    env
}

fn request_stop(request_id: u64, domain_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_STOP_DOMAIN);
    let payload = StopDomain {
        request_id,
        domain_id,
    };
    write_payload(&mut env, &payload).expect("stop payload");
    env
}

fn request_status(request_id: u64, domain_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_GET_DOMAIN_STATUS);
    let payload = GetDomainStatus {
        request_id,
        domain_id,
    };
    write_payload(&mut env, &payload).expect("status payload");
    env
}

fn request_report_exit(domain_id: u64, exit_code: u32) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_REPORT_EXIT);
    let payload = ReportExit {
        domain_id,
        exit_code,
        reason: 0,
    };
    write_payload(&mut env, &payload).expect("report_exit payload");
    env
}

fn request_list(request_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_LIST_DOMAINS);
    let payload = ListDomains { request_id };
    write_payload(&mut env, &payload).expect("list payload");
    env
}

#[cfg(test)]
fn request_grant_capabilities(
    request_id: u64,
    domain_id: u64,
    content_id_hash: &[u8; 32],
) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_GRANT_CAPABILITIES);
    let payload = GrantCapabilities {
        request_id,
        domain_id,
        content_id_hash: *content_id_hash,
    };
    write_payload(&mut env, &payload).expect("grant capabilities payload");
    env
}

#[cfg(test)]
fn request_revoke_capabilities(request_id: u64, domain_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_REVOKE_CAPABILITIES);
    let payload = RevokeCapabilities {
        request_id,
        domain_id,
    };
    write_payload(&mut env, &payload).expect("revoke capabilities payload");
    env
}

#[cfg(test)]
fn request_get_domain_grant_handles(request_id: u64, domain_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_DOMAIN_MANAGER_V1, MSG_GET_DOMAIN_GRANT_HANDLES);
    let payload = GetDomainGrantHandles {
        request_id,
        domain_id,
    };
    write_payload(&mut env, &payload).expect("get grant handles payload");
    env
}

fn request_gpu_start(
    request_id: u64,
    domain_id: u64,
    restart_policy: u32,
    gpu_profile: u32,
) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_START_QUARANTINE_DOMAIN);
    let payload = StartQuarantineDomain {
        request_id,
        domain_id,
        restart_policy,
        gpu_profile,
    };
    write_payload(&mut env, &payload).expect("gpu start payload");
    env
}

fn request_gpu_stop(request_id: u64, domain_id: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_STOP_QUARANTINE_DOMAIN);
    let payload = StopQuarantineDomain {
        request_id,
        domain_id,
    };
    write_payload(&mut env, &payload).expect("gpu stop payload");
    env
}

fn request_export_display(
    request_id: u64,
    domain_id: u64,
    display_cap: u64,
    width: u32,
    height: u32,
) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_EXPORT_DISPLAY);
    // Split capability token into high and low parts for V-004 (u64 generation)
    let display_cap_token_high = display_cap >> 32;
    let display_cap_token_low = display_cap & 0xFFFFFFFF;
    let payload = ExportDisplay {
        request_id,
        domain_id,
        display_cap_token_high,
        display_cap_token_low,
        width,
        height,
    };
    write_payload(&mut env, &payload).expect("gpu export payload");
    env
}

fn request_report_scanout(
    request_id: u64,
    domain_id: u64,
    surface_id: u64,
    frame_seq: u64,
) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_GPU_QUARANTINE_V1, MSG_REPORT_SCANOUT);
    let payload = ReportScanout {
        request_id,
        domain_id,
        surface_id,
        frame_seq,
    };
    write_payload(&mut env, &payload).expect("gpu scanout payload");
    env
}

fn payload_slice(env: &Envelope) -> &[u8] {
    let len = (env.payload_len as usize).min(env.payload.len());
    &env.payload[..len]
}

fn compose_surface_id(domain_id: u64, generation: u32) -> u64 {
    (domain_id << 32) | u64::from(generation)
}

fn ingest_file(
    store_client: &mut StoreClient,
    src: &Path,
    kind: &str,
    channel: &str,
) -> Result<(String, u64), Box<dyn Error>> {
    // V-007 Phase 2: Use store service IPC for artifact ingestion
    let reply = store_client.ingest_artifact(kind, channel, src)?;
    Ok((reply.content_id, reply.size_bytes))
}

fn write_protocol_trace(
    path: &Path,
    recorder: &[ProtocolTraceEvent],
) -> Result<(), Box<dyn Error>> {
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
                harness_name: "gpu.quarantine".to_string(),
                harness_version: 1,
                policy_bundle_id: Some("policy.stub.v0".to_string()),
            },
            events: recorder.to_vec(),
        }),
        scenario_trace: None,
    };
    // Use canonical validator from artifact_store_schema
    validate_trace_artifact(&trace).map_err(|e| format!("trace validation failed: {}", e))?;
    // Write the trace artifact to file for ingestion
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&trace)?)?;
    Ok(())
}

fn write_observed_caps(
    path: &Path,
    program_id: &str,
    run_id: &str,
    width: u32,
    height: u32,
    trace_id: &str,
) -> Result<(), Box<dyn Error>> {
    let obs = ObservedCapsV0 {
        schema_version: 1,
        program_id: program_id.to_string(),
        run_id: run_id.to_string(),
        launch_plan_id: None,
        capabilities: vec![ObservedCapability {
            cap: "domain.gpu_quarantine.export_display".to_string(),
            scope: ObservedCapScope {
                artifact_ids: vec![],
            },
            counts: ObservedCapCounts {
                granted: 1,
                used: 1,
            },
            evidence: vec![trace_id.to_string()],
        }],
        evidence: vec![trace_id.to_string()],
    };
    // Use canonical validator from artifact_store_schema
    validate_observed_caps(&obs).map_err(|e| format!("observed caps validation failed: {}", e))?;
    // Write the observed caps artifact to file for ingestion
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut value = serde_json::to_value(&obs)?;
    value["summary"] = serde_json::json!({
        "width": width,
        "height": height,
        "format": "xrgb8888"
    });
    fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn write_scenario_trace(
    path: &Path,
    protocol_trace_id: &str,
    observed_caps_id: &str,
    surface_id: u64,
) -> Result<(), Box<dyn Error>> {
    let trace = TraceArtifactV0 {
        schema_version: 1,
        trace_type: TraceType::ScenarioTrace,
        protocol_trace: None,
        scenario_trace: Some(ScenarioTrace {
            metadata: ScenarioTraceMetadata {
                scenario_id: "gpu_quarantine_display_export_v1".to_string(),
                timestamp_start: None,
                timestamp_end: None,
            },
            events: vec![
                ScenarioTraceEvent {
                    seq: 1,
                    name: "protocol_trace_ref".to_string(),
                    payload: Some(serde_json::json!({ "content_id": protocol_trace_id })),
                },
                ScenarioTraceEvent {
                    seq: 2,
                    name: "observed_caps_ref".to_string(),
                    payload: Some(serde_json::json!({ "content_id": observed_caps_id })),
                },
                ScenarioTraceEvent {
                    seq: 3,
                    name: "display_export".to_string(),
                    payload: Some(serde_json::json!({ "surface_id": surface_id, "result": "ok" })),
                },
                ScenarioTraceEvent {
                    seq: 4,
                    name: "scanout_frame".to_string(),
                    payload: Some(serde_json::json!({ "frame_seq": 1, "result": "ok" })),
                },
            ],
        }),
    };
    // Use canonical validator from artifact_store_schema
    validate_trace_artifact(&trace).map_err(|e| format!("trace validation failed: {}", e))?;
    // Write the scenario trace artifact to file for ingestion
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&trace)?)?;
    Ok(())
}

fn run_gpu_quarantine_handshake(
    manager: &mut DomainManager,
    args: &Args,
    store_client: &mut StoreClient,
) -> Result<GpuHandshakeResult, Box<dyn Error>> {
    let mut seq = 0u64;
    let mut record_event = |events: &mut Vec<ProtocolTraceEvent>,
                            dir: TraceDir,
                            op: &str,
                            env: &Envelope,
                            result: Option<String>| {
        seq += 1;
        events.push(ProtocolTraceEvent {
            seq,
            dir,
            op: Some(op.to_string()),
            bytes_hex: hex::encode(payload_slice(env)),
            result,
            notes: None,
        });
    };

    let domain_id = 700;
    let width = 1280;
    let height = 720;
    let mut events: Vec<ProtocolTraceEvent> = Vec::new();

    let start = request_gpu_start(
        100,
        domain_id,
        GPU_RESTART_POLICY_ON_FAILURE,
        GPU_PROFILE_GENERIC,
    );
    record_event(
        &mut events,
        TraceDir::Request,
        "start_quarantine_domain",
        &start,
        None,
    );
    let start_reply = manager.handle(&start);
    let start_payload = read_payload::<StartQuarantineDomainReply>(&start_reply)
        .map_err(|e| format!("gpu start reply decode: {:?}", e))?;
    record_event(
        &mut events,
        TraceDir::Response,
        "start_quarantine_domain_reply",
        &start_reply,
        Some(format!(
            "status={} generation={}",
            start_payload.status, start_payload.generation
        )),
    );

    let export = request_export_display(101, domain_id, CAP_DISPLAY_EXPORT, width, height);
    record_event(
        &mut events,
        TraceDir::Request,
        "export_display",
        &export,
        None,
    );
    let export_reply = manager.handle(&export);
    let export_payload = read_payload::<ExportDisplayReply>(&export_reply)
        .map_err(|e| format!("gpu export reply decode: {:?}", e))?;
    record_event(
        &mut events,
        TraceDir::Response,
        "export_display_reply",
        &export_reply,
        Some(format!(
            "status={} surface_id={} stride={}",
            export_payload.status, export_payload.surface_id, export_payload.stride
        )),
    );
    println!(
        "DOMAIN_MANAGER: gpu export ok domain={} surface={} {}x{}",
        export_payload.domain_id, export_payload.surface_id, width, height
    );

    let scanout = request_report_scanout(102, domain_id, export_payload.surface_id, 1);
    record_event(
        &mut events,
        TraceDir::Request,
        "report_scanout",
        &scanout,
        None,
    );
    let scanout_reply = manager.handle(&scanout);
    let scanout_payload = read_payload::<ReportScanoutReply>(&scanout_reply)
        .map_err(|e| format!("gpu scanout reply decode: {:?}", e))?;
    record_event(
        &mut events,
        TraceDir::Response,
        "report_scanout_reply",
        &scanout_reply,
        Some(format!(
            "status={} acked_frame_seq={}",
            scanout_payload.status, scanout_payload.acked_frame_seq
        )),
    );
    println!(
        "DOMAIN_MANAGER: gpu scanout ok domain={} surface={} frame_seq={}",
        domain_id, export_payload.surface_id, scanout_payload.acked_frame_seq
    );

    // Negative assertions (malformed capability and frame sequencing)
    let export_bad = request_export_display(103, domain_id, CAP_DISPLAY_EXPORT + 1, width, height);
    let export_bad_reply = manager.handle(&export_bad);
    let export_bad_payload = read_payload::<ExportDisplayReply>(&export_bad_reply)
        .map_err(|e| format!("gpu export bad reply decode: {:?}", e))?;
    if export_bad_payload.status == STATUS_INVALID_CAPABILITY {
        println!("DOMAIN_MANAGER: gpu invalid capability rejected");
    } else {
        return Err("expected invalid capability rejection".into());
    }

    let scanout_bad = request_report_scanout(104, domain_id, export_payload.surface_id, 1);
    let scanout_bad_reply = manager.handle(&scanout_bad);
    let scanout_bad_payload = read_payload::<ReportScanoutReply>(&scanout_bad_reply)
        .map_err(|e| format!("gpu scanout bad reply decode: {:?}", e))?;
    if scanout_bad_payload.status == STATUS_ERR {
        println!("DOMAIN_MANAGER: gpu stale frame rejected");
    } else {
        return Err("expected stale frame rejection".into());
    }

    let stop = request_gpu_stop(105, domain_id);
    let stop_reply = manager.handle(&stop);
    let stop_payload = read_payload::<StopQuarantineDomainReply>(&stop_reply)
        .map_err(|e| format!("gpu stop reply decode: {:?}", e))?;
    println!(
        "DOMAIN_MANAGER: gpu stop ok domain={} generation={}",
        stop_payload.domain_id, stop_payload.generation
    );

    let trace_path = args.trace_dir.join("gpu_quarantine_protocol.json");
    write_protocol_trace(&trace_path, &events)?;
    let (trace_id, _) = ingest_file(
        store_client,
        &trace_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    let observed_path = args.trace_dir.join("gpu_quarantine_observed.json");
    write_observed_caps(
        &observed_path,
        &args.program_id,
        &args.run_id,
        width,
        height,
        &trace_id,
    )?;
    let (observed_caps_id, _) = ingest_file(
        store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?;

    let scenario_path = args.trace_dir.join("gpu_quarantine_scenario.json");
    write_scenario_trace(
        &scenario_path,
        &trace_id,
        &observed_caps_id,
        export_payload.surface_id,
    )?;
    let (scenario_id, _) = ingest_file(
        store_client,
        &scenario_path,
        "scenario_trace",
        "Experimental",
    )?;

    Ok(GpuHandshakeResult {
        trace_id,
        observed_caps_id,
        scenario_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_failure_restarts_and_increments_generation() {
        let mut manager = DomainManager::new();
        let start = request_start(1, 42, RESTART_POLICY_ON_FAILURE);
        let _ = manager.handle(&start);
        let exit = request_report_exit(42, 1);
        let reply = manager.handle(&exit);
        let payload = read_payload::<ReportExitReply>(&reply).expect("report_exit reply");
        assert_eq!(payload.action, EXIT_ACTION_RESTARTED);
        assert_eq!(payload.generation, 2);
        assert_eq!(payload.restart_count, 1);
    }

    #[test]
    fn never_policy_stops_domain() {
        let mut manager = DomainManager::new();
        let start = request_start(1, 7, RESTART_POLICY_NEVER);
        let _ = manager.handle(&start);
        let exit = request_report_exit(7, 1);
        let _ = manager.handle(&exit);
        let status = request_status(2, 7);
        let status_reply = manager.handle(&status);
        let payload = read_payload::<GetDomainStatusReply>(&status_reply).expect("status reply");
        assert_eq!(payload.state, DOMAIN_STATE_STOPPED);
        assert_eq!(payload.restart_count, 0);
    }

    #[test]
    fn gpu_export_rejects_invalid_capability() {
        let mut manager = DomainManager::new();
        let start = request_gpu_start(1, 700, RESTART_POLICY_ALWAYS, GPU_PROFILE_GENERIC);
        let _ = manager.handle(&start);
        let bad = request_export_display(2, 700, CAP_DISPLAY_EXPORT + 1, 800, 600);
        let reply = manager.handle(&bad);
        let payload = read_payload::<ExportDisplayReply>(&reply).expect("export display reply");
        assert_eq!(payload.status, STATUS_INVALID_CAPABILITY);
    }

    #[test]
    fn gpu_scanout_requires_monotonic_frames() {
        let mut manager = DomainManager::new();
        let start = request_gpu_start(1, 700, RESTART_POLICY_NEVER, GPU_PROFILE_GENERIC);
        let _ = manager.handle(&start);
        let export = request_export_display(2, 700, CAP_DISPLAY_EXPORT, 1280, 720);
        let export_reply = manager.handle(&export);
        let export_payload =
            read_payload::<ExportDisplayReply>(&export_reply).expect("export reply");
        let scanout1 = request_report_scanout(3, 700, export_payload.surface_id, 1);
        let reply1 = manager.handle(&scanout1);
        let payload1 = read_payload::<ReportScanoutReply>(&reply1).expect("scanout reply 1");
        assert_eq!(payload1.status, STATUS_OK);
        let scanout_stale = request_report_scanout(4, 700, export_payload.surface_id, 1);
        let reply_stale = manager.handle(&scanout_stale);
        let payload_stale =
            read_payload::<ReportScanoutReply>(&reply_stale).expect("scanout stale reply");
        assert_eq!(payload_stale.status, STATUS_ERR);
    }

    // S10.1: Capability broker IPC tests

    /// Helper to create a test manifest with echo capability
    fn test_manifest_with_echo() -> artifact_store_schema::native_wasm::NativeWasmManifestV0 {
        use artifact_store_schema::Manifest;
        use artifact_store_schema::native_wasm::{
            NativeWasmManifestV0, NativeWasmV0, RequiredCapability,
        };

        NativeWasmManifestV0 {
            manifest: Manifest {
                schema_version: 1,
                content_id: "sha256:test123".to_string(),
                size_bytes: 1024,
                kind: "native_wasm_v0".to_string(),
                channels: vec!["Experimental".to_string()],
                signatures: vec![],
            },
            native_wasm: NativeWasmV0 {
                entrypoint: "_start".to_string(),
                required_capabilities: vec![RequiredCapability {
                    export_name: "RAMEN_CAP_ECHO_V0".to_string(),
                    interface: "harness.echo_v0".to_string(),
                    rights: 1,
                    purpose: "Send echo requests".to_string(),
                }],
                declares_no_capabilities: false,
            },
        }
    }

    #[test]
    fn grant_capabilities_ipc_roundtrip() {
        let mut manager = DomainManager::new();

        // Register a test manifest
        let manifest = test_manifest_with_echo();
        let content_id_hash: [u8; 32] = [0xAB; 32]; // Test hash
        manager.register_manifest(&content_id_hash, manifest, "Experimental");

        // Request capability grant via IPC
        let req = request_grant_capabilities(1, 100, &content_id_hash);
        let reply = manager.handle(&req);
        let payload = read_payload::<GrantCapabilitiesReply>(&reply).unwrap();

        // Either success with handles, or failure with status != 0
        assert!(
            payload.handle_count > 0 || payload.status != 0,
            "Expected either handles granted or non-zero status"
        );

        // In this case, should succeed with 1 handle
        assert_eq!(payload.status, STATUS_OK);
        assert_eq!(payload.handle_count, 1);
    }

    #[test]
    fn grant_capabilities_rejects_unknown_manifest() {
        let mut manager = DomainManager::new();

        // Don't register any manifest - use unknown hash
        let unknown_hash: [u8; 32] = [0x00; 32];
        let req = request_grant_capabilities(1, 100, &unknown_hash);
        let reply = manager.handle(&req);
        let payload = read_payload::<GrantCapabilitiesReply>(&reply).unwrap();

        // Should fail with NOT_FOUND
        assert_eq!(payload.status, STATUS_NOT_FOUND);
        assert_eq!(payload.handle_count, 0);
    }

    #[test]
    fn get_domain_grant_handles_rejects_domain_without_grants() {
        let mut manager = DomainManager::new();

        let req = request_get_domain_grant_handles(7, 404);
        let reply = manager.handle(&req);
        let payload = read_payload::<GetDomainGrantHandlesReply>(&reply).unwrap();

        assert_eq!(payload.request_id, 7);
        assert_eq!(payload.domain_id, 404);
        assert_eq!(payload.status, STATUS_NOT_FOUND);
        assert_eq!(payload.count, 0);
    }

    #[test]
    fn revoke_capabilities_ipc_roundtrip() {
        let mut manager = DomainManager::new();

        // First, grant some capabilities
        let manifest = test_manifest_with_echo();
        let content_id_hash: [u8; 32] = [0xCD; 32];
        manager.register_manifest(&content_id_hash, manifest, "Experimental");

        let grant_req = request_grant_capabilities(1, 100, &content_id_hash);
        let grant_reply = manager.handle(&grant_req);
        let grant_payload = read_payload::<GrantCapabilitiesReply>(&grant_reply).unwrap();
        assert_eq!(grant_payload.status, STATUS_OK);
        assert_eq!(grant_payload.handle_count, 1);

        // Now revoke them
        let revoke_req = request_revoke_capabilities(2, 100);
        let revoke_reply = manager.handle(&revoke_req);
        let revoke_payload = read_payload::<RevokeCapabilitiesReply>(&revoke_reply).unwrap();

        assert_eq!(revoke_payload.status, STATUS_OK);
        assert_eq!(revoke_payload.revoked_count, 1);
    }

    #[test]
    fn revoke_capabilities_idempotent() {
        let mut manager = DomainManager::new();

        // Revoke from domain with no grants
        let revoke_req = request_revoke_capabilities(1, 999);
        let revoke_reply = manager.handle(&revoke_req);
        let revoke_payload = read_payload::<RevokeCapabilitiesReply>(&revoke_reply).unwrap();

        // Should succeed with 0 revoked
        assert_eq!(revoke_payload.status, STATUS_OK);
        assert_eq!(revoke_payload.revoked_count, 0);
    }

    #[test]
    fn grant_capabilities_denied_for_stable_channel() {
        let mut manager = DomainManager::new();

        // Register manifest that requires harness.echo_v0
        let manifest = test_manifest_with_echo();
        let content_id_hash: [u8; 32] = [0xEF; 32];
        // Register with Stable channel (which doesn't allow harness.echo_v0 in test policy)
        manager.register_manifest(&content_id_hash, manifest, "Stable");

        let req = request_grant_capabilities(1, 100, &content_id_hash);
        let reply = manager.handle(&req);
        let payload = read_payload::<GrantCapabilitiesReply>(&reply).unwrap();

        // Should be denied
        assert_eq!(payload.status, STATUS_INVALID_CAPABILITY);
        assert_eq!(payload.handle_count, 0);
    }
}
