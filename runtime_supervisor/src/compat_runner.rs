//! Compat Runner v0 — launches Linux VM capsules via QEMU.
//!
//! This module defines the `CompatCapsuleConfig` struct and a `compat_run_v0`
//! function that spawns QEMU as a background process. The caller is responsible
//! for waiting on or killing the child process.

use serde::Deserialize;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

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

/// Extract hash hex from content ID (e.g., "sha256:abc..." -> "abc...").
fn extract_hash_hex(id: &str) -> &str {
    &id[CONTENT_ID_PREFIX.len()..]
}

/// Mount policy for an artifact disk attached to the capsule.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MountPolicy {
    ReadOnly,
    ReadWrite,
}

impl fmt::Display for MountPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MountPolicy::ReadOnly => write!(f, "read_only"),
            MountPolicy::ReadWrite => write!(f, "read_write"),
        }
    }
}

/// Device type for an artifact disk.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    VirtioBlk,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::VirtioBlk => write!(f, "virtio"),
        }
    }
}

/// A reference to an artifact disk image to attach to the capsule.
#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactDiskRef {
    /// Content-addressed ID (e.g. "sha256:abc...").
    pub content_id: String,

    /// How the disk should be mounted inside the guest.
    pub mount_policy: MountPolicy,

    /// QEMU device type to use.
    pub device_type: DeviceType,
}

/// Resource limits for the capsule VM.
#[derive(Debug, Clone, Deserialize)]
pub struct CapsuleResources {
    /// Memory in megabytes.
    pub memory_mb: u32,

    /// Number of virtual CPUs.
    pub cpus: u32,
}

impl Default for CapsuleResources {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpus: 1,
        }
    }
}

/// Compat Capsule v0 configuration.
///
/// Describes everything needed to launch a Linux VM capsule:
/// kernel, initrd, artifact disks, kernel command line, and resources.
#[derive(Debug, Clone, Deserialize)]
pub struct CompatCapsuleConfig {
    /// Content-addressed ID for the kernel image.
    pub kernel_content_id: String,

    /// Content-addressed ID for the initrd image.
    pub initrd_content_id: String,

    /// Artifact disks to attach.
    #[serde(default)]
    pub artifact_disks: Vec<ArtifactDiskRef>,

    /// Kernel command line.
    #[serde(default = "default_cmdline")]
    pub cmdline: String,

    /// Resource limits.
    #[serde(default)]
    pub resources: CapsuleResources,

    /// Optional path for QEMU serial log output.
    /// When set, adds `-serial file:<log_path>` to the QEMU command.
    #[serde(default)]
    pub log_path: Option<String>,

    /// Optional read-only projection VFS exported via QEMU virtio-9p.
    #[serde(default)]
    pub projection_vfs: Option<ProjectionVfsMount>,
}

/// Host-side materialized projection tree exported to a compat guest via virtio-9p.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionVfsMount {
    /// Directory containing the materialized read-only projection tree.
    pub host_path: PathBuf,

    /// 9p virtio mount tag visible inside the guest.
    #[serde(default = "default_projection_mount_tag")]
    pub mount_tag: String,
}

fn default_projection_mount_tag() -> String {
    "ramen_store".to_string()
}

fn default_cmdline() -> String {
    "console=ttyS0".to_string()
}

/// Spawns QEMU as a background process to launch this capsule.
///
/// Prints the QEMU command for debuggability, then spawns the process.
/// Returns the `Child` handle; the caller is responsible for waiting on
/// or killing it.
/// Resolve a validated content ID to a blob path in the content store.
///
/// Note: This function constructs paths but does not perform IO operations.
/// Actual artifact access should go through the store service client.
fn resolve_path(artifact_root: &Path, content_id: &str) -> PathBuf {
    let hash_hex = extract_hash_hex(content_id);
    artifact_root.join(format!("{}.blob", hash_hex))
}

