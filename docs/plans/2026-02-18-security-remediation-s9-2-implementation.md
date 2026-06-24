# Security Remediation S9.2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix 5 security vulnerabilities (2 P1, 2 P2, 1 P3) with minimal, targeted changes.

**Architecture:** Each fix is isolated to specific files with no cross-dependencies. Use existing patterns (env parsing, error handling) from the codebase.

**Tech Stack:** Rust, tempfile crate, libc (Unix-specific), existing RamenOS conventions

---

## Task 1: P1 Access-Control Bypass Fix

**Files:**
- Modify: `services/store_service/src/access_control.rs`
- Test: `services/store_service/tests/access_control_tests.rs` (if exists) or inline tests

### Step 1: Add status parsing utility

Add a reusable boolean env parser at the top of `access_control.rs`:

```rust
/// Parse boolean environment variable (1/true/yes/on vs 0/false/no/off)
fn parse_env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_flag_true_values() {
        for val in &["1", "true", "TRUE", "True", "yes", "YES", "on", "ON"] {
            std::env::set_var("TEST_FLAG", val);
            assert!(parse_env_flag("TEST_FLAG"), "expected true for {}", val);
        }
        std::env::remove_var("TEST_FLAG");
    }

    #[test]
    fn test_parse_env_flag_false_values() {
        for val in &["0", "false", "FALSE", "no", "NO", "off", "OFF"] {
            std::env::set_var("TEST_FLAG", val);
            assert!(!parse_env_flag("TEST_FLAG"), "expected false for {}", val);
        }
        std::env::remove_var("TEST_FLAG");
    }

    #[test]
    fn test_parse_env_flag_unset_is_false() {
        std::env::remove_var("TEST_FLAG_UNSET");
        assert!(!parse_env_flag("TEST_FLAG_UNSET"));
    }
}
```

### Step 2: Run tests to verify parsing logic

Run: `cargo test -p store_service parse_env_flag`
Expected: 3 tests PASS

### Step 3: Add new fields to AccessControl struct

Find the `AccessControl` struct and add new fields:

```rust
use std::collections::HashSet;
use std::path::PathBuf;

pub struct AccessControl {
    policy: AccessPolicy,
    pid_whitelist: HashSet<u32>,
    exe_whitelist: Vec<String>,
    /// Exact canonical paths for trusted services (canonicalized at init)
    trusted_paths: HashSet<PathBuf>,
    /// Canonical prefix paths for dev mode (canonicalized at init)
    dev_allowed_roots: Vec<PathBuf>,
    /// Whether dev mode is enabled (from RAMEN_STORE_DEV_MODE)
    dev_mode: bool,
}
```

### Step 4: Update AccessControl::new() to initialize new fields

Update the constructor to initialize new fields and parse config:

```rust
impl AccessControl {
    pub fn new(policy: AccessPolicy) -> Self {
        let dev_mode = parse_env_flag("RAMEN_STORE_DEV_MODE");

        // Parse and canonicalize trusted paths at init
        let trusted_paths: HashSet<PathBuf> = std::env::var("RAMEN_STORE_TRUSTED_PATHS")
            .ok()
            .and_then(|s| {
                let paths: Vec<PathBuf> = s.split(':')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| std::fs::canonicalize(s).ok())
                    .collect();
                if paths.is_empty() { None } else { Some(paths) }
            })
            .map(|p| p.into_iter().collect())
            .unwrap_or_default();

        // Parse and canonicalize dev allowed roots at init
        let dev_allowed_roots: Vec<PathBuf> = std::env::var("RAMEN_STORE_DEV_ALLOWED_ROOTS")
            .ok()
            .and_then(|s| {
                let paths: Vec<PathBuf> = s.split(':')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| std::fs::canonicalize(s).ok())
                    .collect();
                if paths.is_empty() { None } else { Some(paths) }
            })
            .unwrap_or_default();

        Self {
            policy,
            pid_whitelist: HashSet::new(),
            exe_whitelist: Vec::new(),
            trusted_paths,
            dev_allowed_roots,
            dev_mode,
        }
    }
}
```

### Step 5: Add is_known_service_exe method

Add the new method that does proper path validation:

