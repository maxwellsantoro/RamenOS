// V-012 Phase 4: Kernel-side trace service
//
// This module provides a kernel-side trace service that manages domain-scoped
// trace buffers with capability-based access control. The service integrates
// with the existing TraceRing and TraceCapTable from trace_cap.rs.

use crate::domain_registry::{DomainId, DomainRegistry, MAX_DOMAINS};
use crate::trace_cap::{TRACE_RIGHT_ADMIN, TRACE_RIGHT_READ, TraceCapTable};
use crate::trace_ring::DomainTraceRing;
use kernel_api::cap::Handle;
use kernel_api::trace::Event;

/// Status codes for trace service operations
pub const TRACE_STATUS_OK: u32 = 0;
pub const TRACE_STATUS_INVALID_DOMAIN: u32 = 1;
pub const TRACE_STATUS_INVALID_CAPABILITY: u32 = 2;
pub const TRACE_STATUS_PERMISSION_DENIED: u32 = 3;
pub const TRACE_STATUS_INVALID_OFFSET: u32 = 4;
pub const TRACE_STATUS_INVALID_SIZE: u32 = 5;
pub const TRACE_STATUS_BUFFER_NOT_FOUND: u32 = 6;
pub const TRACE_STATUS_BUFFER_EXISTS: u32 = 7;

/// Trace buffer information
#[derive(Debug, Clone, Copy)]
pub struct TraceBufferInfo {
    /// Domain ID this buffer belongs to
    pub domain_id: DomainId,
    /// Size of the trace buffer (in bytes)
    pub size: u32,
    /// Current read offset
    pub read_offset: u64,
    /// Current write offset
    pub write_offset: u64,
}

/// Trace buffer entry
struct TraceBuffer {
    /// Domain ID this buffer belongs to
    domain_id: DomainId,
    /// Size of the buffer (in bytes)
    size: u32,
    /// Read offset (for tracking position)
    read_offset: u64,
    /// Write offset (for tracking position)
    write_offset: u64,
    /// Whether this buffer is active
    active: bool,
}

/// Kernel-side trace service
///
/// Manages domain-scoped trace buffers with capability-based access control.
/// The service integrates with TraceRing for actual buffer management and
/// TraceCapTable for capability validation.
pub struct TraceService {
    /// Trace buffers (one per domain)
    buffers: [Option<TraceBuffer>; MAX_DOMAINS],
    /// Trace capability table for access control
    cap_table: TraceCapTable,
    /// Domain registry for tracking active domains
    domain_registry: DomainRegistry,
    /// Per-domain trace ring for actual buffer storage
    trace_ring: DomainTraceRing,
}

impl TraceService {
    /// Create a new trace service
    pub const fn new() -> Self {
        const EMPTY_BUFFER: Option<TraceBuffer> = None;

        Self {
            buffers: [EMPTY_BUFFER; MAX_DOMAINS],
            cap_table: TraceCapTable::new(),
            domain_registry: DomainRegistry::new(),
            trace_ring: DomainTraceRing::new(),
        }
    }

    /// Initialize the kernel domain (ID 0)
    ///
    /// This should be called during kernel boot to set up domain 0.
    pub fn init_kernel(&mut self) {
        self.domain_registry.init_kernel();
        self.trace_ring.init_kernel();
    }

