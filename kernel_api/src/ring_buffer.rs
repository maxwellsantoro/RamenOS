//! Ring Buffer v0 - Lock-free SPSC data structure for zero-copy data transfer.
//!
//! This module implements a Single Producer/Single Consumer (SPSC) ring buffer
//! that enables zero-copy data transfer between domains through shared memory.
//!
//! # Architecture
//!
//! - **Producer:** Writes data to the ring buffer
//! - **Consumer:** Reads data from the ring buffer
//! - **Shared Memory:** Header and data region live in shared memory
//! - **Lock-Free:** Uses atomic operations with acquire/release semantics
//!
//! # Memory Layout
//!
//! ```text
//! +----------------------+
//! | RingBufferHeader     |  Fixed size (32 bytes)
//! +----------------------+
//! | producer_head: u64   |  AtomicU64 (producer index)
//! | consumer_head: u64   |  AtomicU64 (consumer index)
//! | capacity: u64        |  Total capacity in bytes
//! | flags: u64           |  Cache mode, state flags
//! +----------------------+  <-- Data starts here
//! | Data...              |  Flexible array member
//! |                      |
//! +----------------------+
//! ```
//!
//! # Index Scheme
//!
//! Both indices are **monotonically increasing** (not modulo capacity):
//! - Avoids wrap-around ambiguity
//! - Simplifies empty/full detection
//! - Mask indices on access: `index % capacity`
//!
//! # Cache Modes
//!
//! - `UNCACHED (0)`: Bypass CPU caches entirely (for MMIO)
//! - `WRITE_COMBINE (1)`: Write merging, no read caching
//! - `WRITE_BACK (2)`: Standard caching (default)

use core::sync::atomic::{AtomicU64, Ordering};

/// Cache mode constants for ring buffer flags field.
pub mod cache_mode {
    /// Bypass CPU caches entirely (for MMIO regions)
    pub const UNCACHED: u64 = 0;
    /// Write merging, no read caching
    pub const WRITE_COMBINE: u64 = 1;
    /// Standard caching (default)
    pub const WRITE_BACK: u64 = 2;
}

/// Ring buffer header stored in shared memory.
///
/// This header contains the atomic indices used for synchronization between
/// producer and consumer domains. The header must be placed at the start of
/// the shared memory region.
///
/// # Memory Layout
///
/// ```text
/// Offset 0:  producer_head (AtomicU64)
/// Offset 8:  consumer_head (AtomicU64)
/// Offset 16: capacity (u64)
/// Offset 24: flags (u64)
/// ```
#[repr(C)]
pub struct RingBufferHeader {
    /// Producer write head (monotonically increasing).
    /// Written by producer, read by consumer.
    pub producer_head: AtomicU64,

    /// Consumer read head (monotonically increasing).
    /// Written by consumer, read by producer.
    pub consumer_head: AtomicU64,

    /// Total capacity of the ring buffer in bytes.
    pub capacity: u64,

    /// Flags and configuration.
    /// - bits 0-7: cache_mode (CACHE_MODE_* constants)
    /// - bits 8-15: reserved
    /// - bit 16: initialized flag
    pub flags: u64,
}

/// Error type for write operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// Insufficient space in buffer.
    NoSpace,
    /// Data size exceeds buffer capacity.
    InvalidSize,
}

/// Error type for read operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// No data available to read.
    Empty,
}

/// Lock-free SPSC ring buffer.
///
/// This structure provides unsafe pointers to the shared memory region
/// containing the ring buffer header and data array.
///
/// # Safety
///
/// The caller must ensure:
/// - `header` points to a valid `RingBufferHeader` in shared memory
/// - `data` points to a valid byte array of at least `capacity` bytes
/// - Only one domain acts as producer (calls `try_write`)
/// - Only one domain acts as consumer (calls `try_read`)
/// - The memory region remains valid for the lifetime of this structure
pub struct RingBuffer {
    /// Pointer to ring buffer header in shared memory.
    header: *mut RingBufferHeader,
    /// Pointer to data array in shared memory.
    data: *mut u8,
    /// Cached capacity from shared header (immutable after construction).
    ///
    /// **SECURITY:** This value is read once from shared memory at
    /// construction time and never read from shared memory again.
    /// Prevents TOCTOU where attacker modifies header.capacity.
    capacity: usize,
}