```rust
impl AccessControl {
    /// Check if an executable path is a known trusted service.
    ///
    /// This is the authoritative check used by check_access().
    /// Validates using:
    /// 1. Exact canonical path match against trusted_paths
    /// 2. Dev fallback: basename match + canonical prefix under dev_allowed_roots
    pub fn is_known_service_exe(&self, exe: &str) -> bool {
        let canonical = match std::fs::canonicalize(exe) {
            Ok(p) => p,
            Err(_) => return false,
        };

        // 1. Exact match against pre-canonicalized trusted_paths
        if self.trusted_paths.contains(&canonical) {
            return true;
        }

        // 2. Dev fallback with strict prefix check
        if self.dev_mode {
            let basename = canonical
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if matches!(
                basename,
                "domain_manager" | "runtime_supervisor" | "store_cli" | "store_service"
            ) {
                // Must be under an allowed dev root (canonical prefix match)
                if self.dev_allowed_roots.iter().any(|root| canonical.starts_with(root)) {
                    return true;
                }
            }
        }

        false
    }
}
```

### Step 6: Write test for is_known_service_exe

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_known_service_exe_rejects_basename_only() {
        let dir = tempdir().unwrap();
        let fake_exe = dir.path().join("runtime_supervisor");
        fs::write(&fake_exe, "#!/bin/sh\necho fake").unwrap();

        let ac = AccessControl::new(AccessPolicy::RequireKnownService);

        // Should reject because:
        // 1. Not in trusted_paths
        // 2. Dev mode is off
        assert!(!ac.is_known_service_exe(fake_exe.to_str().unwrap()));
    }

    #[test]
    fn test_is_known_service_exe_accepts_exact_trusted_path() {
        let dir = tempdir().unwrap();
        let trusted_exe = dir.path().join("runtime_supervisor");
        fs::write(&trusted_exe, "#!/bin/sh\necho real").unwrap();
        let canonical = fs::canonicalize(&trusted_exe).unwrap();

        let mut ac = AccessControl::new(AccessPolicy::RequireKnownService);
        ac.trusted_paths.insert(canonical);

        assert!(ac.is_known_service_exe(trusted_exe.to_str().unwrap()));
    }
}
```

### Step 7: Run tests

Run: `cargo test -p store_service is_known_service_exe`
Expected: 2 tests PASS

### Step 8: Update check_access to use is_known_service_exe

Find the `RequireKnownService` branch in `check_access()` and update:

```rust
AccessPolicy::RequireKnownService => {
    // Fail closed if misconfigured
    if self.trusted_paths.is_empty() && !self.dev_mode {
        eprintln!("store_service: MISCONFIGURATION - RequireKnownService policy with no trusted_paths and dev mode off");
        return AccessDecision::Denied;
    }

    if let Some(ref exe) = client.exe_path {
        if self.is_known_service_exe(exe) {
            AccessDecision::Allowed
        } else {
            AccessDecision::Denied
        }
    } else {
        AccessDecision::Denied
    }
}
```

### Step 9: Deprecate ClientInfo::is_known_service

Add deprecation notice to `ClientInfo::is_known_service()`:

```rust
impl ClientInfo {
    /// Check if this client is a known service by basename.
    ///
    /// **DEPRECATED**: This method is kept for backward compatibility and tests only.
    /// Use `AccessControl::is_known_service_exe()` for actual access decisions.
    #[deprecated(since = "0.7.2", note = "Use AccessControl::is_known_service_exe() instead")]
    pub fn is_known_service(&self) -> bool {
        // ... existing implementation unchanged ...
    }
}
```

### Step 10: Run full store_service tests

Run: `cargo test -p store_service`
Expected: All tests PASS

### Step 11: Commit P1 access-control fix

```bash
git add services/store_service/src/access_control.rs
git commit -m "$(cat <<'EOF'
fix(security): close access-control bypass via basename matching

P1 vulnerability: is_known_service() only checked basename after
canonicalization, allowing renamed malicious binaries to pass.

Changes:
- Add AccessControl::is_known_service_exe() with exact canonical path matching
- Add RAMEN_STORE_TRUSTED_PATHS env for explicit trusted paths
- Add RAMEN_STORE_DEV_ALLOWED_ROOTS for dev mode prefix allowlist
- Add RAMEN_STORE_DEV_MODE with proper boolean parsing (1/true/yes/on)
- Fail closed if RequireKnownService with no trusted_paths and dev mode off
- Deprecate ClientInfo::is_known_service() (kept for tests only)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: P1 Predictable Temp File Fix

**Files:**
- Modify: `store_cli/src/main.rs:527-551`
- Modify: `store_cli/Cargo.toml`

