use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

// V-007 Phase 2: Store service client for IPC-based artifact operations
use artifact_store_schema::execution_fabric::{
    ExecutionLaunchPlanV0, ExecutionRunnerConfigPayloadV0, NodeSelectorV0, OutputContractV0,
    ResourceRequestV0, RunnerSelectorV0,
};
use store_service::StoreClient;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Emit a launch plan from a catalog (S0/S1 flow).
    EmitPlan(EmitPlanArgs),
    /// Ingest a file into the installed content store (prints content ID).
    Ingest(IngestArgs),
    /// Validate a trace artifact against schema (S3 flow).
    ValidateTrace(ValidateTraceArgs),
    /// Validate an observed_caps artifact against schema (S3 flow).
    ValidateObservedCaps(ValidateObservedCapsArgs),
    /// Validate a queue_item artifact against schema (S4 flow).
    ValidateQueueItem(ValidateQueueItemArgs),
    /// Explain priority score for a queue_item (S4 flow).
    ExplainPriority(ExplainPriorityArgs),
    /// Generate prerequisites graph from queue_items (S4 flow).
    PrereqGraph(PrereqGraphArgs),
    /// Create a claim for a queue_item (S4 flow).
    Claim(ClaimArgs),
    /// Validate a claim artifact against schema (S4 flow).
    ValidateClaim(ValidateClaimArgs),
    /// Resolve active claim from a claim chain (S4 flow).
    ResolveClaim(ResolveClaimArgs),
    /// Validate a crash_context artifact against schema (S5 flow).
    ValidateCrashContext(ValidateCrashContextArgs),
    /// Validate a graduation artifact against schema (S5 flow).
    ValidateGraduation(ValidateGraduationArgs),
    /// Validate a minimal_policy artifact against schema (S5 flow).
    ValidateMinimalPolicy(ValidateMinimalPolicyArgs),
    /// Validate a projection index artifact against schema (S10.3 flow).
    ValidateProjectionIndex(ValidateProjectionIndexArgs),
    /// Validate an execution launch plan against schema (S10.4 flow).
    ValidateExecutionLaunchPlan(ValidateExecutionLaunchPlanArgs),
    /// Validate and ingest a platform snapshot as store evidence (S10.2 flow).
    IngestPlatformSnapshot(IngestPlatformSnapshotArgs),
    /// Propose a minimal policy from observed_caps (S5 wizard flow).
    ProposePolicy(ProposePolicyArgs),
    /// Show graduation status for a program (S5 wizard flow).
    GraduationStatus(GraduationStatusArgs),
}

#[derive(Args, Debug)]
struct EmitPlanArgs {
    #[arg(long)]
    catalog: PathBuf,

    /// Program ID to emit a plan for.
    #[arg(long)]
    program_id: String,

    #[arg(long)]
    out: PathBuf,

    #[arg(long, default_value = "out/artifacts")]
    artifact_root: PathBuf,

    /// Installed root containing an `artifacts/` directory.
    #[arg(long, default_value = "out/installed")]
    installed_root: PathBuf,

    #[arg(long, default_value = "out/tmp")]
    tmp_root: PathBuf,

    #[arg(long, default_value = "component")]
    kind: String,

    #[arg(long, default_value = "Experimental")]
    channel: String,

    /// Store service socket path for IPC-based artifact operations.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,
}

#[derive(Args, Debug)]
struct IngestArgs {
    /// Source file to ingest.
    #[arg(long)]
    src: PathBuf,

    /// Installed root containing an `artifacts/` directory.
    #[arg(long, default_value = "out/installed")]
    installed_root: PathBuf,

    /// Artifact kind string.
    #[arg(long, default_value = "compat_asset")]
    kind: String,

    /// Channel label.
    #[arg(long, default_value = "Experimental")]
    channel: String,

    /// Optional evidence policy TOML for redaction/size checks before ingest.
    #[arg(long)]
    evidence_policy: Option<PathBuf>,

