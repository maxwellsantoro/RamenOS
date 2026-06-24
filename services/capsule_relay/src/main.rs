// V-007 Phase 3: Capsule relay service using store service IPC
//
// Refactored to use store_service::StoreClient for artifact operations
// instead of direct artifact_store_core IO functions.
//
// This enforces the architectural boundary: services must not bypass
// the store service for artifact access.

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rand::RngCore;

use artifact_store_schema::driver_protocol_trace::DRIVER_PROTOCOL_TRACE_KIND;
use artifact_store_schema::observed_caps::{
    ObservedCapCounts, ObservedCapScope, ObservedCapability, ObservedCapsV0, validate_observed_caps,
};
use artifact_store_schema::trace::{
    ProtocolTrace, ProtocolTraceEvent, ProtocolTraceMetadata, ScenarioTrace, ScenarioTraceEvent,
    ScenarioTraceMetadata, TraceArtifactV0, TraceDir, TraceType, validate_trace_artifact,
};
use clap::Parser;
use kernel_api::generated::{
    EchoReply, EchoRequest, Health, HealthReply, Hello, HelloReply, Shutdown, ShutdownReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};
use store_service::StoreClient;
use store_service::capability::{STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability};

mod vm_backend;

const PROTOCOL_CAPSULE_CONTROL: u32 = 0x200;
const MSG_HELLO: u32 = 1;
const MSG_HELLO_REPLY: u32 = 2;
const MSG_HEALTH: u32 = 3;
const MSG_HEALTH_REPLY: u32 = 4;
const MSG_SHUTDOWN: u32 = 5;
const MSG_SHUTDOWN_REPLY: u32 = 6;

const PROTOCOL_ECHO: u32 = 0x210;
const MSG_ECHO_REQUEST: u32 = 1;
const MSG_ECHO_REPLY: u32 = 2;

const STATUS_OK: u32 = 0;
const STATUS_ERR: u32 = 1;
const STATUS_INVALID: u32 = 2;

const CAPSULE_ID: u64 = 0xC0A5_0000_0000_0001;
const BACKEND_CAPS: u32 = 0x1;

const PROGRAM_ID: &str = "capsule.echo.relay";
const CAPABILITY_NAME: &str = "harness.echo";
const SCENARIO_ID: &str = "capsule.echo.relay.v0";

/// Domain ID for the capsule relay service
const CAPSULE_RELAY_DOMAIN_ID: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapsuleTraceArtifactKind {
    HarnessProtocolTraceV0,
    DriverProtocolTraceV0,
}

impl CapsuleTraceArtifactKind {
    fn all() -> &'static [Self] {
        &[Self::HarnessProtocolTraceV0, Self::DriverProtocolTraceV0]
    }

    fn artifact_kind(self) -> &'static str {
        match self {
            Self::HarnessProtocolTraceV0 => "trace_artifact_v0",
            Self::DriverProtocolTraceV0 => DRIVER_PROTOCOL_TRACE_KIND,
        }
    }
}

const _: () = {
    let _ = [0u8; 64 - core::mem::size_of::<Hello>()];
    let _ = [0u8; 64 - core::mem::size_of::<HelloReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<Health>()];
    let _ = [0u8; 64 - core::mem::size_of::<HealthReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<Shutdown>()];
    let _ = [0u8; 64 - core::mem::size_of::<ShutdownReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<EchoRequest>()];
    let _ = [0u8; 64 - core::mem::size_of::<EchoReply>()];
};

/// Backend trait for capsule agent implementations.
pub trait CapsuleBackend {
    /// Send an envelope to the agent and receive a reply.
    fn call(&mut self, request: &Envelope) -> Result<Envelope, String>;

    /// Graceful shutdown (optional cleanup).
    fn shutdown(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    HostOnly,
    Vm,
}

#[derive(Parser, Debug)]
struct Args {
    /// Execution mode: "host-only" (in-process mock) or "vm" (QEMU + virtio-serial).
    #[arg(long, default_value = "host-only")]
    mode: String,