### Step 1: Add tempfile dependency

Check if tempfile exists in `store_cli/Cargo.toml`, if not add:

```toml
[dependencies]
# ... existing deps ...
tempfile = "3"
```

### Step 2: Rewrite ingest_file to use NamedTempFile

Replace the `ingest_file` function (lines 527-551):

```rust
use tempfile::NamedTempFile;
use std::io::Write;

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

    tmp_file.write_all(&source_bytes)
        .map_err(|e| format!("failed to write temp file: {}", e))?;

    // Sync to disk before passing to service
    tmp_file.as_file().sync_all()
        .map_err(|e| format!("failed to sync temp file: {}", e))?;

    // Get path for IPC transfer (file stays open until dropped)
    let tmp_path = tmp_file.path().to_path_buf();

    let reply = store_client.ingest_artifact(kind, channel, &tmp_path)?;

    // RAII cleanup - NamedTempFile deletes on drop
    drop(tmp_file);

    Ok(reply.content_id)
}
```

### Step 3: Build to verify compilation

Run: `cargo build -p store_cli`
Expected: Compiles without errors

### Step 4: Run store_cli tests

Run: `cargo test -p store_cli`
Expected: All tests PASS

### Step 5: Commit P1 temp file fix

```bash
git add store_cli/src/main.rs store_cli/Cargo.toml
git commit -m "$(cat <<'EOF'
fix(security): use exclusive temp files to prevent symlink attacks

P1 vulnerability: /tmp/ramen_ingest_<pid> was predictable, allowing
TOCTOU/symlink attacks by local attackers.

Changes:
- Replace predictable path with tempfile::NamedTempFile
- Use exclusive creation (O_EXCL) with random names
- Sync to disk before IPC transfer
- RAII cleanup on drop

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: P2 Store Client Status Code Checking

**Files:**
- Create: `services/store_service/src/status.rs`
- Modify: `services/store_service/src/lib.rs` (add mod export)
- Modify: `services/store_service/src/client.rs`
- Modify: `services/store_service/src/main.rs` (use shared constants)

### Step 1: Create shared status module

Create `services/store_service/src/status.rs`:

```rust
//! Shared status codes for store service IPC.
//!
//! These constants are used by both the server (main.rs) and client (client.rs)
//! to ensure consistent status handling.

/// Operation completed successfully
pub const STATUS_OK: u32 = 0;

/// Requested resource not found
pub const STATUS_NOT_FOUND: u32 = 1;

/// I/O error during operation
pub const STATUS_IO_ERROR: u32 = 3;

/// Request validation failed
pub const STATUS_VALIDATION_FAILED: u32 = 4;

/// Permission denied for operation
pub const STATUS_PERMISSION_DENIED: u32 = 5;
```

### Step 2: Export status module in lib.rs

Add to `services/store_service/src/lib.rs`:

```rust
pub mod status;
// Re-export for convenience
pub use status::*;
```

### Step 3: Update server to use shared constants

In `services/store_service/src/main.rs`, replace local constants with imports:

```rust
use store_service::status::*;

// Remove any local STATUS_* constant definitions
```

### Step 4: Add ensure_status_ok to StoreClient

In `services/store_service/src/client.rs`, add the helper:

```rust
use crate::status::*;

