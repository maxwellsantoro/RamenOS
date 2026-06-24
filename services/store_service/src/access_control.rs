// V-007 Phase 4: Capability-Based Access Control with SO_PEERCRED
//
// This module provides the foundation for capability-based access control (CBAC)
// for store service operations with real Unix credential retrieval via SO_PEERCRED.

use std::collections::HashSet;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

/// Store service access rights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessRights {
    pub read: bool,
    pub write: bool,
    pub admin: bool,
}

impl AccessRights {
    /// No access rights
    pub fn none() -> Self {
        Self {
            read: false,
            write: false,
            admin: false,
        }
    }

    /// Read-only access
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            admin: false,
        }
    }

    /// Read and write access
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            admin: false,
        }
    }

    /// Full admin access
    pub fn admin() -> Self {
        Self {
            read: true,
            write: true,
            admin: true,
        }
    }

    /// Check if rights include read
    pub fn can_read(&self) -> bool {
        self.read
    }

    /// Check if rights include write
    pub fn can_write(&self) -> bool {
        self.write
    }

    /// Check if rights include admin
    pub fn can_admin(&self) -> bool {
        self.admin
    }
}

/// Access control decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    /// Access granted
    Allowed,

    /// Access denied - insufficient rights
    Denied,

    /// Access denied - invalid capability
    InvalidCapability,

    /// Access denied - capability expired
    Expired,

    /// Access denied - unknown client
    UnknownClient,
}

/// Client identity information
///
/// V-007 Phase 4: Real Unix credentials retrieved via SO_PEERCRED
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Process ID of the client
    pub pid: Option<u32>,

    /// User ID (from Unix credentials)
    pub uid: Option<u32>,

    /// Group ID (from Unix credentials)
    pub gid: Option<u32>,

    /// Domain ID (from capability presentation, future)
    pub domain_id: Option<u64>,

    /// Access rights (from capability validation, future)
    pub rights: AccessRights,

    /// Process executable path (from /proc/{pid}/exe)
    pub exe_path: Option<String>,

    /// Process command line (from /proc/{pid}/cmdline)
    pub cmdline: Option<String>,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            pid: None,
            uid: None,
            gid: None,
            domain_id: None,
            rights: AccessRights::none(), // V-007 Phase 6: No default rights; access granted via validated capability
            exe_path: None,
            cmdline: None,
        }
    }
}

impl ClientInfo {
    /// Create client info from Unix stream with real SO_PEERCRED retrieval
    ///
    /// V-007 Phase 4: Uses `getsockopt(SO_PEERCRED)` to get actual credentials (Linux-only)
    #[cfg(target_os = "linux")]
    pub fn from_stream(stream: &UnixStream) -> Self {
        use std::os::unix::io::AsRawFd;

        // Get the raw file descriptor
        let fd = stream.as_raw_fd();

        // Get Unix credentials using SO_PEERCRED
        let ucred = unsafe {
            let mut ucred = std::mem::zeroed::<libc::ucred>();
            let mut ucred_len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

            if libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut ucred as *mut _ as *mut libc::c_void,
                &mut ucred_len,
            ) == 0
            {
                Some(ucred)
            } else {
                None
            }
        };