    /// Store service socket path for IPC-based artifact operations.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateTraceArgs {
    /// Trace artifact JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateObservedCapsArgs {
    /// Observed caps JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateQueueItemArgs {
    /// Queue item JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ExplainPriorityArgs {
    /// Queue item JSON file.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct PrereqGraphArgs {
    /// Queue item JSON files (can specify multiple).
    #[arg(long)]
    src: Vec<PathBuf>,

    /// Output file for JSON graph.
    #[arg(long)]
    out: PathBuf,

    /// Also emit DOT format to this file.
    #[arg(long)]
    dot: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ClaimArgs {
    /// Queue item content ID to claim.
    #[arg(long)]
    item: String,

    /// Claimant identifier.
    #[arg(long)]
    claimant: String,

    /// Lease duration in seconds (optional).
    #[arg(long)]
    lease_secs: Option<u64>,

    /// Output file for claim JSON.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateClaimArgs {
    /// Claim JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ResolveClaimArgs {
    /// Claim JSON files (can specify multiple).
    #[arg(long)]
    src: Vec<PathBuf>,

    /// Queue item ID. If omitted, all claims must reference the same queue item.
    #[arg(long)]
    queue_item: Option<String>,

    /// RFC3339 time used for lease evaluation (defaults to current UTC time).
    #[arg(long)]
    now: Option<String>,
}

#[derive(Args, Debug)]
struct ValidateCrashContextArgs {
    /// Crash context JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateGraduationArgs {
    /// Graduation JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateMinimalPolicyArgs {
    /// Minimal policy JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateProjectionIndexArgs {
    /// Projection index JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct ValidateExecutionLaunchPlanArgs {
    /// Execution launch plan JSON file to validate.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Args, Debug)]
struct IngestPlatformSnapshotArgs {
    /// Platform snapshot JSON file to validate and ingest.
    #[arg(long)]
    src: PathBuf,

    /// Store service socket path for IPC-based artifact operations.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,
}

#[derive(Args, Debug)]
struct ProposePolicyArgs {
    /// Program ID.
    #[arg(long)]
    program_id: String,

    /// Target level (compat, posix, wasi, native).
    #[arg(long, default_value = "posix")]
    target_level: String,

    /// Observed caps JSON file (optional).
    #[arg(long)]
    observed_caps: Option<PathBuf>,

    /// Output file for minimal policy JSON.
    #[arg(long)]
    out: PathBuf,

    /// Store service socket path for IPC-based artifact operations.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,
}

#[derive(Args, Debug)]
struct GraduationStatusArgs {
    /// Graduation JSON file.
    #[arg(long)]
    src: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    entries: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    program_id: String,
    runner: String,
    notes: Option<String>,
    #[serde(default)]
    compat_capsule: Option<CatalogCompatCapsule>,
    #[serde(default)]
    gpu_quarantine: Option<CatalogGpuQuarantine>,
    #[serde(default)]
    expected_display_cap_token_high: Option<u64>,
    #[serde(default)]
    expected_display_cap_token_low: Option<u64>,
}

/// Compat capsule configuration as specified in the catalog.
///
/// Paths here are resolved to content-addressed refs when the launch plan
/// is emitted. For v0, compat assets must be provided as paths so the store
/// can ingest them into the installed content store.
#[derive(Debug, Deserialize)]
struct CatalogCompatCapsule {
    #[serde(default)]
    kernel_content_id: Option<String>,
    #[serde(default)]
    initrd_content_id: Option<String>,
    #[serde(default)]
    kernel_path: Option<PathBuf>,
    #[serde(default)]
    initrd_path: Option<PathBuf>,
    #[serde(default)]
    artifact_disks: Vec<CatalogArtifactDisk>,
    #[serde(default = "default_cmdline")]
    cmdline: String,
    #[serde(default)]
    resources: Option<CatalogResources>,
}

#[derive(Debug, Deserialize)]
struct CatalogArtifactDisk {
    #[serde(default)]
    content_id: Option<String>,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default = "default_mount_policy")]
    mount_policy: String,
    #[serde(default = "default_device_type")]
    device_type: String,
}

#[derive(Debug, Deserialize)]
struct CatalogResources {
    #[serde(default = "default_memory_mb")]
    memory_mb: u32,
    #[serde(default = "default_cpus")]
    cpus: u32,
}