    /// Path to Linux kernel (required for --mode vm).
    #[arg(long)]
    kernel: Option<PathBuf>,

    /// Path to initrd with capsule agent (required for --mode vm).
    #[arg(long)]
    initrd: Option<PathBuf>,

    /// Unix socket path for VM communication.
    #[arg(long, default_value = "out/capsule_relay/agent.sock")]
    socket_path: PathBuf,

    /// Timeout in seconds for VM startup.
    #[arg(long, default_value = "30")]
    vm_timeout_secs: u64,

    /// Payload file used for the echo harness.
    #[arg(long, default_value = "out/capsule_relay/payload.bin")]
    payload: PathBuf,

    /// Trace output JSON path.
    #[arg(long, default_value = "out/trace/capsule_relay.json")]
    trace_out: PathBuf,

    /// Path to the store service Unix domain socket.
    #[arg(long, default_value = "/tmp/ramen_store.sock")]
    store_socket: PathBuf,

    /// List capsule relay trace artifact kinds and exit.
    #[arg(long)]
    list_trace_kinds: bool,
}

struct TraceRecorder {
    seq: u64,
    events: Vec<ProtocolTraceEvent>,
}

impl TraceRecorder {
    fn new() -> Self {
        Self {
            seq: 0,
            events: Vec::new(),
        }
    }

    fn record(&mut self, dir: TraceDir, op: &str, payload: &[u8], result: Option<String>) {
        self.seq += 1;
        self.events.push(ProtocolTraceEvent {
            seq: self.seq,
            dir,
            op: Some(op.to_string()),
            bytes_hex: hex::encode(payload),
            result,
            notes: None,
        });
    }
}

struct CapsuleAgent {
    session_id: Option<u64>,
    error_count: u32,
}

impl CapsuleAgent {
    fn new() -> Self {
        Self {
            session_id: None,
            error_count: 0,
        }
    }

    fn handle_control(&mut self, env: &Envelope) -> Result<Envelope, String> {
        if env.protocol != PROTOCOL_CAPSULE_CONTROL {
            return Err("control protocol mismatch".into());
        }
        match env.msg_type {
            MSG_HELLO => {
                let _req: Hello = read_payload(env).map_err(|e| format!("hello: {e:?}"))?;
                // SECURITY: Use cryptographic random session ID instead of XOR.
                // The previous XOR-based scheme was trivially predictable and spoofable.
                let session_id = rand::thread_rng().next_u64();
                self.session_id = Some(session_id);
                let reply = HelloReply {
                    session_id,
                    status: STATUS_OK,
                    reserved: 0,
                };
                let mut out = Envelope::empty(PROTOCOL_CAPSULE_CONTROL, MSG_HELLO_REPLY);
                write_payload(&mut out, &reply).map_err(|e| format!("hello reply: {e:?}"))?;
                Ok(out)
            }
            MSG_HEALTH => {
                let req: Health = read_payload(env).map_err(|e| format!("health: {e:?}"))?;
                let status = if Some(req.session_id) == self.session_id {
                    STATUS_OK
                } else {
                    self.error_count += 1;
                    STATUS_INVALID
                };
                let reply = HealthReply {
                    session_id: req.session_id,
                    status,
                    error_count: self.error_count,
                };
                let mut out = Envelope::empty(PROTOCOL_CAPSULE_CONTROL, MSG_HEALTH_REPLY);
                write_payload(&mut out, &reply).map_err(|e| format!("health reply: {e:?}"))?;
                Ok(out)
            }
            MSG_SHUTDOWN => {
                let req: Shutdown = read_payload(env).map_err(|e| format!("shutdown: {e:?}"))?;
                let status = if Some(req.session_id) == self.session_id {
                    STATUS_OK
                } else {
                    self.error_count += 1;
                    STATUS_INVALID
                };
                let reply = ShutdownReply {
                    session_id: req.session_id,
                    status,
                    reserved: 0,
                };
                let mut out = Envelope::empty(PROTOCOL_CAPSULE_CONTROL, MSG_SHUTDOWN_REPLY);
                write_payload(&mut out, &reply).map_err(|e| format!("shutdown reply: {e:?}"))?;
                Ok(out)
            }
            _ => Err("unknown control message".into()),
        }
    }