impl StoreClient {
    /// Validate that service reply status indicates success.
    /// Returns error with operation context if status is not OK.
    fn ensure_status_ok(&self, op: &str, status: u32) -> Result<(), StoreClientError> {
        match status {
            STATUS_OK => Ok(()),
            STATUS_NOT_FOUND => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: not found", op),
            }),
            STATUS_IO_ERROR => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: I/O error", op),
            }),
            STATUS_VALIDATION_FAILED => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: validation failed", op),
            }),
            STATUS_PERMISSION_DENIED => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: permission denied", op),
            }),
            _ => Err(StoreClientError::ServiceError {
                status,
                message: format!("{}: unknown error (status={})", op, status),
            }),
        }
    }
}
```

### Step 5: Add ServiceError variant to StoreClientError

Find the `StoreClientError` enum and add:

```rust
#[derive(Debug)]
pub enum StoreClientError {
    // ... existing variants ...
    ServiceError {
        status: u32,
        message: String,
    },
}
```

### Step 6: Update get_manifest to check status

Find `get_manifest` method and add status check after request_id validation:

```rust
pub fn get_manifest(&mut self, content_id: &str) -> Result<GetManifestReply, StoreClientError> {
    // ... existing code to send request ...

    let reply: GetManifestReply = bincode::deserialize(&reply_bytes)?;
    self.validate_request_id(reply.request_id, request_id)?;
    self.ensure_status_ok("get_manifest", reply.status)?;  // ADD THIS LINE

    Ok(reply)
}
```

### Step 7: Update get_blob to check status

```rust
pub fn get_blob(&mut self, content_id: &str) -> Result<GetBlobReply, StoreClientError> {
    // ... existing code ...

    let reply: GetBlobReply = bincode::deserialize(&reply_bytes)?;
    self.validate_request_id(reply.request_id, request_id)?;
    self.ensure_status_ok("get_blob", reply.status)?;  // ADD THIS LINE

    Ok(reply)
}
```

### Step 8: Update verify_artifact to check status

```rust
pub fn verify_artifact(&mut self, content_id: &str) -> Result<VerifyArtifactReply, StoreClientError> {
    // ... existing code ...

    let reply: VerifyArtifactReply = bincode::deserialize(&reply_bytes)?;
    self.validate_request_id(reply.request_id, request_id)?;
    self.ensure_status_ok("verify_artifact", reply.status)?;  // ADD THIS LINE

    Ok(reply)
}
```

### Step 9: Update ingest_artifact to check status

```rust
pub fn ingest_artifact(&mut self, kind: &str, channel: &str, src_path: &Path) -> Result<IngestArtifactReply, StoreClientError> {
    // ... existing code ...

    let reply: IngestArtifactReply = bincode::deserialize(&reply_bytes)?;
    self.validate_request_id(reply.request_id, request_id)?;
    self.ensure_status_ok("ingest_artifact", reply.status)?;  // ADD THIS LINE

    Ok(reply)
}
```

### Step 10: Build to verify compilation

Run: `cargo build -p store_service`
Expected: Compiles without errors

### Step 11: Run store_service tests

Run: `cargo test -p store_service`
Expected: All tests PASS

### Step 12: Commit P2 status code fix

```bash
git add services/store_service/src/status.rs services/store_service/src/lib.rs services/store_service/src/client.rs services/store_service/src/main.rs
git commit -m "$(cat <<'EOF'
fix(security): check status codes in store client methods

P2 vulnerability: Client methods returned replies without checking
status field, causing callers to treat failed ops as success.

Changes:
- Add shared status.rs module with STATUS_* constants
- Add StoreClientError::ServiceError variant
- Add ensure_status_ok() helper method
- Apply status checking to get_manifest, get_blob, verify_artifact, ingest_artifact

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: P2 GPU Token Bypass Fix

**Files:**
- Modify: `runtime_supervisor/src/main.rs`
- Modify: `runtime_supervisor/src/gpu_runner.rs`

### Step 1: Update gpu_quarantine_v1 handling in main.rs

Find the `"gpu_quarantine_v1"` match arm and replace with explicit validation:

```rust
"gpu_quarantine_v1" => {
    println!("supervisor: plan ok program_id={} runner={}", plan.program_id, plan.runner);

    let Some(ref cfg) = plan.gpu_quarantine else {
        eprintln!("supervisor: gpu_quarantine_v1 requires gpu_quarantine in plan");
        std::process::exit(2);
    };

    // Require explicit expected token fields (missing = plan error, exit 2)
    let Some(expected_high) = plan.expected_display_cap_token_high else {
        eprintln!("supervisor: gpu_quarantine_v1 requires expected_display_cap_token_high in plan");
        std::process::exit(2);
    };
    let Some(expected_low) = plan.expected_display_cap_token_low else {
        eprintln!("supervisor: gpu_quarantine_v1 requires expected_display_cap_token_low in plan");
        std::process::exit(2);
    };

    let expected_token = DisplayCapToken::new(expected_high, expected_low);

    if let Err(err) = gpu_runner::gpu_run_v1(cfg, expected_token) {
        eprintln!("supervisor: gpu_run_v1 failed err={}", err);
        std::process::exit(3);
    }
}
```

### Step 2: Add zero token rejection in gpu_runner.rs

Find `gpu_run_v1` function and add validation at the start:

```rust
pub fn gpu_run_v1(config: &GpuQuarantineConfig, expected_token: DisplayCapToken) -> Result<(), String> {
    if config.domain_id == 0 {
        return Err("gpu_run_v1: domain_id must be non-zero".to_string());
    }

    // Reject all-zero expected token (configuration error)
    if expected_token.high == 0 && expected_token.low == 0 {
        return Err("gpu_run_v1: expected_token must be non-zero (check plan configuration)".to_string());
    }

    let provided_token = DisplayCapToken::new(
        config.display_cap_token_high,
        config.display_cap_token_low,
    );

    // Reject all-zero provided token
    if provided_token.high == 0 && provided_token.low == 0 {
        return Err("gpu_run_v1: provided display_cap_token must be non-zero".to_string());
    }

    // Existing token comparison
    if provided_token != expected_token {
        return Err(format!(
            "gpu_run_v1: invalid display_cap_token provided={:016x}{:016x} expected={:016x}{:016x}",
            provided_token.high, provided_token.low,
            expected_token.high, expected_token.low
        ));
    }

    // ... rest of existing function unchanged
}
```

### Step 3: Add unit tests for token validation

Add tests in `gpu_runner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> GpuQuarantineConfig {
        GpuQuarantineConfig {
            domain_id: 700,
            display_cap_token_high: 1,
            display_cap_token_low: 1,
            width: 800,
            height: 600,
            gpu_profile: 1,
        }
    }

    #[test]
    fn rejects_zero_expected_token() {
        let cfg = test_config();
        let zero_expected = DisplayCapToken::new(0, 0);
        let err = gpu_run_v1(&cfg, zero_expected).expect_err("zero expected token should fail");
        assert!(err.contains("expected_token must be non-zero"));
    }

    #[test]
    fn rejects_zero_provided_token() {
        let mut cfg = test_config();
        cfg.display_cap_token_high = 0;
        cfg.display_cap_token_low = 0;
        let expected = DisplayCapToken::new(1, 1);
        let err = gpu_run_v1(&cfg, expected).expect_err("zero provided token should fail");
        assert!(err.contains("provided display_cap_token must be non-zero"));
    }

    #[test]
    fn rejects_mismatched_tokens() {
        let mut cfg = test_config();
        cfg.display_cap_token_high = 1;
        cfg.display_cap_token_low = 1;
        let expected = DisplayCapToken::new(2, 2);
        let err = gpu_run_v1(&cfg, expected).expect_err("mismatched tokens should fail");
        assert!(err.contains("invalid display_cap_token"));
    }
}
```

### Step 4: Run gpu_runner tests

Run: `cargo test -p runtime_supervisor gpu_run_v1`
Expected: 3 tests PASS

### Step 5: Build runtime_supervisor

Run: `cargo build -p runtime_supervisor`
Expected: Compiles without errors

### Step 6: Commit P2 GPU token fix

```bash
git add runtime_supervisor/src/main.rs runtime_supervisor/src/gpu_runner.rs
git commit -m "$(cat <<'EOF'
fix(security): reject zero/missing GPU capability tokens

P2 vulnerability: Optional token fields defaulted to 0, allowing
all-zero tokens to bypass capability validation.

Changes:
- Require explicit expected_display_cap_token_high/low in plan (exit 2)
- Reject all-zero expected tokens in gpu_run_v1
- Reject all-zero provided tokens in gpu_run_v1
- Add unit tests for token validation

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: P3 Log-Path Symlink Escape Fix

**Files:**
- Modify: `runtime_supervisor/src/compat_runner.rs`

### Step 1: Update confine_serial_log_path to reject existing paths

Find the function and update to use `symlink_metadata`:

```rust
/// Validate that a requested serial log path is confined to allowed_root.
///
/// Security checks:
/// 1. Parent directory must canonicalize under allowed_root
/// 2. Final path must not already exist (prevents symlink attacks)
pub fn confine_serial_log_path(allowed_root: &Path, requested: &Path) -> Result<PathBuf, String> {
    // Canonicalize allowed root once
    let allowed_root_canon = fs::canonicalize(allowed_root).map_err(|e| {
        format!(
            "failed to canonicalize allowed root {}: {}",
            allowed_root.display(),
            e
        )
    })?;

    // Build candidate path
    let candidate = allowed_root.join(requested);

    // Validate parent is under allowed root (existing logic)
    let parent = candidate.parent().ok_or_else(|| {
        format!("log path has no parent directory: {}", candidate.display())
    })?;

    // Create parent directories if needed (with restrictive permissions)
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| {
            format!("failed to create log directory {}: {}", parent.display(), e)
        })?;
    }

    let parent_canon = fs::canonicalize(parent).map_err(|e| {
        format!(
            "failed to canonicalize log parent {}: {}",
            parent.display(),
            e
        )
    })?;

    if !parent_canon.starts_with(&allowed_root_canon) {
        return Err(format!(
            "log path parent escapes allowed root: {} (resolved to {})",
            candidate.display(),
            parent_canon.display()
        ));
    }

    // NEW: Reject if path already exists (symlink_metadata doesn't follow symlinks)
    match fs::symlink_metadata(&candidate) {
        Ok(_) => {
            return Err(format!(
                "compat log path already exists (possible symlink attack): {}",
                candidate.display()
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Path doesn't exist, proceed
        }
        Err(e) => {
            return Err(format!(
                "failed to check log path {}: {}",
                candidate.display(),
                e
            ))
        }
    }

    Ok(candidate)
}
```

### Step 2: Add atomic file creation with O_NOFOLLOW

Add a new helper function for secure log file creation:

```rust
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

/// Create log file atomically with symlink protection.
///
/// Uses O_NOFOLLOW on Unix to prevent symlink swap attacks.
#[cfg(unix)]
pub fn create_log_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true) // Fail if exists (atomic)
        .mode(0o600) // Restrictive permissions
        .custom_flags(libc::O_NOFOLLOW) // Fail if symlink
        .open(path)
        .map_err(|e| format!("failed to create log file {}: {}", path.display(), e))
}

