# Security Remediation S9.2 Design

**Date**: 2026-02-18
**Scope**: Fix 5 security vulnerabilities (2 P1, 2 P2, 1 P3) identified in security audit

## Summary

| ID | Severity | Vulnerability | File |
|----|----------|---------------|------|
| 1 | P1 | Access-control bypass via basename matching | `services/store_service/src/access_control.rs` |
| 2 | P1 | Predictable temp file path | `store_cli/src/main.rs` |
| 3 | P2 | Store client ignores status codes | `services/store_service/src/client.rs` |
| 4 | P2 | GPU token bypass via zero defaults | `runtime_supervisor/src/main.rs`, `gpu_runner.rs` |
| 5 | P3 | Log-path symlink escape | `runtime_supervisor/src/compat_runner.rs` |

---

## Section 1: P1 Access-Control Bypass Fix

**File**: `services/store_service/src/access_control.rs`

### Problem

`is_known_service()` at line 221 only checks basename after canonicalization. A binary renamed to `runtime_supervisor` passes as trusted.

### Solution

Move known-service evaluation into `AccessControl` with exact canonical path matching.

### Implementation

1. **Add fields to `AccessControl`**:
   ```rust
   trusted_paths: HashSet<PathBuf>,      // Exact canonical paths (canonicalized at init)
   dev_allowed_roots: Vec<PathBuf>,      // Canonical prefixes (canonicalized at init)
   dev_mode: bool,                       // Parsed from RAMEN_STORE_DEV_MODE
   ```

2. **Configuration via env vars** (canonicalized once at startup):
   - `RAMEN_STORE_TRUSTED_PATHS` — colon-separated exact canonical paths
   - `RAMEN_STORE_DEV_ALLOWED_ROOTS` — colon-separated canonical prefixes
   - `RAMEN_STORE_DEV_MODE` — boolean flag (parse `1/true/yes/on` vs `0/false/no/off`)

3. **Add `is_known_service_exe(&self, exe: &str) -> bool`** to `AccessControl`:
   - Canonicalize exe path
   - Check exact match against pre-canonicalized `trusted_paths`
   - If `dev_mode` and basename matches trusted name, check `dev_allowed_roots` with `starts_with()`
   - No repeated canonicalization in hot path

4. **Fail-closed in `RequireKnownService`**:
   ```rust
   if self.trusted_paths.is_empty() && !self.dev_mode {
       eprintln!("store_service: MISCONFIGURATION - RequireKnownService with no trusted_paths");
       return AccessDecision::Denied;
   }
   ```

5. **`check_access()` uses only `AccessControl::is_known_service_exe()`** — deprecate `ClientInfo::is_known_service()` (keep for tests/docs only).

---

## Section 2: P1 Predictable Temp File Fix

**File**: `store_cli/src/main.rs`

### Problem

Lines 543-545 use `/tmp/ramen_ingest_<pid>` which is predictable, allowing symlink/TOCTOU attacks.

### Solution

Use `tempfile::NamedTempFile` for exclusive, unguessable temp file creation.

### Implementation

```rust
use tempfile::NamedTempFile;

fn ingest_file(...) -> Result<String, Box<dyn Error>> {
    let source_bytes = ...;

    // Create exclusive temp file with random name
    let mut tmp_file = NamedTempFile::new_in(&std::env::temp_dir())
        .map_err(|e| format!("failed to create temp file: {}", e))?;

    std::io::Write::write_all(&mut tmp_file, &source_bytes)
        .map_err(|e| format!("failed to write temp file: {}", e))?;

    tmp_file.as_file().sync_all()
        .map_err(|e| format!("failed to sync temp file: {}", e))?;

    let tmp_path = tmp_file.path().to_path_buf();

    let reply = store_client.ingest_artifact(kind, channel, &tmp_path)?;

    // RAII cleanup - NamedTempFile deletes on drop
    drop(tmp_file);

    Ok(reply.content_id)
}
```

### Dependency

Add `tempfile` to `store_cli/Cargo.toml` if not present.

---

## Section 3: P2 Store Client Status Code Checking

**File**: `services/store_service/src/client.rs`

### Problem

Methods return replies without checking `reply.status`. Callers use fields directly even on failure.

### Solution

Add shared status validation helper and call in all client methods.

### Implementation

1. **Move status constants to shared module** (`store_service/src/status.rs`):
   ```rust
   pub const STATUS_OK: u32 = 0;
   pub const STATUS_NOT_FOUND: u32 = 1;
   pub const STATUS_IO_ERROR: u32 = 3;
   pub const STATUS_VALIDATION_FAILED: u32 = 4;
   pub const STATUS_PERMISSION_DENIED: u32 = 5;
   ```

2. **Add helper to `StoreClient`**:
   ```rust
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
   ```

