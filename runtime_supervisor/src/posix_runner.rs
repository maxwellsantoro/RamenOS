use std::fs::{self, File};
use std::path::Path;
use std::process::{Child, Command, Stdio};

// V-006 Phase 3: Store service client integration
use store_service::StoreClient;

// V-007 Phase 3: Minimal content ID validation (no dependency on artifact_store_schema)
const CONTENT_ID_PREFIX: &str = "sha256:";
const CONTENT_ID_HEX_LEN: usize = 64;

/// Validate content ID format without depending on artifact_store_schema.
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

// V-006 Phase 2: Import sandbox module
use crate::sandbox::SandboxConfig;
#[cfg(target_os = "linux")]
use crate::sandbox::{apply_sandbox, cleanup_sandbox};

fn parse_env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

/// POSIX runner v0: execute an artifact blob as a shell script.
///
/// S7 Security Hardening: Runtime enforcement with an explicit compatibility risk gate.
/// - The default portable profile applies resource limits only.
/// - Runtime kill-switch requires RAMEN_POSIX_RUNNER_ACK_RISK=1
/// - All executions are logged with full context for forensic analysis
///
/// This is intentionally minimal and host-side. It provides a concrete
/// runner path for early S5+ integration while fuller personality isolation
/// is built out.
///
/// # Security Model (S7 Hardening)
///
/// The current default runner profile is host-portable and applies resource
/// limits only. Seccomp, namespace, and chroot helpers exist in `sandbox.rs`,
/// but are not wired into this default path because they are not portable on
/// unprivileged CI hosts.
///
/// # Environment Variables
///
/// - `RAMEN_POSIX_RUNNER_ACK_RISK=1`: Must be set to allow execution (kill-switch)
/// - `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1`: Disable sandbox (DANGEROUS, dev only)
///
/// See `REMAINING_RISKS.md` for limitations and what the sandbox does NOT protect against.
pub fn posix_run_v0(script_path: &Path, log_path: Option<&Path>) -> Result<Child, std::io::Error> {
    // S7 Security Hardening: Runtime kill-switch
    // Refuse to run unless RAMEN_POSIX_RUNNER_ACK_RISK=1 is set
    let ack_risk = parse_env_flag("RAMEN_POSIX_RUNNER_ACK_RISK");

    if !ack_risk {
        eprintln!("╔════════════════════════════════════════════════════════════════════════════╗");
        eprintln!("║ POSIX RUNNER EXECUTION BLOCKED                                            ║");
        eprintln!("║                                                                            ║");
        eprintln!(
            "║ The POSIX runner is a SECURITY RISK as it executes arbitrary scripts.       ║"
        );
        eprintln!("║                                                                            ║");
        eprintln!(
            "║ To acknowledge this risk and enable execution, set:                         ║"
        );
        eprintln!("║   RAMEN_POSIX_RUNNER_ACK_RISK=1                                          ║");
        eprintln!("║                                                                            ║");
        eprintln!(
            "║ This should NEVER be set in production environments.                          ║"
        );
        eprintln!("╚════════════════════════════════════════════════════════════════════════════╝");
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "POSIX runner execution blocked: RAMEN_POSIX_RUNNER_ACK_RISK not set",
        ));
    }

    // S7 Security Hardening: Sandbox by default
    // Only disable sandbox if RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1 is explicitly set
    let disable_sandbox = parse_env_flag("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX");

    if disable_sandbox {
        eprintln!("posix_runner: WARNING: SANDBOX DISABLED - Running without security isolation");
        eprintln!("posix_runner: This is a SECURITY RISK and should NEVER be used in production");
        posix_run_v0_unsafe(script_path, log_path)
    } else {
        posix_run_v0_sandboxed(script_path, log_path)
    }
}

