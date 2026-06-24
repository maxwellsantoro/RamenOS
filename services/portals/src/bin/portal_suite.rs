// V-007 Phase 3: Portal suite using store service IPC
//
// Refactored to use store_service::StoreClient for artifact operations
// instead of direct artifact_store_core IO functions.

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
    CaptureFrame, CaptureFrameReply, PostNotification, PostNotificationReply, ReadClipboard,
    ReadClipboardReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};
use store_service::StoreClient;
use store_service::capability::{STORE_RIGHT_READ, STORE_RIGHT_WRITE, StoreCapability};

const PROTOCOL_CLIPBOARD: u32 = 0x110;
const PROTOCOL_NOTIFICATIONS: u32 = 0x120;
const PROTOCOL_SCREEN_CAPTURE: u32 = 0x130;

const MSG_READ_CLIPBOARD: u32 = 1;
const MSG_READ_CLIPBOARD_REPLY: u32 = 2;
const MSG_POST_NOTIFICATION: u32 = 1;
const MSG_POST_NOTIFICATION_REPLY: u32 = 2;
const MSG_CAPTURE_FRAME: u32 = 1;
const MSG_CAPTURE_FRAME_REPLY: u32 = 2;

const STATUS_OK: u32 = 0;

/// Domain ID for the portal suite service
const PORTAL_SUITE_DOMAIN_ID: u64 = 3;

const _: () = {
    let _ = [0u8; 64 - core::mem::size_of::<ReadClipboard>()];
    let _ = [0u8; 64 - core::mem::size_of::<ReadClipboardReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<PostNotification>()];
    let _ = [0u8; 64 - core::mem::size_of::<PostNotificationReply>()];
    let _ = [0u8; 64 - core::mem::size_of::<CaptureFrame>()];
    let _ = [0u8; 64 - core::mem::size_of::<CaptureFrameReply>()];
};

#[derive(Parser, Debug)]
struct Args {
    /// Directory where raw evidence JSON files are written.
    #[arg(long, default_value = "out/trace")]
    trace_dir: PathBuf,

    /// Program ID stamped into observed capabilities.
    #[arg(long, default_value = "org.ramen.portal_suite")]
    program_id: String,

    /// Run ID stamped into observed capabilities.
    #[arg(long, default_value = "portal_suite_run_v1")]
    run_id: String,

    /// Path to the store service Unix domain socket.
    #[arg(long, default_value = "/tmp/ramen_store.sock")]
    store_socket: PathBuf,
}

#[derive(Clone)]
struct PortalEvidence {
    trace_id: String,
    observed_caps_id: String,
    scenario_id: String,
    artifact_id: Option<String>,
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

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    fs::create_dir_all(&args.trace_dir)?;

    // Connect to store service with capability for artifact operations
    let capability = StoreCapability::new(
        PORTAL_SUITE_DOMAIN_ID,
        STORE_RIGHT_READ | STORE_RIGHT_WRITE,
        0, // generation
    );
    let mut store_client = StoreClient::connect_with_capability(
        &args.store_socket,
        PORTAL_SUITE_DOMAIN_ID,
        Some(capability),
    )
    .map_err(|e| {
        format!(
            "Failed to connect to store service at {}: {}. Is the store service running?",
            args.store_socket.display(),
            e
        )
    })?;

    let clipboard = run_clipboard_portal(&mut store_client, &args)?;
    println!(
        "PORTAL_SUITE: clipboard ok trace={} observed={} scenario={}",
        clipboard.trace_id, clipboard.observed_caps_id, clipboard.scenario_id
    );

    let notifications = run_notifications_portal(&mut store_client, &args)?;
    println!(
        "PORTAL_SUITE: notifications ok trace={} observed={} scenario={}",
        notifications.trace_id, notifications.observed_caps_id, notifications.scenario_id
    );

    let screen_capture = run_screen_capture_portal(&mut store_client, &args)?;
    println!(
        "PORTAL_SUITE: screen_capture ok trace={} observed={} scenario={}",
        screen_capture.trace_id, screen_capture.observed_caps_id, screen_capture.scenario_id
    );

    if let Some(id) = clipboard.artifact_id {
        println!("PORTAL_SUITE: clipboard_artifact={}", id);
    }
    if let Some(id) = screen_capture.artifact_id {
        println!("PORTAL_SUITE: screen_capture_artifact={}", id);
    }
    println!("PORTAL_SUITE: ok");
    Ok(())
}