        match ucred {
            Some(creds) => {
                let pid = creds.pid as u32;
                let uid = creds.uid;
                let gid = creds.gid;

                // Try to read process info from /proc
                let (exe_path, cmdline) = Self::read_process_info(pid);

                Self {
                    pid: Some(pid),
                    uid: Some(uid),
                    gid: Some(gid),
                    domain_id: None, // Phase 6: extracted from validated capability in request handlers
                    rights: AccessRights::none(), // V-007 Phase 6: No default rights; access granted via validated capability
                    exe_path,
                    cmdline,
                }
            }
            None => {
                // Fallback to default if SO_PEERCRED fails
                Self::default()
            }
        }
    }

    /// Create client info from Unix stream (non-Linux fallback)
    ///
    /// V-007 Phase 4: Stub implementation for macOS and other platforms
    #[cfg(not(target_os = "linux"))]
    pub fn from_stream(_stream: &UnixStream) -> Self {
        // SO_PEERCRED is Linux-only, so we return default credentials on other platforms
        // In production, this would use platform-specific credential passing mechanisms
        Self::default()
    }

    /// Read process information from /proc filesystem (Linux-only)
    #[cfg(target_os = "linux")]
    fn read_process_info(pid: u32) -> (Option<String>, Option<String>) {
        let pid_str = pid.to_string();

        // Read exe path from /proc/{pid}/exe
        let exe_path = std::fs::read_link(format!("/proc/{}/exe", pid_str))
            .ok()
            .map(|p| p.to_string_lossy().to_string());

        // Read command line from /proc/{pid}/cmdline
        let cmdline = std::fs::read_to_string(format!("/proc/{}/cmdline", pid_str))
            .ok()
            .map(|s| {
                // Replace null bytes with spaces for readability
                s.replace('\0', " ")
            });

        (exe_path, cmdline)
    }

    /// Check if client is a known service
    ///
    /// S7 Security Hardening: Uses exact path matching instead of substring matching.
    /// This prevents bypass via paths like "/tmp/domain_manager_malicious".
    ///
    /// **DEPRECATED**: This method is kept for backward compatibility and tests only.
    /// Use `AccessControl::is_known_service_exe()` for actual access decisions.
    #[deprecated(
        since = "0.7.2",
        note = "Use AccessControl::is_known_service_exe() instead"
    )]
    pub fn is_known_service(&self) -> bool {
        if let Some(ref exe) = self.exe_path {
            // Use exact path matching with canonicalization to prevent bypass
            // Only match if the basename exactly matches a known service name
            if let Ok(canonical) = std::fs::canonicalize(exe) {
                let basename = canonical.file_name().and_then(|n| n.to_str()).unwrap_or("");

                matches!(
                    basename,
                    "domain_manager" | "runtime_supervisor" | "store_cli" | "store_service"
                )
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Check if client can read
    pub fn can_read(&self) -> bool {
        self.rights.can_read()
    }

    /// Check if client can write
    pub fn can_write(&self) -> bool {
        self.rights.can_write()
    }

    /// Check if client can admin
    pub fn can_admin(&self) -> bool {
        self.rights.can_admin()
    }
}

/// Access control checker
///
/// V-007 Phase 4: Enhanced with credential-based policy enforcement
pub struct AccessControl {
    /// Policy mode
    policy: AccessPolicy,

    /// Whitelist of allowed PIDs (for development mode)
    pid_whitelist: HashSet<u32>,

    /// Whitelist of allowed executable paths
    exe_whitelist: Vec<String>,

    /// Exact canonical paths for trusted services (canonicalized at init)
    trusted_paths: HashSet<PathBuf>,

    /// Canonical prefix paths for dev mode (canonicalized at init)
    dev_allowed_roots: Vec<PathBuf>,

    /// Whether dev mode is enabled (from RAMEN_STORE_DEV_MODE)
    dev_mode: bool,
}

/// Access control policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPolicy {
    /// Allow all access (development mode)
    AllowAll,

    /// Require valid credentials (production mode)
    RequireCredentials,

    /// Require known service executable paths
    RequireKnownService,

    /// Specific whitelist (future)
    Whitelist,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::with_policy(AccessPolicy::RequireCredentials)
    }
}

impl AccessControl {
    /// Create new access control checker
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific policy
    pub fn with_policy(policy: AccessPolicy) -> Self {
        let dev_mode = crate::dev_mode::is_dev_mode_enabled();

        // Parse and canonicalize trusted paths at init
        let trusted_paths: HashSet<PathBuf> = std::env::var("RAMEN_STORE_TRUSTED_PATHS")
            .ok()
            .and_then(|s| {
                let paths: Vec<PathBuf> = s
                    .split(':')
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
                let paths: Vec<PathBuf> = s
                    .split(':')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| std::fs::canonicalize(s).ok())
                    .collect();
                if paths.is_empty() { None } else { Some(paths) }
            })
            .unwrap_or_default();

