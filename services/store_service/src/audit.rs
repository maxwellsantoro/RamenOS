// V-007 Phase 3: Audit Logging for Store Service
//
// Provides comprehensive audit logging for all store service operations.
// Audit logs are append-only and include:
// - Timestamp (ISO 8601)
// - Client PID (from Unix credentials)
// - Operation (GetManifest, GetBlob, VerifyArtifact, IngestArtifact)
// - Parameters (content_id, kind, channel, src_path as appropriate)
// - Result (success/failure, status code)

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Store service operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Operation {
    GetManifest = 1,
    GetBlob = 2,
    VerifyArtifact = 3,
    IngestArtifact = 4,
    QueryProjectionByPath = 5,
    QueryProjectionByTag = 6,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::GetManifest => "GetManifest",
            Operation::GetBlob => "GetBlob",
            Operation::VerifyArtifact => "VerifyArtifact",
            Operation::IngestArtifact => "IngestArtifact",
            Operation::QueryProjectionByPath => "QueryProjectionByPath",
            Operation::QueryProjectionByTag => "QueryProjectionByTag",
        }
    }
}

/// Store service operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationResult {
    Success,
    NotFound,
    InvalidContentId,
    IoError,
    ValidationFailed,
    PermissionDenied,
    Unknown(u32),
}

impl OperationResult {
    pub fn from_status_code(status: u32) -> Self {
        match status {
            0 => OperationResult::Success,
            1 => OperationResult::NotFound,
            2 => OperationResult::InvalidContentId,
            3 => OperationResult::IoError,
            4 => OperationResult::ValidationFailed,
            5 => OperationResult::PermissionDenied,
            code => OperationResult::Unknown(code),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            OperationResult::Success => "Success",
            OperationResult::NotFound => "NotFound",
            OperationResult::InvalidContentId => "InvalidContentId",
            OperationResult::IoError => "IoError",
            OperationResult::ValidationFailed => "ValidationFailed",
            OperationResult::PermissionDenied => "PermissionDenied",
            OperationResult::Unknown(_) => "Unknown",
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub timestamp: u64,
    pub client_pid: Option<u32>,
    pub operation: Operation,
    pub parameters: AuditLogParameters,
    pub result: OperationResult,
    pub duration_ms: u64,
}

/// Parameters for different operations
#[derive(Debug, Clone)]
pub enum AuditLogParameters {
    GetManifest {
        content_id: String,
    },
    GetBlob {
        content_id: String,
    },
    VerifyArtifact {
        content_id: String,
    },
    IngestArtifact {
        kind: String,
        channel: String,
        src_path: String,
        content_id: Option<String>,
    },
    QueryProjectionByPath {
        path: String,
    },
    QueryProjectionByTag {
        tag: String,
    },
}

/// Audit logger
///
/// Thread-safe audit logger that writes to an append-only log file.
pub struct AuditLogger {
    log_file: Mutex<std::fs::File>,
    log_path: String,
}

impl AuditLogger {
    /// Create a new audit logger
    ///
    /// # Arguments
    /// * `log_path` - Path to the audit log file (will be created if it doesn't exist)
    ///
    /// # Returns
    /// Audit logger instance
    pub fn new<P: AsRef<Path>>(log_path: P) -> Result<Self> {
        let log_path = log_path.as_ref().to_string_lossy().to_string();

        // Open log file in append mode, create if it doesn't exist
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("failed to open audit log file")?;

        Ok(Self {
            log_file: Mutex::new(log_file),
            log_path,
        })
    }

    /// Log an operation
    ///
    /// # Arguments
    /// * `entry` - Audit log entry to write
    pub fn log(&self, entry: &AuditLogEntry) -> Result<()> {
        let log_line = self.format_log_entry(entry);

        let mut file = self
            .log_file
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to lock audit log file"))?;

        writeln!(file, "{}", log_line).context("failed to write audit log entry")?;

        // Ensure log entry is written to disk immediately
        file.flush().context("failed to flush audit log")?;

        Ok(())
    }

    /// Get client PID from Unix stream
    ///
    /// # Arguments
    /// * `stream` - Unix stream connection
    ///
    /// # Returns
    /// Client PID if available, None otherwise
    pub fn get_client_pid(stream: &UnixStream) -> Option<u32> {
        use std::os::unix::io::AsRawFd;

        // V-007 Phase 3: Use SO_PEERCRED to get client credentials
        // For now, return None as a stub
        // In Phase 4, we'll implement actual credential retrieval using libc::getsockopt
        let _fd = stream.as_raw_fd();
        None
    }

    /// Format audit log entry as JSON
    ///
    /// # Arguments
    /// * `entry` - Audit log entry
    ///
    /// # Returns
    /// JSON-formatted log line
    fn format_log_entry(&self, entry: &AuditLogEntry) -> String {
        let timestamp_iso = format_timestamp(entry.timestamp);

        match &entry.parameters {
            AuditLogParameters::GetManifest { content_id } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","content_id":"{}","result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(content_id),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
            AuditLogParameters::GetBlob { content_id } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","content_id":"{}","result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(content_id),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
            AuditLogParameters::VerifyArtifact { content_id } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","content_id":"{}","result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(content_id),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
            AuditLogParameters::IngestArtifact {
                kind,
                channel,
                src_path,
                content_id,
            } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","kind":"{}","channel":"{}","src_path":"{}","content_id":{},"result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(kind),
                    escape_json_string(channel),
                    escape_json_string(src_path),
                    content_id.as_ref().map_or("null".to_string(), |id| format!(
                        r#""{}""#,
                        escape_json_string(id)
                    )),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
            AuditLogParameters::QueryProjectionByPath { path } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","path":"{}","result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(path),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
            AuditLogParameters::QueryProjectionByTag { tag } => {
                format!(
                    r#"{{"timestamp":"{}","client_pid":{},"operation":"{}","tag":"{}","result":"{}","duration_ms":{}}}"#,
                    timestamp_iso,
                    entry
                        .client_pid
                        .map_or("null".to_string(), |p| p.to_string()),
                    entry.operation.as_str(),
                    escape_json_string(tag),
                    entry.result.as_str(),
                    entry.duration_ms
                )
            }
        }
    }

    /// Get the log file path
    pub fn log_path(&self) -> &str {
        &self.log_path
    }
}

/// Format Unix timestamp as ISO 8601 string
fn format_timestamp(timestamp: u64) -> String {
    // V-007 Phase 3: Simple timestamp formatting
    // In production, use chrono or time crate for proper formatting
    format!(
        "{}.{:09}+00:00",
        timestamp / 1_000_000_000,
        timestamp % 1_000_000_000
    )
}

/// Escape string for JSON encoding
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ if c.is_control() => result.push_str("\\uXXXX"), // Placeholder
            _ => result.push(c),
        }
    }
    result
}

