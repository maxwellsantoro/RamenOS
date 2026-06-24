mod compat_runner;
mod fabric_policy;
mod gpu_runner;
mod launch_plan;
mod native_wasm_runner;
#[cfg(feature = "posix_runner_v0_dev")]
mod posix_runner;
// V-006 Phase 2: Sandbox module for POSIX runner
#[cfg(feature = "posix_runner_v0_dev")]
mod sandbox;

use clap::Parser;
#[cfg(feature = "posix_runner_v0_dev")]
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// V-007 Phase 2: Store service client for IPC-based artifact verification
// S10.1: Native WASM runner for production workloads
use gpu_runner::DisplayCapToken;
use launch_plan::parse_launch_plan;
use store_service::StoreClient;

// V-007 Phase 3: Minimal content ID validation (no dependency on artifact_store_schema)
const CONTENT_ID_PREFIX: &str = "sha256:";
const CONTENT_ID_HEX_LEN: usize = 64;

/// Validate content ID format without depending on artifact_store_schema.
///
/// This function performs minimal validation of the content ID format.
/// Full validation is performed by the store service.
fn validate_content_id_format(id: &str) -> Result<(), String> {
    if !id.starts_with(CONTENT_ID_PREFIX) {
        return Err("content id must start with sha256:".to_string());
    }
    let hex = &id[CONTENT_ID_PREFIX.len()..];
    if hex.len() != CONTENT_ID_HEX_LEN {
        return Err("content id must be sha256 + 64 lowercase hex chars".to_string());
    }
    if !hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("content id must be lowercase hex".to_string());
    }
    Ok(())
}

#[cfg(feature = "posix_runner_v0_dev")]
fn parse_env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    plan: Option<PathBuf>,

    #[arg(long, default_value = "out/installed")]
    installed_root: PathBuf,

    #[arg(long)]
    compat_log_path: Option<PathBuf>,

    #[arg(long)]
    posix_log_path: Option<PathBuf>,

    /// Store service socket path for IPC-based artifact verification.
    #[arg(long, default_value = "/tmp/store_service.sock")]
    store_socket: PathBuf,
}

const POSIX_RUNNER_V0_DISABLED_DETAIL: &str = "runner=posix_runner_v0 disabled; rebuild with --features posix_runner_v0_dev for development use";

// V-006: Security warning for POSIX runner
#[cfg(any(feature = "posix_runner_v0_dev", test))]
const POSIX_RUNNER_V0_SECURITY_WARNING: &str = "\
╔══════════════════════════════════════════════════════════════════════════════╗
║                      ⚠️  SECURITY WARNING ⚠️                                ║
║                                                                            ║
║  POSIX runner v0 is enabled via posix_runner_v0_dev feature flag           ║
║                                                                            ║
║  This is a HIGH SEVERITY security vulnerability:                           ║
║  • Shell scripts execute with full runtime_supervisor privileges          ║
║  • NO sandboxing - can execute arbitrary commands on host system           ║
║  • Can read/write arbitrary files on host filesystem                       ║
║  • Can access network, environment variables, and system resources         ║
║                                                                            ║
║  This feature is DEPRECATED and will be removed in S10.                    ║
║  See: docs/plans/security_remediation_v006_v007_v012.md                   ║
║                                                                            ║
║  To suppress this warning, set: RAMEN_POSIX_RUNNER_ACK_RISK=1              ║
╚══════════════════════════════════════════════════════════════════════════════╝";

#[cfg(test)]
fn posix_runner_v0_dispatch_allowed() -> bool {
    cfg!(feature = "posix_runner_v0_dev")
}

// V-006: Check if risk acknowledgment is set
#[cfg(feature = "posix_runner_v0_dev")]
fn posix_runner_risk_acknowledged() -> bool {
    parse_env_flag("RAMEN_POSIX_RUNNER_ACK_RISK")
}

// V-006: Print security warning (unless suppressed via env var)
#[cfg(feature = "posix_runner_v0_dev")]
fn print_posix_runner_security_warning() {
    if !posix_runner_risk_acknowledged() {
        eprintln!("{}", POSIX_RUNNER_V0_SECURITY_WARNING);
        eprintln!();
    }
}

fn posix_runner_v0_disabled_detail() -> &'static str {
    POSIX_RUNNER_V0_DISABLED_DETAIL
}

