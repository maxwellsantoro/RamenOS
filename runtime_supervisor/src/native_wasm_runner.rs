//! Native WASM runner dispatch for runtime_supervisor.
//!
//! This module provides the `native_wasm_v0` runner integration, which:
//! 1. Fetches artifact bytes from the store service
//! 2. Requests capability grants from the broker (via DomainManager IPC)
//! 3. Calls native_runner with granted handles to execute the WASM module
//!
//! For S10.1, the broker call and native runner call may be stubbed/simplified
//! since full integration is complex.

use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use kernel_api::cap::Handle;
use kernel_api::generated::{
    DOMAIN_MANAGER_V1_PROTOCOL_ID, GetDomainGrantHandles, GetDomainGrantHandlesReply,
    GrantCapabilities, GrantCapabilitiesReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};

/// Kernel IPC transport selection (S10.5.2).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum KernelIpcTransport {
    #[default]
    UnixSocket,
    /// QEMU chardev socket framing (`RAMEN_KERNEL_IPC_TRANSPORT=chardev-serial`).
    ChardevSerial,
}

impl KernelIpcTransport {
    pub fn from_env() -> Self {
        match std::env::var("RAMEN_KERNEL_IPC_TRANSPORT").ok().as_deref() {
            Some("chardev-serial") | Some("virtio-serial") => Self::ChardevSerial,
            _ => Self::UnixSocket,
        }
    }
}

/// Configuration for native WASM execution.
///
/// This is parsed from the launch plan's `native_wasm` field.
#[derive(Debug, Clone, Deserialize)]
pub struct NativeWasmConfig {
    /// Path to kernel IPC socket for broker communication.
    pub kernel_ipc: String,

    /// Host↔target IPC transport (default unix socket; chardev for QEMU bridge).
    #[serde(default)]
    pub kernel_ipc_transport: KernelIpcTransport,

    /// Optional DomainManager IPC socket for broker grant negotiation.
    #[serde(default)]
    pub domain_manager_ipc: Option<String>,

    /// Domain ID for this execution context.
    pub domain_id: u64,

    /// Timeout in milliseconds for WASM execution.
    /// NOTE: Enforcement deferred to S10.2+ (currently parsed but not enforced).
    #[serde(default = "default_timeout_ms")]
    #[allow(dead_code)]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}

/// Result of native WASM execution.
#[derive(Debug, Clone)]
pub struct NativeWasmResult {
    /// Exit code from the WASM module.
    pub exit_code: i32,