3. **Apply to all four methods**: `get_manifest()`, `get_blob()`, `verify_artifact()`, `ingest_artifact()` — call after `validate_request_id()`.

---

## Section 4: P2 GPU Token Bypass Fix

**Files**: `runtime_supervisor/src/main.rs`, `runtime_supervisor/src/gpu_runner.rs`

### Problem

Optional token fields default to `0`. All-zero tokens pass equality check.

### Solution

Require explicit tokens, reject zero values.

### Implementation

1. **In `main.rs`** — explicit validation with `exit(2)` for plan errors:
   ```rust
   "gpu_quarantine_v1" => {
       match plan.gpu_quarantine {
           Some(ref cfg) => {
               let Some(expected_high) = plan.expected_display_cap_token_high else {
                   eprintln!("supervisor: gpu_quarantine_v1 requires expected_display_cap_token_high");
                   std::process::exit(2);
               };
               let Some(expected_low) = plan.expected_display_cap_token_low else {
                   eprintln!("supervisor: gpu_quarantine_v1 requires expected_display_cap_token_low");
                   std::process::exit(2);
               };

               let expected_token = DisplayCapToken::new(expected_high, expected_low);

               if let Err(err) = gpu_runner::gpu_run_v1(cfg, expected_token) {
                   eprintln!("supervisor: gpu_run_v1 failed err={}", err);
                   std::process::exit(3);
               }
           }
           None => {
               eprintln!("supervisor: runner=gpu_quarantine_v1 requires gpu_quarantine");
               std::process::exit(2);
           }
       }
   }
   ```

2. **In `gpu_runner.rs`** — reject zero tokens:
   ```rust
   pub fn gpu_run_v1(config: &GpuQuarantineConfig, expected_token: DisplayCapToken) -> Result<(), String> {
       if config.domain_id == 0 {
           return Err("gpu_run_v1: domain_id must be non-zero".into());
       }

       if expected_token.high == 0 && expected_token.low == 0 {
           return Err("gpu_run_v1: expected_token must be non-zero".into());
       }

       let provided_token = DisplayCapToken::new(config.display_cap_token_high, config.display_cap_token_low);

       if provided_token.high == 0 && provided_token.low == 0 {
           return Err("gpu_run_v1: provided display_cap_token must be non-zero".into());
       }

       // ... rest
   }
   ```

3. **Tests**:
   - Missing expected token fields → exit(2)
   - Zero expected token → rejected
   - Zero provided token → rejected

---

## Section 5: P3 Log-Path Symlink Escape Fix

**File**: `runtime_supervisor/src/compat_runner.rs`

### Problem

`confine_serial_log_path()` validates parent but not final path. Symlink at target escapes root.

### Solution

Atomic file creation with symlink rejection, avoid path reopen when possible.

### Implementation

1. **In `confine_serial_log_path`** — reject if path exists:
   ```rust
   pub fn confine_serial_log_path(allowed_root: &Path, requested: &Path) -> Result<PathBuf, String> {
       // ... existing parent canonicalization ...

       let candidate = allowed_root.join(requested);

       // Use symlink_metadata to not follow symlinks
       match fs::symlink_metadata(&candidate) {
           Ok(_) => return Err(format!(
               "compat log path already exists: {}",
               candidate.display()
           )),
           Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // OK
           Err(e) => return Err(format!(
               "failed to check log path {}: {}",
               candidate.display(), e
           )),
       }

       Ok(candidate)
   }
   ```

2. **Create atomically with `O_NOFOLLOW`** (in runner setup):
   ```rust
   #[cfg(unix)]
   let file = OpenOptions::new()
       .write(true)
       .create_new(true)
       .mode(0o600)
       .custom_flags(libc::O_NOFOLLOW)
       .open(&log_path)?;

   #[cfg(not(unix))]
   let file = OpenOptions::new()
       .write(true)
       .create_new(true)
       .open(&log_path)?;
   ```

3. **Preferred: Avoid path reopen** via `-serial stdio` with FD redirect:
   ```rust
   let log_file = create_log_file(&log_path)?;
   let stdout = process::Stdio::from(log_file);

   Command::new(qemu_path)
       .arg("-serial")
       .arg("stdio")
       .stdout(stdout)
       .spawn()?;
   ```

4. **Document residual risk** if `-serial file:<path>` is kept.

5. **Test**: Pre-create symlink at candidate path, assert rejection.

---

## Implementation Order

1. `services/store_service/src/access_control.rs` (P1)
2. `store_cli/src/main.rs` (P1 temp file)
3. `services/store_service/src/client.rs` + `status.rs` (P2)
4. `runtime_supervisor/src/main.rs` + `gpu_runner.rs` (P2)
5. `runtime_supervisor/src/compat_runner.rs` (P3)

## Testing

- Run existing foundry gates after each fix
- Add unit tests for new validation logic
- Security-specific tests for bypass attempts
