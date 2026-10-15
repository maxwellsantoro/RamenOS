// V-012 Phase 4: Kernel-side trace service
//
// This module provides a kernel-side trace service that manages domain-scoped
// trace buffers with capability-based access control. The service integrates
// with the existing TraceRing and TraceCapTable from trace_cap.rs.

use crate::domain_registry::{DomainId, DomainRegistry, MAX_DOMAINS};
use crate::trace_cap::{TRACE_RIGHT_ADMIN, TRACE_RIGHT_READ, TraceCapTable};
use crate::trace_ring::{DomainTraceRing, RETENTION_BYTES, RETENTION_EVENTS};
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
    /// Deprecated v1 requested size; not the ring's retention capacity.
    pub size: u32,
    /// Current read offset
    pub read_offset: u64,
    /// Current write offset
    pub write_offset: u64,
}

/// Truthful fixed-ring metadata, separate from the deprecated v1 size field.
#[derive(Debug, Clone, Copy)]
pub struct FixedTraceBufferInfo {
    pub domain_id: DomainId,
    pub retention_events: u32,
    pub retention_bytes: u32,
    pub max_delivery_bytes: u32,
    pub read_offset: u64,
    pub write_offset: u64,
}

/// A delivery receipt binds both exact capability generations and the copied range.
#[derive(Clone, Copy)]
pub(crate) struct TraceReadLease {
    id: u64,
    source: Handle,
    pub(crate) destination: Handle,
    pub(crate) destination_offset: u64,
    pub(crate) data_len: u32,
    offset: u64,
    next_offset: u64,
}

/// Completed receipts reconcile their original outcome without revisiting a
/// destination whose lifetime ended after delivery was already acknowledged.
pub(crate) enum TraceDeliveryReceipt {
    Pending(TraceReadLease),
    Committed(u64),
}

/// Trace buffer entry
struct TraceBuffer {
    /// Domain ID this buffer belongs to
    domain_id: DomainId,
    /// Deprecated v1 requested size, retained for compatible legacy metadata.
    size: u32,
    /// Effective whole-event delivery limit, independent of retention capacity.
    max_delivery_bytes: u32,
    /// Whether this buffer is active
    active: bool,
    pending: Option<TraceReadLease>,
    committed: Option<TraceReadLease>,
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
    // Zero retires issuance permanently; IDs survive stream generation changes.
    next_lease_id: u64,
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
            next_lease_id: 1,
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