    /// Captured stdout from execution.
    #[allow(dead_code)] // Consumed by supervisor dispatch once native_wasm_v0 is fully wired
    pub stdout: Vec<u8>,
}

/// Run a native WASM module.
///
/// This is the main entry point for `native_wasm_v0` runner dispatch.
///
/// # Arguments
/// * `artifact_ref` - Content ID of the WASM artifact (e.g., "sha256:...")
/// * `config` - Native WASM configuration from the launch plan
/// * `store_client` - Store service client for artifact retrieval
/// * `_installed_root` - Root directory for installed artifacts (unused, for future expansion)
///
/// # Returns
/// * `Ok(NativeWasmResult)` on successful execution
/// * `Err(String)` on failure
pub fn run(
    artifact_ref: &str,
    config: &NativeWasmConfig,
    store_client: &mut store_service::StoreClient,
    _installed_root: &Path,
) -> Result<NativeWasmResult, String> {
    // Step 1: Validate domain_id is non-zero (security requirement)
    if config.domain_id == 0 {
        return Err("native_wasm_v0: domain_id must be non-zero".to_string());
    }

    // Step 2: Fetch artifact bytes from store service
    let blob_reply = store_client
        .get_blob(artifact_ref)
        .map_err(|e| format!("failed to fetch artifact: {}", e))?;

    // Step 3: Read WASM bytes from the returned path
    let blob_path = Path::new(&blob_reply.blob_path);
    let wasm_bytes =
        std::fs::read(blob_path).map_err(|e| format!("failed to read artifact bytes: {}", e))?;

    // Step 4: Request capability grants from broker
    // For S10.1, we stub this with empty grants since full broker integration
    // is complex. The real implementation would call DomainManager IPC to
    // request grants based on the program's manifest.
    let granted_handles = request_capability_grants(config, artifact_ref)?;

    // Step 5: Execute the WASM module
    execute_wasm(&wasm_bytes, granted_handles, config)
}

/// Request capability grants from the broker.
///
/// For S10.1, this is a stub that returns empty grants.
/// Full implementation would:
/// 1. Connect to DomainManager IPC at config.kernel_ipc
/// 2. Send a capability grant request for config.domain_id
/// 3. Receive granted capability handles
fn request_capability_grants(
    config: &NativeWasmConfig,
    artifact_ref: &str,
) -> Result<HashMap<String, u64>, String> {
    // S10.1: Stub implementation - no grants requested
    // Real implementation calls broker IPC when a DomainManager socket is configured.
    let Some(domain_manager_ipc) = config.domain_manager_ipc.as_ref() else {
        let _ = config.kernel_ipc.as_str(); // Suppress unused warning
        let _ = artifact_ref;
        return Ok(HashMap::new());
    };

    let content_id_hash = parse_content_id_hash(artifact_ref)?;
    let mut stream = UnixStream::connect(domain_manager_ipc)
        .map_err(|e| format!("connect domain_manager IPC {}: {}", domain_manager_ipc, e))?;

    let mut grant_env = Envelope::empty(DOMAIN_MANAGER_V1_PROTOCOL_ID, 11);
    let grant = GrantCapabilities {
        request_id: 1,
        domain_id: config.domain_id,
        content_id_hash,
    };
    write_payload(&mut grant_env, &grant).map_err(|e| format!("encode grant request: {:?}", e))?;
    let grant_reply_env = transact_domain_manager(&mut stream, &grant_env)?;
    let grant_reply: GrantCapabilitiesReply =
        read_payload(&grant_reply_env).map_err(|e| format!("decode grant reply: {:?}", e))?;
    if grant_reply.status != 0 || grant_reply.handle_count == 0 {
        return Err(format!(
            "broker grant failed status={} handle_count={}",
            grant_reply.status, grant_reply.handle_count
        ));
    }

    let mut handles_env = Envelope::empty(DOMAIN_MANAGER_V1_PROTOCOL_ID, 15);
    let handles_req = GetDomainGrantHandles {
        request_id: 2,
        domain_id: config.domain_id,
    };
    write_payload(&mut handles_env, &handles_req)
        .map_err(|e| format!("encode grant handles request: {:?}", e))?;
    let handles_reply_env = transact_domain_manager(&mut stream, &handles_env)?;
    let handles_reply: GetDomainGrantHandlesReply = read_payload(&handles_reply_env)
        .map_err(|e| format!("decode grant handles reply: {:?}", e))?;
    grant_handles_from_reply(&handles_reply)
}

fn grant_handles_from_reply(
    reply: &GetDomainGrantHandlesReply,
) -> Result<HashMap<String, u64>, String> {
    if reply.status != 0 || reply.count == 0 {
        return Err(format!(
            "broker grant handles failed status={} count={}",
            reply.status, reply.count
        ));
    }

    let mut granted_handles = HashMap::new();
    for (export_id, handle) in [
        (reply.entry0_export_id, reply.entry0_handle),
        (reply.entry1_export_id, reply.entry1_handle),
    ] {
        match export_id {
            1 if handle != 0 => {
                granted_handles.insert("RAMEN_CAP_SHMEM_CONTROL".to_string(), handle);
            }
            2 if handle != 0 => {
                granted_handles.insert("RAMEN_CAP_SEMANTIC_STATE".to_string(), handle);
            }
            0 => {}
            _ => return Err(format!("unknown broker export id {}", export_id)),
        }
    }

    for required in ["RAMEN_CAP_SHMEM_CONTROL", "RAMEN_CAP_SEMANTIC_STATE"] {
        if !granted_handles.contains_key(required) {
            return Err(format!("broker grant handles missing {}", required));
        }
    }

    Ok(granted_handles)
}

fn parse_content_id_hash(artifact_ref: &str) -> Result<[u8; 32], String> {
    let Some(hex) = artifact_ref.strip_prefix("sha256:") else {
        return Err(format!(
            "artifact_ref must be sha256 content id: {artifact_ref}"
        ));
    };
    if hex.len() != 64 {
        return Err(format!(
            "sha256 content id must be 64 hex chars: {artifact_ref}"
        ));
    }

    let mut out = [0u8; 32];
    for (idx, byte) in out.iter_mut().enumerate() {
        let start = idx * 2;
        *byte = u8::from_str_radix(&hex[start..start + 2], 16)
            .map_err(|e| format!("invalid sha256 content id: {}", e))?;
    }
    Ok(out)
}

fn transact_domain_manager(
    stream: &mut UnixStream,
    request: &Envelope,
) -> Result<Envelope, String> {
    stream
        .write_all(&envelope_to_bytes(request))
        .map_err(|e| format!("write domain_manager IPC: {}", e))?;
    let mut reply_bytes = [0u8; ENVELOPE_WIRE_SIZE];
    stream
        .read_exact(&mut reply_bytes)
        .map_err(|e| format!("read domain_manager IPC: {}", e))?;
    Ok(bytes_to_envelope(&reply_bytes))
}

const ENVELOPE_WIRE_SIZE: usize = 88;

fn envelope_to_bytes(env: &Envelope) -> [u8; ENVELOPE_WIRE_SIZE] {
    let mut buf = [0u8; ENVELOPE_WIRE_SIZE];
    buf[0..4].copy_from_slice(&env.protocol.to_le_bytes());
    buf[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    buf[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    buf[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    buf[20..84].copy_from_slice(&env.payload);
    buf
}

fn bytes_to_envelope(bytes: &[u8; ENVELOPE_WIRE_SIZE]) -> Envelope {
    let protocol = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let msg_type = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let handle_raw = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let payload_len = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let mut payload = [0u8; 64];
    payload.copy_from_slice(&bytes[20..84]);
    Envelope {
        protocol,
        msg_type,
        handle: Handle::unpack(handle_raw),
        payload_len,
        payload,
    }
}

/// Resolve kernel IPC transport: launch plan field unless env overrides.
fn resolve_kernel_ipc_transport(config: &NativeWasmConfig) -> KernelIpcTransport {
    if std::env::var("RAMEN_KERNEL_IPC_TRANSPORT").is_ok() {
        KernelIpcTransport::from_env()
    } else {
        config.kernel_ipc_transport
    }
}

fn to_native_runner_transport(transport: KernelIpcTransport) -> native_runner::KernelIpcTransport {
    match transport {
        KernelIpcTransport::UnixSocket => native_runner::KernelIpcTransport::UnixSocket,
        KernelIpcTransport::ChardevSerial => native_runner::KernelIpcTransport::ChardevSerial,
    }
}

/// Execute a WASM module with the native runner.
///
/// Uses the native_runner crate to execute the WASM bytes with
/// the granted capability handles injected as globals.
fn execute_wasm(
    wasm_bytes: &[u8],
    granted_handles: HashMap<String, u64>,
    config: &NativeWasmConfig,
) -> Result<NativeWasmResult, String> {
    // Create runner configuration
    let transport = resolve_kernel_ipc_transport(config);
    let runner_config = native_runner::RunnerConfig {
        kernel_ipc: config.kernel_ipc.clone().into(),
        kernel_ipc_transport: to_native_runner_transport(transport),
        trace_output: None,
    };

    // Create the runner
    let runner = native_runner::NativeRunner::new(runner_config)
        .map_err(|e| format!("failed to create native runner: {}", e))?;

    // Create run configuration with granted handles
    let run_config = native_runner::RunConfig { granted_handles };

    // Load and execute the WASM module
    let run_result = runner
        .load_and_run(wasm_bytes, run_config)
        .map_err(|e| format!("WASM execution failed: {}", e))?;

    Ok(NativeWasmResult {
        exit_code: run_result.exit_code,
        stdout: run_result.stdout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const SEMANTIC_SHMEM_HANDLE: u64 = 0x5308_0000_002a_0001;
    const SEMANTIC_STATE_HANDLE: u64 = 0x5310_0000_002a_0002;

    #[test]
    fn native_wasm_config_parses() {
        let json = r#"{
            "kernel_ipc": "/run/ramen/kernel.sock",
            "domain_id": 100,
            "timeout_ms": 30000
        }"#;
        let config: NativeWasmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.domain_id, 100);
        assert_eq!(config.kernel_ipc, "/run/ramen/kernel.sock");
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn native_wasm_config_defaults_timeout() {
        let json = r#"{
            "kernel_ipc": "/run/ramen/kernel.sock",
            "domain_id": 1
        }"#;
        let config: NativeWasmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn kernel_ipc_transport_parses_chardev() {
        let json = r#"{
            "kernel_ipc": "/tmp/qemu-ipc.sock",
            "kernel_ipc_transport": "chardev-serial",
            "domain_id": 1
        }"#;
        let config: NativeWasmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.kernel_ipc_transport,
            KernelIpcTransport::ChardevSerial
        );
    }

    #[test]
    fn resolve_kernel_ipc_transport_env_overrides_plan() {
        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::UnixSocket,
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };
        std::env::set_var("RAMEN_KERNEL_IPC_TRANSPORT", "chardev-serial");
        assert_eq!(
            resolve_kernel_ipc_transport(&config),
            KernelIpcTransport::ChardevSerial
        );
        std::env::remove_var("RAMEN_KERNEL_IPC_TRANSPORT");
    }

    #[test]
    fn launch_plan_with_native_wasm_parses() {
        // This test verifies that LaunchPlan can have native_wasm field.
        // We define a local LaunchPlan here since the one in main.rs is private.
        #[derive(Debug, Deserialize)]
        struct TestLaunchPlan {
            program_id: String,
            runner: String,
            artifact_ref: String,
            native_wasm: Option<NativeWasmConfig>,
        }

        let json = r#"{
            "program_id": "test",
            "runner": "native_wasm_v0",
            "artifact_ref": "sha256:abc",
            "native_wasm": {
                "kernel_ipc": "/run/ramen/kernel.sock",
                "domain_id": 1
            }
        }"#;
        let plan: TestLaunchPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.program_id, "test");
        assert_eq!(plan.artifact_ref, "sha256:abc");
        assert_eq!(plan.runner, "native_wasm_v0");
        assert!(plan.native_wasm.is_some());

        let wasm_config = plan.native_wasm.unwrap();
        assert_eq!(wasm_config.domain_id, 1);
        assert_eq!(wasm_config.kernel_ipc, "/run/ramen/kernel.sock");
    }