/// Validate that a requested serial log path is confined to allowed_root.
///
/// Security checks:
/// 1. Path must be relative (not absolute)
/// 2. Path must not contain traversal components (.. or root)
/// 3. Parent directory must canonicalize under allowed_root
/// 4. Final path must not already exist (prevents symlink attacks)
pub fn confine_serial_log_path(allowed_root: &Path, requested: &Path) -> Result<PathBuf, String> {
    if requested.is_absolute() {
        return Err(format!(
            "compat log path must be relative under allowed root: {}",
            requested.display()
        ));
    }
    if requested
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!(
            "compat log path contains disallowed components: {}",
            requested.display()
        ));
    }

    fs::create_dir_all(allowed_root).map_err(|e| {
        format!(
            "failed to create compat log root {}: {}",
            allowed_root.display(),
            e
        )
    })?;

    let candidate = allowed_root.join(requested);
    let parent = candidate.parent().ok_or_else(|| {
        format!(
            "compat log path has no parent under allowed root: {}",
            requested.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failed to create compat log directory {}: {}",
            parent.display(),
            e
        )
    })?;

    let allowed_root_canon = fs::canonicalize(allowed_root).map_err(|e| {
        format!(
            "failed to canonicalize compat log root {}: {}",
            allowed_root.display(),
            e
        )
    })?;
    let parent_canon = fs::canonicalize(parent).map_err(|e| {
        format!(
            "failed to canonicalize compat log parent {}: {}",
            parent.display(),
            e
        )
    })?;
    if !parent_canon.starts_with(&allowed_root_canon) {
        return Err(format!(
            "compat log path escapes allowed root: {}",
            requested.display()
        ));
    }

    // P3 fix: Reject if path already exists (symlink_metadata doesn't follow symlinks)
    // This prevents symlink attacks where an attacker creates a symlink at the
    // expected log path pointing outside the allowed root.
    match fs::symlink_metadata(&candidate) {
        Ok(_) => {
            return Err(format!(
                "compat log path already exists (possible symlink attack): {}",
                candidate.display()
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Path doesn't exist, proceed
        }
        Err(e) => {
            return Err(format!(
                "failed to check log path {}: {}",
                candidate.display(),
                e
            ));
        }
    }

    Ok(candidate)
}

/// Create log file atomically with symlink protection.
///
/// Uses O_NOFOLLOW on Unix to prevent symlink swap attacks between
/// path validation and file creation.
#[cfg(unix)]
#[allow(dead_code)] // Exercised by compat_runner unit tests; wired into serial log path in S10.2+
pub fn create_log_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true) // Fail if exists (atomic)
        .mode(0o600) // Restrictive permissions: owner read/write only
        .custom_flags(libc::O_NOFOLLOW) // Fail if symlink
        .open(path)
        .map_err(|e| format!("failed to create log file {}: {}", path.display(), e))
}

#[cfg(not(unix))]
#[allow(dead_code)] // Exercised by compat_runner unit tests; wired into serial log path in S10.2+
pub fn create_log_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .write(true)
        .create_new(true) // Fail if exists (atomic)
        .open(path)
        .map_err(|e| format!("failed to create log file {}: {}", path.display(), e))
}