    /// Create a trace stream using deprecated v1 size admission and metadata.
    /// The actual ring always retains 64 events; use the fixed-capacity API for
    /// truthful capacity metadata and an explicit whole-event delivery limit.
    ///
    /// # Arguments
    /// - `domain_id`: Domain ID to create buffer for
    /// - `size`: Legacy power-of-two size and delivery clamp
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
        self.create_buffer(domain_id, size, false)
    }

    /// Create a fixed 64-event stream with an explicit whole-event delivery limit.
    #[allow(clippy::result_unit_err)]
    pub fn create_fixed_trace_buffer(
        &mut self,
        domain_id: DomainId,
        max_delivery_bytes: u32,
    ) -> Result<(u32, Handle), ()> {
        self.create_buffer(domain_id, max_delivery_bytes, true)
    }

    /// Shared admission preserves v1's legacy size values and validation order.
    fn create_buffer(
        &mut self,
        domain_id: DomainId,
        size: u32,
        fixed: bool,
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

        // V1 size is deprecated metadata and a delivery clamp, not retention.
        // The successor accepts only whole-event limits within actual capacity.
        const EVENT_BYTES: u32 = core::mem::size_of::<Event>() as u32;
        let valid = if fixed {
            (EVENT_BYTES..=RETENTION_BYTES).contains(&size) && size.is_multiple_of(EVENT_BYTES)
        } else {
            size >= EVENT_BYTES && size.is_power_of_two()
        };
        if !valid {
            return Ok((TRACE_STATUS_INVALID_SIZE, Handle::INVALID));
        }
        let max_delivery_bytes = size.min(RETENTION_BYTES) / EVENT_BYTES * EVENT_BYTES;

        // Allocate trace capability with READ rights
        let cap_index = self
            .cap_table
            .allocate(domain_id, TRACE_RIGHT_READ | TRACE_RIGHT_ADMIN)
            .map_err(|_| ())?;

        // A new capability generation must not observe an old stream's events.
        self.trace_ring.reset_buffer(domain_id);
        // Create trace buffer entry
        self.buffers[idx] = Some(TraceBuffer {
            domain_id,
            size,
            max_delivery_bytes,
            active: true,
            pending: None,
            committed: None,
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

    /// Resolve only a live, exact trace capability without inspecting buffer state.
    pub(crate) fn cap_domain(&self, handle: Handle) -> Option<DomainId> {
        if handle.kind != kernel_api::cap::HandleKind::Trace
            || !kernel_api::cap::is_canonical_generation(handle.generation)
        {
            return None;
        }
        let cap = self.cap_table.get(handle.index as usize)?;
        (cap.in_use && cap.generation == handle.generation).then_some(cap.domain_id)
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

    /// Validate READ authority without inspecting any trace cursor or event data.
    pub(crate) fn readable_domain(&self, source: Handle) -> Result<DomainId, u32> {
        let domain = self
            .cap_domain(source)
            .ok_or(TRACE_STATUS_INVALID_CAPABILITY)?;
        if !self
            .cap_table
            .check_right(source.index as usize, TRACE_RIGHT_READ)
        {
            return Err(TRACE_STATUS_PERMISSION_DENIED);
        }
        if self.buffers[domain as usize]
            .as_ref()
            .is_none_or(|b| !b.active)
        {
            return Err(TRACE_STATUS_BUFFER_NOT_FOUND);
        }
        Ok(domain)
    }

    /// Copy initialized events to an exclusively held, prevalidated Shmem view.
    /// No cursor advances until the exact receipt is explicitly committed.
    pub(crate) fn prepare_delivery(
        &mut self,
        source: Handle,
        destination: Handle,
        destination_offset: u64,
        offset: u64,
        length: u32,
        out: &mut [u8],
    ) -> Result<(u64, u64, u32), u32> {
        let domain = self.readable_domain(source)?;
        const EVENT_BYTES: usize = 24;
        if length > 1536
            || !(length as usize).is_multiple_of(EVENT_BYTES)
            || length as usize > out.len()
        {
            return Err(TRACE_STATUS_INVALID_SIZE);
        }
        let limit = self.buffers[domain as usize]
            .as_ref()
            .unwrap()
            .max_delivery_bytes as usize;
        let count = (length as usize).min(limit) / EVENT_BYTES;
        let mut events = [Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 64];
        let actual = self
            .trace_ring
            .peek_at(domain, offset, &mut events[..count])
            .ok_or(TRACE_STATUS_INVALID_OFFSET)?;
        if actual == 0 {
            // Zero-length and empty reads neither replace nor issue a receipt.
            return Ok((0, offset, 0));
        }
        if self.next_lease_id == 0 {
            return Err(TRACE_STATUS_INVALID_SIZE);
        }
        let data_len = (actual * EVENT_BYTES) as u32;
        let next_offset = offset
            .checked_add(data_len as u64)
            .ok_or(TRACE_STATUS_INVALID_OFFSET)?;
        for (event, bytes) in events[..actual]
            .iter()
            .zip(out.chunks_exact_mut(EVENT_BYTES))
        {
            bytes[..4].copy_from_slice(&event.tag.to_le_bytes());
            bytes[4..8].fill(0);
            bytes[8..16].copy_from_slice(&event.arg0.to_le_bytes());
            bytes[16..24].copy_from_slice(&event.arg1.to_le_bytes());
        }
        let id = self.next_lease_id;
        self.next_lease_id = id.checked_add(1).unwrap_or(0);
        self.buffers[domain as usize].as_mut().unwrap().pending = Some(TraceReadLease {
            id,
            source,
            destination,
            destination_offset,
            data_len,
            offset,
            next_offset,
        });
        Ok((id, next_offset, data_len))
    }

    /// Resolve a receipt only after live source and READ-right validation.
    /// A pending receipt still needs live destination validation before commit;
    /// the last completed receipt only reconciles its original outcome.
    pub(crate) fn delivery_receipt(
        &self,
        source: Handle,
        id: u64,
    ) -> Result<TraceDeliveryReceipt, u32> {
        let domain = self.readable_domain(source)?;
        let buffer = self.buffers[domain as usize].as_ref().unwrap();
        if let Some(committed) = buffer.committed {
            if committed.id == id && committed.source == source && id != 0 {
                return Ok(TraceDeliveryReceipt::Committed(committed.next_offset));
            }
        }
        buffer
            .pending
            .filter(|receipt| receipt.id == id && receipt.source == source && id != 0)
            .map(TraceDeliveryReceipt::Pending)
            .ok_or(TRACE_STATUS_INVALID_CAPABILITY)
    }

    pub(crate) fn commit_delivery(&mut self, source: Handle, id: u64) -> Result<u64, u32> {
        let domain = self.readable_domain(source)?;
        let buffer = self.buffers[domain as usize].as_mut().unwrap();
        if let Some(committed) = buffer.committed {
            if committed.id == id && committed.source == source && id != 0 {
                return Ok(committed.next_offset);
            }
        }
        let pending = buffer
            .pending
            .filter(|receipt| receipt.id == id && receipt.source == source && id != 0)
            .ok_or(TRACE_STATUS_INVALID_CAPABILITY)?;
        if !self
            .trace_ring
            .commit_at(domain, pending.offset, pending.data_len)
        {
            return Err(TRACE_STATUS_INVALID_OFFSET);
        }
        buffer.pending = None;
        buffer.committed = Some(pending);
        Ok(pending.next_offset)
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

        // Return complete events only. A sub-event request is denied without
        // consuming data; zero-length requests are harmless at the current cursor.
        const EVENT_BYTES: usize = core::mem::size_of::<Event>();
        const _: [(); 24] = [(); EVENT_BYTES];
        // Preserve v1's short-read rejection before applying the effective
        // whole-event clamp shared with fixed-capacity streams.
        let read_len = (length as usize).min(out.len()).min(buffer.size as usize);
        if read_len > 0 && read_len < EVENT_BYTES {
            return Ok((TRACE_STATUS_INVALID_SIZE, 0));
        }

        // Read from trace ring buffer
        let mut events = [Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 64]; // Max 64 events per read

        let count =
            (read_len.min(buffer.max_delivery_bytes as usize) / EVENT_BYTES).min(events.len());
        let Some(event_count) = self
            .trace_ring
            .read_at(domain_id, offset, &mut events[..count])
        else {
            return Ok((TRACE_STATUS_INVALID_OFFSET, 0));
        };
        // Preserve the supported 24-byte Event layout with initialized padding;
        // never expose Rust struct padding through a raw byte slice.
        for (event, bytes) in events[..event_count]
            .iter()
            .zip(out.chunks_exact_mut(EVENT_BYTES))
        {
            bytes[..4].copy_from_slice(&event.tag.to_le_bytes());
            bytes[4..8].fill(0);
            bytes[8..16].copy_from_slice(&event.arg0.to_le_bytes());
            bytes[16..24].copy_from_slice(&event.arg1.to_le_bytes());
        }
        Ok((TRACE_STATUS_OK, (event_count * EVENT_BYTES) as u32))
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

        let (read_offset, write_offset) = self.trace_ring.positions(domain_id);
        let info = TraceBufferInfo {
            domain_id: buffer.domain_id,
            size: buffer.size,
            read_offset,
            write_offset,
        };

        Ok((TRACE_STATUS_OK, info))
    }

    /// Query actual retention and delivery limits for either stream version.
    #[allow(clippy::result_unit_err)]
    pub fn get_fixed_trace_info(
        &mut self,
        trace_cap: Handle,
    ) -> Result<(u32, FixedTraceBufferInfo), ()> {
        let empty = FixedTraceBufferInfo {
            domain_id: 0,
            retention_events: 0,
            retention_bytes: 0,
            max_delivery_bytes: 0,
            read_offset: 0,
            write_offset: 0,
        };
        let domain_id = match self.readable_domain(trace_cap) {
            Ok(domain_id) => domain_id,
            Err(status) => return Ok((status, empty)),
        };
        let Some(buffer) = self.buffers[domain_id as usize]
            .as_ref()
            .filter(|b| b.active)
        else {
            return Ok((TRACE_STATUS_BUFFER_NOT_FOUND, empty));
        };
        let (read_offset, write_offset) = self.trace_ring.positions(domain_id);
        Ok((
            TRACE_STATUS_OK,
            FixedTraceBufferInfo {
                domain_id,
                retention_events: RETENTION_EVENTS,
                retention_bytes: RETENTION_BYTES,
                max_delivery_bytes: buffer.max_delivery_bytes,
                read_offset,
                write_offset,
            },
        ))
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

    /// Gate-only fixture access; absent from the default runtime.
    #[cfg(any(test, feature = "test_protocols"))]
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

#[cfg(test)]
mod review_stream {
    use super::*;
    fn service() -> (TraceService, Handle) {
        let mut s = TraceService::new();
        s.init_kernel();
        let (_, cap) = s.create_trace_buffer(0, 4096).unwrap();
        (s, cap)
    }
    #[test]
    fn review_trace_short_and_zero_reads_preserve_remaining_events() {
        let (mut s, cap) = service();
        s.trace_ring_mut().emit(0, 10, 20, 30);
        s.trace_ring_mut().emit(0, 11, 21, 31);
        assert_eq!(
            s.read_trace(cap, 0, 0, &mut []).unwrap(),
            (TRACE_STATUS_OK, 0)
        );
        assert_eq!(
            s.read_trace(cap, 0, 1, &mut [0u8; 1]).unwrap(),
            (TRACE_STATUS_INVALID_SIZE, 0)
        );
        let mut out = [0u8; 24];
        assert_eq!(
            s.read_trace(cap, 0, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        assert_eq!(u32::from_le_bytes(out[..4].try_into().unwrap()), 10);
        assert_eq!(&out[4..8], &[0; 4]);
        assert_eq!(
            s.read_trace(cap, 24, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        assert_eq!(u32::from_le_bytes(out[..4].try_into().unwrap()), 11);
        s.trace_ring_mut().emit(0, 12, 22, 32);
        assert_eq!(
            s.read_trace(cap, 48, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
    }
    #[test]
    fn review_trace_metadata_and_offsets_follow_actual_stream_and_reopen() {
        let (mut s, cap) = service();
        s.register_domain(1, "other").unwrap();
        let (_, other) = s.create_trace_buffer(1, 4096).unwrap();
        s.trace_ring_mut().emit(1, 55, 0, 0);
        s.trace_ring_mut().emit(0, 10, 20, 30);
        s.trace_ring_mut().emit(0, 11, 21, 31);
        let (_, info) = s.get_trace_info(cap).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (0, 48));
        let mut out = [0u8; 24];
        assert_eq!(
            s.read_trace(cap, 1, 24, &mut out).unwrap(),
            (TRACE_STATUS_INVALID_OFFSET, 0)
        );
        assert_eq!(
            s.read_trace(cap, 0, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        let (_, info) = s.get_trace_info(cap).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (24, 48));
        assert_eq!(
            s.read_trace(cap, 24, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        let (_, info) = s.get_trace_info(cap).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (48, 48));
        assert_eq!(s.destroy_trace_buffer(cap).unwrap(), TRACE_STATUS_OK);
        let (_, next) = s.create_trace_buffer(0, 4096).unwrap();
        assert_eq!(
            s.read_trace(cap, 0, 24, &mut out).unwrap().0,
            TRACE_STATUS_INVALID_CAPABILITY
        );
        let (_, info) = s.get_trace_info(next).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (0, 0));
        let (_, other_info) = s.get_trace_info(other).unwrap();
        assert_eq!((other_info.read_offset, other_info.write_offset), (0, 24));
        assert_eq!(
            s.read_trace(other, 0, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        assert_eq!(u32::from_le_bytes(out[..4].try_into().unwrap()), 55);
        s.trace_ring_mut().emit(0, 12, 22, 32);
        assert_eq!(
            s.read_trace(next, 0, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
    }
    #[test]
    fn review_trace_overflow_requires_resync_and_continues_beyond_capacity() {
        let (mut s, cap) = service();
        let mut out = [0u8; 24];
        for n in 0..70 {
            s.trace_ring_mut().emit(0, n, 0, 0);
        }
        let (_, info) = s.get_trace_info(cap).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (6 * 24, 70 * 24));
        assert_eq!(
            s.read_trace(cap, 0, 24, &mut out).unwrap(),
            (TRACE_STATUS_INVALID_OFFSET, 0)
        );
        assert_eq!(
            s.read_trace(cap, info.read_offset, 24, &mut out).unwrap(),
            (TRACE_STATUS_OK, 24)
        );
        for _ in 0..180 {
            let (_, info) = s.get_trace_info(cap).unwrap();
            s.trace_ring_mut().emit(0, 99, 0, 0);
            let (_, fresh) = s.get_trace_info(cap).unwrap();
            let offset = info.read_offset.max(fresh.read_offset);
            assert_eq!(
                s.read_trace(cap, offset, 24, &mut out).unwrap(),
                (TRACE_STATUS_OK, 24)
            );
        }
    }
}

#[cfg(test)]
mod review_delivery_retirement {
    use super::*;

    #[test]
    fn review_trace_delivery_lease_exhaustion_preserves_last_receipt_and_never_reuses_id() {
        let mut service = TraceService::new();
        service.init_kernel();
        let (_, source) = service.create_trace_buffer(0, 4096).unwrap();
        let destination = Handle {
            kind: kernel_api::cap::HandleKind::Shmem,
            index: 1,
            generation: 1,
        };
        service.trace_ring().emit(0, 1, 2, 3);
        service.next_lease_id = u64::MAX;
        let mut bytes = [0xA5; 24];
        assert_eq!(
            service.prepare_delivery(source, destination, 0, 0, 24, &mut bytes),
            Ok((u64::MAX, 24, 24))
        );
        let delivered = bytes;
        assert_eq!(
            service.prepare_delivery(source, destination, 0, 0, 24, &mut bytes),
            Err(TRACE_STATUS_INVALID_SIZE)
        );
        assert_eq!(bytes, delivered);
        assert_eq!(service.trace_ring().positions(0), (0, 24));
        assert_eq!(service.commit_delivery(source, u64::MAX), Ok(24));
        service.trace_ring().emit(0, 4, 5, 6);
        assert_eq!(
            service.prepare_delivery(source, destination, 0, 24, 24, &mut bytes),
            Err(TRACE_STATUS_INVALID_SIZE)
        );
        assert_eq!(bytes, delivered);
        assert_eq!(service.commit_delivery(source, u64::MAX), Ok(24));
        assert_eq!(service.trace_ring().positions(0), (24, 48));
        assert_eq!(service.destroy_trace_buffer(source).unwrap(), 0);
        let (_, next) = service.create_trace_buffer(0, 4096).unwrap();
        service.trace_ring().emit(0, 7, 8, 9);
        assert_eq!(
            service.prepare_delivery(next, destination, 0, 0, 24, &mut bytes),
            Err(TRACE_STATUS_INVALID_SIZE)
        );
        assert_eq!(
            service.commit_delivery(next, u64::MAX),
            Err(TRACE_STATUS_INVALID_CAPABILITY)
        );
        assert_eq!(service.trace_ring().positions(0), (0, 24));
    }
}

#[cfg(test)]
mod review_fixed_capacity {
    use super::*;

    fn destination() -> Handle {
        Handle {
            kind: kernel_api::cap::HandleKind::Shmem,
            index: 1,
            generation: 1,
        }
    }

    #[test]
    fn review_fixed_trace_metadata_preserves_legacy_values_and_reports_actual_capacity() {
        let mut service = TraceService::new();
        service.init_kernel();
        let (_, legacy) = service.create_trace_buffer(0, 4096).unwrap();
        assert_eq!(service.get_trace_info(legacy).unwrap().1.size, 4096);
        let (status, info) = service.get_fixed_trace_info(legacy).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        assert_eq!(
            (
                info.domain_id,
                info.retention_events,
                info.retention_bytes,
                info.max_delivery_bytes
            ),
            (0, 64, 1536, 1536)
        );
        assert_eq!((info.read_offset, info.write_offset), (0, 0));
        service.destroy_trace_buffer(legacy).unwrap();
        let (status, fixed) = service.create_fixed_trace_buffer(0, 1536).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        assert_ne!(fixed, legacy);
        let (_, info) = service.get_fixed_trace_info(fixed).unwrap();
        assert_eq!(
            (
                info.retention_events,
                info.retention_bytes,
                info.max_delivery_bytes
            ),
            (64, 1536, 1536)
        );
        assert_eq!(
            service.get_fixed_trace_info(legacy).unwrap().0,
            TRACE_STATUS_INVALID_CAPABILITY
        );
        assert_eq!(
            service.get_fixed_trace_info(fixed).unwrap().0,
            TRACE_STATUS_OK
        );
    }

    #[test]
    fn review_fixed_trace_exact_retention_overflow_resync_and_receipt_retry() {
        let mut service = TraceService::new();
        service.init_kernel();
        let (_, source) = service.create_fixed_trace_buffer(0, 1536).unwrap();
        for tag in 0..64 {
            service.trace_ring().emit(0, tag, tag as u64 + 100, 200);
        }
        let (_, info) = service.get_fixed_trace_info(source).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (0, 1536));
        let mut all = [0xA5; 1536];
        let pending = service
            .prepare_delivery(source, destination(), 0, 0, 1536, &mut all)
            .unwrap();
        assert_eq!((pending.1, pending.2), (1536, 1536));
        for (tag, bytes) in all.chunks_exact(24).enumerate() {
            assert_eq!(
                u32::from_le_bytes(bytes[..4].try_into().unwrap()),
                tag as u32
            );
            assert_eq!(&bytes[4..8], &[0; 4]);
            assert_eq!(
                u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
                tag as u64 + 100
            );
        }
        service.trace_ring().emit(0, 64, 164, 200);
        let (_, info) = service.get_fixed_trace_info(source).unwrap();
        assert_eq!((info.read_offset, info.write_offset), (24, 1560));
        assert_eq!(
            service.commit_delivery(source, pending.0),
            Err(TRACE_STATUS_INVALID_OFFSET)
        );
        let before = all;
        assert_eq!(
            service.prepare_delivery(source, destination(), 0, 0, 1536, &mut all),
            Err(TRACE_STATUS_INVALID_OFFSET)
        );
        assert_eq!(all, before);
        let fresh = service
            .prepare_delivery(source, destination(), 0, info.read_offset, 24, &mut all)
            .unwrap();
        assert_eq!(u32::from_le_bytes(all[..4].try_into().unwrap()), 1);
        assert_eq!(service.commit_delivery(source, fresh.0), Ok(48));
        service.trace_ring().emit(0, 65, 165, 200);
        assert_eq!(service.commit_delivery(source, fresh.0), Ok(48));
        assert_eq!(service.trace_ring().positions(0), (48, 1584));
        let next = service
            .prepare_delivery(source, destination(), 0, 48, 24, &mut all)
            .unwrap();
        assert_eq!(u32::from_le_bytes(all[..4].try_into().unwrap()), 2);
        assert_eq!(service.commit_delivery(source, next.0), Ok(72));
    }

    #[test]
    fn review_fixed_trace_short_delivery_retains_full_capacity_and_other_domain_progress() {
        let mut service = TraceService::new();
        service.init_kernel();
        service.register_domain(1, "other").unwrap();
        let (_, short) = service.create_fixed_trace_buffer(0, 24).unwrap();
        let (_, other) = service.create_fixed_trace_buffer(1, 48).unwrap();
        for tag in 0..64 {
            service.trace_ring().emit(0, tag, 0, 0);
        }
        service.trace_ring().emit(1, 99, 0, 0);
        let (_, info) = service.get_fixed_trace_info(short).unwrap();
        assert_eq!(
            (
                info.retention_events,
                info.retention_bytes,
                info.max_delivery_bytes
            ),
            (64, 1536, 24)
        );
        assert_eq!((info.read_offset, info.write_offset), (0, 1536));
        let mut out = [0xA5; 1536];
        let pending = service
            .prepare_delivery(short, destination(), 0, 0, 1536, &mut out)
            .unwrap();
        assert_eq!((pending.1, pending.2), (24, 24));
        assert!(out[24..].iter().all(|byte| *byte == 0xA5));
        assert_eq!(service.commit_delivery(short, pending.0), Ok(24));
        assert_eq!(service.trace_ring().positions(1), (0, 24));
        let next = service
            .prepare_delivery(other, destination(), 0, 0, 48, &mut out)
            .unwrap();
        assert_eq!(next.2, 24);
        assert_eq!(u32::from_le_bytes(out[..4].try_into().unwrap()), 99);
        assert_eq!(service.commit_delivery(other, next.0), Ok(24));
        assert_eq!(service.trace_ring().positions(0), (24, 1536));
    }

    #[test]
    fn review_fixed_trace_invalid_limits_and_handles_preserve_next_operation() {
        let mut service = TraceService::new();
        service.init_kernel();
        for limit in [0, 1, 23, 25, 32, 1535, 1537, 4096, u32::MAX] {
            assert_eq!(
                service.create_fixed_trace_buffer(0, limit).unwrap(),
                (TRACE_STATUS_INVALID_SIZE, Handle::INVALID)
            );
            assert_eq!(service.trace_ring().positions(0), (0, 0));
        }
        let (status, source) = service.create_fixed_trace_buffer(0, 48).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        service.trace_ring().emit(0, 7, 8, 9);
        assert_eq!(
            service.create_fixed_trace_buffer(0, 24).unwrap(),
            (TRACE_STATUS_BUFFER_EXISTS, Handle::INVALID)
        );
        assert_eq!(
            service.get_fixed_trace_info(Handle::INVALID).unwrap().0,
            TRACE_STATUS_INVALID_CAPABILITY
        );
        let mut stale = source;
        stale.generation += 1;
        assert_eq!(
            service.get_fixed_trace_info(stale).unwrap().0,
            TRACE_STATUS_INVALID_CAPABILITY
        );
        assert_eq!(service.trace_ring().positions(0), (0, 24));
        let mut bytes = [0xA5; 48];
        let delivered = service
            .prepare_delivery(source, destination(), 0, 0, 48, &mut bytes)
            .unwrap();
        assert_eq!(service.commit_delivery(source, delivered.0), Ok(24));
        assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 7);
        service.destroy_trace_buffer(source).unwrap();
        let (status, next) = service.create_fixed_trace_buffer(0, 24).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        service.trace_ring().emit(0, 10, 11, 12);
        assert_eq!(
            service.commit_delivery(source, delivered.0),
            Err(TRACE_STATUS_INVALID_CAPABILITY)
        );
        assert_eq!(
            service.commit_delivery(next, delivered.0),
            Err(TRACE_STATUS_INVALID_CAPABILITY)
        );
        let delivered = service
            .prepare_delivery(next, destination(), 0, 0, 24, &mut bytes)
            .unwrap();
        assert_eq!(service.commit_delivery(next, delivered.0), Ok(24));
        assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 10);
    }
}

#[cfg(test)]
mod review_fixed_capacity_read_right {
    use super::*;
    #[test]
    fn denied_info_exposes_no_metadata_then_restored_right_progresses() {
        let mut service = TraceService::new();
        service.init_kernel();
        let (status, cap) = service.create_fixed_trace_buffer(0, 24).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        service
            .cap_table_mut()
            .revoke_rights(cap.index as usize, TRACE_RIGHT_READ)
            .unwrap();
        let (status, denied) = service.get_fixed_trace_info(cap).unwrap();
        assert_eq!(status, TRACE_STATUS_PERMISSION_DENIED);
        assert_eq!(
            (
                denied.retention_events,
                denied.retention_bytes,
                denied.max_delivery_bytes,
                denied.read_offset,
                denied.write_offset
            ),
            (0, 0, 0, 0, 0)
        );
        service
            .cap_table_mut()
            .grant_rights(cap.index as usize, TRACE_RIGHT_READ)
            .unwrap();
        let (status, info) = service.get_fixed_trace_info(cap).unwrap();
        assert_eq!(status, TRACE_STATUS_OK);
        assert_eq!(
            (
                info.retention_events,
                info.retention_bytes,
                info.max_delivery_bytes
            ),
            (64, 1536, 24)
        );
    }
}