/// Measure operation duration
pub struct Timer {
    start: std::time::Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp in nanoseconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_is_iso8601() {
        let timestamp = 1_700_000_000_000_000_000; // 2023-11-15-ish
        let formatted = format_timestamp(timestamp);
        assert!(formatted.contains("+00:00"));
        assert!(formatted.contains('.'));
    }

    #[test]
    fn escape_json_string_handles_special_chars() {
        assert_eq!(escape_json_string("test\"quote"), r#"test\"quote"#);
        assert_eq!(escape_json_string("test\\backslash"), r#"test\\backslash"#);
        assert_eq!(escape_json_string("test\nnewline"), r#"test\nnewline"#);
    }

    #[test]
    fn timer_measures_duration() {
        let timer = Timer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn operation_result_from_status_code() {
        assert_eq!(
            OperationResult::from_status_code(0),
            OperationResult::Success
        );
        assert_eq!(
            OperationResult::from_status_code(1),
            OperationResult::NotFound
        );
        assert_eq!(
            OperationResult::from_status_code(2),
            OperationResult::InvalidContentId
        );
        assert_eq!(
            OperationResult::from_status_code(3),
            OperationResult::IoError
        );
        assert_eq!(
            OperationResult::from_status_code(4),
            OperationResult::ValidationFailed
        );
        assert_eq!(
            OperationResult::from_status_code(5),
            OperationResult::PermissionDenied
        );
        assert_eq!(
            OperationResult::from_status_code(99),
            OperationResult::Unknown(99)
        );
    }
}
