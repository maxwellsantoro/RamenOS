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