// SAFETY: Send and Sync are safe because:
// - The ring buffer uses atomic operations for synchronization
// - Producer and consumer never write to the same atomic
// - Memory is shared but access is coordinated via acquire/release semantics
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    /// Creates a new ring buffer from pointers to shared memory.
    ///
    /// # Arguments
    ///
    /// * `header` - Pointer to ring buffer header in shared memory
    /// * `data` - Pointer to data array in shared memory
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - `header` points to a valid `RingBufferHeader` aligned to 8 bytes
    /// - `data` points to a valid byte array of at least `header.capacity` bytes
    /// - The memory region remains valid for the lifetime of this structure
    /// - Only one domain calls `try_write` (producer)
    /// - Only one domain calls `try_read` (consumer)
    pub unsafe fn from_raw_parts(header: *mut RingBufferHeader, data: *mut u8) -> Self {
        assert!(
            !header.is_null(),
            "ring buffer header pointer must not be null"
        );
        assert!(!data.is_null(), "ring buffer data pointer must not be null");

        // **SECURITY:** Read capacity from shared header ONCE and cache it.
        // This value becomes immutable after construction.
        let capacity = unsafe { (*header).capacity } as usize;
        assert!(
            capacity > 0,
            "ring buffer capacity must be greater than zero"
        );
        assert!(
            capacity.is_power_of_two(),
            "ring buffer capacity must be a power of two (for index rollover)"
        );
        assert!(
            capacity <= isize::MAX as usize,
            "ring buffer capacity exceeds maximum supported size"
        );
        Self {
            header,
            data,
            capacity,
        }
    }

    /// Returns the capacity of the ring buffer in bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Attempts to write data to the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Data written successfully
    /// * `Err(WriteError::NoSpace)` - Insufficient space
    /// * `Err(WriteError::InvalidSize)` - Data larger than capacity
    ///
    /// # Safety
    ///
    /// Caller must be the designated producer domain.
    pub fn try_write(&mut self, data: &[u8]) -> Result<(), WriteError> {
        // SAFETY: header pointer is valid by construction
        let header = unsafe { &*self.header };

        // 1. Check data size fits in capacity
        if data.len() > self.capacity {
            return Err(WriteError::InvalidSize);
        }

        // 2. Calculate available space
        let producer_head = header.producer_head.load(Ordering::Acquire);
        let consumer_head = header.consumer_head.load(Ordering::Acquire);
        let used = producer_head.saturating_sub(consumer_head) as usize;

        if data.len() > (self.capacity - used) {
            return Err(WriteError::NoSpace);
        }

        // 3. Write data (may wrap around)
        let capacity = self.capacity;
        let capacity_u64 = capacity as u64;
        let start = (producer_head % capacity_u64) as usize;
        let end = ((producer_head + data.len() as u64) % capacity_u64) as usize;

        // SAFETY: data pointer is valid and points to at least capacity bytes
        let data_slice = unsafe { core::slice::from_raw_parts_mut(self.data, capacity) };

        if start < end || end == 0 {
            // Simple case: no wrap (or end wraps to 0 which means write to end)
            if data.len() <= capacity - start {
                // Single contiguous write
                data_slice[start..start + data.len()].copy_from_slice(data);
            } else {
                // Wraps around - split write
                let first_chunk = capacity - start;
                data_slice[start..].copy_from_slice(&data[..first_chunk]);
                data_slice[..data.len() - first_chunk].copy_from_slice(&data[first_chunk..]);
            }
        } else {
            // Wrap case: split write
            let first_chunk = capacity - start;
            data_slice[start..].copy_from_slice(&data[..first_chunk]);
            data_slice[..end].copy_from_slice(&data[first_chunk..]);
        }

        // 4. Update producer head (release semantic)
        header
            .producer_head
            .store(producer_head + data.len() as u64, Ordering::Release);

        Ok(())
    }

    /// Attempts to read data from the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to read into
    ///
    /// # Returns
    ///
    /// * `Ok(n)` - Number of bytes read
    /// * `Err(ReadError::Empty)` - No data available
    ///
    /// # Safety
    ///
    /// Caller must be the designated consumer domain.
    pub fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
        // SAFETY: header pointer is valid by construction
        let header = unsafe { &*self.header };

        // 1. Calculate available data
        let producer_head = header.producer_head.load(Ordering::Acquire);
        let consumer_head = header.consumer_head.load(Ordering::Acquire);
        let available = producer_head.saturating_sub(consumer_head) as usize;

        if available == 0 {
            return Err(ReadError::Empty);
        }

        // 2. Clamp to buffer size
        let to_read = buf.len().min(available);

        // 3. Read data (may wrap around)
        let capacity = self.capacity;
        let capacity_u64 = capacity as u64;
        let start = (consumer_head % capacity_u64) as usize;
        let end = ((consumer_head + to_read as u64) % capacity_u64) as usize;

        // SAFETY: data pointer is valid and points to at least capacity bytes
        let data_slice = unsafe { core::slice::from_raw_parts(self.data, capacity) };

        if start < end || end == 0 {
            // Simple case: no wrap (or end wraps to 0 which means read to end)
            if to_read <= capacity - start {
                // Single contiguous read
                buf[..to_read].copy_from_slice(&data_slice[start..start + to_read]);
            } else {
                // Wraps around - split read
                let first_chunk = capacity - start;
                buf[..first_chunk].copy_from_slice(&data_slice[start..]);
                buf[first_chunk..to_read].copy_from_slice(&data_slice[..to_read - first_chunk]);
            }
        } else {
            // Wrap case: split read
            let first_chunk = capacity - start;
            buf[..first_chunk].copy_from_slice(&data_slice[start..]);
            buf[first_chunk..to_read].copy_from_slice(&data_slice[..end]);
        }

        // 4. Update consumer head (release semantic)
        header
            .consumer_head
            .store(consumer_head + to_read as u64, Ordering::Release);

        Ok(to_read)
    }

    /// Returns the number of bytes available for reading.
    pub fn available_read(&self) -> usize {
        // SAFETY: header pointer is valid by construction
        let header = unsafe { &*self.header };
        let producer_head = header.producer_head.load(Ordering::Acquire);
        let consumer_head = header.consumer_head.load(Ordering::Acquire);
        producer_head.saturating_sub(consumer_head) as usize
    }

    /// Returns the number of bytes available for writing.
    pub fn available_write(&self) -> usize {
        // **SECURITY:** Uses cached capacity instead of reading from shared memory
        self.capacity - self.available_read()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.available_read() == 0
    }

    /// Returns true if the buffer is full.
    pub fn is_full(&self) -> bool {
        self.available_write() == 0
    }

    /// Returns the cache mode from the flags field.
    pub fn cache_mode(&self) -> u64 {
        // SAFETY: header pointer is valid by construction
        let header = unsafe { &*self.header };
        header.flags & 0xFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a ring buffer for testing.
    ///
    /// This allocates stable heap storage so raw pointers remain valid for the full test scope.
    /// In production, the ring buffer would point to shared memory.
    unsafe fn test_ring_buffer(
        capacity: usize,
    ) -> (
        std::boxed::Box<RingBufferHeader>,
        RingBuffer,
        std::boxed::Box<[u8; 4096]>,
    ) {
        let mut header = std::boxed::Box::new(RingBufferHeader {
            producer_head: AtomicU64::new(0),
            consumer_head: AtomicU64::new(0),
            capacity: capacity as u64,
            flags: cache_mode::WRITE_BACK,
        });

        let mut data = std::boxed::Box::new([0u8; 4096]);

        let rb =
            RingBuffer::from_raw_parts((&mut *header) as *mut RingBufferHeader, data.as_mut_ptr());
        (header, rb, data)
    }

    #[test]
    fn ring_buffer_initially_empty() {
        unsafe {
            let (_header, rb, _data) = test_ring_buffer(1024);
            assert!(rb.is_empty());
            assert!(!rb.is_full());
            assert_eq!(rb.available_read(), 0);
            assert_eq!(rb.available_write(), 1024);
        }
    }

    #[test]
    fn ring_buffer_write_then_read() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(1024);

            let write_data = b"Hello, world!";
            rb.try_write(write_data).unwrap();

            assert!(!rb.is_empty());
            assert_eq!(rb.available_read(), 13);
            assert_eq!(rb.available_write(), 1024 - 13);

            let mut read_buf = [0u8; 64];
            let n = rb.try_read(&mut read_buf).unwrap();

            assert_eq!(n, 13);
            assert_eq!(&read_buf[..n], write_data);
            assert!(rb.is_empty());
        }
    }

    #[test]
    fn ring_buffer_write_when_full_returns_no_space() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(16);

            // Fill the buffer
            rb.try_write(&[0xAA; 16]).unwrap();

            assert!(rb.is_full());
            assert_eq!(rb.available_write(), 0);

            // Try to write more
            let result = rb.try_write(&[0xBB; 1]);
            assert_eq!(result, Err(WriteError::NoSpace));
        }
    }

    #[test]
    fn ring_buffer_read_when_empty_returns_empty() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(1024);

            let mut buf = [0u8; 64];
            let result = rb.try_read(&mut buf);

            assert_eq!(result, Err(ReadError::Empty));
        }
    }

    #[test]
    fn ring_buffer_write_larger_than_capacity_returns_error() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(16);

            let result = rb.try_write(&[0u8; 32]);
            assert_eq!(result, Err(WriteError::InvalidSize));
        }
    }

    #[test]
    fn ring_buffer_wrap_around_write_and_read() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(16);

            // Write and read to advance indices
            rb.try_write(&[1; 8]).unwrap();
            let mut buf = [0u8; 8];
            rb.try_read(&mut buf).unwrap();

            // Now producer_head and consumer_head are at 8
            // Write 16 bytes which will wrap around
            let write_data = [2; 16];
            rb.try_write(&write_data).unwrap();

            assert!(rb.is_full());

            // Read back the data (should wrap)
            let mut read_buf = [0u8; 16];
            rb.try_read(&mut read_buf).unwrap();

            assert_eq!(&read_buf[..], &write_data[..]);
        }
    }

    #[test]
    fn ring_buffer_partial_read() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(1024);

            rb.try_write(&[1, 2, 3, 4, 5]).unwrap();

            let mut buf = [0u8; 2];
            let n = rb.try_read(&mut buf).unwrap();

            assert_eq!(n, 2);
            assert_eq!(&buf[..], &[1, 2]);
            assert_eq!(rb.available_read(), 3);
        }
    }

    #[test]
    fn ring_buffer_multiple_write_read_cycles() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(64);

            for i in 0..10 {
                let write_data = [i as u8; 5];
                rb.try_write(&write_data).unwrap();

                let mut buf = [0u8; 5];
                let n = rb.try_read(&mut buf).unwrap();

                assert_eq!(n, 5);
                assert_eq!(&buf[..], &write_data[..]);
            }
        }
    }

    #[test]
    fn ring_buffer_cache_mode() {
        unsafe {
            let mut header = RingBufferHeader {
                producer_head: AtomicU64::new(0),
                consumer_head: AtomicU64::new(0),
                capacity: 1024,
                flags: cache_mode::UNCACHED,
            };

            let mut data = [0u8; 1024];
            let rb = RingBuffer::from_raw_parts(&mut header, data.as_mut_ptr());

            assert_eq!(rb.cache_mode(), cache_mode::UNCACHED);
        }
    }

    #[test]
    fn ring_buffer_write_read_with_exact_capacity() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(16);

            // Write exactly capacity bytes
            let write_data = b"Hello, 16 bytes!";
            rb.try_write(write_data).unwrap();

            assert!(rb.is_full());

            // Read back exactly capacity bytes
            let mut read_buf = [0u8; 16];
            let n = rb.try_read(&mut read_buf).unwrap();

            assert_eq!(n, 16);
            assert_eq!(&read_buf[..], write_data);
            assert!(rb.is_empty());
        }
    }

    #[test]
    fn ring_buffer_monotonic_indices() {
        unsafe {
            let (_header, mut rb, _data) = test_ring_buffer(16);

            // Perform multiple write/read cycles
            for _ in 0..3 {
                rb.try_write(&[1, 2, 3, 4]).unwrap();
                let mut buf = [0u8; 4];
                rb.try_read(&mut buf).unwrap();
            }

            // Check that indices have increased
            let header = &*rb.header;
            let producer_head = header.producer_head.load(Ordering::Acquire);
            let consumer_head = header.consumer_head.load(Ordering::Acquire);

            // After 3 cycles of writing/reading 4 bytes each
            assert_eq!(producer_head, 12);
            assert_eq!(consumer_head, 12);
        }
    }

    #[test]
    #[should_panic(expected = "ring buffer capacity must be greater than zero")]
    fn ring_buffer_rejects_zero_capacity() {
        unsafe {
            let mut header = RingBufferHeader {
                producer_head: AtomicU64::new(0),
                consumer_head: AtomicU64::new(0),
                capacity: 0,
                flags: cache_mode::WRITE_BACK,
            };
            let mut data = [0u8; 1];
            let _rb = RingBuffer::from_raw_parts(&mut header, data.as_mut_ptr());
        }
    }
}
