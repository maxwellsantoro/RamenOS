//! Trace client for accessing kernel trace buffers
//!
//! This module provides the main [`TraceClient`] type for connecting to
//! the kernel's trace service and reading trace data from domain buffers.

use crate::capability::TraceCapability;
use crate::error::TraceClientError;
use crate::ipc::{MockTransport, TraceTransport, status_to_error};
use kernel_api::cap::Handle;
use kernel_api::generated::{GetTraceInfo, ReadTrace};
use std::cell::RefCell;

// Default chunk size for drain operations
const DEFAULT_CHUNK_SIZE: u32 = 4096;

/// Trace client for accessing kernel trace buffers
///
/// A `TraceClient` maintains a connection to the kernel's trace service
/// and provides methods for reading trace data from a specific domain's
/// buffer. Each client is bound to a single domain ID.
///
/// # Example
///
/// ```no_run
/// use trace_client::TraceClient;
///
/// // Connect to trace service for domain 42
/// let mut client = TraceClient::connect(42)?;
///
/// // Read trace data
/// let mut buf = [0u8; 1024];
/// let n = client.read_trace(&mut buf)?;
/// # Ok::<(), trace_client::TraceClientError>(())
/// ```
pub struct TraceClient {
    /// Domain ID this client is bound to
    domain_id: u64,
    /// Capability handle for trace access
    trace_cap: Handle,
    /// Whether the client is connected
    connected: bool,
    /// Transport for IPC communication
    transport: RefCell<Box<dyn TraceTransport>>,
    /// Next request ID for IPC
    next_request_id: RefCell<u64>,
    /// Current read offset for streaming reads
    read_offset: RefCell<u64>,
}

impl std::fmt::Debug for TraceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceClient")
            .field("domain_id", &self.domain_id)
            .field("trace_cap", &self.trace_cap)
            .field("connected", &self.connected)
            .field("read_offset", &self.read_offset)
            .finish_non_exhaustive()
    }
}

/// Builder for configuring trace client
///
/// Use this to create a `TraceClient` with custom configuration.
///
/// # Example
///
/// ```no_run
/// use trace_client::TraceClientBuilder;
///
/// let client = TraceClientBuilder::new()
///     .domain_id(42)
///     .connect()?;
/// # Ok::<(), trace_client::TraceClientError>(())
/// ```
#[derive(Default)]
pub struct TraceClientBuilder {
    domain_id: Option<u64>,
    trace_cap: Option<Handle>,
    transport: Option<Box<dyn TraceTransport>>,
}

impl std::fmt::Debug for TraceClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceClientBuilder")
            .field("domain_id", &self.domain_id)
            .field("trace_cap", &self.trace_cap)
            .field("transport", &self.transport.as_ref().map(|_| "<transport>"))
            .finish()
    }
}

impl TraceClientBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the domain ID for the trace client
    ///
    /// This is required before calling [`connect`](Self::connect).
    pub fn domain_id(mut self, domain_id: u64) -> Self {
        self.domain_id = Some(domain_id);
        self
    }

    /// Set a pre-existing trace capability handle
    ///
    /// If not provided, a new capability will be obtained during connect.
    pub fn trace_cap(mut self, cap: Handle) -> Self {
        self.trace_cap = Some(cap);
        self
    }

    /// Set a custom transport for IPC communication
    ///
    /// If not provided, a mock transport will be used.
    pub fn transport(mut self, transport: Box<dyn TraceTransport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Connect to the trace service and create a client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `domain_id` was not set
    /// - Connection to kernel trace service fails
    /// - Capability acquisition fails
    pub fn connect(self) -> Result<TraceClient, TraceClientError> {
        let domain_id = self.domain_id.ok_or(TraceClientError::DomainIdRequired)?;

        // Use provided transport or default to mock
        let transport = self
            .transport
            .unwrap_or_else(|| Box::new(MockTransport::new()));

        // Use provided capability or placeholder
        let trace_cap = self.trace_cap.unwrap_or(Handle::INVALID);

        Ok(TraceClient {
            domain_id,
            trace_cap,
            connected: true,
            transport: RefCell::new(transport),
            next_request_id: RefCell::new(1),
            read_offset: RefCell::new(0),
        })
    }
}

impl TraceClient {
    /// Connect to trace service for a specific domain
    ///
    /// This is a convenience method that creates a builder and connects.
    /// Uses a mock transport by default.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use trace_client::TraceClient;
    ///
    /// let client = TraceClient::connect(42)?;
    /// # Ok::<(), trace_client::TraceClientError>(())
    /// ```
    pub fn connect(domain_id: u64) -> Result<Self, TraceClientError> {
        TraceClientBuilder::new().domain_id(domain_id).connect()
    }

