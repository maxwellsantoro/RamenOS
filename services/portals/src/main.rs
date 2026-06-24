// V-007 Phase 3: Portal service using store service IPC
//
// Refactored to use store_service::StoreClient for artifact operations
// instead of direct artifact_store_core IO functions.
//
// This enforces the architectural boundary: services must not bypass
// the store service for artifact access.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use artifact_store_schema::observed_caps::{
    ObservedCapCounts, ObservedCapScope, ObservedCapability, ObservedCapsV0, validate_observed_caps,
};
use artifact_store_schema::trace::{
    ProtocolTrace, ProtocolTraceEvent, ProtocolTraceMetadata, ScenarioTrace, ScenarioTraceEvent,
    ScenarioTraceMetadata, TraceArtifactV0, TraceDir, TraceType, validate_trace_artifact,
};
use clap::Parser;
use kernel_api::generated::{
    Cancel, CancelReply, OpenFileRo, OpenFileRoReply, ResolveToken, ResolveTokenReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};
use store_service::StoreClient;
use store_service::capability::{STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability};

const PROTOCOL_FILE_PICKER: u32 = 0x100;
const MSG_OPEN_FILE_RO: u32 = 1;
const MSG_OPEN_FILE_RO_REPLY: u32 = 2;
const MSG_RESOLVE_TOKEN: u32 = 3;
const MSG_RESOLVE_TOKEN_REPLY: u32 = 4;
const MSG_CANCEL: u32 = 5;
const MSG_CANCEL_REPLY: u32 = 6;

const STATUS_OK: u32 = 0;
const STATUS_ERR: u32 = 1;
const STATUS_INVALID: u32 = 2;
const STATUS_CANCELED: u32 = 3;

const TOKEN_MAGIC: u64 = 0x504F_0000_0000_0000;
const TOKEN_MASK: u64 = 0xFFFF_0000_0000_0000;

/// Domain ID for the portals service
const PORTALS_DOMAIN_ID: u64 = 1;

const _: () = {
    let _ = [0u8; 64 - core::mem::size_of::<OpenFileRo>()];
    let _ = [0u8; 64 - core::mem::size_of::<OpenFileRoReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<ResolveToken>()];
    let _ = [0u8; 64 - core::mem::size_of::<ResolveTokenReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<Cancel>()];
    let _ = [0u8; 64 - core::mem::size_of::<CancelReply>()];
};

#[derive(Parser, Debug)]
struct Args {
    /// Input file the portal will allow reading (RO).
    #[arg(long, default_value = "out/portal/demo.txt")]
    input: PathBuf,

    /// Installed root containing an `artifacts/` directory.
    #[arg(long, default_value = "out/installed")]
    installed_root: PathBuf,

    /// Trace output JSON path.
    #[arg(long, default_value = "out/trace/portal_file_ro.json")]
    trace_out: PathBuf,

    /// Path to the store service Unix domain socket.
    #[arg(long, default_value = "/tmp/ramen_store.sock")]
    store_socket: PathBuf,
}

#[derive(Clone)]
struct FileSelection {
    content_id: String,
    content_hash: u64,
    size_bytes: u64,
    canceled: bool,
}

struct FilePickerBroker {
    allowed_path: PathBuf,
    next_token: u64,
    selections: HashMap<u64, FileSelection>,
    store_client: StoreClient,
}

impl FilePickerBroker {
    fn new(allowed_path: PathBuf, store_client: StoreClient) -> Self {
        Self {
            allowed_path,
            next_token: 1,
            selections: HashMap::new(),
            store_client,
        }
    }

    fn open_file_ro(&mut self) -> Result<(u64, FileSelection), String> {
        let (content_id, size_bytes) = ingest_file(
            &mut self.store_client,
            &self.allowed_path,
            "portal_file",
            "Experimental",
        )
        .map_err(|e| format!("ingest failed: {}", e))?;
        let content_hash = content_id_hash(&content_id)?;
        let token = self.allocate_token();
        let selection = FileSelection {
            content_id,
            content_hash,
            size_bytes,
            canceled: false,
        };
        self.selections.insert(token, selection.clone());
        Ok((token, selection))
    }