    fn handle_echo(&mut self, env: &Envelope) -> Result<Envelope, String> {
        if env.protocol != PROTOCOL_ECHO {
            return Err("echo protocol mismatch".into());
        }
        if env.msg_type != MSG_ECHO_REQUEST {
            return Err("unknown echo message".into());
        }
        let req: EchoRequest = read_payload(env).map_err(|e| format!("echo: {e:?}"))?;
        let status = if req.payload_len == 0 {
            self.error_count += 1;
            STATUS_ERR
        } else {
            STATUS_OK
        };
        let reply = EchoReply {
            request_id: req.request_id,
            payload_len: req.payload_len,
            status,
        };
        let mut out = Envelope::empty(PROTOCOL_ECHO, MSG_ECHO_REPLY);
        write_payload(&mut out, &reply).map_err(|e| format!("echo reply: {e:?}"))?;
        Ok(out)
    }
}

impl CapsuleBackend for CapsuleAgent {
    fn call(&mut self, request: &Envelope) -> Result<Envelope, String> {
        match request.protocol {
            PROTOCOL_CAPSULE_CONTROL => self.handle_control(request),
            PROTOCOL_ECHO => self.handle_echo(request),
            _ => Err(format!("unknown protocol: {}", request.protocol)),
        }
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn payload_slice(env: &Envelope) -> &[u8] {
    let len = (env.payload_len as usize).min(env.payload.len());
    &env.payload[..len]
}

/// Ensure the payload directory and file exist, with path traversal protection.
///
/// # Security
/// This function validates that the resolved path does not escape the allowed
/// base directory, preventing path traversal attacks.
fn ensure_payload(path: &Path) -> Result<(), Box<dyn Error>> {
    // Get the base directory from environment or use default
    let base_dir = env::var("CAPSULE_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/capsule_payloads"));

    // Resolve the path to get its canonical form (follows symlinks, removes . and ..)
    // If the path doesn't exist yet, use the parent directory for validation
    let canonical_path = if path.exists() {
        path.canonicalize()?
    } else {
        // For non-existent paths, check the parent directory
        let parent = path.parent().ok_or("path has no parent directory")?;
        if parent.exists() {
            let canonical_parent = parent.canonicalize()?;
            canonical_parent.join(path.file_name().ok_or("path has no filename")?)
        } else {
            // Create parent directories if they don't exist
            fs::create_dir_all(parent)?;
            let canonical_parent = parent.canonicalize()?;
            canonical_parent.join(path.file_name().ok_or("path has no filename")?)
        }
    };

    // SECURITY: Verify the resolved path is within the base directory
    let canonical_base = if base_dir.exists() {
        base_dir.canonicalize()?
    } else {
        fs::create_dir_all(&base_dir)?;
        base_dir.canonicalize()?
    };

    if !canonical_path.starts_with(&canonical_base) {
        return Err(format!(
            "path traversal attempt blocked: {:?} is not within {:?}",
            canonical_path, canonical_base
        )
        .into());
    }

    // Create the file if it doesn't exist
    if !path.exists() {
        fs::write(path, b"capsule_relay_payload_v0\n")?;
    }
    Ok(())
}

/// Ingest a file via store service IPC.
///
/// This function uses the store service client to ingest artifacts,
/// enforcing the architectural boundary that services must not
/// directly access artifact_store_core IO functions.
fn ingest_file(
    client: &mut StoreClient,
    src: &Path,
    kind: &str,
    channel: &str,
) -> Result<(String, u64), Box<dyn Error>> {
    let reply = client.ingest_artifact(kind, channel, src)?;
    Ok((reply.content_id, reply.size_bytes))
}

fn sibling_path(base: &Path, suffix: &str) -> PathBuf {
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("trace");
    parent.join(format!("{}_{}.json", stem, suffix))
}

fn write_trace(
    path: &Path,
    recorder: &TraceRecorder,
    harness_name: &str,
    harness_version: u32,
    capsule_id: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let trace = TraceArtifactV0 {
        schema_version: 1,
        trace_type: TraceType::ProtocolTrace,
        protocol_trace: Some(ProtocolTrace {
            metadata: ProtocolTraceMetadata {
                trace_id: None,
                timestamp_start: None,
                timestamp_end: None,
                capsule_id,
                capsule_image: None,
                harness_name: harness_name.to_string(),
                harness_version,
                policy_bundle_id: Some("policy.stub.v0".to_string()),
            },
            events: recorder.events.clone(),
        }),
        scenario_trace: None,
    };
    validate_trace_artifact(&trace)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&trace)?;
    fs::write(path, json)?;
    Ok(())
}

fn write_observed_caps(
    path: &Path,
    program_id: &str,
    run_id: &str,
    artifact_id: &str,
    evidence_trace_id: &str,
) -> Result<(), Box<dyn Error>> {
    let obs = ObservedCapsV0 {
        schema_version: 1,
        program_id: program_id.to_string(),
        run_id: run_id.to_string(),
        launch_plan_id: None,
        capabilities: vec![ObservedCapability {
            cap: CAPABILITY_NAME.to_string(),
            scope: ObservedCapScope {
                artifact_ids: vec![artifact_id.to_string()],
            },
            counts: ObservedCapCounts {
                granted: 1,
                used: 1,
            },
            evidence: vec![evidence_trace_id.to_string()],
        }],
        evidence: vec![evidence_trace_id.to_string()],
    };
    validate_observed_caps(&obs)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&obs)?;
    fs::write(path, json)?;
    Ok(())
}