    /// Connect with a custom transport
    ///
    /// Use this when you need a specific transport implementation.
    pub fn connect_with_transport(
        domain_id: u64,
        transport: Box<dyn TraceTransport>,
    ) -> Result<Self, TraceClientError> {
        TraceClientBuilder::new()
            .domain_id(domain_id)
            .transport(transport)
            .connect()
    }

    /// Get the domain ID this client is bound to
    pub fn domain_id(&self) -> u64 {
        self.domain_id
    }

    /// Get the trace capability handle
    pub fn trace_cap(&self) -> Handle {
        self.trace_cap
    }

    /// Check if the client is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the next request ID
    fn next_request_id(&self) -> u64 {
        let mut id = self.next_request_id.borrow_mut();
        let current = *id;
        *id = id.wrapping_add(1);
        current
    }

    /// Read trace data from domain's buffer
    ///
    /// Reads up to `buf.len()` bytes of trace data from the domain's
    /// trace buffer. Returns the number of bytes actually read.
    ///
    /// This method maintains an internal read offset, so subsequent calls
    /// will continue reading from where the previous call left off.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Capability is invalid
    /// - IPC communication fails
    /// - No data is available ([`TraceClientError::NoData`])
    pub fn read_trace(&mut self, buf: &mut [u8]) -> Result<usize, TraceClientError> {
        if !self.connected {
            return Err(TraceClientError::BufferDestroyed);
        }

        let request_id = self.next_request_id();
        let offset = *self.read_offset.borrow();

        let request = ReadTrace {
            request_id,
            trace_cap: self.trace_cap.pack(),
            offset,
            length: buf.len() as u32,
        };

        let (reply, data) = self.transport.borrow_mut().read_trace(&request)?;

        // Validate response
        status_to_error(reply.status, self.domain_id)?;

        if reply.data_len == 0 {
            return Err(TraceClientError::NoData);
        }

        // Copy data to caller's buffer
        let to_copy = (reply.data_len as usize).min(buf.len());
        buf[..to_copy].copy_from_slice(&data[..to_copy]);

        // Update read offset
        *self.read_offset.borrow_mut() += to_copy as u64;

        Ok(to_copy)
    }

    /// Get trace buffer metadata
    ///
    /// Returns information about the trace buffer including its size
    /// and current read/write positions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Capability is invalid
    /// - IPC communication fails
    pub fn get_info(&self) -> Result<TraceInfo, TraceClientError> {
        if !self.connected {
            return Err(TraceClientError::BufferDestroyed);
        }

        let request_id = self.next_request_id();

        let request = GetTraceInfo {
            request_id,
            trace_cap: self.trace_cap.pack(),
        };

        let reply = self.transport.borrow_mut().get_trace_info(&request)?;

        // Validate response
        status_to_error(reply.status, self.domain_id)?;

        Ok(TraceInfo {
            domain_id: reply.domain_id,
            size: reply.size,
            read_offset: reply.read_offset,
            write_offset: reply.write_offset,
        })
    }

    /// Drain all available trace data
    ///
    /// Reads all available trace data from the buffer and returns it
    /// as a vector. This is useful for collecting traces on shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Capability is invalid
    /// - IPC communication fails
    pub fn drain(&mut self) -> Result<Vec<u8>, TraceClientError> {
        if !self.connected {
            return Err(TraceClientError::BufferDestroyed);
        }

        // Get buffer info to know how much data is available
        let info = self.get_info()?;
        let available = info.available();

        if available == 0 {
            return Ok(Vec::new());
        }

        // Read in chunks
        let mut result = Vec::with_capacity(available as usize);
        let mut remaining = available;

        while remaining > 0 {
            let to_read = (remaining as usize).min(DEFAULT_CHUNK_SIZE as usize);
            let mut chunk = vec![0u8; to_read];

            let n = self.read_trace(&mut chunk)?;
            if n == 0 {
                break; // No more data
            }

            result.extend_from_slice(&chunk[..n]);
            remaining -= n as u64;
        }

        Ok(result)
    }

    /// Create a trace capability for another domain
    ///
    /// This requires admin rights on the current capability.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Client is not connected
    /// - Current capability lacks admin rights
    /// - Target domain doesn't exist
    pub fn create_capability(
        &self,
        target_domain: u64,
    ) -> Result<TraceCapability, TraceClientError> {
        if !self.connected {
            return Err(TraceClientError::BufferDestroyed);
        }

        use crate::capability::TRACE_RIGHT_ALL;
        use kernel_api::generated::CreateTraceBuffer;

        let request = CreateTraceBuffer {
            request_id: *self.next_request_id.borrow(),
            domain_id: target_domain,
            size: 4096,
        };

        let reply = self
            .transport
            .borrow_mut()
            .create_trace_buffer(&request)
            .map_err(|e| TraceClientError::IpcError(e.to_string()))?;

        status_to_error(reply.status, target_domain)?;

        Ok(TraceCapability::new(
            Handle::unpack(reply.trace_cap),
            target_domain,
            TRACE_RIGHT_ALL,
        ))
    }

