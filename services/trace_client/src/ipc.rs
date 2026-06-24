//! IPC client for trace_service_v1 protocol
//!
//! This module provides the IPC layer for communicating with the kernel's
//! trace service. It defines a transport trait that can be implemented for
//! different communication mechanisms (syscalls, VM-to-host bridges, etc.).

use crate::error::TraceClientError;

// Import trace service types from kernel_api generated module
use kernel_api::generated::{
    CreateTraceBuffer, CreateTraceBufferReply, DestroyTraceBuffer, DestroyTraceBufferReply,
    GetTraceInfo, GetTraceInfoReply, ReadTrace, ReadTraceReply,
};

/// Status codes from kernel trace service
pub const TRACE_STATUS_OK: u32 = 0;
pub const TRACE_STATUS_INVALID_DOMAIN: u32 = 1;
pub const TRACE_STATUS_INVALID_CAPABILITY: u32 = 2;
pub const TRACE_STATUS_PERMISSION_DENIED: u32 = 3;
pub const TRACE_STATUS_INVALID_OFFSET: u32 = 4;
pub const TRACE_STATUS_INVALID_SIZE: u32 = 5;
pub const TRACE_STATUS_BUFFER_NOT_FOUND: u32 = 6;
pub const TRACE_STATUS_BUFFER_EXISTS: u32 = 7;

/// Transport trait for IPC communication
///
/// This trait abstracts the communication mechanism between user-space
/// and the kernel trace service. Implementations can use syscalls,
/// VM-to-host bridges, or other mechanisms.
///
/// # Future Implementations
///
/// - `SyscallTransport`: Direct syscall interface when available
/// - `VmBridgeTransport`: Communication via virtio-serial or similar
/// - `MockTransport`: For testing without kernel interaction
pub trait TraceTransport {
    /// Send a CreateTraceBuffer request and receive reply
    fn create_trace_buffer(
        &mut self,
        request: &CreateTraceBuffer,
    ) -> Result<CreateTraceBufferReply, TraceClientError>;

    /// Send a DestroyTraceBuffer request and receive reply
    fn destroy_trace_buffer(
        &mut self,
        request: &DestroyTraceBuffer,
    ) -> Result<DestroyTraceBufferReply, TraceClientError>;

    /// Send a ReadTrace request and receive reply with data
    ///
    /// Returns (reply, data) where data is the trace buffer contents.
    fn read_trace(
        &mut self,
        request: &ReadTrace,
    ) -> Result<(ReadTraceReply, Vec<u8>), TraceClientError>;

    /// Send a GetTraceInfo request and receive reply
    fn get_trace_info(
        &mut self,
        request: &GetTraceInfo,
    ) -> Result<GetTraceInfoReply, TraceClientError>;
}

/// Mock transport for testing
///
/// This transport returns predictable responses for unit testing
/// without requiring kernel interaction.
#[derive(Debug, Default)]
pub struct MockTransport {
    /// Whether to simulate errors
    simulate_error: bool,
    /// Data to return on read
    mock_data: Vec<u8>,
}

impl MockTransport {
    /// Create a new mock transport
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to simulate errors
    pub fn with_simulate_error(mut self, simulate: bool) -> Self {
        self.simulate_error = simulate;
        self
    }

    /// Set mock data to return on read
    pub fn with_mock_data(mut self, data: Vec<u8>) -> Self {
        self.mock_data = data;
        self
    }
}

impl TraceTransport for MockTransport {
    fn create_trace_buffer(
        &mut self,
        request: &CreateTraceBuffer,
    ) -> Result<CreateTraceBufferReply, TraceClientError> {
        if self.simulate_error {
            return Err(TraceClientError::IpcError("simulated error".to_string()));
        }

        Ok(CreateTraceBufferReply {
            request_id: request.request_id,
            status: TRACE_STATUS_OK,
            trace_cap: 0x1234_5678_9ABC_DEF0, // Mock handle
        })
    }

    fn destroy_trace_buffer(
        &mut self,
        request: &DestroyTraceBuffer,
    ) -> Result<DestroyTraceBufferReply, TraceClientError> {
        if self.simulate_error {
            return Err(TraceClientError::IpcError("simulated error".to_string()));
        }

        Ok(DestroyTraceBufferReply {
            request_id: request.request_id,
            status: TRACE_STATUS_OK,
        })
    }

    fn read_trace(
        &mut self,
        request: &ReadTrace,
    ) -> Result<(ReadTraceReply, Vec<u8>), TraceClientError> {
        if self.simulate_error {
            return Err(TraceClientError::IpcError("simulated error".to_string()));
        }

        // Return mock data (up to requested length)
        let offset = request.offset as usize;
        let length = request.length as usize;

        if offset >= self.mock_data.len() {
            // No data available
            return Ok((
                ReadTraceReply {
                    request_id: request.request_id,
                    status: TRACE_STATUS_OK,
                    data_len: 0,
                },
                Vec::new(),
            ));
        }

        let end = (offset + length).min(self.mock_data.len());
        let data = self.mock_data[offset..end].to_vec();

        Ok((
            ReadTraceReply {
                request_id: request.request_id,
                status: TRACE_STATUS_OK,
                data_len: data.len() as u32,
            },
            data,
        ))
    }