#[derive(Debug, Deserialize)]
struct CatalogGpuQuarantine {
    domain_id: u64,
    display_cap_token_high: u64,
    display_cap_token_low: u64,
    width: u32,
    height: u32,
    #[serde(default = "default_gpu_profile")]
    gpu_profile: u32,
}

fn default_gpu_profile() -> u32 {
    1
}

fn default_cmdline() -> String {
    "console=ttyS0".to_string()
}
fn default_mount_policy() -> String {
    "read_only".to_string()
}
fn default_device_type() -> String {
    "virtio_blk".to_string()
}
fn default_memory_mb() -> u32 {
    512
}
fn default_cpus() -> u32 {
    1
}

#[derive(Debug, Serialize)]
struct LaunchCompatCapsule {
    kernel_content_id: String,
    initrd_content_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    artifact_disks: Vec<LaunchArtifactDisk>,
    cmdline: String,
    resources: LaunchResources,
}

#[derive(Debug, Serialize)]
struct LaunchArtifactDisk {
    content_id: String,
    mount_policy: String,
    device_type: String,
}

#[derive(Debug, Serialize)]
struct LaunchResources {
    memory_mb: u32,
    cpus: u32,
}

#[derive(Debug, Serialize)]
struct LaunchGpuQuarantine {
    domain_id: u64,
    display_cap_token_high: u64,
    display_cap_token_low: u64,
    width: u32,
    height: u32,
    gpu_profile: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::EmitPlan(args) => emit_plan(args),
        Command::Ingest(args) => ingest(args),
        Command::ValidateTrace(args) => validate_trace(args),
        Command::ValidateObservedCaps(args) => validate_observed_caps(args),
        Command::ValidateQueueItem(args) => validate_queue_item(args),
        Command::ExplainPriority(args) => explain_priority(args),
        Command::PrereqGraph(args) => prereq_graph(args),
        Command::Claim(args) => claim(args),
        Command::ValidateClaim(args) => validate_claim(args),
        Command::ResolveClaim(args) => resolve_claim(args),
        Command::ValidateCrashContext(args) => validate_crash_context(args),
        Command::ValidateGraduation(args) => validate_graduation(args),
        Command::ValidateMinimalPolicy(args) => validate_minimal_policy(args),
        Command::ValidateProjectionIndex(args) => validate_projection_index(args),
        Command::ValidateExecutionLaunchPlan(args) => validate_execution_launch_plan(args),
        Command::IngestPlatformSnapshot(args) => ingest_platform_snapshot(args),
        Command::ProposePolicy(args) => propose_policy(args),
        Command::GraduationStatus(args) => graduation_status(args),
    }
}