fn write_scenario_trace(
    path: &Path,
    scenario_id: &str,
    echo_trace_id: &str,
    control_trace_id: &str,
    observed_caps_id: &str,
    payload_id: &str,
) -> Result<(), Box<dyn Error>> {
    let trace = TraceArtifactV0 {
        schema_version: 1,
        trace_type: TraceType::ScenarioTrace,
        protocol_trace: None,
        scenario_trace: Some(ScenarioTrace {
            metadata: ScenarioTraceMetadata {
                scenario_id: scenario_id.to_string(),
                timestamp_start: None,
                timestamp_end: None,
            },
            events: vec![
                ScenarioTraceEvent {
                    seq: 1,
                    name: "protocol_trace_ref".to_string(),
                    payload: Some(serde_json::json!({
                        "content_id": echo_trace_id,
                        "name": "echo"
                    })),
                },
                ScenarioTraceEvent {
                    seq: 2,
                    name: "protocol_trace_ref".to_string(),
                    payload: Some(serde_json::json!({
                        "content_id": control_trace_id,
                        "name": "control"
                    })),
                },
                ScenarioTraceEvent {
                    seq: 3,
                    name: "observed_caps_ref".to_string(),
                    payload: Some(serde_json::json!({
                        "content_id": observed_caps_id
                    })),
                },
                ScenarioTraceEvent {
                    seq: 4,
                    name: "payload_ref".to_string(),
                    payload: Some(serde_json::json!({
                        "content_id": payload_id
                    })),
                },
            ],
        }),
    };
    validate_trace_artifact(&trace)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&trace)?;
    fs::write(path, json)?;
    Ok(())
}