fn main() {
    let args = Args::parse();

    // V-006: Print security warning on startup if POSIX runner is enabled
    #[cfg(feature = "posix_runner_v0_dev")]
    print_posix_runner_security_warning();

    // V-007 Phase 2: Connect to store service for IPC-based artifact verification
    let mut store_client =
        StoreClient::connect(&args.store_socket).expect("failed to connect to store service");

    if let Some(plan_path) = args.plan {
        let raw = fs::read_to_string(&plan_path).expect("read plan");
        let mut plan = parse_launch_plan(&raw).expect("parse plan");
        let artifact_root = args.installed_root.join("artifacts");
        if let Err(err) = verify_content_id(&plan.artifact_ref, &artifact_root, &mut store_client) {
            eprintln!("supervisor: {}", err);
            std::process::exit(2);
        }

        match fabric_policy::consult_always_local(&plan) {
            Ok(decision) => {
                println!(
                    "supervisor: fabric policy execution_id={:?} node_id={} routed_remote={} dispatch_locally={}",
                    decision.execution_id,
                    decision.node_id,
                    decision.routed_remote,
                    decision.dispatch_locally
                );
                plan.fabric_execution_id = decision.execution_id;
                plan.fabric_node_id = decision.node_id;
            }
            Err(status) => {
                eprintln!("supervisor: fabric policy denied status={}", status);
                std::process::exit(2);
            }
        }

        let _ = plan.notes.as_ref();

        match plan.runner.as_str() {
            "native_stub" => {
                println!(
                    "supervisor: plan ok program_id={} runner={}",
                    plan.program_id, plan.runner
                );
            }
            "linux_vm_v0" => {
                println!(
                    "supervisor: plan ok program_id={} runner={}",
                    plan.program_id, plan.runner
                );
                match plan.compat_capsule {
                    Some(ref capsule) => {
                        if let Err(err) = verify_content_id(
                            &capsule.kernel_content_id,
                            &artifact_root,
                            &mut store_client,
                        ) {
                            eprintln!("supervisor: {}", err);
                            std::process::exit(2);
                        }
                        if let Err(err) = verify_content_id(
                            &capsule.initrd_content_id,
                            &artifact_root,
                            &mut store_client,
                        ) {
                            eprintln!("supervisor: {}", err);
                            std::process::exit(2);
                        }
                        for disk in &capsule.artifact_disks {
                            if let Err(err) = verify_content_id(
                                &disk.content_id,
                                &artifact_root,
                                &mut store_client,
                            ) {
                                eprintln!("supervisor: {}", err);
                                std::process::exit(2);
                            }
                        }
                        let shutdown = Arc::new(AtomicBool::new(false));
                        let shutdown_flag = Arc::clone(&shutdown);
                        if let Err(err) = ctrlc::set_handler(move || {
                            shutdown_flag.store(true, Ordering::SeqCst);
                        }) {
                            eprintln!("supervisor: failed to install signal handler err={}", err);
                            std::process::exit(3);
                        }
                        let mut capsule_cfg = capsule.clone();
                        if capsule_cfg.log_path.is_none() {
                            capsule_cfg.log_path = args
                                .compat_log_path
                                .as_ref()
                                .map(|p| p.display().to_string());
                        }
                        if let Some(ref requested) = capsule_cfg.log_path {
                            let allowed_root = args.installed_root.join("logs").join("compat");
                            let requested = Path::new(requested);
                            match compat_runner::confine_serial_log_path(&allowed_root, requested) {
                                Ok(confined) => {
                                    capsule_cfg.log_path = Some(confined.display().to_string());
                                }
                                Err(err) => {
                                    eprintln!("supervisor: {}", err);
                                    std::process::exit(2);
                                }
                            }
                        }
                        match compat_runner::compat_run_v0(&capsule_cfg, &artifact_root) {
                            Ok(mut child) => {
                                println!("supervisor: compat_run_v0 spawned pid={}", child.id());
                                match wait_child_with_shutdown(&mut child, &shutdown) {
                                    Ok(exit) => {
                                        println!(
                                            "supervisor: compat_run_v0 exited status={}",
                                            exit
                                        );
                                        if shutdown.load(Ordering::SeqCst) {
                                            std::process::exit(130);
                                        }
                                        if !exit.success() {
                                            std::process::exit(3);
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!(
                                            "supervisor: compat_run_v0 wait failed err={}",
                                            err
                                        );
                                        std::process::exit(3);
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("supervisor: compat_run_v0 failed err={}", err);
                                std::process::exit(3);
                            }
                        }
                    }
                    None => {
                        eprintln!("supervisor: runner=linux_vm_v0 requires compat_capsule in plan");
                        std::process::exit(2);
                    }
                }
            }
            "posix_runner_v0" => {
                println!(
                    "supervisor: plan ok program_id={} runner={}",
                    plan.program_id, plan.runner
                );
                #[cfg(feature = "posix_runner_v0_dev")]
                {
                    let shutdown = Arc::new(AtomicBool::new(false));
                    let shutdown_flag = Arc::clone(&shutdown);
                    if let Err(err) = ctrlc::set_handler(move || {
                        shutdown_flag.store(true, Ordering::SeqCst);
                    }) {
                        eprintln!("supervisor: failed to install signal handler err={}", err);
                        std::process::exit(3);
                    }
                    // Validate content ID format
                    if let Err(err) = validate_content_id_format(&plan.artifact_ref) {
                        eprintln!(
                            "supervisor: artifact id invalid ref={} err={}",
                            plan.artifact_ref, err
                        );
                        std::process::exit(2);
                    }

                    // V-006: Print warning before each script execution
                    if !posix_runner_risk_acknowledged() {
                        eprintln!(
                            "supervisor: [POSIX_RUNNER_V0] Executing shell script via store service - SECURITY RISK"
                        );
                        eprintln!(
                            "supervisor: [POSIX_RUNNER_V0] Artifact ID: {}",
                            plan.artifact_ref
                        );
                        eprintln!();
                    }

                    // V-006 Phase 3: Use store-integrated execution with signature verification
                    // New approach: Fetch from store service, verify signatures, then execute
                    match posix_runner::posix_run_v0_from_store_verified(
                        &plan.artifact_ref,
                        &mut store_client,
                        args.posix_log_path.as_deref(),
                    ) {
                        Ok(mut child) => {
                            println!("supervisor: posix_run_v0 spawned pid={}", child.id());
                            match wait_child_with_shutdown(&mut child, &shutdown) {
                                Ok(exit) => {
                                    println!("supervisor: posix_run_v0 exited status={}", exit);
                                    if shutdown.load(Ordering::SeqCst) {
                                        std::process::exit(130);
                                    }
                                    if !exit.success() {
                                        std::process::exit(3);
                                    }
                                }
                                Err(err) => {
                                    eprintln!("supervisor: posix_run_v0 wait failed err={}", err);
                                    std::process::exit(3);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("supervisor: posix_run_v0 failed err={}", err);
                            std::process::exit(3);
                        }
                    }
                }

                #[cfg(not(feature = "posix_runner_v0_dev"))]
                {
                    let _ = args.posix_log_path.as_ref();
                    eprintln!("supervisor: {}", posix_runner_v0_disabled_detail());
                    std::process::exit(2);
                }
            }
            "gpu_quarantine_v1" => {
                println!(
                    "supervisor: plan ok program_id={} runner={}",
                    plan.program_id, plan.runner
                );

                let Some(ref cfg) = plan.gpu_quarantine else {
                    eprintln!("supervisor: gpu_quarantine_v1 requires gpu_quarantine in plan");
                    std::process::exit(2);
                };

                // Require explicit expected token fields (missing = plan error, exit 2)
                let Some(expected_high) = plan.expected_display_cap_token_high else {
                    eprintln!(
                        "supervisor: gpu_quarantine_v1 requires expected_display_cap_token_high in plan"
                    );
                    std::process::exit(2);
                };
                let Some(expected_low) = plan.expected_display_cap_token_low else {
                    eprintln!(
                        "supervisor: gpu_quarantine_v1 requires expected_display_cap_token_low in plan"
                    );
                    std::process::exit(2);
                };

                let expected_token = DisplayCapToken::new(expected_high, expected_low);

                if let Err(err) = gpu_runner::gpu_run_v1(cfg, expected_token) {
                    eprintln!("supervisor: gpu_run_v1 failed err={}", err);
                    std::process::exit(3);
                }
            }
            "native_wasm_v0" => {
                println!(
                    "supervisor: plan ok program_id={} runner={}",
                    plan.program_id, plan.runner
                );

                let Some(ref cfg) = plan.native_wasm else {
                    eprintln!("supervisor: native_wasm_v0 requires native_wasm config");
                    std::process::exit(2);
                };

                match native_wasm_runner::run(
                    &plan.artifact_ref,
                    cfg,
                    &mut store_client,
                    &args.installed_root,
                ) {
                    Ok(result) => {
                        println!(
                            "supervisor: native_wasm_v0 exited code={}",
                            result.exit_code
                        );
                        if result.exit_code != 0 {
                            std::process::exit(result.exit_code);
                        }
                    }
                    Err(err) => {
                        eprintln!("supervisor: native_wasm_v0 failed: {}", err);
                        std::process::exit(3);
                    }
                }
            }
            other => {
                eprintln!("supervisor: unknown runner={}", other);
                std::process::exit(2);
            }
        }
        return;
    }

    // Day 0: host binary only, used to validate workspace wiring.
    println!("runtime_supervisor (host stub): ok");
}

fn verify_content_id(
    id: &str,
    artifact_root: &Path,
    store_client: &mut StoreClient,
) -> Result<(), String> {
    // V-007 Phase 2: Use store service IPC for artifact verification instead of direct filesystem access
    let _ = artifact_root; // Not used when verifying via store service

    // First, validate the content ID format (minimal validation without artifact_store_schema dependency)
    validate_content_id_format(id)?;

    // Use store service to verify the artifact exists and is valid
    let reply = store_client
        .verify_artifact(id)
        .map_err(|err| format!("artifact verification failed: {}", err))?;

    if reply.valid != 1 {
        return Err(format!("artifact invalid: {}", id));
    }

    Ok(())
}

fn wait_child_with_shutdown(
    child: &mut std::process::Child,
    shutdown: &AtomicBool,
) -> Result<std::process::ExitStatus, std::io::Error> {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            let _ = child.kill();
        }
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_content_id_rejects_traversal_payloads() {
        // Note: Tests now skip store_client connection since it requires a running service
        // The format validation is tested via validate_content_id_format
        for bad in ["sha256:../x", "sha256:../../etc/passwd", "sha256:abc/def"] {
            let err = validate_content_id_format(bad).unwrap_err();
            assert_eq!(err, "content id must be sha256 + 64 lowercase hex chars");
        }
    }

    #[test]
    fn verify_content_id_rejects_non_hex_and_wrong_prefix() {
        let bad_hex = "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        let err_hex = validate_content_id_format(bad_hex).unwrap_err();
        assert_eq!(err_hex, "content id must be lowercase hex");

        let bad_prefix = "sha257:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let err_prefix = validate_content_id_format(bad_prefix).unwrap_err();
        assert_eq!(err_prefix, "content id must start with sha256:");
    }

    #[test]
    fn posix_runner_v0_disabled_detail_is_deterministic() {
        assert_eq!(
            posix_runner_v0_disabled_detail(),
            "runner=posix_runner_v0 disabled; rebuild with --features posix_runner_v0_dev for development use"
        );
    }

    // V-006: Tests for warning functionality
    #[test]
    fn security_warning_contains_critical_info() {
        // Verify the security warning contains key phrases
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("SECURITY WARNING"));
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("HIGH SEVERITY"));
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("posix_runner_v0_dev"));
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("DEPRECATED")); // UPPERCASE as in warning
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("RAMEN_POSIX_RUNNER_ACK_RISK"));
        assert!(POSIX_RUNNER_V0_SECURITY_WARNING.contains("arbitrary commands"));
    }

    #[test]
    fn security_warning_mentions_remediation_plan() {
        assert!(
            POSIX_RUNNER_V0_SECURITY_WARNING.contains("security_remediation_v006_v007_v012.md")
        );
    }

    #[cfg(not(feature = "posix_runner_v0_dev"))]
    #[test]
    fn posix_runner_v0_disabled_without_feature() {
        assert!(!posix_runner_v0_dispatch_allowed());
    }

    #[cfg(feature = "posix_runner_v0_dev")]
    #[test]
    fn posix_runner_v0_enabled_with_feature() {
        assert!(posix_runner_v0_dispatch_allowed());
    }
}