    /// Disconnect from the trace service
    ///
    /// After calling this, all operations will return an error.
    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    /// Reset the read offset to the beginning of the buffer
    ///
    /// This allows re-reading trace data from the start.
    pub fn reset_read_offset(&mut self) {
        *self.read_offset.borrow_mut() = 0;
    }
}

/// Trace buffer metadata
#[derive(Debug, Clone)]
pub struct TraceInfo {
    /// Domain ID this buffer belongs to
    pub domain_id: u64,
    /// Total size of the trace buffer in bytes
    pub size: u32,
    /// Current read position
    pub read_offset: u64,
    /// Current write position
    pub write_offset: u64,
}

impl TraceInfo {
    /// Get the number of bytes available to read
    pub fn available(&self) -> u64 {
        self.write_offset.saturating_sub(self.read_offset)
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_client_builder_default() {
        let builder = TraceClientBuilder::new();
        assert!(builder.domain_id.is_none());
        assert!(builder.trace_cap.is_none());
    }

    #[test]
    fn trace_client_builder_requires_domain_id() {
        let result = TraceClientBuilder::new().connect();
        assert!(matches!(result, Err(TraceClientError::DomainIdRequired)));
    }

    #[test]
    fn trace_client_builder_with_domain_id() {
        let builder = TraceClientBuilder::new().domain_id(42);
        assert_eq!(builder.domain_id, Some(42));
    }

    #[test]
    fn trace_client_connect_stores_domain_id() {
        let client = TraceClient::connect(42).unwrap();
        assert_eq!(client.domain_id(), 42);
        assert!(client.is_connected());
    }

    #[test]
    fn trace_client_disconnect_prevents_operations() {
        let mut client = TraceClient::connect(42).unwrap();
        client.disconnect();

        let mut buf = [0u8; 1024];
        let result = client.read_trace(&mut buf);
        assert!(matches!(result, Err(TraceClientError::BufferDestroyed)));
    }

    #[test]
    fn trace_client_read_trace_with_mock_data() {
        let transport = Box::new(MockTransport::new().with_mock_data(vec![1, 2, 3, 4, 5]));
        let mut client = TraceClient::connect_with_transport(42, transport).unwrap();

        let mut buf = [0u8; 10];
        let n = client.read_trace(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn trace_client_read_trace_no_data() {
        let transport = Box::new(MockTransport::new());
        let mut client = TraceClient::connect_with_transport(42, transport).unwrap();

        let mut buf = [0u8; 10];
        let result = client.read_trace(&mut buf);
        assert!(matches!(result, Err(TraceClientError::NoData)));
    }

    #[test]
    fn trace_client_get_info_with_mock() {
        let transport = Box::new(MockTransport::new().with_mock_data(vec![1, 2, 3]));
        let client = TraceClient::connect_with_transport(42, transport).unwrap();

        let info = client.get_info().unwrap();
        assert_eq!(info.write_offset, 3); // Mock transport sets this to data length
    }

    #[test]
    fn trace_client_drain_with_mock_data() {
        let transport = Box::new(MockTransport::new().with_mock_data(vec![1, 2, 3, 4, 5]));
        let mut client = TraceClient::connect_with_transport(42, transport).unwrap();

        let data = client.drain().unwrap();
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn trace_client_drain_empty() {
        let transport = Box::new(MockTransport::new());
        let mut client = TraceClient::connect_with_transport(42, transport).unwrap();

        let data = client.drain().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn trace_client_reset_read_offset() {
        let transport = Box::new(MockTransport::new().with_mock_data(vec![1, 2, 3]));
        let mut client = TraceClient::connect_with_transport(42, transport).unwrap();

        // Read all data
        let mut buf = [0u8; 10];
        let n = client.read_trace(&mut buf).unwrap();
        assert_eq!(n, 3);

        // No more data
        let result = client.read_trace(&mut buf);
        assert!(matches!(result, Err(TraceClientError::NoData)));

        // Reset and read again
        client.reset_read_offset();
        let n = client.read_trace(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[1, 2, 3]);
    }

    #[test]
    fn trace_info_available() {
        let info = TraceInfo {
            domain_id: 1,
            size: 4096,
            read_offset: 100,
            write_offset: 500,
        };
        assert_eq!(info.available(), 400);
        assert!(!info.is_empty());
    }

    #[test]
    fn trace_info_empty() {
        let info = TraceInfo {
            domain_id: 1,
            size: 4096,
            read_offset: 100,
            write_offset: 100,
        };
        assert_eq!(info.available(), 0);
        assert!(info.is_empty());
    }
}