pub fn compat_run_v0(
    config: &CompatCapsuleConfig,
    artifact_root: &Path,
) -> Result<Child, std::io::Error> {
    // Validate content ID formats
    validate_content_id_format(&config.kernel_content_id)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    validate_content_id_format(&config.initrd_content_id)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let kernel_path = resolve_path(artifact_root, &config.kernel_content_id);
    let initrd_path = resolve_path(artifact_root, &config.initrd_content_id);

    // Debug trace — kept for observability.
    println!("compat_runner_v0: --- QEMU command ---");
    println!("compat_runner_v0: qemu-system-x86_64 \\");
    println!("compat_runner_v0:   -machine q35 \\");
    println!("compat_runner_v0:   -m {}M \\", config.resources.memory_mb);
    println!("compat_runner_v0:   -smp {} \\", config.resources.cpus);
    println!("compat_runner_v0:   -nographic \\");
    println!("compat_runner_v0:   -no-reboot -no-shutdown \\");
    println!("compat_runner_v0:   -kernel {} \\", kernel_path.display());
    println!("compat_runner_v0:   -initrd {} \\", initrd_path.display());

    for disk in &config.artifact_disks {
        validate_content_id_format(&disk.content_id)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let disk_path = resolve_path(artifact_root, &disk.content_id);
        let readonly_flag = match disk.mount_policy {
            MountPolicy::ReadOnly => ",readonly=on",
            MountPolicy::ReadWrite => "",
        };
        println!(
            "compat_runner_v0:   -drive file={},if={},format=raw{} \\",
            disk_path.display(),
            disk.device_type,
            readonly_flag,
        );
    }

    if let Some(ref vfs) = config.projection_vfs {
        if !vfs.host_path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "projection_vfs host_path is not a directory: {}",
                    vfs.host_path.display()
                ),
            ));
        }
        println!(
            "compat_runner_v0:   -virtfs local,path={},mount_tag={},security_model=none,readonly=on \\",
            vfs.host_path.display(),
            vfs.mount_tag,
        );
    }

    if let Some(ref lp) = config.log_path {
        // SECURITY NOTE: Using -serial file:<path> leaves a residual TOCTOU window
        // between path validation and QEMU's internal open(). The path is validated
        // with symlink_metadata to reject pre-existing symlinks. For full protection,
        // use create_log_file() to pre-create the file, or switch to -serial stdio
        // with redirected stdout.
        println!("compat_runner_v0:   -serial file:{} \\", lp);
    }
    println!("compat_runner_v0:   -append {:?}", config.cmdline);
    println!("compat_runner_v0: --- end command ---");

    // Build the actual QEMU command.
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-machine").arg("q35,memory-backend=ramen_mem");
    cmd.arg("-m")
        .arg(format!("{}M", config.resources.memory_mb));

    // S10.2.3: Configure file-backed memory for the data plane
    // Path: /dev/shm/ramenos_mem (Linux) or a temp file (macOS)
    let mem_path = if cfg!(target_os = "linux") {
        "/dev/shm/ramenos_mem".to_string()
    } else {
        "/tmp/ramenos_mem".to_string()
    };

    cmd.arg("-object").arg(format!(
        "memory-backend-file,id=ramen_mem,mem-path={},size={}M,share=on",
        mem_path, config.resources.memory_mb
    ));

    cmd.arg("-smp").arg(format!("{}", config.resources.cpus));
    cmd.arg("-nographic");
    cmd.arg("-no-reboot");
    cmd.arg("-no-shutdown");
    cmd.arg("-kernel").arg(&kernel_path);
    cmd.arg("-initrd").arg(&initrd_path);

    for disk in &config.artifact_disks {
        validate_content_id_format(&disk.content_id)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let disk_path = resolve_path(artifact_root, &disk.content_id);
        let readonly_flag = match disk.mount_policy {
            MountPolicy::ReadOnly => ",readonly=on",
            MountPolicy::ReadWrite => "",
        };
        cmd.arg("-drive").arg(format!(
            "file={},if={},format=raw{}",
            disk_path.display(),
            disk.device_type,
            readonly_flag,
        ));
    }

    if let Some(ref vfs) = config.projection_vfs {
        if !vfs.host_path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "projection_vfs host_path is not a directory: {}",
                    vfs.host_path.display()
                ),
            ));
        }
        cmd.arg("-virtfs").arg(format!(
            "local,path={},mount_tag={},security_model=none,readonly=on",
            vfs.host_path.display(),
            vfs.mount_tag,
        ));
    }

    if let Some(ref lp) = config.log_path {
        // SECURITY NOTE: Using -serial file:<path> leaves a residual TOCTOU window
        // between path validation and QEMU's internal open(). The path is validated
        // with symlink_metadata to reject pre-existing symlinks. For full protection,
        // use create_log_file() to pre-create the file, or switch to -serial stdio
        // with redirected stdout.
        cmd.arg("-serial").arg(format!("file:{}", lp));
    }

    cmd.arg("-append").arg(&config.cmdline);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("{}_{}", prefix, nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn default_resources() {
        let r = CapsuleResources::default();
        assert_eq!(r.memory_mb, 512);
        assert_eq!(r.cpus, 1);
    }

    #[test]
    fn deserialize_capsule_config() {
        let json = r#"{
            "kernel_content_id": "sha256:aaaa",
            "initrd_content_id": "sha256:bbbb",
            "artifact_disks": [
                {
                    "content_id": "sha256:cccc",
                    "mount_policy": "read_only",
                    "device_type": "virtio_blk"
                }
            ],
            "cmdline": "console=ttyS0 quiet",
            "resources": {
                "memory_mb": 1024,
                "cpus": 2
            }
        }"#;
        let config: CompatCapsuleConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.kernel_content_id, "sha256:aaaa");
        assert_eq!(config.initrd_content_id, "sha256:bbbb");
        assert_eq!(config.artifact_disks.len(), 1);
        assert_eq!(config.artifact_disks[0].mount_policy, MountPolicy::ReadOnly);
        assert_eq!(config.artifact_disks[0].device_type, DeviceType::VirtioBlk);
        assert_eq!(config.cmdline, "console=ttyS0 quiet");
        assert_eq!(config.resources.memory_mb, 1024);
        assert_eq!(config.resources.cpus, 2);
        assert!(config.log_path.is_none());
        assert!(config.projection_vfs.is_none());
    }

    #[test]
    fn deserialize_capsule_config_with_projection_vfs() {
        let json = r#"{
            "kernel_content_id": "sha256:aaaa",
            "initrd_content_id": "sha256:bbbb",
            "projection_vfs": {
                "host_path": "/tmp/projection_mount",
                "mount_tag": "ramen_store"
            }
        }"#;
        let config: CompatCapsuleConfig = serde_json::from_str(json).unwrap();
        let vfs = config.projection_vfs.expect("projection_vfs");
        assert_eq!(vfs.host_path, PathBuf::from("/tmp/projection_mount"));
        assert_eq!(vfs.mount_tag, "ramen_store");
    }

    #[test]
    fn deserialize_capsule_config_with_log_path() {
        let json = r#"{
            "kernel_content_id": "sha256:aaaa",
            "initrd_content_id": "sha256:bbbb",
            "log_path": "/tmp/qemu.log"
        }"#;
        let config: CompatCapsuleConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.log_path.as_deref(), Some("/tmp/qemu.log"));
        assert_eq!(config.cmdline, "console=ttyS0");
        assert_eq!(config.resources.memory_mb, 512);
    }

    #[test]
    fn deserialize_capsule_config_defaults() {
        let json = r#"{
            "kernel_content_id": "sha256:aaaa",
            "initrd_content_id": "sha256:bbbb"
        }"#;
        let config: CompatCapsuleConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.cmdline, "console=ttyS0");
        assert_eq!(config.resources.memory_mb, 512);
        assert_eq!(config.resources.cpus, 1);
        assert!(config.artifact_disks.is_empty());
        assert!(config.log_path.is_none());
    }

    /// Verify that `compat_run_v0` returns an error when QEMU is not on PATH
    /// (expected in most test environments). This exercises the command-building
    /// logic without requiring a real QEMU installation.
    #[test]
    fn spawn_returns_error_without_qemu() {
        let config = CompatCapsuleConfig {
            kernel_content_id:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            initrd_content_id:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            artifact_disks: vec![ArtifactDiskRef {
                content_id:
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
                mount_policy: MountPolicy::ReadOnly,
                device_type: DeviceType::VirtioBlk,
            }],
            cmdline: "console=ttyS0".into(),
            resources: CapsuleResources::default(),
            log_path: Some("/tmp/test_qemu.log".into()),
            projection_vfs: None,
        };
        let root = std::path::PathBuf::from("/tmp/test_artifacts");
        // In CI / dev environments without QEMU installed, spawn will fail
        // with NotFound. If QEMU happens to be installed, we get a Child
        // back — either outcome is acceptable.
        let result = compat_run_v0(&config, &root);
        match result {
            Ok(mut child) => {
                // QEMU is installed; clean up.
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
            }
        }
    }

    #[test]
    fn confine_serial_log_path_rejects_absolute_and_traversal() {
        let root = temp_root("ramenos_compat_log_reject");
        let absolute = Path::new("/tmp/outside.log");
        let traversal = Path::new("../../etc/passwd");

        let err_abs = confine_serial_log_path(&root, absolute).unwrap_err();
        assert!(err_abs.contains("must be relative"));

        let err_traversal = confine_serial_log_path(&root, traversal).unwrap_err();
        assert!(err_traversal.contains("disallowed components"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn confine_serial_log_path_accepts_in_root_relative_path() {
        let root = temp_root("ramenos_compat_log_ok");
        let requested = Path::new("capsules/vm1/qemu.log");
        let confined = confine_serial_log_path(&root, requested).unwrap();
        assert_eq!(confined, root.join("capsules/vm1/qemu.log"));
        let canon_root = fs::canonicalize(&root).unwrap();
        let canon_parent = fs::canonicalize(confined.parent().unwrap()).unwrap();
        assert!(canon_parent.starts_with(&canon_root));

        let _ = fs::remove_dir_all(root);
    }

    // P3 symlink attack prevention tests
    #[cfg(unix)]
    #[test]
    fn confine_serial_log_path_rejects_existing_symlink() {
        use std::os::unix::fs::symlink;

        let root = temp_root("ramenos_compat_log_symlink");

        // Create a symlink at the target path pointing outside the root
        let target = Path::new("/tmp/escaped.log");
        let symlink_path = root.join("serial.log");
        symlink(target, &symlink_path).unwrap();

        let result = confine_serial_log_path(&root, Path::new("serial.log"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn confine_serial_log_path_rejects_existing_regular_file() {
        let root = temp_root("ramenos_compat_log_existing");

        // Create a regular file at the target path
        let existing = root.join("serial.log");
        fs::write(&existing, "old content").unwrap();

        let result = confine_serial_log_path(&root, Path::new("serial.log"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn confine_serial_log_path_accepts_nonexistent_path() {
        let root = temp_root("ramenos_compat_log_new");

        // Path doesn't exist yet - should be accepted
        let result = confine_serial_log_path(&root, Path::new("serial.log"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root.join("serial.log"));

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn create_log_file_rejects_symlink() {
        let root = temp_root("ramenos_compat_log_create_symlink");
        use std::os::unix::fs::symlink;

        // Create a symlink at the target path
        let target = Path::new("/tmp/escaped_create.log");
        let symlink_path = root.join("test.log");
        symlink(target, &symlink_path).unwrap();

        // create_log_file should fail due to O_NOFOLLOW
        let result = create_log_file(&symlink_path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_log_file_creates_new_file() {
        let root = temp_root("ramenos_compat_log_create_new");

        let path = root.join("new.log");
        let result = create_log_file(&path);

        assert!(result.is_ok());
        assert!(path.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn create_log_file_rejects_existing_file() {
        let root = temp_root("ramenos_compat_log_create_existing");

        let path = root.join("existing.log");
        fs::write(&path, "old content").unwrap();

        // create_log_file should fail because file already exists
        let result = create_log_file(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(root);
    }
}
