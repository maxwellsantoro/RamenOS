// services/native_runner/src/error.rs
//! Error types for the native runner.

use thiserror::Error;

/// Top-level runner errors.
#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("WASM compilation failed: {0}")]
    WasmCompile(String),

    #[error("WASM instantiation failed: {0}")]
    WasmInstantiate(String),

    #[error("Missing required capability: {0}")]
    MissingCapability(String),

    #[error("Failed to set global: {0}")]
    GlobalSet(String),

    #[error("Harness call failed: {0}")]
    HarnessCall(String),

    #[error("Kernel IPC error: {0}")]
    KernelIpc(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Status codes returned to WASM modules.
/// Must match kernel_api status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Status {
    Ok = 0,
    InvalidCapability = 1,
    PermissionDenied = 2,
    InvalidArgument = 3,
    WouldBlock = 4,
    IoError = 5,
    InternalError = 6,
    KernelError = 7,
}

impl Status {
    /// Convert from u32 status code to Status enum.
    /// Returns InternalError for unknown codes (fail-closed).
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Status::Ok,
            1 => Status::InvalidCapability,
            2 => Status::PermissionDenied,
            3 => Status::InvalidArgument,
            4 => Status::WouldBlock,
            5 => Status::IoError,
            6 => Status::InternalError,
            7 => Status::KernelError,
            _ => Status::InternalError, // Unknown codes treated as internal error
        }
    }
}

impl From<Status> for i32 {
    fn from(status: Status) -> i32 {
        status as i32
    }
}