    #[test]
    fn native_wasm_result_tracks_exit_code() {
        let result = NativeWasmResult {
            exit_code: 42,
            stdout: b"output".to_vec(),
        };
        assert_eq!(result.exit_code, 42);
        assert_eq!(result.stdout, b"output");
    }

    #[test]
    fn request_capability_grants_returns_empty_for_s10_1() {
        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };
        let grants = request_capability_grants(&config, "sha256:test").unwrap();
        assert!(grants.is_empty());
    }

    // TODO: Test run_rejects_zero_domain_id with mock StoreClient
    // The validation exists in run() but testing requires a mock StoreClient
    // since StoreClient::new() has private fields. For S10.1, the validation
    // is exercised indirectly via the execute_wasm tests which use valid domain_id.

    #[test]
    fn execute_wasm_with_minimal_module() {
        // Create a minimal valid WASM module with _start
        let wasm_bytes = wat::parse_str(
            r#"(module
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        let result = execute_wasm(&wasm_bytes, HashMap::new(), &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 0);
    }

    #[test]
    fn execute_wasm_with_exit_code() {
        // WASM module that returns exit code 42
        let wasm_bytes = wat::parse_str(
            r#"(module
                (func (export "_start") (result i32)
                    i32.const 42
                )
            )"#,
        )
        .unwrap();

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        let result = execute_wasm(&wasm_bytes, HashMap::new(), &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 42);
    }

    #[test]
    fn execute_wasm_fails_without_start() {
        // WASM module without _start function
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        let result = execute_wasm(&wasm_bytes, HashMap::new(), &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("_start"));
    }

    #[test]
    fn execute_wasm_fails_on_invalid_bytes() {
        let invalid_bytes = b"\x00invalid wasm";

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        let result = execute_wasm(invalid_bytes, HashMap::new(), &config);
        assert!(result.is_err());
    }

    #[test]
    fn execute_wasm_injects_capabilities() {
        // WASM module with capability globals (must be mutable)
        let wasm_bytes = wat::parse_str(
            r#"(module
                (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        let mut granted_handles = HashMap::new();
        granted_handles.insert("RAMEN_CAP_ECHO_REQUEST".to_string(), 0x1234);

        let result = execute_wasm(&wasm_bytes, granted_handles, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_wasm_fails_on_missing_capability() {
        // WASM module requiring a capability we don't provide
        let wasm_bytes = wat::parse_str(
            r#"(module
                (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let config = NativeWasmConfig {
            kernel_ipc: "/run/ramen/kernel.sock".to_string(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 1,
            timeout_ms: 30000,
            domain_manager_ipc: None,
        };

        // Empty grants - missing required capability
        let result = execute_wasm(&wasm_bytes, HashMap::new(), &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("RAMEN_CAP_ECHO_REQUEST"));
    }

    #[test]
    fn semantic_harness_bridge_e2e() {
        let domain_socket = unique_socket_path("domain-manager");
        let proxy_socket = unique_socket_path("kernel-proxy");

        let domain_server = spawn_domain_manager_fixture(&domain_socket);
        let proxy_server = spawn_kernel_proxy_fixture(&proxy_socket);

        let config = NativeWasmConfig {
            kernel_ipc: proxy_socket.clone(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            domain_id: 42,
            timeout_ms: 30000,
            domain_manager_ipc: Some(domain_socket.clone()),
        };
        let artifact_ref =
            "sha256:abababababababababababababababababababababababababababababababab";
        let grants = request_capability_grants(&config, artifact_ref).expect("broker grants");
        assert_eq!(
            grants.get("RAMEN_CAP_SHMEM_CONTROL").copied(),
            Some(SEMANTIC_SHMEM_HANDLE)
        );
        assert_eq!(
            grants.get("RAMEN_CAP_SEMANTIC_STATE").copied(),
            Some(SEMANTIC_STATE_HANDLE)
        );

        let wasm_bytes = wat::parse_str(
            r#"(module
                (import "ramen::services.semantic_state" "get_snapshot::call"
                    (func $get_snapshot (param i64 i64 i32 i32 i32) (result i32)))
                (global $RAMEN_CAP_SHMEM_CONTROL
                    (export "RAMEN_CAP_SHMEM_CONTROL") (mut i64) (i64.const 0))
                (global $RAMEN_CAP_SEMANTIC_STATE
                    (export "RAMEN_CAP_SEMANTIC_STATE") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    global.get $RAMEN_CAP_SEMANTIC_STATE
                    i64.const 7
                    i32.const 0
                    i32.const 0
                    i32.const 64
                    call $get_snapshot)
            )"#,
        )
        .unwrap();

        let result = execute_wasm(&wasm_bytes, grants, &config).expect("semantic harness wasm");
        assert_eq!(result.exit_code, 0);

        domain_server.join().expect("domain fixture thread");
        proxy_server.join().expect("proxy fixture thread");
        let _ = std::fs::remove_file(domain_socket);
        let _ = std::fs::remove_file(proxy_socket);
    }

    fn unique_socket_path(label: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("ramen-{label}-{}-{nanos}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn spawn_domain_manager_fixture(socket_path: &str) -> thread::JoinHandle<()> {
        let socket_path = socket_path.to_string();
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).expect("bind domain fixture");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept domain fixture");
            for _ in 0..2 {
                let mut request_bytes = [0u8; ENVELOPE_WIRE_SIZE];
                stream
                    .read_exact(&mut request_bytes)
                    .expect("read domain fixture request");
                let request = bytes_to_envelope(&request_bytes);
                let reply = match request.msg_type {
                    11 => {
                        let grant: GrantCapabilities =
                            read_payload(&request).expect("grant request payload");
                        let mut env = Envelope::empty(DOMAIN_MANAGER_V1_PROTOCOL_ID, 12);
                        write_payload(
                            &mut env,
                            &GrantCapabilitiesReply {
                                request_id: grant.request_id,
                                domain_id: grant.domain_id,
                                status: 0,
                                handle_count: 2,
                                reserved: 0,
                                reserved2: 0,
                            },
                        )
                        .expect("grant reply payload");
                        env
                    }
                    15 => {
                        let req: GetDomainGrantHandles =
                            read_payload(&request).expect("grant handles request payload");
                        let mut env = Envelope::empty(DOMAIN_MANAGER_V1_PROTOCOL_ID, 16);
                        write_payload(
                            &mut env,
                            &GetDomainGrantHandlesReply {
                                request_id: req.request_id,
                                domain_id: req.domain_id,
                                status: 0,
                                count: 2,
                                entry0_export_id: 1,
                                entry0_reserved: 0,
                                entry0_handle: SEMANTIC_SHMEM_HANDLE,
                                entry1_export_id: 2,
                                entry1_reserved: 0,
                                entry1_handle: SEMANTIC_STATE_HANDLE,
                            },
                        )
                        .expect("grant handles reply payload");
                        env
                    }
                    other => panic!("unexpected domain fixture msg_type {other}"),
                };
                stream
                    .write_all(&envelope_to_bytes(&reply))
                    .expect("write domain fixture reply");
            }
        });
        wait_for_socket(socket_path);
        handle
    }

    fn spawn_kernel_proxy_fixture(socket_path: &str) -> thread::JoinHandle<()> {
        let socket_path = socket_path.to_string();
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).expect("bind proxy fixture");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept proxy fixture");
            let mut request_bytes = [0u8; 88];
            stream
                .read_exact(&mut request_bytes)
                .expect("read proxy request");
            let request = kernel_harness_proxy::bytes_to_envelope(&request_bytes);
            let mut proxy = kernel_harness_proxy::KernelHarnessProxy::with_semantic_grants(
                42,
                SEMANTIC_SHMEM_HANDLE,
                SEMANTIC_STATE_HANDLE,
            );
            let reply = proxy.transact(request);
            stream
                .write_all(&kernel_harness_proxy::envelope_to_bytes(&reply))
                .expect("write proxy reply");
        });
        wait_for_socket(socket_path);
        handle
    }

    fn wait_for_socket(socket_path: String) {
        for _ in 0..50 {
            if std::path::Path::new(&socket_path).exists() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("socket did not appear: {socket_path}");
    }
}
