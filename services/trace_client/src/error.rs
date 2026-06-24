//! Error types for the trace client library

use thiserror::Error;

/// Errors from trace client operations
#[derive(Debug, Error)]
pub enum TraceClientError {
    /// IPC communication error
    #[error("IPC error: {0}")]
    IpcError(String),

    /// Capability denied for the requested operation
    #[error("Capability denied for domain {domain_id}")]
    CapabilityDenied {
        /// Domain ID that was denied access
        domain_id: u64,
    },

    /// Invalid trace handle provided
    #[error("Invalid trace handle")]
    InvalidHandle,

    /// Trace buffer not found for the specified domain
    #[error("Trace buffer not found for domain {domain_id}")]
    BufferNotFound {
        /// Domain ID that has no trace buffer
        domain_id: u64,
    },

    /// Trace buffer has been destroyed
    #[error("Trace buffer destroyed")]
    BufferDestroyed,

    /// Domain ID is required but not provided
    #[error("Domain ID is required")]
    DomainIdRequired,

    /// Read operation returned no data
    #[error("No trace data available")]
    NoData,

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Unexpected response from kernel
    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),
}

// Implement From for common error types
impl From<bincode::Error> for TraceClientError {
    fn from(err: bincode::Error) -> Self {
        TraceClientError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_capability_denied() {
        let err = TraceClientError::CapabilityDenied { domain_id: 42 };
        assert_eq!(format!("{}", err), "Capability denied for domain 42");
    }

    #[test]
    fn error_display_buffer_not_found() {
        let err = TraceClientError::BufferNotFound { domain_id: 123 };
        assert_eq!(format!("{}", err), "Trace buffer not found for domain 123");
    }

    #[test]
    fn error_display_ipc_error() {
        let err = TraceClientError::IpcError("connection refused".to_string());
        assert_eq!(format!("{}", err), "IPC error: connection refused");
    }
}