fn run_clipboard_portal(
    store_client: &mut StoreClient,
    args: &Args,
) -> Result<PortalEvidence, Box<dyn Error>> {
    let mut recorder = TraceRecorder::new();

    let req = ReadClipboard {
        request_id: 1,
        format: 1,
    };
    let mut req_env = Envelope::empty(PROTOCOL_CLIPBOARD, MSG_READ_CLIPBOARD);
    write_payload_checked(&mut req_env, &req)?;
    recorder.record(
        TraceDir::Request,
        "read_clipboard",
        payload_slice(&req_env),
        None,
    );

    let clipboard_payload = args.trace_dir.join("clipboard_payload.txt");
    fs::write(&clipboard_payload, b"portal_clipboard_demo_v1\n")?;
    let (artifact_id, size_bytes) = ingest_file(
        store_client,
        &clipboard_payload,
        "portal_clipboard_blob",
        "Experimental",
    )?;
    let reply = ReadClipboardReply {
        request_id: 1,
        status: STATUS_OK,
        content_id_hash: content_id_hash(&artifact_id)?,
        size_bytes,
    };
    let mut reply_env = Envelope::empty(PROTOCOL_CLIPBOARD, MSG_READ_CLIPBOARD_REPLY);
    write_payload_checked(&mut reply_env, &reply)?;
    let reply_payload = read_payload_checked::<ReadClipboardReply>(&reply_env)?;
    recorder.record(
        TraceDir::Response,
        "read_clipboard_reply",
        payload_slice(&reply_env),
        Some(format!(
            "status={} size_bytes={}",
            reply_payload.status, reply_payload.size_bytes
        )),
    );

    let trace_path = args.trace_dir.join("portal_clipboard.json");
    write_protocol_trace(&trace_path, "portal.clipboard", &recorder)?;
    let (trace_id, _) = ingest_file(
        store_client,
        &trace_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    let observed_path = args.trace_dir.join("portal_clipboard_observed.json");
    write_observed_caps(
        &observed_path,
        &args.program_id,
        &args.run_id,
        "portal.clipboard",
        std::slice::from_ref(&artifact_id),
        &trace_id,
    )?;
    let (observed_caps_id, _) = ingest_file(
        store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?;

    let scenario_path = args.trace_dir.join("portal_clipboard_scenario.json");
    write_scenario_trace(
        &scenario_path,
        "clipboard_read_demo_v1",
        "clipboard_read",
        &trace_id,
        &observed_caps_id,
        Some(&artifact_id),
    )?;
    let (scenario_id, _) = ingest_file(
        store_client,
        &scenario_path,
        "scenario_trace",
        "Experimental",
    )?;

    Ok(PortalEvidence {
        trace_id,
        observed_caps_id,
        scenario_id,
        artifact_id: Some(artifact_id),
    })
}

fn run_notifications_portal(
    store_client: &mut StoreClient,
    args: &Args,
) -> Result<PortalEvidence, Box<dyn Error>> {
    let mut recorder = TraceRecorder::new();

    let req = PostNotification {
        request_id: 1,
        channel: 1,
        title_len: 12,
        body_len: 24,
    };
    let mut req_env = Envelope::empty(PROTOCOL_NOTIFICATIONS, MSG_POST_NOTIFICATION);
    write_payload_checked(&mut req_env, &req)?;
    recorder.record(
        TraceDir::Request,
        "post_notification",
        payload_slice(&req_env),
        None,
    );

    let reply = PostNotificationReply {
        request_id: 1,
        status: STATUS_OK,
        notification_id: 42,
        reserved: 0,
    };
    let mut reply_env = Envelope::empty(PROTOCOL_NOTIFICATIONS, MSG_POST_NOTIFICATION_REPLY);
    write_payload_checked(&mut reply_env, &reply)?;
    let reply_payload = read_payload_checked::<PostNotificationReply>(&reply_env)?;
    recorder.record(
        TraceDir::Response,
        "post_notification_reply",
        payload_slice(&reply_env),
        Some(format!(
            "status={} notification_id={}",
            reply_payload.status, reply_payload.notification_id
        )),
    );

    let trace_path = args.trace_dir.join("portal_notifications.json");
    write_protocol_trace(&trace_path, "portal.notifications", &recorder)?;
    let (trace_id, _) = ingest_file(
        store_client,
        &trace_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    let observed_path = args.trace_dir.join("portal_notifications_observed.json");
    write_observed_caps(
        &observed_path,
        &args.program_id,
        &args.run_id,
        "portal.notifications",
        &[],
        &trace_id,
    )?;
    let (observed_caps_id, _) = ingest_file(
        store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?;

    let scenario_path = args.trace_dir.join("portal_notifications_scenario.json");
    write_scenario_trace(
        &scenario_path,
        "notifications_post_demo_v1",
        "notification_post",
        &trace_id,
        &observed_caps_id,
        None,
    )?;
    let (scenario_id, _) = ingest_file(
        store_client,
        &scenario_path,
        "scenario_trace",
        "Experimental",
    )?;

    Ok(PortalEvidence {
        trace_id,
        observed_caps_id,
        scenario_id,
        artifact_id: None,
    })
}

fn run_screen_capture_portal(
    store_client: &mut StoreClient,
    args: &Args,
) -> Result<PortalEvidence, Box<dyn Error>> {
    let mut recorder = TraceRecorder::new();

    let req = CaptureFrame {
        request_id: 1,
        display_id: 0,
        quality: 90,
        reserved: 0,
    };
    let mut req_env = Envelope::empty(PROTOCOL_SCREEN_CAPTURE, MSG_CAPTURE_FRAME);
    write_payload_checked(&mut req_env, &req)?;
    recorder.record(
        TraceDir::Request,
        "capture_frame",
        payload_slice(&req_env),
        None,
    );

    let frame_payload = args.trace_dir.join("screen_capture_frame.raw");
    fs::write(&frame_payload, b"ramenos_screen_capture_frame_v1\n")?;
    let (artifact_id, size_bytes) = ingest_file(
        store_client,
        &frame_payload,
        "portal_screen_capture_frame",
        "Experimental",
    )?;

    let reply = CaptureFrameReply {
        request_id: 1,
        status: STATUS_OK,
        content_id_hash: content_id_hash(&artifact_id)?,
        size_bytes,
    };
    let mut reply_env = Envelope::empty(PROTOCOL_SCREEN_CAPTURE, MSG_CAPTURE_FRAME_REPLY);
    write_payload_checked(&mut reply_env, &reply)?;
    let reply_payload = read_payload_checked::<CaptureFrameReply>(&reply_env)?;
    recorder.record(
        TraceDir::Response,
        "capture_frame_reply",
        payload_slice(&reply_env),
        Some(format!(
            "status={} size_bytes={}",
            reply_payload.status, reply_payload.size_bytes
        )),
    );

    let trace_path = args.trace_dir.join("portal_screen_capture.json");
    write_protocol_trace(&trace_path, "portal.screen_capture", &recorder)?;
    let (trace_id, _) = ingest_file(
        store_client,
        &trace_path,
        "trace_artifact_v0",
        "Experimental",
    )?;

    let observed_path = args.trace_dir.join("portal_screen_capture_observed.json");
    write_observed_caps(
        &observed_path,
        &args.program_id,
        &args.run_id,
        "portal.screen_capture",
        std::slice::from_ref(&artifact_id),
        &trace_id,
    )?;
    let (observed_caps_id, _) = ingest_file(
        store_client,
        &observed_path,
        "observed_caps_v0",
        "Experimental",
    )?;

    let scenario_path = args.trace_dir.join("portal_screen_capture_scenario.json");
    write_scenario_trace(
        &scenario_path,
        "screen_capture_demo_v1",
        "screen_capture",
        &trace_id,
        &observed_caps_id,
        Some(&artifact_id),
    )?;
    let (scenario_id, _) = ingest_file(
        store_client,
        &scenario_path,
        "scenario_trace",
        "Experimental",
    )?;

    Ok(PortalEvidence {
        trace_id,
        observed_caps_id,
        scenario_id,
        artifact_id: Some(artifact_id),
    })
}

fn payload_slice(env: &Envelope) -> &[u8] {
    let len = (env.payload_len as usize).min(env.payload.len());
    &env.payload[..len]
}

fn write_payload_checked<T: Copy>(env: &mut Envelope, value: &T) -> Result<(), Box<dyn Error>> {
    write_payload(env, value).map_err(|e| format!("wire write error: {:?}", e).into())
}

fn read_payload_checked<T: Copy>(env: &Envelope) -> Result<T, Box<dyn Error>> {
    read_payload(env).map_err(|e| format!("wire read error: {:?}", e).into())
}

fn content_id_hash(content_id: &str) -> Result<u64, Box<dyn Error>> {
    let hex = content_id.strip_prefix("sha256:").unwrap_or(content_id);
    if hex.len() < 16 {
        return Err("content_id too short".into());
    }
    let prefix = &hex[..16];
    Ok(u64::from_str_radix(prefix, 16)?)
}

/// Ingest a file via store service IPC.
fn ingest_file(
    client: &mut StoreClient,
    src: &Path,
    kind: &str,
    channel: &str,
) -> Result<(String, u64), Box<dyn Error>> {
    let reply = client.ingest_artifact(kind, channel, src)?;
    Ok((reply.content_id, reply.size_bytes))
}

fn write_protocol_trace(
    path: &Path,
    harness_name: &str,
    recorder: &TraceRecorder,
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
                harness_name: harness_name.to_string(),
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
    cap: &str,
    artifact_ids: &[String],
    evidence_trace_id: &str,
) -> Result<(), Box<dyn Error>> {
    let obs = ObservedCapsV0 {
        schema_version: 1,
        program_id: program_id.to_string(),
        run_id: run_id.to_string(),
        launch_plan_id: None,
        capabilities: vec![ObservedCapability {
            cap: cap.to_string(),
            scope: ObservedCapScope {
                artifact_ids: artifact_ids.to_vec(),
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
    event_name: &str,
    protocol_trace_id: &str,
    observed_caps_id: &str,
    artifact_id: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut events = vec![
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
            name: event_name.to_string(),
            payload: Some(serde_json::json!({ "result": "ok" })),
        },
    ];

    if let Some(id) = artifact_id {
        events.push(ScenarioTraceEvent {
            seq: 4,
            name: "artifact_ref".to_string(),
            payload: Some(serde_json::json!({ "content_id": id })),
        });
    }

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
            events,
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