fn emit_plan(args: EmitPlanArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.catalog)?;
    let catalog: Catalog = serde_json::from_str(&raw)?;
    let entry = select_entry(catalog.entries, &args.program_id)?;
    // Resolve catalog-relative paths from CWD (project root), not from
    // the catalog file's parent directory.  Gate scripts and `just` recipes
    // always run from the project root, so catalog paths like
    // "out/compat_s2/kernel/bzImage" are project-root-relative.
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // V-007 Phase 2: Connect to store service for IPC-based artifact operations
    let mut store_client = StoreClient::connect(&args.store_socket)?;

    let mut artifact_ref = None;

    // Build optional compat capsule from catalog entry.
    let compat_capsule = match entry.compat_capsule {
        Some(cc) => Some(build_compat_capsule(
            &args,
            &base_dir,
            cc,
            &mut artifact_ref,
            &mut store_client,
        )?),
        None => None,
    };

    let gpu_quarantine = entry.gpu_quarantine.map(build_gpu_quarantine).transpose()?;

    let artifact_ref = match artifact_ref {
        Some(id) => id,
        None => {
            fs::create_dir_all(&args.tmp_root)?;
            let artifact_src = write_demo_blob(&args.tmp_root, &entry.runner)?;
            // V-007 Phase 2: Use store service IPC for artifact ingestion
            let reply = store_client.ingest_artifact(&args.kind, &args.channel, &artifact_src)?;
            reply.content_id
        }
    };

    let resource_request = compat_capsule.as_ref().map(|cc| ResourceRequestV0 {
        schema_version: 1,
        cpu_cores: cc.resources.cpus,
        memory_mb: cc.resources.memory_mb,
        gpu_profile: gpu_quarantine.as_ref().map(|gpu| gpu.gpu_profile),
    });

    let runner_payload = ExecutionRunnerConfigPayloadV0 {
        notes: entry.notes.clone(),
        compat_capsule: compat_capsule
            .as_ref()
            .and_then(|cc| serde_json::to_value(cc).ok()),
        gpu_quarantine: gpu_quarantine
            .as_ref()
            .and_then(|gpu| serde_json::to_value(gpu).ok()),
        native_wasm: None,
        expected_display_cap_token_high: entry.expected_display_cap_token_high,
        expected_display_cap_token_low: entry.expected_display_cap_token_low,
    };

    let plan = ExecutionLaunchPlanV0 {
        schema_version: 1,
        program_id: entry.program_id,
        artifact_ref,
        runner: RunnerSelectorV0 {
            runner: entry.runner,
        },
        resource_request,
        node_selector: Some(NodeSelectorV0 { node_id: 0 }),
        capability_policy_ref: None,
        input_mounts: vec![],
        output_contract: OutputContractV0 {
            stdout_ref: None,
            stderr_ref: None,
            result_ref: None,
        },
        runner_config: runner_payload
            .to_runner_config()
            .map_err(|err| format!("failed to encode runner_config: {err}"))?,
    };

    artifact_store_schema::execution_fabric::validate_execution_launch_plan(&plan)
        .map_err(|err| format!("invalid execution launch plan: {}", err.0))?;

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&plan)?;
    fs::write(&args.out, json)?;
    println!(
        "store: emitted execution launch plan: {}",
        args.out.display()
    );
    Ok(())
}

fn build_gpu_quarantine(cfg: CatalogGpuQuarantine) -> Result<LaunchGpuQuarantine, Box<dyn Error>> {
    if cfg.domain_id == 0 {
        return Err("gpu_quarantine domain_id must be non-zero".into());
    }
    if cfg.width == 0 || cfg.height == 0 {
        return Err("gpu_quarantine width/height must be non-zero".into());
    }
    Ok(LaunchGpuQuarantine {
        domain_id: cfg.domain_id,
        display_cap_token_high: cfg.display_cap_token_high,
        display_cap_token_low: cfg.display_cap_token_low,
        width: cfg.width,
        height: cfg.height,
        gpu_profile: cfg.gpu_profile,
    })
}

fn ingest(args: IngestArgs) -> Result<(), Box<dyn Error>> {
    let policy = load_evidence_policy_opt(args.evidence_policy.as_ref())?;
    // V-007 Phase 2: Connect to store service for IPC-based artifact operations
    let mut store_client = StoreClient::connect(&args.store_socket)?;
    let content_id = ingest_file(
        &mut store_client,
        &args.src,
        &args.kind,
        &args.channel,
        policy.as_ref(),
    )?;

    // Print content ID for scripting use.
    println!("{}", content_id);
    Ok(())
}

fn validate_trace(args: ValidateTraceArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let trace: artifact_store_schema::trace::TraceArtifactV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::trace::validate_trace_artifact(&trace)?;
    println!("store: trace ok: {}", args.src.display());
    Ok(())
}

fn validate_observed_caps(args: ValidateObservedCapsArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let obs: artifact_store_schema::observed_caps::ObservedCapsV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::observed_caps::validate_observed_caps(&obs)?;
    println!("store: observed_caps ok: {}", args.src.display());
    Ok(())
}

fn write_demo_blob(root: &Path, runner: &str) -> Result<PathBuf, std::io::Error> {
    let path = root.join(match runner {
        "posix_runner_v0" => "demo_posix.sh",
        _ => "demo_input.txt",
    });
    let bytes: &[u8] = match runner {
        "posix_runner_v0" => b"#!/bin/sh\necho POSIX_RUNNER_V0: hello\n",
        _ => b"ramen_s1_demo_blob_v0\n",
    };
    fs::write(&path, bytes)?;
    Ok(path)
}