    fn resolve_token(&self, token: u64) -> Result<FileSelection, String> {
        let selection = self.validate_token(token)?;
        if selection.canceled {
            return Err("token canceled".into());
        }
        Ok(selection.clone())
    }

    fn cancel_token(&mut self, token: u64) -> Result<(), String> {
        let selection = self
            .selections
            .get_mut(&token)
            .ok_or_else(|| "token unknown".to_string())?;
        selection.canceled = true;
        Ok(())
    }

    fn read_by_token(
        &mut self,
        token: u64,
        requested_content_id: &str,
        max_len: usize,
    ) -> Result<Vec<u8>, String> {
        // Clone selection data to avoid borrow conflict
        let content_id = {
            let selection = self.validate_token(token)?;
            if selection.canceled {
                return Err("token canceled".into());
            }
            if selection.content_id != requested_content_id {
                return Err("token scope mismatch".into());
            }
            selection.content_id.clone()
        };

        // Use store service IPC to get blob path
        let reply = self
            .store_client
            .get_blob(&content_id)
            .map_err(|e| format!("store service get_blob failed: {}", e))?;

        let blob_path = PathBuf::from(&reply.blob_path);
        let data = fs::read(&blob_path).map_err(|e| format!("read blob failed: {}", e))?;
        Ok(data.into_iter().take(max_len).collect())
    }

    fn validate_token(&self, token: u64) -> Result<&FileSelection, String> {
        if (token & TOKEN_MASK) != TOKEN_MAGIC {
            return Err("token invalid magic".into());
        }
        self.selections
            .get(&token)
            .ok_or_else(|| "token unknown".to_string())
    }

    fn allocate_token(&mut self) -> u64 {
        let token = TOKEN_MAGIC | (self.next_token & 0x0000_FFFF_FFFF_FFFF);
        self.next_token += 1;
        token
    }
}

struct TraceRecorder {
    seq: u64,
    events: Vec<ProtocolTraceEvent>,
}