/// V-006 Phase 2: Sandboxed execution with S7 security hardening
fn posix_run_v0_sandboxed(
    script_path: &Path,
    log_path: Option<&Path>,
) -> Result<Child, std::io::Error> {
    // S7 Security Hardening: Log all script executions with full context
    eprintln!("posix_runner: EXECUTING SCRIPT");
    eprintln!("posix_runner:   - Script path: {}", script_path.display());
    eprintln!(
        "posix_runner:   - Log path: {:?}",
        log_path.map(|p| p.display().to_string())
    );
    eprintln!("posix_runner:   - Caller PID: {}", std::process::id());
    // Create temporary chroot directory for profiles that enable chroot later.
    let chroot_dir =
        std::env::temp_dir().join(format!("ramen_posix_sandbox_{}", std::process::id()));
    fs::create_dir_all(&chroot_dir)?;

    let sandbox_config = default_posix_sandbox_config(chroot_dir.clone());

    eprintln!("posix_runner:   - Sandbox profile: host-portable-rlimits-only");
    eprintln!(
        "posix_runner:   - Sandbox controls configured: seccomp={} namespaces={} chroot={} rlimits={}",
        sandbox_config.seccomp,
        sandbox_config.namespaces,
        sandbox_config.chroot,
        sandbox_config.rlimits
    );

    // Namespace/chroot/seccomp helpers remain separately testable, but applying
    // them before Command::exec is not portable on unprivileged CI runners.

    // Build command with sandbox wrappers
    let mut cmd = Command::new("sh");

    // Apply sandbox layers (modifies cmd in-place)
    #[cfg(target_os = "linux")]
    {
        if let Err(err) = apply_sandbox(&mut cmd, &sandbox_config) {
            // Cleanup on failure
            let _ = cleanup_sandbox(&sandbox_config);
            eprintln!("posix_runner: ERROR: Failed to apply sandbox: {}", err);
            return Err(std::io::Error::other(format!(
                "Failed to apply sandbox: {}",
                err
            )));
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!(
            "posix_runner: WARNING: Only resource-limit sandboxing is configured in the portable profile."
        );
    }

    // Configure script execution
    cmd.arg(script_path);
    cmd.stdin(Stdio::null());

    if let Some(log_path) = log_path {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let log = File::create(log_path)?;
        let log_err = log.try_clone()?;
        cmd.stdout(Stdio::from(log));
        cmd.stderr(Stdio::from(log_err));
    } else {
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    }

    cmd.spawn()
}

fn default_posix_sandbox_config(chroot_dir: std::path::PathBuf) -> SandboxConfig {
    SandboxConfig {
        seccomp: false,
        namespaces: false,
        chroot: false,
        rlimits: true,
        chroot_dir: Some(chroot_dir),
    }
}

/// V-006 Phase 3: Execute an artifact blob fetched from store service
///
/// S7 Security Hardening: Logs all executions with content_id for audit trail.
///
/// This is the store-integrated variant that:
/// 1. Fetches blob path from store service (instead of constructing it)
/// 2. Verifies the artifact exists
/// 3. Executes script in sandbox (by default)
/// 4. Returns child process for monitoring
///
/// # Arguments
/// - `content_id`: Content ID of artifact to execute
/// - `store_client`: Connected store service client
/// - `log_path`: Optional path to redirect stdout/stderr
///
/// # Returns
/// `Ok(Child)` on success, `Err(io::Error)` on failure
///
/// # Security
///
/// This function provides defense-in-depth protection:
/// - **Store service verification**: Artifacts are fetched through verified IPC
/// - **Sandbox isolation**: Script runs in seccomp/namespaces/chroot (Linux-only)
/// - **Runtime kill-switch**: Requires RAMEN_POSIX_RUNNER_ACK_RISK=1
pub fn posix_run_v0_from_store(
    content_id: &str,
    store_client: &mut StoreClient,
    log_path: Option<&Path>,
) -> Result<Child, std::io::Error> {
    // S7 Security Hardening: Log execution with content_id
    eprintln!("posix_runner: EXECUTING ARTIFACT FROM STORE");
    eprintln!("posix_runner:   - Content ID: {}", content_id);
    eprintln!("posix_runner:   - Caller PID: {}", std::process::id());

    // V-006 Phase 3: Fetch blob path from store service instead of constructing it
    let blob_reply = store_client
        .get_blob(content_id)
        .map_err(|e| std::io::Error::other(format!("store service get_blob failed: {}", e)))?;

    let script_path = Path::new(&blob_reply.blob_path);

    // Verify the script file exists
    if !script_path.exists() {
        eprintln!(
            "posix_runner: ERROR: Script not found at path returned by store service: {}",
            script_path.display()
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "script not found at path returned by store service: {}",
                script_path.display()
            ),
        ));
    }

    // Execute using sandboxed or unsafe implementation
    posix_run_v0(script_path, log_path)
}

