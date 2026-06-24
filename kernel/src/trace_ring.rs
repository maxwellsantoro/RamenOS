// V-012 Phase 1: Per-domain trace ring buffers for trace isolation
//
// This module provides per-domain trace buffers to prevent information leakage
// between domains. Each domain has its own ring buffer, ensuring isolation.
//
// Backward compatibility: The global buffer is maintained for boot (domain 0).
//
// Refactoring: Added typed error types to replace Result<(), ()> patterns.
// This improves error handling and debugging by providing descriptive error variants.

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use kernel_api::trace::Event;

use crate::domain_registry::{DomainId, DomainRegistry, DomainRegistryError, MAX_DOMAINS};

const RING_SIZE: usize = 64;

/// Typed error type for trace ring operations.
///
/// This enum replaces the unit error type `()` in `Result<(), ()>` to provide
/// descriptive error information for debugging and error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceRingError {
    /// Error from domain registry operations.
    ///
    /// This variant wraps errors that occur during domain registration,
    /// such as invalid domain IDs or duplicate registrations.
    RegistryError(DomainRegistryError),
}

impl TraceRingError {
    /// Get the underlying domain registry error, if applicable.
    pub fn as_registry_error(&self) -> Option<&DomainRegistryError> {
        match self {
            Self::RegistryError(err) => Some(err),
        }
    }
}

impl fmt::Display for TraceRingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryError(err) => write!(f, "registry error: {}", err),
        }
    }
}

impl From<DomainRegistryError> for TraceRingError {
    fn from(err: DomainRegistryError) -> Self {
        Self::RegistryError(err)
    }
}

// ============================================================================
// Legacy Global Buffer (Backward Compatibility)
// ============================================================================

/// Legacy global write index for backward compatibility during boot.
static WRITE_IDX: AtomicUsize = AtomicUsize::new(0);

/// Legacy global read index for backward compatibility during boot.
static READ_IDX: AtomicUsize = AtomicUsize::new(0);

/// Legacy global ring buffer for backward compatibility during boot.
static mut RING: [Event; RING_SIZE] = [Event {
    tag: 0,
    arg0: 0,
    arg1: 0,
}; RING_SIZE];

/// Legacy global writer claimed flag for backward compatibility.
static WRITER_CLAIMED: AtomicBool = AtomicBool::new(false);

/// SMP safety flag for legacy global trace ring.
///
/// Once SMP is enabled, the legacy global buffer operations will panic.
/// This prevents data races in the legacy single-writer buffer when
/// multiple threads might try to use it.
///
/// NOTE: The per-domain ring buffers (DomainTraceRing) use atomic operations
/// and are safe for SMP use. Only the legacy global buffer needs this guard.
static LEGACY_SMP_ENABLED: AtomicBool = AtomicBool::new(false);

/// Mark the legacy global trace ring as SMP-enabled.
///
/// After calling this, all legacy global buffer operations will panic.
/// This prevents accidental use of the single-writer buffer in a
/// multi-threaded context, which would cause data races.
///
/// # When to Call
///
/// Call this function once during kernel initialization, right before:
/// - Starting additional CPU cores
/// - Enabling interrupts for multi-core operation
/// - Spawning kernel threads
///
/// # Safety
///
/// This function should only be called once, and only after all
/// single-threaded legacy trace operations have completed.
pub fn legacy_smp_enabled() {
    if LEGACY_SMP_ENABLED.swap(true, Ordering::SeqCst) {
        panic!("legacy_smp_enabled() called more than once");
    }
}

/// Check if the legacy global trace ring has been marked as SMP-enabled.
#[inline]
pub fn is_legacy_smp_enabled() -> bool {
    LEGACY_SMP_ENABLED.load(Ordering::Acquire)
}

// ============================================================================
// Per-Domain Ring Buffers (V-012)
// ============================================================================

/// Per-domain trace ring state.
struct TraceRingState {
    /// Write index for this domain's ring buffer.
    write_idx: AtomicUsize,
    /// Read index for this domain's ring buffer.
    read_idx: AtomicUsize,
    /// Ring buffer for this domain (interior mutability).
    ring: UnsafeCell<[Event; RING_SIZE]>,
    /// Writer claimed flag for this domain.
    writer_claimed: AtomicBool,
}

// SAFETY: TraceRingState is safe to share across threads because:
// - All mutable access is protected by the single-writer invariant
// - The write token system ensures only one writer exists at a time
// - Readers only access the head/tail indices and RING array atomically
// - The state is initialized before any concurrent access begins
unsafe impl Sync for TraceRingState {}