    /// Register a new domain with the given ID and name
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(())` if registration fails
    #[allow(clippy::result_unit_err)]
    pub fn register_domain(&mut self, id: DomainId, name: &str) -> Result<(), ()> {
        self.domain_registry.register(id, name).map_err(|_| ())
    }

    /// Create a new trace buffer for a domain
    ///
    /// # Arguments
    /// - `domain_id`: Domain ID to create buffer for
    /// - `size`: Size of the buffer in bytes
    ///
    /// # Returns
    /// - `Ok((status, trace_cap))` on success with status and capability handle
    /// - `Err(())` on internal error
    #[allow(clippy::result_unit_err)]
    pub fn create_trace_buffer(
        &mut self,
        domain_id: DomainId,
        size: u32,
    ) -> Result<(u32, Handle), ()> {
        // Validate domain ID
        if domain_id >= MAX_DOMAINS as DomainId {
            return Ok((TRACE_STATUS_INVALID_DOMAIN, Handle::INVALID));
        }

        // Check if domain is registered
        if !self.domain_registry.is_registered(domain_id) {
            return Ok((TRACE_STATUS_INVALID_DOMAIN, Handle::INVALID));
        }

        // Check if buffer already exists for this domain
        let idx = domain_id as usize;
        if self.buffers[idx].is_some() {
            return Ok((TRACE_STATUS_BUFFER_EXISTS, Handle::INVALID));
        }

        // Validate size (must be non-zero and power of two for ring buffer)
        if size == 0 || !size.is_power_of_two() {
            return Ok((TRACE_STATUS_INVALID_SIZE, Handle::INVALID));
        }

        // Allocate trace capability with READ rights
        let cap_index = self
            .cap_table
            .allocate(domain_id, TRACE_RIGHT_READ | TRACE_RIGHT_ADMIN)
            .map_err(|_| ())?;

        // Create trace buffer entry
        self.buffers[idx] = Some(TraceBuffer {
            domain_id,
            size,
            read_offset: 0,
            write_offset: 0,
            active: true,
        });

        // Create handle from capability index
        let cap = self.cap_table.get(cap_index).unwrap();
        let handle = Handle {
            kind: kernel_api::cap::HandleKind::Trace,
            index: cap_index as u32,
            generation: cap.generation,
        };

        Ok((TRACE_STATUS_OK, handle))
    }

    /// Destroy a trace buffer
    ///
    /// # Arguments
    /// - `trace_cap`: Trace capability handle
    ///
    /// # Returns
    /// - `Ok(status)` on success with status code
    /// - `Err(())` on internal error
    #[allow(clippy::result_unit_err)]
    pub fn destroy_trace_buffer(&mut self, trace_cap: Handle) -> Result<u32, ()> {
        // Validate handle kind
        if trace_cap.kind != kernel_api::cap::HandleKind::Trace {
            return Ok(TRACE_STATUS_INVALID_CAPABILITY);
        }

        // Validate handle
        let cap_index = trace_cap.index as usize;
        if cap_index >= MAX_DOMAINS {
            return Ok(TRACE_STATUS_INVALID_CAPABILITY);
        }

        // Get capability and validate generation
        let cap = match self.cap_table.get(cap_index) {
            Some(c) => c,
            None => return Ok(TRACE_STATUS_INVALID_CAPABILITY),
        };

        if cap.generation != trace_cap.generation {
            return Ok(TRACE_STATUS_INVALID_CAPABILITY);
        }

        // Check ADMIN rights
        if !cap.can_admin() {
            return Ok(TRACE_STATUS_PERMISSION_DENIED);
        }

        // Get domain ID from capability
        let domain_id = cap.domain_id;
        let idx = domain_id as usize;

        // Check if buffer exists and is active
        if self.buffers[idx]
            .as_ref()
            .is_none_or(|buffer| !buffer.active)
        {
            return Ok(TRACE_STATUS_BUFFER_NOT_FOUND);
        }

        // Deallocate buffer
        self.buffers[idx] = None;

        // Deallocate capability
        self.cap_table.deallocate(cap_index).map_err(|_| ())?;

        Ok(TRACE_STATUS_OK)
    }

    /// Read from a trace buffer
    ///
    /// # Arguments
    /// - `trace_cap`: Trace capability handle
    /// - `offset`: Offset to start reading from
    /// - `length`: Number of bytes to read
    /// - `out`: Output buffer for trace data
    ///
    /// # Returns
    /// - `Ok((status, bytes_read))` on success with status and bytes read
    /// - `Err(())` on internal error
    #[allow(clippy::result_unit_err)]
    pub fn read_trace(
        &mut self,
        trace_cap: Handle,
        offset: u64,
        length: u32,
        out: &mut [u8],
    ) -> Result<(u32, u32), ()> {
        // Validate handle kind
        if trace_cap.kind != kernel_api::cap::HandleKind::Trace {
            return Ok((TRACE_STATUS_INVALID_CAPABILITY, 0));
        }

        // Validate handle
        let cap_index = trace_cap.index as usize;
        if cap_index >= MAX_DOMAINS {
            return Ok((TRACE_STATUS_INVALID_CAPABILITY, 0));
        }

        // Get capability and validate generation
        let cap = match self.cap_table.get(cap_index) {
            Some(c) => c,
            None => return Ok((TRACE_STATUS_INVALID_CAPABILITY, 0)),
        };

        if cap.generation != trace_cap.generation {
            return Ok((TRACE_STATUS_INVALID_CAPABILITY, 0));
        }

        // Check READ rights
        if !cap.can_read() {
            return Ok((TRACE_STATUS_PERMISSION_DENIED, 0));
        }

        // Get domain ID from capability
        let domain_id = cap.domain_id;
        let idx = domain_id as usize;

        // Check if buffer exists
        let buffer = match self.buffers[idx] {
            Some(ref b) => b,
            None => return Ok((TRACE_STATUS_BUFFER_NOT_FOUND, 0)),
        };

        // Validate offset
        if offset >= buffer.size as u64 {
            return Ok((TRACE_STATUS_INVALID_OFFSET, 0));
        }

        // Calculate actual read length
        let available = buffer.size as u64 - offset;
        let length_u64 = length as u64;
        let out_len_u64 = out.len() as u64;
        let read_len = length_u64.min(available).min(out_len_u64) as u32;

        // Read from trace ring buffer
        let mut events = [Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 64]; // Max 64 events per read

        let event_count = self.trace_ring.read(domain_id, &mut events);

        // Convert events to bytes
        let bytes_to_copy = (event_count * core::mem::size_of::<Event>()) as u32;
        let actual_bytes = bytes_to_copy.min(read_len);

        // Copy event data to output buffer
        let src_bytes = unsafe {
            core::slice::from_raw_parts(
                events.as_ptr() as *const u8,
                event_count * core::mem::size_of::<Event>(),
            )
        };

        let actual_bytes_usize = actual_bytes as usize;
        let copy_len = actual_bytes_usize.min(out.len());
        out[..copy_len].copy_from_slice(&src_bytes[..copy_len]);

        // Update read offset
        if let Some(ref mut b) = self.buffers[idx] {
            b.read_offset = offset + actual_bytes as u64;
        }

        Ok((TRACE_STATUS_OK, copy_len as u32))
    }

    /// Get trace buffer information
    ///
    /// # Arguments
    /// - `trace_cap`: Trace capability handle
    ///
    /// # Returns
    /// - `Ok((status, info))` on success with status and buffer info
    /// - `Err(())` on internal error
    #[allow(clippy::result_unit_err)]
    pub fn get_trace_info(&mut self, trace_cap: Handle) -> Result<(u32, TraceBufferInfo), ()> {
        // Validate handle kind
        if trace_cap.kind != kernel_api::cap::HandleKind::Trace {
            return Ok((
                TRACE_STATUS_INVALID_CAPABILITY,
                TraceBufferInfo {
                    domain_id: 0,
                    size: 0,
                    read_offset: 0,
                    write_offset: 0,
                },
            ));
        }

        // Validate handle
        let cap_index = trace_cap.index as usize;
        if cap_index >= MAX_DOMAINS {
            return Ok((
                TRACE_STATUS_INVALID_CAPABILITY,
                TraceBufferInfo {
                    domain_id: 0,
                    size: 0,
                    read_offset: 0,
                    write_offset: 0,
                },
            ));
        }

        // Get capability and validate generation
        let cap = match self.cap_table.get(cap_index) {
            Some(c) => c,
            None => {
                return Ok((
                    TRACE_STATUS_INVALID_CAPABILITY,
                    TraceBufferInfo {
                        domain_id: 0,
                        size: 0,
                        read_offset: 0,
                        write_offset: 0,
                    },
                ));
            }
        };

        if cap.generation != trace_cap.generation {
            return Ok((
                TRACE_STATUS_INVALID_CAPABILITY,
                TraceBufferInfo {
                    domain_id: 0,
                    size: 0,
                    read_offset: 0,
                    write_offset: 0,
                },
            ));
        }

        // Get domain ID from capability
        let domain_id = cap.domain_id;
        let idx = domain_id as usize;

        // Check if buffer exists
        let buffer = match self.buffers[idx] {
            Some(ref b) => b,
            None => {
                return Ok((
                    TRACE_STATUS_BUFFER_NOT_FOUND,
                    TraceBufferInfo {
                        domain_id: 0,
                        size: 0,
                        read_offset: 0,
                        write_offset: 0,
                    },
                ));
            }
        };

        let info = TraceBufferInfo {
            domain_id: buffer.domain_id,
            size: buffer.size,
            read_offset: buffer.read_offset,
            write_offset: buffer.write_offset,
        };

        Ok((TRACE_STATUS_OK, info))
    }

    /// Get the trace capability table (for testing)
    #[cfg(test)]
    pub fn cap_table(&self) -> &TraceCapTable {
        &self.cap_table
    }

    /// Get the trace capability table mutably (for testing)
    #[cfg(test)]
    pub fn cap_table_mut(&mut self) -> &mut TraceCapTable {
        &mut self.cap_table
    }

    /// Get the domain registry (for testing)
    #[cfg(test)]
    pub fn domain_registry(&self) -> &DomainRegistry {
        &self.domain_registry
    }

    /// Get the trace ring (for testing)
    #[cfg(test)]
    pub fn trace_ring(&self) -> &DomainTraceRing {
        &self.trace_ring
    }

    /// Get the trace ring mutably (for testing)
    #[cfg(test)]
    pub fn trace_ring_mut(&mut self) -> &mut DomainTraceRing {
        &mut self.trace_ring
    }
}

impl Default for TraceService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_service() -> TraceService {
        let mut service = TraceService::new();
        service.init_kernel();
        service
    }

    #[test]
    fn create_trace_buffer_succeeds_with_valid_params() {
        let mut service = setup_test_service();

        // Create trace buffer for domain 0
        let (status, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_OK);
        assert_ne!(trace_cap, Handle::INVALID);
        assert_eq!(trace_cap.kind, kernel_api::cap::HandleKind::Trace);
    }

    #[test]
    fn create_trace_buffer_fails_with_invalid_domain() {
        let mut service = setup_test_service();

        // Try to create trace buffer for invalid domain
        let (status, trace_cap) = service
            .create_trace_buffer(MAX_DOMAINS as DomainId, 4096)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_DOMAIN);
        assert_eq!(trace_cap, Handle::INVALID);
    }

    #[test]
    fn create_trace_buffer_fails_with_invalid_size() {
        let mut service = setup_test_service();

        // Try to create trace buffer with invalid size (not power of two)
        let (status, trace_cap) = service
            .create_trace_buffer(0, 4095)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_SIZE);
        assert_eq!(trace_cap, Handle::INVALID);
    }

    #[test]
    fn create_trace_buffer_fails_with_zero_size() {
        let mut service = setup_test_service();

        // Try to create trace buffer with zero size
        let (status, trace_cap) = service
            .create_trace_buffer(0, 0)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_SIZE);
        assert_eq!(trace_cap, Handle::INVALID);
    }

    #[test]
    fn create_trace_buffer_fails_for_unregistered_domain() {
        let mut service = setup_test_service();

        // Try to create trace buffer for unregistered domain
        let (status, trace_cap) = service
            .create_trace_buffer(1, 4096)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_DOMAIN);
        assert_eq!(trace_cap, Handle::INVALID);
    }

    #[test]
    fn create_trace_buffer_fails_if_buffer_exists() {
        let mut service = setup_test_service();

        // Create first trace buffer
        let (status, _) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");
        assert_eq!(status, TRACE_STATUS_OK);

        // Try to create second trace buffer for same domain
        let (status, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_BUFFER_EXISTS);
        assert_eq!(trace_cap, Handle::INVALID);
    }

    #[test]
    fn destroy_trace_buffer_succeeds_with_valid_cap() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Destroy trace buffer
        let status = service
            .destroy_trace_buffer(trace_cap)
            .expect("destroy_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_OK);
    }

    #[test]
    fn destroy_trace_buffer_fails_with_invalid_cap() {
        let mut service = setup_test_service();

        // Try to destroy with invalid handle kind
        let invalid_cap = Handle {
            kind: kernel_api::cap::HandleKind::Ipc,
            index: 0,
            generation: 1,
        };

        let status = service
            .destroy_trace_buffer(invalid_cap)
            .expect("destroy_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_CAPABILITY);
    }

    #[test]
    fn destroy_trace_buffer_fails_with_stale_generation() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Modify generation to make it stale
        let stale_cap = Handle {
            kind: trace_cap.kind,
            index: trace_cap.index,
            generation: trace_cap.generation + 1,
        };

        let status = service
            .destroy_trace_buffer(stale_cap)
            .expect("destroy_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_INVALID_CAPABILITY);
    }

    #[test]
    fn destroy_trace_buffer_fails_without_admin_rights() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Manually revoke ADMIN rights from capability
        let cap_index = trace_cap.index as usize;
        service
            .cap_table_mut()
            .revoke_rights(cap_index, TRACE_RIGHT_ADMIN)
            .expect("revoke_rights failed");

        // Try to destroy without ADMIN rights
        let status = service
            .destroy_trace_buffer(trace_cap)
            .expect("destroy_trace_buffer failed");

        assert_eq!(status, TRACE_STATUS_PERMISSION_DENIED);
    }

    #[test]
    fn read_trace_succeeds_with_valid_cap() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Emit some trace events
        service.trace_ring_mut().emit(0, 0x1000, 42, 100);
        service.trace_ring_mut().emit(0, 0x1001, 43, 101);

        // Read trace
        let mut out = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(trace_cap, 0, 1024, &mut out)
            .expect("read_trace failed");

        assert_eq!(status, TRACE_STATUS_OK);
        // Each Event is 16 bytes (4 + 8 + 8), so 2 events = 32 bytes
        assert!(bytes_read > 0);
    }

    #[test]
    fn read_trace_fails_with_invalid_cap() {
        let mut service = setup_test_service();

        // Try to read with invalid handle kind
        let invalid_cap = Handle {
            kind: kernel_api::cap::HandleKind::Ipc,
            index: 0,
            generation: 1,
        };

        let mut out = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(invalid_cap, 0, 1024, &mut out)
            .expect("read_trace failed");

        assert_eq!(status, TRACE_STATUS_INVALID_CAPABILITY);
        assert_eq!(bytes_read, 0);
    }

    #[test]
    fn read_trace_fails_with_invalid_offset() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Try to read with offset beyond buffer size
        let mut out = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(trace_cap, 5000, 1024, &mut out)
            .expect("read_trace failed");

        assert_eq!(status, TRACE_STATUS_INVALID_OFFSET);
        assert_eq!(bytes_read, 0);
    }

    #[test]
    fn read_trace_fails_without_read_rights() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Manually revoke READ rights from capability
        let cap_index = trace_cap.index as usize;
        service
            .cap_table_mut()
            .revoke_rights(cap_index, TRACE_RIGHT_READ)
            .expect("revoke_rights failed");

        // Try to read without READ rights
        let mut out = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(trace_cap, 0, 1024, &mut out)
            .expect("read_trace failed");

        assert_eq!(status, TRACE_STATUS_PERMISSION_DENIED);
        assert_eq!(bytes_read, 0);
    }

    #[test]
    fn get_trace_info_returns_correct_metadata() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Get trace info
        let (status, info) = service
            .get_trace_info(trace_cap)
            .expect("get_trace_info failed");

        assert_eq!(status, TRACE_STATUS_OK);
        assert_eq!(info.domain_id, 0);
        assert_eq!(info.size, 4096);
        assert_eq!(info.read_offset, 0);
        assert_eq!(info.write_offset, 0);
    }

    #[test]
    fn get_trace_info_fails_with_invalid_cap() {
        let mut service = setup_test_service();

        // Try to get info with invalid handle kind
        let invalid_cap = Handle {
            kind: kernel_api::cap::HandleKind::Ipc,
            index: 0,
            generation: 1,
        };

        let (status, info) = service
            .get_trace_info(invalid_cap)
            .expect("get_trace_info failed");

        assert_eq!(status, TRACE_STATUS_INVALID_CAPABILITY);
        assert_eq!(info.domain_id, 0);
        assert_eq!(info.size, 0);
    }

    #[test]
    fn get_trace_info_fails_for_nonexistent_buffer() {
        let mut service = setup_test_service();

        // Create and destroy trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");
        service
            .destroy_trace_buffer(trace_cap)
            .expect("destroy_trace_buffer failed");

        // Try to get info for destroyed buffer
        let (status, info) = service
            .get_trace_info(trace_cap)
            .expect("get_trace_info failed");

        assert_eq!(status, TRACE_STATUS_INVALID_CAPABILITY);
        assert_eq!(info.domain_id, 0);
    }

    #[test]
    fn trace_service_isolation_prevents_cross_domain_access() {
        let mut service = setup_test_service();

        // Register domain 1
        service
            .register_domain(1, "test_domain")
            .expect("register_domain failed");

        // Create trace buffer for domain 0
        let (_, cap0) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Create trace buffer for domain 1
        let (_, cap1) = service
            .create_trace_buffer(1, 4096)
            .expect("create_trace_buffer failed");

        // Emit events to domain 0
        service.trace_ring_mut().emit(0, 0x1000, 42, 100);

        // Emit events to domain 1
        service.trace_ring_mut().emit(1, 0x2000, 43, 101);

        // Read from domain 0 with cap0
        let mut out0 = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(cap0, 0, 1024, &mut out0)
            .expect("read_trace failed");
        assert_eq!(status, TRACE_STATUS_OK);
        assert!(bytes_read > 0);

        // Read from domain 1 with cap1
        let mut out1 = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(cap1, 0, 1024, &mut out1)
            .expect("read_trace failed");
        assert_eq!(status, TRACE_STATUS_OK);
        assert!(bytes_read > 0);
    }

    #[test]
    fn trace_service_full_lifecycle() {
        let mut service = setup_test_service();

        // Create trace buffer
        let (_, trace_cap) = service
            .create_trace_buffer(0, 4096)
            .expect("create_trace_buffer failed");

        // Emit events
        service.trace_ring_mut().emit(0, 0x1000, 42, 100);
        service.trace_ring_mut().emit(0, 0x1001, 43, 101);

        // Read events
        let mut out = [0u8; 1024];
        let (status, bytes_read) = service
            .read_trace(trace_cap, 0, 1024, &mut out)
            .expect("read_trace failed");
        assert_eq!(status, TRACE_STATUS_OK);
        assert!(bytes_read > 0);

        // Get info
        let (status, info) = service
            .get_trace_info(trace_cap)
            .expect("get_trace_info failed");
        assert_eq!(status, TRACE_STATUS_OK);
        assert_eq!(info.domain_id, 0);

        // Destroy buffer
        let status = service
            .destroy_trace_buffer(trace_cap)
            .expect("destroy_trace_buffer failed");
        assert_eq!(status, TRACE_STATUS_OK);
    }
}