#[cfg(not(unix))]
pub fn create_log_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("failed to create log file {}: {}", path.display(), e))
}
```

### Step 3: Add libc dependency if needed

Check `runtime_supervisor/Cargo.toml` for libc, add if missing:

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

### Step 4: Add test for symlink rejection

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::os::unix::fs::symlink;

    #[test]
    fn test_rejects_existing_symlink() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a symlink at the target path
        let target = dir.path().join("escaped.log");
        let symlink_path = root.join("serial.log");
        symlink(&target, &symlink_path).unwrap();

        let result = confine_serial_log_path(root, Path::new("serial.log"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_rejects_existing_regular_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a regular file at the target path
        let existing = root.join("serial.log");
        fs::write(&existing, "old content").unwrap();

        let result = confine_serial_log_path(root, Path::new("serial.log"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_accepts_nonexistent_path() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let result = confine_serial_log_path(root, Path::new("serial.log"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root.join("serial.log"));
    }
}
```

### Step 5: Run compat_runner tests

Run: `cargo test -p runtime_supervisor confine_serial_log_path`
Expected: 3 tests PASS

### Step 6: Build runtime_supervisor

Run: `cargo build -p runtime_supervisor`
Expected: Compiles without errors

### Step 7: Commit P3 symlink fix

```bash
git add runtime_supervisor/src/compat_runner.rs runtime_supervisor/Cargo.toml
git commit -m "$(cat <<'EOF'
fix(security): harden log file creation against symlink attacks

P3 vulnerability: confine_serial_log_path validated parent but not
final path, allowing symlink escapes.

Changes:
- Use symlink_metadata to detect existing paths (doesn't follow symlinks)
- Reject if path already exists (prevents symlink/race attacks)
- Add create_log_file helper with O_NOFOLLOW on Unix
- Add unit tests for symlink rejection

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Run Full Test Suite

### Step 1: Run all tests

Run: `cargo test --workspace`
Expected: All tests PASS

### Step 2: Run foundry gates

Run: `just foundry-all-s0-s1-s2-s3`
Expected: All gates PASS

### Step 3: Final commit (if needed)

```bash
git status
# If any uncommitted changes:
git add -A
git commit -m "$(cat <<'EOF'
chore: cleanup after security remediation S9.2

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

| Task | Vulnerability | Files Changed |
|------|---------------|---------------|
| 1 | P1 Access-control bypass | `access_control.rs` |
| 2 | P1 Temp file attack | `store_cli/src/main.rs`, `Cargo.toml` |
| 3 | P2 Status codes ignored | `status.rs`, `lib.rs`, `client.rs`, `main.rs` |
| 4 | P2 GPU token bypass | `main.rs`, `gpu_runner.rs` |
| 5 | P3 Symlink escape | `compat_runner.rs`, `Cargo.toml` |
| 6 | Validation | Full test suite + foundry gates |