fn ingest_file(
    store_client: &mut StoreClient,
    src: &Path,
    kind: &str,
    channel: &str,
    policy: Option<&artifact_store_schema::evidence_policy::EvidencePolicyV0>,
) -> Result<String, Box<dyn Error>> {
    // Apply redaction/size limits if policy provided
    let source_bytes = if let Some(policy) = policy {
        let raw = fs::read(src)?;
        policy.redact_and_limit(&raw, kind)?
    } else {
        fs::read(src)?
    };

    // Create exclusive temp file with random name (symlink-safe)
    let mut tmp_file = NamedTempFile::new_in(std::env::temp_dir())
        .map_err(|e| format!("failed to create temp file: {}", e))?;

    tmp_file
        .write_all(&source_bytes)
        .map_err(|e| format!("failed to write temp file: {}", e))?;

    // Sync to disk before passing to service
    tmp_file
        .as_file()
        .sync_all()
        .map_err(|e| format!("failed to sync temp file: {}", e))?;

    // Get path for IPC transfer (file stays open until dropped)
    let tmp_path = tmp_file.path().to_path_buf();

    let reply = store_client.ingest_artifact(kind, channel, &tmp_path)?;

    // RAII cleanup - NamedTempFile deletes on drop
    drop(tmp_file);

    Ok(reply.content_id)
}

fn load_evidence_policy_opt(
    path: Option<&PathBuf>,
) -> Result<Option<artifact_store_schema::evidence_policy::EvidencePolicyV0>, Box<dyn Error>> {
    match path {
        Some(path) => Ok(Some(
            artifact_store_schema::evidence_policy::load_evidence_policy(path)?,
        )),
        None => Ok(None),
    }
}

fn select_entry(
    entries: Vec<CatalogEntry>,
    program_id: &str,
) -> Result<CatalogEntry, Box<dyn Error>> {
    if entries.is_empty() {
        return Err("catalog is empty".into());
    }
    let mut matches = entries
        .into_iter()
        .filter(|entry| entry.program_id == program_id)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(format!("program_id not found: {}", program_id).into());
    }
    if matches.len() > 1 {
        return Err(format!("program_id ambiguous: {}", program_id).into());
    }
    Ok(matches.remove(0))
}

fn resolve_compat_id(
    args: &EmitPlanArgs,
    base_dir: &Path,
    kind: &str,
    path: Option<&PathBuf>,
    content_id: Option<&String>,
    store_client: &mut StoreClient,
) -> Result<String, Box<dyn Error>> {
    let path = match path {
        Some(p) => p,
        None => {
            if content_id.is_some() {
                return Err(format!(
                    "compat asset must be provided as path for ingestion: {}",
                    kind
                )
                .into());
            }
            return Err(format!("compat asset path missing: {}", kind).into());
        }
    };
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    if !resolved.exists() {
        return Err(format!("compat asset missing: {}", resolved.display()).into());
    }
    ingest_file(store_client, &resolved, kind, &args.channel, None)
}

fn build_compat_capsule(
    args: &EmitPlanArgs,
    base_dir: &Path,
    cc: CatalogCompatCapsule,
    artifact_ref: &mut Option<String>,
    store_client: &mut StoreClient,
) -> Result<LaunchCompatCapsule, Box<dyn Error>> {
    let kernel_id = resolve_compat_id(
        args,
        base_dir,
        "compat_kernel",
        cc.kernel_path.as_ref(),
        cc.kernel_content_id.as_ref(),
        store_client,
    )?;
    let initrd_id = resolve_compat_id(
        args,
        base_dir,
        "compat_initrd",
        cc.initrd_path.as_ref(),
        cc.initrd_content_id.as_ref(),
        store_client,
    )?;
    let mut disks: Vec<LaunchArtifactDisk> = Vec::new();
    for disk in cc.artifact_disks {
        let content_id = resolve_compat_id(
            args,
            base_dir,
            "compat_disk",
            disk.path.as_ref(),
            disk.content_id.as_ref(),
            store_client,
        )?;
        if artifact_ref.is_none() {
            *artifact_ref = Some(content_id.clone());
        }
        disks.push(LaunchArtifactDisk {
            content_id,
            mount_policy: disk.mount_policy,
            device_type: disk.device_type,
        });
    }
    if disks.is_empty() {
        return Err("compat_capsule requires at least one artifact disk".into());
    }
    Ok(LaunchCompatCapsule {
        kernel_content_id: kernel_id,
        initrd_content_id: initrd_id,
        artifact_disks: disks,
        cmdline: cc.cmdline,
        resources: match cc.resources {
            Some(r) => LaunchResources {
                memory_mb: r.memory_mb,
                cpus: r.cpus,
            },
            None => LaunchResources {
                memory_mb: default_memory_mb(),
                cpus: default_cpus(),
            },
        },
    })
}