impl TraceRecorder {
    fn new() -> Self {
        Self {
            seq: 0,
            events: vec![],
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

struct PortalServer {
    broker: FilePickerBroker,
}

impl PortalServer {
    fn new(broker: FilePickerBroker) -> Self {
        Self { broker }
    }

    fn handle_envelope(
        &mut self,
        env: &Envelope,
        recorder: &mut TraceRecorder,
        record: bool,
    ) -> Envelope {
        if env.protocol != PROTOCOL_FILE_PICKER {
            return Envelope::empty(env.protocol, env.msg_type);
        }

        match env.msg_type {
            MSG_OPEN_FILE_RO => {
                if record {
                    recorder.record(TraceDir::Request, "open_file_ro", payload_slice(env), None);
                }
                let req = match read_payload::<OpenFileRo>(env) {
                    Ok(req) => req,
                    Err(_) => {
                        return self.open_reply(0, 0, STATUS_ERR, recorder, record);
                    }
                };
                match self.broker.open_file_ro() {
                    Ok((token, _selection)) => {
                        self.open_reply(req.request_id, token, STATUS_OK, recorder, record)
                    }
                    Err(_) => self.open_reply(req.request_id, 0, STATUS_ERR, recorder, record),
                }
            }
            MSG_RESOLVE_TOKEN => {
                if record {
                    recorder.record(TraceDir::Request, "resolve_token", payload_slice(env), None);
                }
                let req = match read_payload::<ResolveToken>(env) {
                    Ok(req) => req,
                    Err(_) => {
                        return self.resolve_reply(0, STATUS_ERR, 0, 0, recorder, record);
                    }
                };
                match self.broker.resolve_token(req.token) {
                    Ok(selection) => self.resolve_reply(
                        req.token,
                        STATUS_OK,
                        selection.content_hash,
                        selection.size_bytes,
                        recorder,
                        record,
                    ),
                    Err(err) => {
                        let status = if err.contains("canceled") {
                            STATUS_CANCELED
                        } else if err.contains("magic") || err.contains("unknown") {
                            STATUS_INVALID
                        } else {
                            STATUS_ERR
                        };
                        self.resolve_reply(req.token, status, 0, 0, recorder, record)
                    }
                }
            }
            MSG_CANCEL => {
                if record {
                    recorder.record(TraceDir::Request, "cancel", payload_slice(env), None);
                }
                let req = match read_payload::<Cancel>(env) {
                    Ok(req) => req,
                    Err(_) => {
                        return self.cancel_reply(0, STATUS_ERR, recorder, record);
                    }
                };
                let status = if self.broker.cancel_token(req.token).is_ok() {
                    STATUS_OK
                } else {
                    STATUS_INVALID
                };
                self.cancel_reply(req.token, status, recorder, record)
            }
            _ => Envelope::empty(env.protocol, env.msg_type),
        }
    }

    fn open_reply(
        &self,
        request_id: u64,
        token: u64,
        status: u32,
        recorder: &mut TraceRecorder,
        record: bool,
    ) -> Envelope {
        let reply = OpenFileRoReply {
            request_id,
            token,
            status,
            reserved: 0,
        };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_OPEN_FILE_RO_REPLY);
        let _ = write_payload(&mut env, &reply);
        if record {
            recorder.record(
                TraceDir::Response,
                "open_file_ro_reply",
                payload_slice(&env),
                Some(format!("status={}", status)),
            );
        }
        env
    }

    fn resolve_reply(
        &self,
        token: u64,
        status: u32,
        content_hash: u64,
        size_bytes: u64,
        recorder: &mut TraceRecorder,
        record: bool,
    ) -> Envelope {
        let reply = ResolveTokenReply {
            token,
            status,
            reserved: 0,
            content_id_hash: content_hash,
            size_bytes,
        };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_RESOLVE_TOKEN_REPLY);
        let _ = write_payload(&mut env, &reply);
        if record {
            recorder.record(
                TraceDir::Response,
                "resolve_token_reply",
                payload_slice(&env),
                Some(format!("status={}", status)),
            );
        }
        env
    }

    fn cancel_reply(
        &self,
        token: u64,
        status: u32,
        recorder: &mut TraceRecorder,
        record: bool,
    ) -> Envelope {
        let reply = CancelReply {
            token,
            status,
            reserved: 0,
        };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_CANCEL_REPLY);
        let _ = write_payload(&mut env, &reply);
        if record {
            recorder.record(
                TraceDir::Response,
                "cancel_reply",
                payload_slice(&env),
                Some(format!("status={}", status)),
            );
        }
        env
    }
}

struct PortalClient {
    request_id: u64,
}

impl PortalClient {
    fn new() -> Self {
        Self { request_id: 1 }
    }