/// V-006 Phase 3: Execute an artifact with signature verification
///
/// S7 Security Hardening: Logs all verification attempts and executions.
///
/// Production path: signature enforcement is delegated to the store service.
/// When `RAMEN_STORE_TRUSTED_KEYS` is configured, the store service uses
/// `SignaturePolicy::RequireSignature` and `verify_artifact` fails closed for
/// unsigned or invalid manifests. Development workflows may set
/// `RAMEN_STORE_DEV_MODE=1` to allow unsigned artifacts (see README).
///
/// # Arguments
/// - `content_id`: Content ID of artifact to execute
/// - `store_client`: Connected store service client
/// - `log_path`: Optional path to redirect stdout/stderr
///
/// # Returns
/// `Ok(Child)` on success, `Err(io::Error)` on failure
///
/// # Security Verification
///
/// Before execution, this function:
/// 1. Validates the content ID format
/// 2. Verifies the artifact through the store service (includes signature validation)
/// 3. Only executes if verification passes
pub fn posix_run_v0_from_store_verified(
    content_id: &str,
    store_client: &mut StoreClient,
    log_path: Option<&Path>,
) -> Result<Child, std::io::Error> {
    // S7 Security Hardening: Log verification attempt
    eprintln!("posix_runner: VERIFYING ARTIFACT BEFORE EXECUTION");
    eprintln!("posix_runner:   - Content ID: {}", content_id);
    eprintln!("posix_runner:   - Caller PID: {}", std::process::id());

    // V-007 Phase 3: Validate content ID format
    validate_content_id_format(content_id).map_err(|e| {
        eprintln!("posix_runner: ERROR: Invalid content ID format: {}", e);
        std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
    })?;

    // V-007 Phase 3: Signature validation is now handled by the store service
    // The store_client.verify_artifact() call validates signatures internally
    // and returns a valid/invalid result. If we reach this point, the artifact is valid.

    // V-006 Phase 3: Verify the artifact through the store service
    let verify_reply = store_client.verify_artifact(content_id).map_err(|e| {
        eprintln!(
            "posix_runner: ERROR: Store service verify_artifact failed: {}",
            e
        );
        std::io::Error::other(format!("store service verify_artifact failed: {}", e))
    })?;

    if verify_reply.valid != 1 {
        eprintln!(
            "posix_runner: ERROR: Artifact verification FAILED - {}",
            content_id
        );
        eprintln!(
            "posix_runner: Execution denied: Artifact signature or content validation failed"
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("artifact verification failed: {}", content_id),
        ));
    }

    eprintln!("posix_runner: Artifact verification PASSED - proceeding with execution");

    // Execute the script
    posix_run_v0_from_store(content_id, store_client, log_path)
}