// --- S4 Queue Item handlers ---

fn validate_queue_item(args: ValidateQueueItemArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let item: artifact_store_schema::queue_item::QueueItemV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::queue_item::validate_queue_item(&item)?;
    println!("store: queue_item ok: {}", args.src.display());
    Ok(())
}

fn explain_priority(args: ExplainPriorityArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let item: artifact_store_schema::queue_item::QueueItemV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::queue_item::validate_queue_item(&item)?;

    let explanations = artifact_store_schema::queue_item::explain_priority(&item);
    println!("store: priority explanation for {}", item.program_id);
    for explanation in explanations {
        println!("  - {}", explanation);
    }
    Ok(())
}

fn prereq_graph(args: PrereqGraphArgs) -> Result<(), Box<dyn Error>> {
    let mut items = Vec::new();
    for src in &args.src {
        let raw = fs::read_to_string(src)?;
        let item: artifact_store_schema::queue_item::QueueItemV0 = serde_json::from_str(&raw)?;
        artifact_store_schema::queue_item::validate_queue_item(&item)?;
        items.push(item);
    }

    let mut graph = artifact_store_schema::prereq_graph::PrereqGraph::new();
    graph.add_queue_items(&items);

    // Write JSON graph
    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&graph)?;
    fs::write(&args.out, json)?;
    println!("store: prereq graph written to {}", args.out.display());

    // Optionally write DOT format
    if let Some(dot_path) = &args.dot {
        let dot = graph.to_dot();
        fs::write(dot_path, dot)?;
        println!("store: prereq graph DOT written to {}", dot_path.display());
    }

    // Report high-leverage prereqs
    let high_leverage = graph.high_leverage_prereqs();
    if !high_leverage.is_empty() {
        println!("store: high-leverage prereqs (block multiple items):");
        for (prereq, count) in high_leverage {
            println!("  - {} (blocks {} items)", prereq, count);
        }
    }

    Ok(())
}

fn claim(args: ClaimArgs) -> Result<(), Box<dyn Error>> {
    let claim =
        artifact_store_schema::claim::create_claim(&args.item, &args.claimant, args.lease_secs);
    artifact_store_schema::claim::validate_claim(&claim)?;

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&claim)?;
    fs::write(&args.out, &json)?;
    println!("store: claim written to {}", args.out.display());
    Ok(())
}

fn validate_claim(args: ValidateClaimArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let claim: artifact_store_schema::claim::ClaimV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::claim::validate_claim(&claim)?;
    println!("store: claim ok: {}", args.src.display());
    Ok(())
}

