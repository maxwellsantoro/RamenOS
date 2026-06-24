//! Error types for domain manager operations.
//!
//! These errors are returned when domain manager operations fail,
//! allowing proper error handling instead of panics.

use std::fmt;

/// Error type for domain manager operations.
#[derive(Debug)]
#[allow(dead_code)] // Variants are part of public API, used in future implementations
pub enum DomainManagerError {
    /// Failed to serialize payload for IPC reply
    PayloadSerialization(String),
    /// Invalid request parameters
    InvalidRequest(String),
    /// Domain not found
    DomainNotFound(u64),
    /// Internal error
    InternalError(String),
}

impl fmt::Display for DomainManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadSerialization(msg) => {
                write!(f, "payload serialization failed: {}", msg)
            }
            Self::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            Self::DomainNotFound(id) => write!(f, "domain not found: {}", id),
            Self::InternalError(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for DomainManagerError {}