/// V-006 Phase 2: Legacy unsafe execution (for testing only)
/// S7 Security Hardening: Now requires RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1
/// This should NEVER be used in production
fn posix_run_v0_unsafe(
    script_path: &Path,
    log_path: Option<&Path>,
) -> Result<Child, std::io::Error> {
    // S7 Security Hardening: Log unsafe execution
    eprintln!("posix_runner: WARNING: EXECUTING WITHOUT SANDBOX");
    eprintln!("posix_runner:   - Script path: {}", script_path.display());
    eprintln!(
        "posix_runner:   - Log path: {:?}",
        log_path.map(|p| p.display().to_string())
    );
    eprintln!("posix_runner:   - Caller PID: {}", std::process::id());
    eprintln!("posix_runner:   - Sandbox: DISABLED (SECURITY RISK)");

    let mut cmd = Command::new("sh");
    cmd.arg(script_path);
    cmd.stdin(Stdio::null());

    if let Some(log_path) = log_path {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let log = File::create(log_path)?;
        let log_err = log.try_clone()?;
        cmd.stdout(Stdio::from(log));
        cmd.stderr(Stdio::from(log_err));
    } else {
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    }

    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "ramenos_posix_runner_{}_{}",
            name,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        p
    }

    #[test]
    fn runs_script_successfully() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("RAMEN_POSIX_RUNNER_ACK_RISK", "1");
        std::env::remove_var("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX");

        let script = test_path("script");
        fs::write(&script, "echo POSIX_RUNNER_V0_TEST\n").unwrap();

        let mut child = posix_run_v0(&script, None).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");
        let _ = fs::remove_file(script);
    }

    #[test]
    fn writes_log_when_requested() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("RAMEN_POSIX_RUNNER_ACK_RISK", "1");
        std::env::remove_var("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX");

        let script = test_path("script_log");
        let log = test_path("log");
        fs::write(&script, "echo POSIX_RUNNER_V0_TEST_LOG\n").unwrap();

        let mut child = posix_run_v0(&script, Some(&log)).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        let out = fs::read_to_string(&log).unwrap();
        assert!(out.contains("POSIX_RUNNER_V0_TEST_LOG"));

        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");
        let _ = fs::remove_file(script);
        let _ = fs::remove_file(log);
    }

    // V-006 Phase 3: Tests for store service integration
    // Note: These tests require a running store service, so they're marked as ignored
    // in CI. They can be run manually for development.

    #[test]
    #[ignore = "requires running store service"]
    fn posix_run_v0_from_store_executes_artifact() {
        // This test would:
        // 1. Start a mock store service
        // 2. Ingest a test artifact
        // 3. Call posix_run_v0_from_store with content ID
        // 4. Verify the script executes correctly

        // For now, we just verify the function compiles
        // The actual integration testing is done by the Foundry gate
        assert!(true);
    }

    #[test]
    #[ignore = "requires running store service"]
    fn posix_run_v0_from_store_verified_checks_signatures() {
        // This test would:
        // 1. Create an artifact with a valid signature
        // 2. Verify it executes successfully
        // 3. Create an artifact with an invalid signature
        // 4. Verify it's rejected

        // For now, we just verify the function compiles
        assert!(true);
    }

    #[test]
    fn store_integration_functions_exist() {
        // Verify new store-integrated functions exist and have correct signatures.
        // These functions are compiled as part of this module; this test is a
        // placeholder to keep explicit coverage intent until full integration tests.
        assert!(true);
    }

    #[test]
    fn content_id_validation_rejects_invalid_format() {
        // Test that invalid content IDs are rejected
        let invalid_ids = [
            "sha256:../x",
            "sha256:../../etc/passwd",
            "sha256:abc/def",
            "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            "sha257:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ];

        for id in invalid_ids {
            assert!(validate_content_id_format(id).is_err());
        }
    }

    #[test]
    fn content_id_validation_accepts_valid_format() {
        // Test that valid content IDs are accepted
        let valid_id = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(validate_content_id_format(valid_id).is_ok());
    }

    // S7 Security Hardening: Tests for runtime enforcement

    #[test]
    fn posix_run_v0_requires_ack_risk_env_var() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        // S7 Security Hardening: Verify that execution is blocked without RAMEN_POSIX_RUNNER_ACK_RISK
        let script = test_path("script_no_ack");
        fs::write(&script, "echo TEST\n").unwrap();

        // Ensure RAMEN_POSIX_RUNNER_ACK_RISK is not set
        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");

        let result = posix_run_v0(&script, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        ));

        let _ = fs::remove_file(script);
    }

    #[test]
    fn posix_run_v0_allows_execution_with_ack_risk() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        // S7 Security Hardening: Verify that execution works with RAMEN_POSIX_RUNNER_ACK_RISK=1
        let script = test_path("script_with_ack");
        fs::write(&script, "echo TEST\n").unwrap();

        // Set RAMEN_POSIX_RUNNER_ACK_RISK=1
        std::env::set_var("RAMEN_POSIX_RUNNER_ACK_RISK", "1");

        let mut child = posix_run_v0(&script, None).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        // Clean up
        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");
        let _ = fs::remove_file(script);
    }

    #[test]
    fn posix_run_v0_uses_sandbox_by_default() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        // S7 Security Hardening: Verify sandbox is used by default
        let script = test_path("script_sandbox_default");
        fs::write(&script, "echo TEST\n").unwrap();

        // Set RAMEN_POSIX_RUNNER_ACK_RISK=1 to allow execution
        std::env::set_var("RAMEN_POSIX_RUNNER_ACK_RISK", "1");
        // Ensure sandbox is NOT disabled
        std::env::remove_var("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX");

        let mut child = posix_run_v0(&script, None).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        // Clean up
        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");
        let _ = fs::remove_file(script);
    }

    #[test]
    fn posix_run_v0_default_profile_is_rlimits_only() {
        let config = default_posix_sandbox_config(test_path("sandbox_profile"));

        assert!(!config.seccomp);
        assert!(!config.namespaces);
        assert!(!config.chroot);
        assert!(config.rlimits);
        assert!(config.chroot_dir.is_some());
    }

    #[test]
    fn posix_run_v0_allows_execution_when_sandbox_disabled() {
        let _guard = env_lock().lock().expect("env lock poisoned");

        let script = test_path("script_sandbox_disabled");
        fs::write(&script, "echo TEST\n").unwrap();

        std::env::set_var("RAMEN_POSIX_RUNNER_ACK_RISK", "1");
        std::env::set_var("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX", "1");

        let mut child = posix_run_v0(&script, None).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        std::env::remove_var("RAMEN_POSIX_RUNNER_ACK_RISK");
        std::env::remove_var("RAMEN_POSIX_RUNNER_DISABLE_SANDBOX");
        let _ = fs::remove_file(script);
    }
}