    fn open_file_ro(
        &mut self,
        server: &mut PortalServer,
        recorder: &mut TraceRecorder,
    ) -> Result<u64, String> {
        let req = OpenFileRo {
            request_id: self.request_id,
            purpose: 0,
            allow_multiple: 0,
            reserved0: 0,
            reserved1: 0,
        };
        self.request_id += 1;
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_OPEN_FILE_RO);
        write_payload(&mut env, &req).map_err(|_| "open_file_ro encode failed".to_string())?;
        let reply_env = server.handle_envelope(&env, recorder, true);
        let reply = read_payload::<OpenFileRoReply>(&reply_env)
            .map_err(|_| "open_file_ro reply decode failed".to_string())?;
        if reply.status != STATUS_OK {
            return Err(format!("open_file_ro status={}", reply.status));
        }
        Ok(reply.token)
    }

    fn resolve_token(
        &self,
        server: &mut PortalServer,
        recorder: &mut TraceRecorder,
        token: u64,
    ) -> Result<ResolveTokenReply, String> {
        let req = ResolveToken { token };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_RESOLVE_TOKEN);
        write_payload(&mut env, &req).map_err(|_| "resolve_token encode failed".to_string())?;
        let reply_env = server.handle_envelope(&env, recorder, true);
        let reply = read_payload::<ResolveTokenReply>(&reply_env)
            .map_err(|_| "resolve_token reply decode failed".to_string())?;
        Ok(reply)
    }

    fn resolve_token_untraced(
        &self,
        server: &mut PortalServer,
        token: u64,
    ) -> Result<ResolveTokenReply, String> {
        let req = ResolveToken { token };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_RESOLVE_TOKEN);
        write_payload(&mut env, &req).map_err(|_| "resolve_token encode failed".to_string())?;
        let reply_env = server.handle_envelope(&env, &mut TraceRecorder::new(), false);
        let reply = read_payload::<ResolveTokenReply>(&reply_env)
            .map_err(|_| "resolve_token reply decode failed".to_string())?;
        Ok(reply)
    }

    fn cancel(&self, server: &mut PortalServer, token: u64) -> Result<CancelReply, String> {
        let req = Cancel { token };
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_CANCEL);
        write_payload(&mut env, &req).map_err(|_| "cancel encode failed".to_string())?;
        let reply_env = server.handle_envelope(&env, &mut TraceRecorder::new(), false);
        let reply = read_payload::<CancelReply>(&reply_env)
            .map_err(|_| "cancel reply decode failed".to_string())?;
        Ok(reply)
    }

    fn bad_payload(&self, server: &mut PortalServer) -> Result<OpenFileRoReply, String> {
        let mut env = Envelope::empty(PROTOCOL_FILE_PICKER, MSG_OPEN_FILE_RO);
        env.payload_len = 0;
        let reply_env = server.handle_envelope(&env, &mut TraceRecorder::new(), false);
        let reply = read_payload::<OpenFileRoReply>(&reply_env)
            .map_err(|_| "bad payload reply decode failed".to_string())?;
        Ok(reply)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    ensure_demo_file(&args.input)?;

    // Connect to store service with capability for artifact operations
    let capability = StoreCapability::new(
        PORTALS_DOMAIN_ID,
        STORE_RIGHT_READ | STORE_RIGHT_WRITE,
        0, // generation
    );
    let store_client = StoreClient::connect_with_capability(
        &args.store_socket,
        PORTALS_DOMAIN_ID,
        Some(capability),
    )
    .map_err(|e| {
        format!(
            "Failed to connect to store service at {}: {}. Is the store service running?",
            args.store_socket.display(),
            e
        )
    })?;

    let broker = FilePickerBroker::new(args.input.clone(), store_client);
    let mut server = PortalServer::new(broker);
    let mut recorder = TraceRecorder::new();
    let mut client = PortalClient::new();

    let token = client.open_file_ro(&mut server, &mut recorder)?;
    println!("PORTAL_FILE_PICKER: open ok token={}", token);

    let reply = client.resolve_token(&mut server, &mut recorder, token)?;
    if reply.status != STATUS_OK {
        return Err(format!("resolve_token status={}", reply.status).into());
    }
    println!(
        "PORTAL_FILE_PICKER: resolve ok content_hash=0x{:x} size={}",
        reply.content_id_hash, reply.size_bytes
    );

    let selection = server
        .broker
        .resolve_token(token)
        .map_err(|e| format!("broker resolve failed: {}", e))?;
    let data = server
        .broker
        .read_by_token(token, &selection.content_id, 16)
        .map_err(|e| format!("read_by_token failed: {}", e))?;
    if data.is_empty() {
        return Err("read_by_token returned empty".into());
    }
    println!("PORTAL_FILE_PICKER: read ok");

    let bad = client.bad_payload(&mut server)?;
    if bad.status != STATUS_ERR {
        return Err("bad payload not rejected".into());
    }
    println!("PORTAL_FILE_PICKER: bad payload rejected ok");

    // For scope checking, we need to ingest another file
    let other_path = args.input.with_extension("other");
    fs::write(&other_path, b"portal_other_v0\n")?;

    let (other_id, _) = ingest_file(
        &mut server.broker.store_client,
        &other_path,
        "portal_file",
        "Experimental",
    )?;

    let scope_check = server
        .broker
        .read_by_token(token, &other_id, 8)
        .err()
        .unwrap_or_else(|| "token scope allowed".to_string());
    if !scope_check.contains("scope") {
        return Err("token scope not enforced".into());
    }
    println!("PORTAL_FILE_PICKER: token scope ok");

    let cancel = client.cancel(&mut server, token)?;
    if cancel.status != STATUS_OK {
        return Err("cancel failed".into());
    }
    let post = client.resolve_token_untraced(&mut server, token)?;
    if post.status != STATUS_CANCELED {
        return Err("token reuse after cancel not rejected".into());
    }
    println!("PORTAL_FILE_PICKER: cancel rejects reuse ok");

    let trace_path = &args.trace_out;
    write_trace(trace_path, &recorder)?;

    // Ingest trace/evidence via the existing broker store client.
    let trace_id = ingest_file(
        &mut server.broker.store_client,
        trace_path,
        "trace_artifact_v0",
        "Experimental",
    )?
    .0;

    let observed_path = sibling_path(trace_path, "observed_caps");
    write_observed_caps(
        &observed_path,
        "portal.file_picker.ro",
        &trace_id,
        &selection.content_id,
        &trace_id,
    )?;
    let observed_caps_id = ingest_file(
        &mut server.broker.store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?
    .0;

    let scenario_path = sibling_path(trace_path, "scenario_trace");
    write_scenario_trace(
        &scenario_path,
        "portal.file_picker.ro",
        &trace_id,
        &observed_caps_id,
        &selection.content_id,
    )?;
    let scenario_trace_id = ingest_file(
        &mut server.broker.store_client,
        &scenario_path,
        "trace_artifact_v0",
        "Experimental",
    )?
    .0;

    println!("PORTAL_FILE_PICKER: trace written {}", trace_path.display());
    println!("PORTAL_FILE_PICKER: trace_content_id={}", trace_id);
    println!(
        "PORTAL_FILE_PICKER: observed_caps_content_id={}",
        observed_caps_id
    );
    println!(
        "PORTAL_FILE_PICKER: scenario_trace_content_id={}",
        scenario_trace_id
    );
    println!("PORTAL_FILE_PICKER: content_id={}", selection.content_id);
    println!("PORTAL_FILE_PICKER: ok");
    Ok(())
}

fn payload_slice(env: &Envelope) -> &[u8] {
    let len = (env.payload_len as usize).min(env.payload.len());
    &env.payload[..len]
}

fn ensure_demo_file(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        fs::write(path, b"portal_file_picker_demo_v0\n")?;
    }
    Ok(())
}

fn content_id_hash(content_id: &str) -> Result<u64, String> {
    let hex = content_id.strip_prefix("sha256:").unwrap_or(content_id);
    if hex.len() < 16 {
        return Err("content_id too short".into());
    }
    let prefix = &hex[..16];
    u64::from_str_radix(prefix, 16).map_err(|_| "content_id hash parse failed".into())
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

fn write_trace(path: &Path, recorder: &TraceRecorder) -> Result<(), Box<dyn Error>> {
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
                harness_name: "portal.file_picker".to_string(),
                harness_version: 1,
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
            cap: "portal.file_picker.ro".to_string(),
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
    protocol_trace_id: &str,
    observed_caps_id: &str,
    artifact_id: &str,
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
                    payload: Some(serde_json::json!({ "content_id": protocol_trace_id })),
                },
                ScenarioTraceEvent {
                    seq: 2,
                    name: "observed_caps_ref".to_string(),
                    payload: Some(serde_json::json!({ "content_id": observed_caps_id })),
                },
                ScenarioTraceEvent {
                    seq: 3,
                    name: "selection".to_string(),
                    payload: Some(serde_json::json!({ "artifact_id": artifact_id })),
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