fn resolve_claim(args: ResolveClaimArgs) -> Result<(), Box<dyn Error>> {
    if args.src.is_empty() {
        return Err("resolve-claim requires at least one --src".into());
    }

    let now = match args.now {
        Some(s) => OffsetDateTime::parse(&s, &Rfc3339)
            .map_err(|_| format!("invalid --now RFC3339 timestamp: {}", s))?,
        None => OffsetDateTime::now_utc(),
    };

    let mut claims = Vec::new();
    for src in &args.src {
        let raw = fs::read_to_string(src)?;
        let claim: artifact_store_schema::claim::ClaimV0 = serde_json::from_str(&raw)?;
        artifact_store_schema::claim::validate_claim(&claim)?;
        claims.push(claim);
    }

    let queue_item = match args.queue_item {
        Some(q) => q,
        None => claims
            .first()
            .map(|c| c.queue_item_id.clone())
            .ok_or("no claims provided")?,
    };

    for claim in &claims {
        if claim.queue_item_id != queue_item {
            return Err(format!(
                "claim queue_item mismatch: expected {} got {}",
                queue_item, claim.queue_item_id
            )
            .into());
        }
    }

    let winner = artifact_store_schema::claim::resolve_latest_valid_claim(&claims, now)?;
    match winner {
        Some(winner) => {
            println!(
                "store: claim winner queue_item={} claimant={} timestamp={}",
                winner.queue_item_id, winner.claimant_id, winner.timestamp
            );
        }
        None => {
            println!("store: no active claim queue_item={}", queue_item);
        }
    }
    Ok(())
}

// --- S5 Wizard handlers ---

fn validate_crash_context(args: ValidateCrashContextArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let ctx: artifact_store_schema::crash_context::CrashContextV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::crash_context::validate_crash_context(&ctx)?;
    println!("store: crash_context ok: {}", args.src.display());
    Ok(())
}

fn validate_graduation(args: ValidateGraduationArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let grad: artifact_store_schema::graduation::GraduationV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::graduation::validate_graduation(&grad)?;
    println!("store: graduation ok: {}", args.src.display());
    Ok(())
}

fn validate_minimal_policy(args: ValidateMinimalPolicyArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let policy: artifact_store_schema::minimal_policy::MinimalPolicyV0 =
        serde_json::from_str(&raw)?;
    artifact_store_schema::minimal_policy::validate_minimal_policy(&policy)?;
    println!("store: minimal_policy ok: {}", args.src.display());
    Ok(())
}

fn validate_projection_index(args: ValidateProjectionIndexArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let index: artifact_store_schema::projection_storage::ProjectionIndexV0 =
        serde_json::from_str(&raw)?;
    artifact_store_schema::projection_storage::validate_projection_index(&index)?;
    println!("store: projection_index ok: {}", args.src.display());
    Ok(())
}

fn validate_execution_launch_plan(
    args: ValidateExecutionLaunchPlanArgs,
) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let plan: artifact_store_schema::execution_fabric::ExecutionLaunchPlanV0 =
        serde_json::from_str(&raw)?;
    artifact_store_schema::execution_fabric::validate_execution_launch_plan(&plan)?;
    println!("store: execution_launch_plan ok: {}", args.src.display());
    Ok(())
}

fn ingest_platform_snapshot(args: IngestPlatformSnapshotArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let snapshot: artifact_store_schema::semantic_state::PlatformSnapshotV0 =
        serde_json::from_str(&raw)?;
    artifact_store_schema::semantic_state::validate_platform_snapshot(&snapshot)?;
    let mut store_client = StoreClient::connect(&args.store_socket)?;
    let reply =
        store_client.ingest_artifact("platform_snapshot_v0", "semantic_state", &args.src)?;
    println!(
        "store: platform_snapshot ingested content_id={} size={}",
        reply.content_id, reply.size_bytes
    );
    Ok(())
}