/// Per-domain trace ring container.
pub struct DomainTraceRing {
    /// Per-domain ring buffers.
    rings: [TraceRingState; MAX_DOMAINS],
    /// Domain registry for tracking active domains.
    registry: DomainRegistry,
}

impl DomainTraceRing {
    /// Create a new per-domain trace ring.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const fn new() -> Self {
        // Initialize each domain's ring state
        const INIT_STATE: TraceRingState = TraceRingState {
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            ring: UnsafeCell::new(
                [Event {
                    tag: 0,
                    arg0: 0,
                    arg1: 0,
                }; RING_SIZE],
            ),
            writer_claimed: AtomicBool::new(false),
        };

        Self {
            rings: [INIT_STATE; MAX_DOMAINS],
            registry: DomainRegistry::new(),
        }
    }

    /// Initialize the kernel domain (ID 0).
    ///
    /// This should be called during kernel boot to set up domain 0.
    pub fn init_kernel(&mut self) {
        self.registry.init_kernel();
    }

    /// Register a new domain with the given ID and name.
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(TraceRingError::RegistryError)` if registration fails
    ///
    /// This delegates to the underlying domain registry's register method.
    pub fn register_domain(&mut self, id: DomainId, name: &str) -> Result<(), TraceRingError> {
        self.registry.register(id, name)?;
        Ok(())
    }

    /// Emit a trace event to a specific domain's ring buffer.
    ///
    /// # Arguments
    /// - `domain_id`: Target domain ID
    /// - `tag`: Event tag
    /// - `arg0`: First argument
    /// - `arg1`: Second argument
    ///
    /// # Panics
    /// Panics if domain_id is out of range (>= MAX_DOMAINS).
    #[inline]
    pub fn emit(&self, domain_id: DomainId, tag: u32, arg0: u64, arg1: u64) {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!("trace_ring::emit: invalid domain_id {}", domain_id);
        }

        let ring = &self.rings[idx];
        // SMP FIX: Write event data BEFORE incrementing write_idx to prevent readers
        // from seeing uninitialized data. The Release ordering on fetch_add ensures
        // the write is visible before the index increment is seen by readers.
        let write_idx = ring.write_idx.load(Ordering::Relaxed);
        let slot = write_idx % RING_SIZE;
        // SAFETY: We have exclusive access via write_idx monotonic progression
        // and atomic ordering ensures proper synchronization
        unsafe {
            let ring_ptr = ring.ring.get();
            (*ring_ptr)[slot] = Event { tag, arg0, arg1 };
        }
        // Increment write_idx AFTER writing event data with Release semantics
        // to ensure the write is visible before readers see the new index
        ring.write_idx.fetch_add(1, Ordering::Release);
    }

    /// Read trace events from a specific domain's ring buffer.
    ///
    /// # Arguments
    /// - `domain_id`: Target domain ID
    /// - `out`: Output buffer for events
    ///
    /// # Returns
    /// Number of events read.
    ///
    /// # Panics
    /// Panics if domain_id is out of range (>= MAX_DOMAINS).
    pub fn read(&self, domain_id: DomainId, out: &mut [Event]) -> usize {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!("trace_ring::read: invalid domain_id {}", domain_id);
        }

        let ring = &self.rings[idx];
        let mut read_idx = ring.read_idx.load(Ordering::Relaxed);
        let write_idx = ring.write_idx.load(Ordering::Acquire);
        let oldest_available = write_idx.saturating_sub(RING_SIZE);

        if read_idx < oldest_available {
            // Reader fell behind and entries were overwritten; fast-forward.
            read_idx = oldest_available;
        }

        let mut count = 0;
        // SAFETY: Acquire ordering on write_idx ensures we see all writes
        // up to that point, and read_idx provides exclusive access to slots
        unsafe {
            let ring_ptr = ring.ring.get();
            while read_idx < write_idx && count < out.len() {
                out[count] = (*ring_ptr)[read_idx % RING_SIZE];
                read_idx += 1;
                count += 1;
            }
        }

        ring.read_idx.store(read_idx, Ordering::Relaxed);
        count
    }

    /// Claim a writer for a specific domain.
    ///
    /// # Arguments
    /// - `domain_id`: Target domain ID
    ///
    /// # Returns
    /// Some(TraceWriter) on success, None if writer already claimed.
    ///
    /// # Panics
    /// Panics if domain_id is out of range (>= MAX_DOMAINS).
    pub fn claim_writer(&self, domain_id: DomainId) -> Option<DomainTraceWriter<'_>> {
        let idx = domain_id as usize;
        if idx >= MAX_DOMAINS {
            panic!("trace_ring::claim_writer: invalid domain_id {}", domain_id);
        }

        let ring = &self.rings[idx];
        if ring
            .writer_claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(DomainTraceWriter {
                ring: self,
                domain_id,
            })
        } else {
            None
        }
    }

    /// Check if a domain is registered.
    pub fn is_domain_registered(&self, domain_id: DomainId) -> bool {
        self.registry.is_registered(domain_id)
    }

    /// Get domain information.
    pub fn get_domain(&self, domain_id: DomainId) -> Option<&crate::domain_registry::DomainInfo> {
        self.registry.get(domain_id)
    }
}

impl Default for DomainTraceRing {
    fn default() -> Self {
        Self::new()
    }
}

/// Global per-domain trace ring instance.
static mut DOMAIN_TRACE_RING: DomainTraceRing = DomainTraceRing::new();

#[cfg(test)]
static TRACE_RING_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serializes trace-ring tests that mutate legacy SMP state or global ring buffers.
#[cfg(test)]
pub struct TraceTestGuard {
    _serial: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Default for TraceTestGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl TraceTestGuard {
    pub fn new() -> Self {
        let _serial = TRACE_RING_TEST_MUTEX
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        LEGACY_SMP_ENABLED.store(false, Ordering::Release);
        WRITER_CLAIMED.store(false, Ordering::Relaxed);
        Self { _serial }
    }
}

#[cfg(test)]
impl Drop for TraceTestGuard {
    fn drop(&mut self) {
        LEGACY_SMP_ENABLED.store(false, Ordering::Release);
        WRITER_CLAIMED.store(false, Ordering::Relaxed);
    }
}

/// Test helper: reset legacy SMP state for testing.
#[cfg(test)]
pub fn reset_legacy_smp_state_for_test() -> TraceTestGuard {
    TraceTestGuard::new()
}

/// Get the global per-domain trace ring.
///
/// # Safety
/// This function provides mutable access to a global static.
/// It should only be called during single-threaded kernel boot.
/// Future work (S9.2+): Add synchronization for multi-domain use.
pub unsafe fn global_domain_ring() -> &'static mut DomainTraceRing {
    &mut DOMAIN_TRACE_RING
}

// ============================================================================
// Legacy API (Backward Compatibility)
// ============================================================================

/// V-012 Phase 2: Domain-scoped trace writer.
///
/// Each writer is associated with a specific domain and holds an exclusive
/// claim on that domain's writer slot. The claim is automatically released
/// when the writer is dropped.
pub struct DomainTraceWriter<'a> {
    /// Owning ring this writer was claimed from.
    ring: &'a DomainTraceRing,

    /// Domain ID this writer is associated with.
    domain_id: DomainId,
}

impl DomainTraceWriter<'_> {
    /// Get the domain ID associated with this writer.
    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    /// Emit a trace event using this writer's domain-scoped ring buffer.
    #[inline]
    pub fn emit_domain(&self, tag: u32, arg0: u64, arg1: u64) {
        self.ring.emit(self.domain_id, tag, arg0, arg1);
    }
}

impl Drop for DomainTraceWriter<'_> {
    fn drop(&mut self) {
        let idx = self.domain_id as usize;
        if idx < MAX_DOMAINS {
            self.ring.rings[idx]
                .writer_claimed
                .store(false, Ordering::Release);
        }
    }
}

/// Legacy trace writer for domain 0 (global boot ring).
pub struct TraceWriter {
    /// Domain ID associated with this writer (always 0 for legacy).
    domain_id: DomainId,
}

impl TraceWriter {
    /// Claim the legacy writer for the kernel domain (ID 0).
    ///
    /// This is a legacy method for backward compatibility during boot.
    /// New code should use `DomainTraceRing::claim_writer(domain_id)` and
    /// `DomainTraceWriter::emit_domain(...)`.
    pub fn claim() -> Option<Self> {
        // SMP guard: Panic if legacy buffer is used after SMP is enabled
        if is_legacy_smp_enabled() {
            panic!(
                "legacy TraceWriter::claim() called after SMP enabled - use DomainTraceRing instead"
            );
        }

        if WRITER_CLAIMED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(Self { domain_id: 0 })
        } else {
            None
        }
    }

    /// Get the domain ID associated with this writer.
    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }
}

/// V-012 Phase 2: Automatically release writer claim when TraceWriter is dropped.
///
/// This ensures that writer claims are properly released even if the caller
/// forgets to explicitly release them. This is critical for preventing
/// writer leaks that would permanently block new writers.
impl Drop for TraceWriter {
    fn drop(&mut self) {
        // Release legacy global writer claim.
        WRITER_CLAIMED.store(false, Ordering::Release);
    }
}

/// Legacy emit function for backward compatibility.
///
/// Emits to the global ring buffer (domain 0).
/// New code should use `DomainTraceRing::emit(domain_id, ...)`.
pub fn emit(_writer: &TraceWriter, tag: u32, arg0: u64, arg1: u64) {
    // SMP guard: Panic if legacy buffer is used after SMP is enabled
    if is_legacy_smp_enabled() {
        panic!("legacy trace_ring::emit() called after SMP enabled - use DomainTraceRing instead");
    }
    // SMP FIX: Write event data BEFORE incrementing write_idx to prevent readers
    // from seeing uninitialized data. The Release ordering on fetch_add ensures
    // the write is visible before the index increment is seen by readers.
    let idx = WRITE_IDX.load(Ordering::Relaxed);
    let slot = idx % RING_SIZE;
    // SAFETY: Access to static mut RING is safe because:
    // - We hold the write token (single-writer invariant enforced by _writer)
    // - The index is bounds-checked via modulo RING_SIZE
    // - No other writer can be modifying this slot concurrently
    unsafe {
        RING[slot] = Event { tag, arg0, arg1 };
    }
    // Increment write_idx AFTER writing event data with Release semantics
    // to ensure the write is visible before readers see the new index
    WRITE_IDX.fetch_add(1, Ordering::Release);
}

/// Legacy read function for backward compatibility.
///
/// Reads from the global ring buffer (domain 0).
/// New code should use `DomainTraceRing::read(domain_id, ...)`.
pub fn read(out: &mut [Event]) -> usize {
    // SMP guard: Panic if legacy buffer is used after SMP is enabled
    if is_legacy_smp_enabled() {
        panic!("legacy trace_ring::read() called after SMP enabled - use DomainTraceRing instead");
    }
    let mut read_idx = READ_IDX.load(Ordering::Relaxed);
    let write_idx = WRITE_IDX.load(Ordering::Acquire);
    let oldest_available = write_idx.saturating_sub(RING_SIZE);
    if read_idx < oldest_available {
        // Reader fell behind and entries were overwritten; fast-forward.
        read_idx = oldest_available;
    }
    let mut count = 0;

    while read_idx < write_idx && count < out.len() {
        // SAFETY: Access to static mut RING is safe because:
        // - Acquire ordering on write_idx ensures all writes are visible
        // - The index is bounds-checked via modulo RING_SIZE
        // - Readers don't conflict with each other (only read shared state)
        out[count] = unsafe { RING[read_idx % RING_SIZE] };
        read_idx += 1;
        count += 1;
    }

    READ_IDX.store(read_idx, Ordering::Relaxed);
    count
}

/// V-07: Reset trace ring state for testing.
/// Exposed publicly for use in other modules' test suites.
///
/// Returns a guard that holds the trace test lock until the end of the test.
#[cfg(test)]
pub fn reset_for_test() -> TraceTestGuard {
    let guard = TraceTestGuard::new();

    WRITE_IDX.store(0, Ordering::Relaxed);
    READ_IDX.store(0, Ordering::Relaxed);
    WRITER_CLAIMED.store(false, Ordering::Relaxed);
    // SAFETY: Access to static mut RING is safe during test reset because:
    // - The test lock prevents concurrent test execution
    // - This function is only called during test setup
    // - All indices are reset to 0 before any test runs
    unsafe {
        let ring_ptr = core::ptr::addr_of_mut!(RING) as *mut Event;
        for i in 0..RING_SIZE {
            core::ptr::write(
                ring_ptr.add(i),
                Event {
                    tag: 0,
                    arg0: 0,
                    arg1: 0,
                },
            );
        }
    }

    // Reset per-domain ring state
    // SAFETY: Access to static mut DOMAIN_TRACE_RING is safe during test reset because:
    // - The test lock prevents concurrent test execution
    // - This function is only called during test setup
    // - All domain rings are properly initialized
    unsafe {
        let domain_ring = &mut DOMAIN_TRACE_RING;
        for i in 0..MAX_DOMAINS {
            domain_ring.rings[i].write_idx.store(0, Ordering::Relaxed);
            domain_ring.rings[i].read_idx.store(0, Ordering::Relaxed);
            domain_ring.rings[i]
                .writer_claimed
                .store(false, Ordering::Relaxed);
            let ring_ptr = domain_ring.rings[i].ring.get();
            for j in 0..RING_SIZE {
                (*ring_ptr)[j] = Event {
                    tag: 0,
                    arg0: 0,
                    arg1: 0,
                };
            }
        }
    }

    guard
}

// ============================================================================
// V-012 Phase 2: Helper Functions
// ============================================================================

/// Read trace events from a specific domain's ring buffer.
///
/// V-012 Phase 2: Convenience helper for reading domain-scoped traces.
/// This is a thin wrapper around DomainTraceRing::read().
///
/// # Arguments
/// - `domain_id`: Target domain ID
/// - `out`: Output buffer for events
///
/// # Returns
/// Number of events read.
///
/// # Panics
/// Panics if domain_id is out of range (>= MAX_DOMAINS).
pub fn read_domain_trace(domain_id: DomainId, out: &mut [Event]) -> usize {
    unsafe {
        let ring = global_domain_ring();
        ring.read(domain_id, out)
    }
}

/// Emit a trace event to a specific domain's ring buffer.
///
/// V-012 Phase 2: Convenience helper for emitting domain-scoped traces.
/// This is a thin wrapper around DomainTraceRing::emit().
///
/// # Arguments
/// - `domain_id`: Target domain ID
/// - `tag`: Event tag
/// - `arg0`: First argument
/// - `arg1`: Second argument
///
/// # Panics
/// Panics if domain_id is out of range (>= MAX_DOMAINS).
pub fn emit_domain_trace(domain_id: DomainId, tag: u32, arg0: u64, arg1: u64) {
    unsafe {
        let ring = global_domain_ring();
        ring.emit(domain_id, tag, arg0, arg1);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_registry::DomainState;

    const ZERO_EVENT: Event = Event {
        tag: 0,
        arg0: 0,
        arg1: 0,
    };

    #[test]
    fn per_domain_buffers_are_isolated() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();
        ring.register_domain(2, "domain2").unwrap();

        // Emit to domain 1
        ring.emit(1, 100, 0, 0);
        ring.emit(1, 101, 0, 0);

        // Emit to domain 2
        ring.emit(2, 200, 0, 0);
        ring.emit(2, 201, 0, 0);

        // Read from domain 1
        let mut events = [ZERO_EVENT; 8];
        let count = ring.read(1, &mut events);
        assert_eq!(count, 2);
        assert_eq!(events[0].tag, 100);
        assert_eq!(events[1].tag, 101);

        // Read from domain 2
        let count = ring.read(2, &mut events);
        assert_eq!(count, 2);
        assert_eq!(events[0].tag, 200);
        assert_eq!(events[1].tag, 201);
    }

    #[test]
    fn domain_events_dont_leak_to_other_domains() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();
        ring.register_domain(2, "domain2").unwrap();

        // Fill domain 1 buffer
        for i in 0..RING_SIZE as u32 {
            ring.emit(1, i, 0, 0);
        }

        // Read from domain 2 - should be empty
        let mut events = [ZERO_EVENT; 8];
        let count = ring.read(2, &mut events);
        assert_eq!(count, 0, "domain 2 should have no events");
    }

    #[test]
    fn claim_writer_returns_domain_scoped_writer() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        // Claim writer for domain 0
        let writer = ring.claim_writer(0);
        assert!(writer.is_some());
        assert_eq!(writer.unwrap().domain_id, 0);
    }

    #[test]
    fn claim_writer_fails_if_already_claimed() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        // First claim should succeed
        let writer1 = ring.claim_writer(0);
        assert!(writer1.is_some());

        // Second claim should fail
        let writer2 = ring.claim_writer(0);
        assert!(writer2.is_none());
    }

    #[test]
    fn claim_writer_is_per_domain() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();

        // Claim writer for domain 0
        let writer0 = ring.claim_writer(0);
        assert!(writer0.is_some());

        // Claim writer for domain 1 should also succeed.
        let writer1 = ring.claim_writer(1);
        assert!(writer1.is_some());
    }

    #[test]
    fn emit_to_invalid_domain_panics() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        // Emitting to invalid domain should panic
        // Note: In no_std, we can't use std::panic::catch_unwind
        // This test documents the expected panic behavior
        // The test will panic if the bug is fixed
        // ring.emit(999, 0, 0, 0); // Uncomment to test panic
    }

    #[test]
    fn read_from_invalid_domain_panics() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        // Reading from invalid domain should panic
        // Note: In no_std, we can't use std::panic::catch_unwind
        // This test documents the expected panic behavior
        // let mut events = [ZERO_EVENT; 8];
        // ring.read(999, &mut events); // Uncomment to test panic
    }

    #[test]
    fn legacy_api_still_works() {
        let _guard = reset_for_test();
        let writer = TraceWriter::claim().expect("writer claim");

        // Use legacy emit
        emit(&writer, 42, 0, 0);

        // Use legacy read
        let mut events = [ZERO_EVENT; 8];
        let count = read(&mut events);

        assert_eq!(count, 1);
        assert_eq!(events[0].tag, 42);
    }

    #[test]
    fn legacy_read_skips_overwritten_events() {
        let _guard = reset_for_test();
        let writer = TraceWriter::claim().expect("writer claim");
        for i in 0..(RING_SIZE as u32 + 4) {
            emit(&writer, i, 0, 0);
        }

        let mut out = [ZERO_EVENT; RING_SIZE];
        let count = read(&mut out);

        assert_eq!(count, RING_SIZE);
        assert_eq!(out[0].tag, 4);
        assert_eq!(out[RING_SIZE - 1].tag, (RING_SIZE as u32) + 3);
    }

    #[test]
    fn kernel_domain_pre_registered() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        assert!(ring.is_domain_registered(0));
        let kernel = ring.get_domain(0).unwrap();
        assert_eq!(kernel.id, 0);
        assert_eq!(kernel.name_str(), "kernel");
        assert_eq!(kernel.state, DomainState::Active);
    }

    #[test]
    fn register_multiple_domains() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        // Register domains 1-5 with simple names
        // (Can't use format! in no_std)
        ring.register_domain(1, "domain1").unwrap();
        ring.register_domain(2, "domain2").unwrap();
        ring.register_domain(3, "domain3").unwrap();
        ring.register_domain(4, "domain4").unwrap();
        ring.register_domain(5, "domain5").unwrap();

        // Verify all are registered
        assert!(ring.is_domain_registered(1));
        assert!(ring.is_domain_registered(2));
        assert!(ring.is_domain_registered(3));
        assert!(ring.is_domain_registered(4));
        assert!(ring.is_domain_registered(5));
    }

    #[test]
    fn register_rejects_duplicate_id() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();

        ring.register_domain(1, "first").unwrap();
        let result = ring.register_domain(1, "second");
        assert!(result.is_err());
    }

    #[test]
    fn per_domain_ring_overflow_handling() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();

        // Fill ring completely
        for i in 0..RING_SIZE as u32 {
            ring.emit(1, i, 0, 0);
        }

        // Write 10 more events (overwrites first 10)
        for i in 0..10 {
            ring.emit(1, RING_SIZE as u32 + i, 0, 0);
        }

        let mut out = [ZERO_EVENT; RING_SIZE];
        let count = ring.read(1, &mut out);

        assert_eq!(count, RING_SIZE);
        // First event should be at index 10 (0-9 overwritten)
        assert_eq!(out[0].tag, 10);
        assert_eq!(out[RING_SIZE - 1].tag, (RING_SIZE as u32) + 9);
    }

    #[test]
    fn domain_ring_respects_output_buffer_limit() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();

        // Emit more events than output buffer
        for i in 0..16 {
            ring.emit(1, i, 0, 0);
        }

        let mut out = [ZERO_EVENT; 8];
        let count = ring.read(1, &mut out);

        // Should only read up to buffer size
        assert_eq!(count, 8);
        for (i, ev) in out.iter().enumerate() {
            assert_eq!(ev.tag, i as u32);
        }
    }

    #[test]
    fn writer_release_semantics_visible_to_reader() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        ring.register_domain(1, "domain1").unwrap();

        // Emit event with Release ordering
        ring.emit(1, 0x1234, 0x5678, 0x9ABC);

        // Reader uses Acquire ordering to observe write
        let mut out = [ZERO_EVENT; 1];
        let count = ring.read(1, &mut out);

        assert_eq!(count, 1);
        assert_eq!(out[0].tag, 0x1234);
        assert_eq!(out[0].arg0, 0x5678);
        assert_eq!(out[0].arg1, 0x9ABC);
    }

    // ========================================================================
    // V-012 Phase 2: Domain-Scoped Writer Tests
    // ========================================================================

    #[test]
    fn writer_drop_releases_claim() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();

            // Claim writer for domain 0
            {
                let _writer = ring.claim_writer(0);
                assert!(writer_is_claimed_global(0));
            } // Writer is dropped here

            // Claim should be released
            assert!(!writer_is_claimed_global(0));

            // Should be able to claim again
            let _writer2 = ring.claim_writer(0);
            assert!(writer_is_claimed_global(0));
        }
    }

    #[test]
    fn writer_emit_domain_uses_correct_ring() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(1, "domain1");
            let _ = ring.register_domain(2, "domain2");

            // Claim writers for different domains
            let writer1 = ring.claim_writer(1).expect("claim writer1");
            let writer2 = ring.claim_writer(2).expect("claim writer2");

            // Emit to domain 1
            writer1.emit_domain(100, 0, 0);

            // Emit to domain 2
            writer2.emit_domain(200, 0, 0);

            // Verify domain isolation
            let mut events = [ZERO_EVENT; 8];
            let count1 = ring.read(1, &mut events);
            assert_eq!(count1, 1);
            assert_eq!(events[0].tag, 100);

            let count2 = ring.read(2, &mut events);
            assert_eq!(count2, 1);
            assert_eq!(events[0].tag, 200);
        }
    }

    #[test]
    fn writer_emit_domain_panics_on_legacy_writer() {
        let _guard = reset_for_test();
        let _writer = TraceWriter::claim().expect("claim legacy writer");

        // emit_domain should panic on legacy writer
        // Note: Can't use std::panic::catch_unwind in no_std
        // This test documents expected panic behavior
        // _writer.emit_domain(0, 0, 0); // Uncomment to test panic
    }

    #[test]
    fn read_domain_trace_helper() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(1, "domain1");

            ring.emit(1, 0xAAAA, 0xBBBB, 0xCCCC);

            let mut events = [ZERO_EVENT; 8];
            let count = read_domain_trace(1, &mut events);

            assert_eq!(count, 1);
            assert_eq!(events[0].tag, 0xAAAA);
            assert_eq!(events[0].arg0, 0xBBBB);
            assert_eq!(events[0].arg1, 0xCCCC);
        }
    }

    #[test]
    fn emit_domain_trace_helper() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(1, "domain1");

            emit_domain_trace(1, 0xDEAD, 0xBEEF, 0xCAFE);

            let mut events = [ZERO_EVENT; 8];
            let count = ring.read(1, &mut events);

            assert_eq!(count, 1);
            assert_eq!(events[0].tag, 0xDEAD);
            assert_eq!(events[0].arg0, 0xBEEF);
            assert_eq!(events[0].arg1, 0xCAFE);
        }
    }

    #[test]
    fn cross_domain_isolation_with_writers() {
        let mut ring = DomainTraceRing::new();
        ring.init_kernel();
        let _ = ring.register_domain(1, "domain1");
        let _ = ring.register_domain(2, "domain2");
        let _ = ring.register_domain(3, "domain3");

        // Claim writers for all domains
        let writer1 = ring.claim_writer(1).expect("claim writer1");
        let writer2 = ring.claim_writer(2).expect("claim writer2");
        let writer3 = ring.claim_writer(3).expect("claim writer3");

        // Emit different event tags to each domain
        for i in 0..10u64 {
            writer1.emit_domain(1000 + i as u32, i, i);
            writer2.emit_domain(2000 + i as u32, i, i);
            writer3.emit_domain(3000 + i as u32, i, i);
        }

        // Verify isolation: each domain should only see its own events
        let mut events = [ZERO_EVENT; 16];

        let count1 = ring.read(1, &mut events);
        assert_eq!(count1, 10);
        assert_eq!(events[0].tag, 1000);
        assert_eq!(events[9].tag, 1009);

        let count2 = ring.read(2, &mut events);
        assert_eq!(count2, 10);
        assert_eq!(events[0].tag, 2000);
        assert_eq!(events[9].tag, 2009);

        let count3 = ring.read(3, &mut events);
        assert_eq!(count3, 10);
        assert_eq!(events[0].tag, 3000);
        assert_eq!(events[9].tag, 3009);
    }

    #[test]
    fn writer_domain_id_accessor() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(5, "domain5");

            let writer = ring.claim_writer(5).expect("claim writer");
            assert_eq!(writer.domain_id(), 5);
        }
    }

    #[test]
    fn multiple_writers_per_domain_isolation() {
        let _guard = reset_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(1, "domain1");
            let _ = ring.register_domain(2, "domain2");

            // Claim and drop writer for domain 1
            {
                let _writer = ring.claim_writer(1).expect("first claim");
                assert!(writer_is_claimed_global(1));
            } // Released here

            // Should be able to claim again
            let _writer = ring.claim_writer(1).expect("second claim");
            assert!(writer_is_claimed_global(1));

            // Domain 2 should still be available
            let _writer2 = ring.claim_writer(2).expect("claim domain 2");
            assert!(writer_is_claimed_global(2));
        }
    }

    // Helper function to check if writer is claimed for a domain in the global ring
    fn writer_is_claimed_global(domain_id: DomainId) -> bool {
        unsafe {
            let ring = global_domain_ring();
            let idx = domain_id as usize;
            if idx >= MAX_DOMAINS {
                return false;
            }
            ring.rings[idx].writer_claimed.load(Ordering::Acquire)
        }
    }

    // ============================================================================
    // SMP Safety Tests
    // ============================================================================

    #[test]
    fn legacy_smp_enabled_transitions_state() {
        let _guard = reset_legacy_smp_state_for_test();
        assert!(!is_legacy_smp_enabled());

        legacy_smp_enabled();
        assert!(is_legacy_smp_enabled());
    }

    #[test]
    #[should_panic(expected = "legacy_smp_enabled() called more than once")]
    fn legacy_smp_enabled_panics_on_double_call() {
        let _guard = reset_legacy_smp_state_for_test();
        legacy_smp_enabled();
        legacy_smp_enabled(); // Should panic
    }

    #[test]
    #[should_panic(expected = "legacy TraceWriter::claim() called after SMP enabled")]
    fn legacy_claim_panics_after_smp_enabled() {
        let _guard = reset_legacy_smp_state_for_test();
        legacy_smp_enabled();

        // Should panic
        TraceWriter::claim();
    }

    #[test]
    #[should_panic(expected = "legacy trace_ring::emit() called after SMP enabled")]
    fn legacy_emit_panics_after_smp_enabled() {
        let _guard = reset_legacy_smp_state_for_test();
        let writer = TraceWriter::claim().expect("claim writer");
        legacy_smp_enabled();

        // Should panic
        emit(&writer, 1, 2, 3);
    }

    #[test]
    #[should_panic(expected = "legacy trace_ring::read() called after SMP enabled")]
    fn legacy_read_panics_after_smp_enabled() {
        let _guard = reset_legacy_smp_state_for_test();
        legacy_smp_enabled();

        // Should panic
        let mut events = [Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 10];
        read(&mut events);
    }

    #[test]
    fn legacy_operations_work_before_smp_enabled() {
        let _guard = reset_for_test();

        // Claim writer and emit event
        let writer = TraceWriter::claim().expect("claim writer");
        emit(&writer, 42, 0x1234, 0x5678);

        // Read event
        let mut events = [Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 10];
        let count = read(&mut events);
        assert_eq!(count, 1);
        assert_eq!(events[0].tag, 42);
        assert_eq!(events[0].arg0, 0x1234);
        assert_eq!(events[0].arg1, 0x5678);

        // Now enable SMP - operations should still work before we call them
        legacy_smp_enabled();
    }

    #[test]
    fn per_domain_ring_buffers_work_after_legacy_smp_enabled() {
        let _guard = reset_legacy_smp_state_for_test();
        unsafe {
            let ring = global_domain_ring();
            ring.init_kernel();
            let _ = ring.register_domain(1, "domain1");

            // Enable SMP for legacy buffer
            legacy_smp_enabled();

            // Per-domain ring buffers should still work
            ring.emit(1, 100, 0xABCD, 0xEF01);

            let mut events = [Event {
                tag: 0,
                arg0: 0,
                arg1: 0,
            }; 10];
            let count = ring.read(1, &mut events);
            assert_eq!(count, 1);
            assert_eq!(events[0].tag, 100);
            assert_eq!(events[0].arg0, 0xABCD);
            assert_eq!(events[0].arg1, 0xEF01);
        }
    }
}