    fn get_trace_info(
        &mut self,
        request: &GetTraceInfo,
    ) -> Result<GetTraceInfoReply, TraceClientError> {
        if self.simulate_error {
            return Err(TraceClientError::IpcError("simulated error".to_string()));
        }

        Ok(GetTraceInfoReply {
            request_id: request.request_id,
            status: TRACE_STATUS_OK,
            domain_id: 0, // Mock domain
            size: 4096,   // Mock size
            read_offset: 0,
            write_offset: self.mock_data.len() as u64,
        })
    }
}

/// Convert kernel status code to TraceClientError
pub fn status_to_error(status: u32, domain_id: u64) -> Result<(), TraceClientError> {
    match status {
        TRACE_STATUS_OK => Ok(()),
        TRACE_STATUS_INVALID_DOMAIN => Err(TraceClientError::BufferNotFound { domain_id }),
        TRACE_STATUS_INVALID_CAPABILITY => Err(TraceClientError::InvalidHandle),
        TRACE_STATUS_PERMISSION_DENIED => Err(TraceClientError::CapabilityDenied { domain_id }),
        TRACE_STATUS_INVALID_OFFSET => {
            Err(TraceClientError::IpcError("invalid offset".to_string()))
        }
        TRACE_STATUS_INVALID_SIZE => Err(TraceClientError::IpcError("invalid size".to_string())),
        TRACE_STATUS_BUFFER_NOT_FOUND => Err(TraceClientError::BufferNotFound { domain_id }),
        TRACE_STATUS_BUFFER_EXISTS => Err(TraceClientError::IpcError(
            "buffer already exists".to_string(),
        )),
        _ => Err(TraceClientError::UnexpectedResponse(format!(
            "unknown status: {}",
            status
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_transport_create_trace_buffer() {
        let mut transport = MockTransport::new();
        let request = CreateTraceBuffer {
            request_id: 1,
            domain_id: 42,
            size: 4096,
        };

        let reply = transport.create_trace_buffer(&request).unwrap();
        assert_eq!(reply.request_id, 1);
        assert_eq!(reply.status, TRACE_STATUS_OK);
        assert_ne!(reply.trace_cap, 0);
    }

    #[test]
    fn mock_transport_read_trace_empty() {
        let mut transport = MockTransport::new();
        let request = ReadTrace {
            request_id: 1,
            trace_cap: 0x1234,
            offset: 0,
            length: 1024,
        };

        let (reply, data) = transport.read_trace(&request).unwrap();
        assert_eq!(reply.status, TRACE_STATUS_OK);
        assert_eq!(reply.data_len, 0);
        assert!(data.is_empty());
    }

    #[test]
    fn mock_transport_read_trace_with_data() {
        let mut transport = MockTransport::new().with_mock_data(vec![1, 2, 3, 4, 5]);
        let request = ReadTrace {
            request_id: 1,
            trace_cap: 0x1234,
            offset: 0,
            length: 1024,
        };

        let (reply, data) = transport.read_trace(&request).unwrap();
        assert_eq!(reply.status, TRACE_STATUS_OK);
        assert_eq!(reply.data_len, 5);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn mock_transport_read_trace_with_offset() {
        let mut transport = MockTransport::new().with_mock_data(vec![1, 2, 3, 4, 5]);
        let request = ReadTrace {
            request_id: 1,
            trace_cap: 0x1234,
            offset: 2,
            length: 2,
        };

        let (reply, data) = transport.read_trace(&request).unwrap();
        assert_eq!(reply.status, TRACE_STATUS_OK);
        assert_eq!(reply.data_len, 2);
        assert_eq!(data, vec![3, 4]);
    }

    #[test]
    fn mock_transport_simulate_error() {
        let mut transport = MockTransport::new().with_simulate_error(true);
        let request = CreateTraceBuffer {
            request_id: 1,
            domain_id: 42,
            size: 4096,
        };

        let result = transport.create_trace_buffer(&request);
        assert!(matches!(result, Err(TraceClientError::IpcError(_))));
    }

    #[test]
    fn status_to_error_ok() {
        assert!(status_to_error(TRACE_STATUS_OK, 0).is_ok());
    }

    #[test]
    fn status_to_error_invalid_domain() {
        let result = status_to_error(TRACE_STATUS_INVALID_DOMAIN, 42);
        assert!(matches!(
            result,
            Err(TraceClientError::BufferNotFound { domain_id: 42 })
        ));
    }

    #[test]
    fn status_to_error_permission_denied() {
        let result = status_to_error(TRACE_STATUS_PERMISSION_DENIED, 42);
        assert!(matches!(
            result,
            Err(TraceClientError::CapabilityDenied { domain_id: 42 })
        ));
    }
}