fn env_with_payload<T: Copy>(protocol: u32, msg_type: u32, value: &T) -> Result<Envelope, String> {
    let mut env = Envelope::empty(protocol, msg_type);
    write_payload(&mut env, value).map_err(|e| format!("payload write: {e:?}"))?;
    Ok(env)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.list_trace_kinds {
        for kind in CapsuleTraceArtifactKind::all() {
            println!("{}", kind.artifact_kind());
        }
        return Ok(());
    }

    // Parse mode
    let mode = match args.mode.as_str() {
        "host-only" => Mode::HostOnly,
        "vm" => Mode::Vm,
        _ => return Err(format!("invalid mode: {} (use 'host-only' or 'vm')", args.mode).into()),
    };

    // Connect to store service with capability for artifact operations
    let capability = StoreCapability::new(
        CAPSULE_RELAY_DOMAIN_ID,
        STORE_RIGHT_READ | STORE_RIGHT_WRITE,
        0, // generation
    );
    let mut store_client = StoreClient::connect_with_capability(
        &args.store_socket,
        CAPSULE_RELAY_DOMAIN_ID,
        Some(capability),
    )
    .map_err(|e| {
        format!(
            "Failed to connect to store service at {}: {}. Is the store service running?",
            args.store_socket.display(),
            e
        )
    })?;

    // Create backend based on mode
    let mut backend: Box<dyn CapsuleBackend> = match mode {
        Mode::HostOnly => {
            println!("CAPSULE_RELAY: mode = host-only");
            Box::new(CapsuleAgent::new())
        }
        Mode::Vm => {
            let kernel = args
                .kernel
                .as_ref()
                .ok_or("--kernel is required for --mode vm")?;
            let initrd = args
                .initrd
                .as_ref()
                .ok_or("--initrd is required for --mode vm")?;
            println!("CAPSULE_RELAY: mode = vm");
            Box::new(vm_backend::VmBackend::spawn(
                kernel,
                initrd,
                &args.socket_path,
                Duration::from_secs(args.vm_timeout_secs),
            )?)
        }
    };

    ensure_payload(&args.payload)?;
    let (payload_id, payload_size) = ingest_file(
        &mut store_client,
        &args.payload,
        "capsule_payload_v0",
        "Experimental",
    )?;
    let payload_len: u32 = payload_size
        .try_into()
        .map_err(|_| "payload too large for echo payload_len")?;

    let mut control_trace = TraceRecorder::new();
    let mut echo_trace = TraceRecorder::new();

    let hello_req = Hello {
        capsule_id: CAPSULE_ID,
        backend_caps: BACKEND_CAPS,
        version_major: 0,
        version_minor: 0,
    };
    let hello_env = env_with_payload(PROTOCOL_CAPSULE_CONTROL, MSG_HELLO, &hello_req)?;
    control_trace.record(TraceDir::Request, "hello", payload_slice(&hello_env), None);
    let hello_reply_env = backend.call(&hello_env)?;
    control_trace.record(
        TraceDir::Response,
        "hello_reply",
        payload_slice(&hello_reply_env),
        None,
    );
    let hello_reply: HelloReply = read_payload(&hello_reply_env).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("hello_reply: {e:?}"),
        )
    })?;
    if hello_reply.status != STATUS_OK {
        return Err("hello failed".into());
    }
    println!("CAPSULE_RELAY: hello ok");

    let health_req = Health {
        session_id: hello_reply.session_id,
    };
    let health_env = env_with_payload(PROTOCOL_CAPSULE_CONTROL, MSG_HEALTH, &health_req)?;
    control_trace.record(
        TraceDir::Request,
        "health",
        payload_slice(&health_env),
        None,
    );
    let health_reply_env = backend.call(&health_env)?;
    control_trace.record(
        TraceDir::Response,
        "health_reply",
        payload_slice(&health_reply_env),
        None,
    );
    let health_reply: HealthReply = read_payload(&health_reply_env).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("health_reply: {e:?}"),
        )
    })?;
    if health_reply.status != STATUS_OK {
        return Err("health failed".into());
    }
    println!("CAPSULE_RELAY: health ok");

    let echo_req = EchoRequest {
        request_id: 1,
        payload_len,
        reserved: 0,
    };
    let echo_env = env_with_payload(PROTOCOL_ECHO, MSG_ECHO_REQUEST, &echo_req)?;
    echo_trace.record(
        TraceDir::Request,
        "echo_request",
        payload_slice(&echo_env),
        None,
    );
    let echo_reply_env = backend.call(&echo_env)?;
    echo_trace.record(
        TraceDir::Response,
        "echo_reply",
        payload_slice(&echo_reply_env),
        None,
    );
    let echo_reply: EchoReply = read_payload(&echo_reply_env).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("echo_reply: {e:?}"),
        )
    })?;
    if echo_reply.status != STATUS_OK
        || echo_reply.request_id != echo_req.request_id
        || echo_reply.payload_len != echo_req.payload_len
    {
        return Err("echo failed".into());
    }
    println!("CAPSULE_RELAY: echo ok");

    // Bad payload test — only in host-only mode (VM mode has different error semantics)
    if mode == Mode::HostOnly {
        let mut bad_env = Envelope::empty(PROTOCOL_ECHO, MSG_ECHO_REQUEST);
        bad_env.payload_len = 4;
        if backend.call(&bad_env).is_ok() {
            return Err("bad payload accepted".into());
        }
        println!("CAPSULE_RELAY: bad payload rejected ok");
    }

    let shutdown_req = Shutdown {
        session_id: hello_reply.session_id,
        reason: 0,
        reserved: 0,
    };
    let shutdown_env = env_with_payload(PROTOCOL_CAPSULE_CONTROL, MSG_SHUTDOWN, &shutdown_req)?;
    control_trace.record(
        TraceDir::Request,
        "shutdown",
        payload_slice(&shutdown_env),
        None,
    );
    let shutdown_reply_env = backend.call(&shutdown_env)?;
    control_trace.record(
        TraceDir::Response,
        "shutdown_reply",
        payload_slice(&shutdown_reply_env),
        None,
    );
    let shutdown_reply: ShutdownReply = read_payload(&shutdown_reply_env).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("shutdown_reply: {e:?}"),
        )
    })?;
    if shutdown_reply.status != STATUS_OK {
        return Err("shutdown failed".into());
    }
    println!("CAPSULE_RELAY: shutdown ok");

    let capsule_id_str = Some(format!("capsule-{CAPSULE_ID:016x}"));
    let trace_control_path = sibling_path(&args.trace_out, "control");
    let trace_echo_path = args.trace_out.clone();
    let observed_path = sibling_path(&args.trace_out, "observed");
    let scenario_path = sibling_path(&args.trace_out, "scenario");

    write_trace(
        &trace_control_path,
        &control_trace,
        "capsule.control",
        0,
        capsule_id_str.clone(),
    )?;
    write_trace(
        &trace_echo_path,
        &echo_trace,
        "harness.echo",
        0,
        capsule_id_str,
    )?;

    let (control_trace_id, _) = ingest_file(
        &mut store_client,
        &trace_control_path,
        "trace_artifact_v0",
        "Experimental",
    )?;
    let (echo_trace_id, _) = ingest_file(
        &mut store_client,
        &trace_echo_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    write_observed_caps(
        &observed_path,
        PROGRAM_ID,
        &echo_trace_id,
        &payload_id,
        &echo_trace_id,
    )?;
    let (observed_caps_id, _) = ingest_file(
        &mut store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?;

    write_scenario_trace(
        &scenario_path,
        SCENARIO_ID,
        &echo_trace_id,
        &control_trace_id,
        &observed_caps_id,
        &payload_id,
    )?;
    let (scenario_trace_id, _) = ingest_file(
        &mut store_client,
        &scenario_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    println!("CAPSULE_RELAY: trace_content_id={}", echo_trace_id);
    println!(
        "CAPSULE_RELAY: control_trace_content_id={}",
        control_trace_id
    );
    println!(
        "CAPSULE_RELAY: observed_caps_content_id={}",
        observed_caps_id
    );
    println!(
        "CAPSULE_RELAY: scenario_trace_content_id={}",
        scenario_trace_id
    );
    println!("CAPSULE_RELAY: payload_content_id={}", payload_id);

    // Clean up backend (important for VM mode)
    backend.shutdown()?;

    println!("CAPSULE_RELAY: ok");

    Ok(())
}