fn propose_policy(args: ProposePolicyArgs) -> Result<(), Box<dyn Error>> {
    use artifact_store_schema::queue_item::TargetLevel;

    // Parse target level
    let target_level = match args.target_level.as_str() {
        "compat" => TargetLevel::Compat,
        "posix" => TargetLevel::Posix,
        "wasi" => TargetLevel::Wasi,
        "native" => TargetLevel::Native,
        _ => return Err(format!("invalid target level: {}", args.target_level).into()),
    };

    // Load observed caps if provided
    let (observed_caps_ref, observed_capabilities) = if let Some(ref obs_path) = args.observed_caps
    {
        let raw = fs::read_to_string(obs_path)?;
        let obs: artifact_store_schema::observed_caps::ObservedCapsV0 = serde_json::from_str(&raw)?;
        artifact_store_schema::observed_caps::validate_observed_caps(&obs)?;

        // V-007 Phase 2: Connect to store service for IPC-based artifact operations
        let mut store_client = StoreClient::connect(&args.store_socket)?;

        // Ingest the observed caps file to get a content ID
        let content_id = ingest_file(
            &mut store_client,
            obs_path,
            "observed_caps",
            "Experimental",
            None,
        )?;

        // Extract capabilities for policy proposal
        let caps: Vec<_> = obs
            .capabilities
            .iter()
            .map(|c| (c.cap.clone(), c.counts.used, c.counts.granted > 0))
            .collect();

        (Some(content_id), caps)
    } else {
        (None, vec![])
    };

    // Propose minimal policy
    let mut policy = artifact_store_schema::minimal_policy::propose_minimal_policy(
        &args.program_id,
        target_level,
        observed_caps_ref,
        &observed_capabilities,
    );
    policy.generate_summary();

    // Validate and write
    artifact_store_schema::minimal_policy::validate_minimal_policy(&policy)?;

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&policy)?;
    fs::write(&args.out, &json)?;
    println!("store: minimal policy written to {}", args.out.display());

    // Print summary
    if let Some(summary) = &policy.summary {
        println!("{}", summary);
    }

    Ok(())
}

fn graduation_status(args: GraduationStatusArgs) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(&args.src)?;
    let mut grad: artifact_store_schema::graduation::GraduationV0 = serde_json::from_str(&raw)?;
    artifact_store_schema::graduation::validate_graduation(&grad)?;

    // Recompute level status from attempts (handles JSON where level_status
    // was serialized empty or is stale).
    grad.update_status();

    println!("store: graduation status for {}", grad.program_id);
    println!("  Current level: {}", grad.current_level.as_str());
    println!("  Target level: {}", grad.target_level.as_str());
    println!("  Progression: {}", grad.progression_summary());
    println!("  Total attempts: {}", grad.attempts.len());

    // Show level summary
    for status in &grad.level_status {
        if status.attempt_count > 0 {
            println!(
                "  {} - {} attempts, {} success, {} crash",
                status.level.as_str(),
                status.attempt_count,
                status.success_count,
                status.crash_count
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use artifact_store_schema::execution_fabric::{
        ExecutionLaunchPlanV0, ExecutionRunnerConfigPayloadV0, NodeSelectorV0, OutputContractV0,
        ResourceRequestV0, RunnerSelectorV0,
    };

    #[test]
    fn canonical_execution_launch_plan_roundtrip() {
        let payload = ExecutionRunnerConfigPayloadV0 {
            notes: Some("demo".into()),
            ..Default::default()
        };
        let plan = ExecutionLaunchPlanV0 {
            schema_version: 1,
            program_id: "ramen.demo.hello".into(),
            artifact_ref: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .into(),
            runner: RunnerSelectorV0 {
                runner: "native_stub".into(),
            },
            resource_request: Some(ResourceRequestV0 {
                schema_version: 1,
                cpu_cores: 1,
                memory_mb: 512,
                gpu_profile: None,
            }),
            node_selector: Some(NodeSelectorV0 { node_id: 0 }),
            capability_policy_ref: None,
            input_mounts: vec![],
            output_contract: OutputContractV0 {
                stdout_ref: None,
                stderr_ref: None,
                result_ref: None,
            },
            runner_config: payload.to_runner_config().expect("runner config"),
        };

        artifact_store_schema::execution_fabric::validate_execution_launch_plan(&plan)
            .expect("valid plan");

        let json = serde_json::to_string_pretty(&plan).expect("serialize");
        let reparsed: ExecutionLaunchPlanV0 = serde_json::from_str(&json).expect("parse");
        assert_eq!(reparsed.schema_version, 1);
        assert_eq!(reparsed.program_id, "ramen.demo.hello");
        assert_eq!(reparsed.runner.runner, "native_stub");

        let payload = ExecutionRunnerConfigPayloadV0::from_runner_config(&reparsed.runner_config)
            .expect("decode payload")
            .expect("payload present");
        assert_eq!(payload.notes.as_deref(), Some("demo"));
    }
}