        let mut ac = Self {
            policy,
            pid_whitelist: HashSet::new(),
            exe_whitelist: Vec::new(),
            trusted_paths,
            dev_allowed_roots,
            dev_mode,
        };

        // Add default service paths to whitelist
        ac.add_default_service_whitelist();

        ac
    }

    /// Add default service paths to whitelist
    fn add_default_service_whitelist(&mut self) {
        // Common development paths
        self.exe_whitelist.push("domain_manager".to_string());
        self.exe_whitelist.push("runtime_supervisor".to_string());
        self.exe_whitelist.push("store_cli".to_string());
        self.exe_whitelist.push("store_service".to_string());
    }

    /// Add a PID to the whitelist
    pub fn whitelist_pid(&mut self, pid: u32) {
        self.pid_whitelist.insert(pid);
    }

    /// Add an executable path to the whitelist
    pub fn whitelist_exe(&mut self, exe_path: String) {
        self.exe_whitelist.push(exe_path);
    }

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
            let basename = canonical.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if matches!(
                basename,
                "domain_manager" | "runtime_supervisor" | "store_cli" | "store_service"
            ) {
                // Must be under an allowed dev root (canonical prefix match)
                if self
                    .dev_allowed_roots
                    .iter()
                    .any(|root| canonical.starts_with(root))
                {
                    return true;
                }
            }
        }

        false
    }

    /// S7 Security Hardening: Check if an executable is whitelisted using exact matching
    ///
    /// This method supports two whitelist modes:
    /// 1. Exact path matching: If whitelist starts with "/", it's treated as an exact path
    /// 2. Basename matching: If whitelist doesn't start with "/", it's treated as a basename
    ///
    /// All paths are canonicalized to resolve symlinks and normalize paths.
    /// This prevents bypass via paths like "/tmp/domain_manager_malicious".
    fn is_exe_whitelisted(&self, exe: &str, whitelist: &str) -> bool {
        // Canonicalize the executable path to resolve symlinks
        let canonical_exe = match std::fs::canonicalize(exe) {
            Ok(path) => path,
            Err(_) => return false, // Invalid path, deny access
        };

        // Check if whitelist is an absolute path (exact matching)
        if whitelist.starts_with('/') {
            // Exact path matching: canonicalize both paths and compare
            let canonical_whitelist = match std::fs::canonicalize(whitelist) {
                Ok(path) => path,
                Err(_) => return false, // Invalid whitelist entry, deny access
            };

            canonical_exe == canonical_whitelist
        } else {
            // Basename matching: compare only the filename
            let basename = canonical_exe
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            basename == whitelist
        }
    }

    /// Check if client can perform read operation
    ///
    /// V-007 Phase 4: Enhanced credential-based validation
    pub fn can_read(&self, client: &ClientInfo, _artifact_id: &str) -> AccessDecision {
        self.check_access(client, AccessRights::read_only())
    }

    /// Check if client can perform write operation
    ///
    /// V-007 Phase 4: Enhanced credential-based validation
    pub fn can_write(&self, client: &ClientInfo) -> AccessDecision {
        self.check_access(client, AccessRights::read_write())
    }

    /// Check if client can perform admin operation
    ///
    /// V-007 Phase 4: Enhanced credential-based validation
    pub fn can_admin(&self, client: &ClientInfo) -> AccessDecision {
        self.check_access(client, AccessRights::admin())
    }

    /// Internal access checking logic with security logging
    fn check_access(&self, client: &ClientInfo, required_rights: AccessRights) -> AccessDecision {
        let decision = match self.policy {
            AccessPolicy::AllowAll => AccessDecision::Allowed,

            AccessPolicy::RequireCredentials => {
                // Must have valid credentials (PID, UID, GID)
                if client.pid.is_none() || client.uid.is_none() || client.gid.is_none() {
                    eprintln!(
                        "store_service: ACCESS DENIED - Unknown client (missing credentials)"
                    );
                    eprintln!("store_service:   - PID: {:?}", client.pid);
                    eprintln!("store_service:   - UID: {:?}", client.uid);
                    eprintln!("store_service:   - GID: {:?}", client.gid);
                    eprintln!("store_service:   - Exe: {:?}", client.exe_path);
                    eprintln!("store_service:   - Required rights: {:?}", required_rights);
                    return AccessDecision::UnknownClient;
                }

                // Check if PID is whitelisted (if whitelist is non-empty)
                if !self.pid_whitelist.is_empty() {
                    if let Some(pid) = client.pid {
                        if !self.pid_whitelist.contains(&pid) {
                            eprintln!("store_service: ACCESS DENIED - PID not whitelisted");
                            eprintln!("store_service:   - PID: {}", pid);
                            eprintln!("store_service:   - UID: {:?}", client.uid);
                            eprintln!("store_service:   - GID: {:?}", client.gid);
                            eprintln!("store_service:   - Exe: {:?}", client.exe_path);
                            eprintln!("store_service:   - Required rights: {:?}", required_rights);
                            return AccessDecision::Denied;
                        }
                    }
                }

                AccessDecision::Allowed
            }

            AccessPolicy::RequireKnownService => {
                // Must have valid credentials
                if client.pid.is_none() || client.uid.is_none() || client.gid.is_none() {
                    eprintln!(
                        "store_service: ACCESS DENIED - Unknown client (missing credentials)"
                    );
                    eprintln!("store_service:   - PID: {:?}", client.pid);
                    eprintln!("store_service:   - UID: {:?}", client.uid);
                    eprintln!("store_service:   - GID: {:?}", client.gid);
                    eprintln!("store_service:   - Exe: {:?}", client.exe_path);
                    eprintln!("store_service:   - Required rights: {:?}", required_rights);
                    return AccessDecision::UnknownClient;
                }

                // Fail closed if misconfigured
                if self.trusted_paths.is_empty() && !self.dev_mode {
                    eprintln!(
                        "store_service: MISCONFIGURATION - RequireKnownService policy with no trusted_paths and dev mode off"
                    );
                    return AccessDecision::Denied;
                }

                // Check if executable is a known service using exact canonical path matching
                if let Some(ref exe) = client.exe_path {
                    if self.is_known_service_exe(exe) {
                        AccessDecision::Allowed
                    } else {
                        // Also check exe whitelist using exact matching (for custom services)
                        let is_whitelisted = self
                            .exe_whitelist
                            .iter()
                            .any(|whitelist| self.is_exe_whitelisted(exe, whitelist));

                        if is_whitelisted {
                            AccessDecision::Allowed
                        } else {
                            eprintln!("store_service: ACCESS DENIED - Executable not whitelisted");
                            eprintln!("store_service:   - PID: {:?}", client.pid);
                            eprintln!("store_service:   - UID: {:?}", client.uid);
                            eprintln!("store_service:   - GID: {:?}", client.gid);
                            eprintln!("store_service:   - Exe: {}", exe);
                            eprintln!("store_service:   - Required rights: {:?}", required_rights);
                            AccessDecision::Denied
                        }
                    }
                } else {
                    eprintln!("store_service: ACCESS DENIED - Unknown executable path");
                    eprintln!("store_service:   - PID: {:?}", client.pid);
                    eprintln!("store_service:   - UID: {:?}", client.uid);
                    eprintln!("store_service:   - GID: {:?}", client.gid);
                    eprintln!("store_service:   - Required rights: {:?}", required_rights);
                    AccessDecision::Denied
                }
            }

            AccessPolicy::Whitelist => {
                // Check if PID is whitelisted
                if let Some(pid) = client.pid {
                    if self.pid_whitelist.contains(&pid) {
                        return AccessDecision::Allowed;
                    }
                }

                // Check if exe is whitelisted using exact matching
                if let Some(ref exe) = client.exe_path {
                    if self
                        .exe_whitelist
                        .iter()
                        .any(|w| self.is_exe_whitelisted(exe, w))
                    {
                        return AccessDecision::Allowed;
                    }
                }

                eprintln!("store_service: ACCESS DENIED - Not in whitelist");
                eprintln!("store_service:   - PID: {:?}", client.pid);
                eprintln!("store_service:   - UID: {:?}", client.uid);
                eprintln!("store_service:   - GID: {:?}", client.gid);
                eprintln!("store_service:   - Exe: {:?}", client.exe_path);
                eprintln!("store_service:   - Required rights: {:?}", required_rights);
                AccessDecision::Denied
            }
        };

        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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

    #[test]
    fn access_rights_default_is_none() {
        let rights = AccessRights::none();
        assert!(!rights.can_read());
        assert!(!rights.can_write());
        assert!(!rights.can_admin());
    }

    #[test]
    fn access_rights_read_only() {
        let rights = AccessRights::read_only();
        assert!(rights.can_read());
        assert!(!rights.can_write());
        assert!(!rights.can_admin());
    }

    #[test]
    fn access_rights_read_write() {
        let rights = AccessRights::read_write();
        assert!(rights.can_read());
        assert!(rights.can_write());
        assert!(!rights.can_admin());
    }

    #[test]
    fn access_rights_admin() {
        let rights = AccessRights::admin();
        assert!(rights.can_read());
        assert!(rights.can_write());
        assert!(rights.can_admin());
    }

    #[test]
    fn client_info_default_has_no_access() {
        let client = ClientInfo::default();
        assert!(!client.can_read());
        assert!(!client.can_write());
        assert!(!client.can_admin());
    }

    #[test]
    fn access_control_allow_all_policy() {
        let ac = AccessControl::with_policy(AccessPolicy::AllowAll);
        let client = ClientInfo::default();

        assert_eq!(ac.can_read(&client, "test"), AccessDecision::Allowed);
        assert_eq!(ac.can_write(&client), AccessDecision::Allowed);
        assert_eq!(ac.can_admin(&client), AccessDecision::Allowed);
    }

    #[test]
    fn access_control_default_policy_is_require_credentials() {
        let ac = AccessControl::new();
        let client = ClientInfo::default();

        // S7 Security Hardening: Default is now RequireCredentials (fail-closed)
        assert_eq!(ac.can_read(&client, "test"), AccessDecision::UnknownClient);
    }

    #[test]
    fn access_control_require_credentials_rejects_no_pid() {
        let ac = AccessControl::with_policy(AccessPolicy::RequireCredentials);
        let client = ClientInfo {
            pid: None,
            uid: Some(1000),
            gid: Some(1000),
            ..Default::default()
        };

        assert_eq!(ac.can_read(&client, "test"), AccessDecision::UnknownClient);
    }

    #[test]
    fn access_control_require_credentials_accepts_valid() {
        let ac = AccessControl::with_policy(AccessPolicy::RequireCredentials);
        let client = ClientInfo {
            pid: Some(1234),
            uid: Some(1000),
            gid: Some(1000),
            ..Default::default()
        };

        assert_eq!(ac.can_read(&client, "test"), AccessDecision::Allowed);
    }

    #[test]
    fn access_control_pid_whitelist() {
        let mut ac = AccessControl::with_policy(AccessPolicy::RequireCredentials);
        ac.whitelist_pid(100);

        let client_whitelisted = ClientInfo {
            pid: Some(100),
            uid: Some(1000),
            gid: Some(1000),
            ..Default::default()
        };

        let client_not_whitelisted = ClientInfo {
            pid: Some(200),
            uid: Some(1000),
            gid: Some(1000),
            ..Default::default()
        };

        assert_eq!(
            ac.can_read(&client_whitelisted, "test"),
            AccessDecision::Allowed
        );
        assert_eq!(
            ac.can_read(&client_not_whitelisted, "test"),
            AccessDecision::Denied
        ); // V-007 Phase 4: Whitelist is enforced when non-empty
    }

    #[test]
    #[allow(deprecated)]
    fn client_info_is_known_service() {
        let temp_dir = TempDir::new().unwrap();
        let exe = temp_dir.path().join("domain_manager");
        fs::write(&exe, b"#!/bin/sh\n").unwrap();
        let client = ClientInfo {
            exe_path: Some(exe.to_string_lossy().to_string()),
            ..Default::default()
        };

        assert!(client.is_known_service());
    }

    #[test]
    #[allow(deprecated)]
    fn client_info_is_not_known_service() {
        let client = ClientInfo {
            exe_path: Some("/path/to/unknown_binary".to_string()),
            ..Default::default()
        };

        assert!(!client.is_known_service());
    }

    #[test]
    fn access_control_exe_whitelist() {
        // Use RequireCredentials policy for testing exe whitelist
        // (RequireKnownService now fails closed without trusted_paths or dev_mode)
        let mut ac = AccessControl::with_policy(AccessPolicy::Whitelist);
        ac.whitelist_exe("my_custom_service".to_string());

        let temp_dir = TempDir::new().unwrap();
        let exe = temp_dir.path().join("my_custom_service");
        fs::write(&exe, b"#!/bin/sh\n").unwrap();
        let client = ClientInfo {
            pid: Some(1234),
            uid: Some(1000),
            gid: Some(1000),
            exe_path: Some(exe.to_string_lossy().to_string()),
            ..Default::default()
        };

        assert_eq!(ac.can_read(&client, "test"), AccessDecision::Allowed);
    }

    // S7 Security Hardening: Tests for exact path matching

    #[test]
    #[allow(deprecated)]
    fn is_known_service_uses_exact_basename_matching() {
        let client = ClientInfo {
            exe_path: Some("/tmp/domain_manager_malicious".to_string()),
            ..Default::default()
        };

        // Should NOT be a known service because basename doesn't match exactly
        // (substring matching would allow this bypass)
        assert!(!client.is_known_service());
    }

    #[test]
    #[allow(deprecated)]
    fn is_known_service_accepts_valid_basenames() {
        let temp_dir = TempDir::new().unwrap();
        let valid_services = vec![
            "domain_manager",
            "runtime_supervisor",
            "store_cli",
            "store_service",
        ];

        for exe_name in valid_services {
            let exe_path = temp_dir.path().join(exe_name);
            fs::write(&exe_path, b"#!/bin/sh\n").unwrap();
            let client = ClientInfo {
                exe_path: Some(exe_path.to_string_lossy().to_string()),
                ..Default::default()
            };
            assert!(
                client.is_known_service(),
                "{} should be a known service",
                exe_name
            );
        }
    }

    #[test]
    fn access_control_default_is_require_credentials() {
        let ac = AccessControl::new();

        // Default should be RequireCredentials (fail-closed)
        let client_no_creds = ClientInfo::default();
        assert_eq!(
            ac.can_read(&client_no_creds, "test"),
            AccessDecision::UnknownClient
        );

        // Client with credentials should be allowed
        let client_with_creds = ClientInfo {
            pid: Some(1234),
            uid: Some(1000),
            gid: Some(1000),
            ..Default::default()
        };
        assert_eq!(
            ac.can_read(&client_with_creds, "test"),
            AccessDecision::Allowed
        );
    }

    // Tests for parse_env_flag utility

    #[test]
    fn test_parse_env_flag_true_values() {
        const ENV_NAME: &str = "RAMEN_STORE_SERVICE_TEST_PARSE_ENV_TRUE_FLAG";
        for val in &["1", "true", "TRUE", "True", "yes", "YES", "on", "ON"] {
            std::env::set_var(ENV_NAME, val);
            assert!(parse_env_flag(ENV_NAME), "expected true for {}", val);
        }
        std::env::remove_var(ENV_NAME);
    }

    #[test]
    fn test_parse_env_flag_false_values() {
        const ENV_NAME: &str = "RAMEN_STORE_SERVICE_TEST_PARSE_ENV_FALSE_FLAG";
        for val in &["0", "false", "FALSE", "no", "NO", "off", "OFF"] {
            std::env::set_var(ENV_NAME, val);
            assert!(!parse_env_flag(ENV_NAME), "expected false for {}", val);
        }
        std::env::remove_var(ENV_NAME);
    }

    #[test]
    fn test_parse_env_flag_unset_is_false() {
        const ENV_NAME: &str = "RAMEN_STORE_SERVICE_TEST_PARSE_ENV_UNSET_FLAG";
        std::env::remove_var(ENV_NAME);
        assert!(!parse_env_flag(ENV_NAME));
    }

    // Tests for is_known_service_exe

    #[test]
    fn test_is_known_service_exe_rejects_basename_only() {
        let dir = TempDir::new().unwrap();
        let fake_exe = dir.path().join("runtime_supervisor");
        fs::write(&fake_exe, "#!/bin/sh\necho fake").unwrap();

        let ac = AccessControl::with_policy(AccessPolicy::RequireKnownService);

        // Should reject because:
        // 1. Not in trusted_paths
        // 2. Dev mode is off
        assert!(!ac.is_known_service_exe(fake_exe.to_str().unwrap()));
    }

    #[test]
    fn test_is_known_service_exe_accepts_exact_trusted_path() {
        let dir = TempDir::new().unwrap();
        let trusted_exe = dir.path().join("runtime_supervisor");
        fs::write(&trusted_exe, "#!/bin/sh\necho real").unwrap();
        let canonical = fs::canonicalize(&trusted_exe).unwrap();

        let mut ac = AccessControl::with_policy(AccessPolicy::RequireKnownService);
        ac.trusted_paths.insert(canonical);

        assert!(ac.is_known_service_exe(trusted_exe.to_str().unwrap()));
    }

    #[test]
    fn test_is_known_service_exe_rejects_nonexistent_path() {
        let ac = AccessControl::with_policy(AccessPolicy::RequireKnownService);

        // Non-existent path should be rejected (canonicalize fails)
        assert!(!ac.is_known_service_exe("/nonexistent/runtime_supervisor"));
    }

    #[test]
    fn test_require_known_service_fails_closed_without_config() {
        // Create an AccessControl with RequireKnownService but no trusted_paths and no dev_mode
        // This should fail closed
        let ac = AccessControl::with_policy(AccessPolicy::RequireKnownService);

        let temp_dir = TempDir::new().unwrap();
        let exe = temp_dir.path().join("runtime_supervisor");
        fs::write(&exe, b"#!/bin/sh\n").unwrap();

        let client = ClientInfo {
            pid: Some(1234),
            uid: Some(1000),
            gid: Some(1000),
            exe_path: Some(exe.to_string_lossy().to_string()),
            ..Default::default()
        };

        // Should be denied because:
        // 1. trusted_paths is empty
        // 2. dev_mode is off
        // This is the fail-closed behavior
        assert_eq!(ac.can_read(&client, "test"), AccessDecision::Denied);
    }

    #[test]
    #[allow(deprecated)]
    fn client_info_is_known_service_backward_compat() {
        // This test uses the deprecated method to ensure backward compatibility
        let temp_dir = TempDir::new().unwrap();
        let exe = temp_dir.path().join("domain_manager");
        fs::write(&exe, b"#!/bin/sh\n").unwrap();
        let client = ClientInfo {
            exe_path: Some(exe.to_string_lossy().to_string()),
            ..Default::default()
        };

        // Deprecated method should still work for backward compat
        assert!(client.is_known_service());
    }
}
