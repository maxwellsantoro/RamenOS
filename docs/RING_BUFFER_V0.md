# Ring Buffer v0 Specification

**Last Updated:** 2026-06-24
**Status:** Implemented v0 reference
**Stability:** Experimental
**Dependencies:** S8 Phase 4 (MMU Integration)

## Overview

The ring buffer is a lock-free Single Producer/Single Consumer (SPSC) data structure that enables zero-copy data transfer between domains through shared memory. It lives entirely within a shared memory region and requires no kernel mediation for read/write operations after initial setup.

## Memory Layout

```
+----------------------+
| RingBufferHeader     |  Fixed size (32 bytes)
+----------------------+
| producer_head: u64   |  AtomicU64 (producer index)
| consumer_head: u64   |  AtomicU64 (consumer index)
| capacity: u64        |  Total capacity in bytes
| flags: u64           |  Cache mode, state flags
+----------------------+  <-- Data starts here
| Data...              |  Flexible array member
|                      |
+----------------------+
```

**Total Size:** `32 + capacity` bytes

**Alignment:** 8-byte aligned for atomic operations

## Data Structures

### Header (Shared Memory)

```rust
#[repr(C)]
pub struct RingBufferHeader {
    /// Producer write head (monotonically increasing)
    /// Written by producer, read by consumer
    producer_head: AtomicU64,

    /// Consumer read head (monotonically increasing)
    /// Written by consumer, read by producer
    consumer_head: AtomicU64,

    /// Total capacity of the ring buffer in bytes
    capacity: u64,

    /// Flags and configuration
    /// bit 0-7: cache_mode (CACHE_MODE_* constants)
    /// bit 8-15: reserved
    /// bit 16: initialized flag
    flags: u64,
}
```

### Indices

Both indices are **monotonically increasing** (not modulo capacity). This:
- Avoids wrap-around ambiguity
- Simplifies empty/full detection
- Requires masking on each access: `index % capacity`

**Index Space:** u64 (effectively infinite for practical use)

**Example:**
```
capacity = 1024
producer_head = 5000
consumer_head = 4000

available_read = (5000 - 4000) = 1024 (full buffer)
available_write = 1024 - 1024 = 0 (buffer full)

After consumer reads 512 bytes:
consumer_head = 4512
available_read = 5000 - 4512 = 488
available_write = 1024 - 488 = 536
```

## Operations

### try_write (Producer Domain)

```rust
/// Attempt to write data to the ring buffer
///
/// # Arguments
/// * `data` - Bytes to write
///
/// # Returns
/// * `Ok(())` - Data written successfully
/// * `Err(WriteError::NoSpace)` - Insufficient space
/// * `Err(WriteError::InvalidSize)` - Data larger than capacity
///
/// # Safety
/// Caller must be the designated producer domain
pub fn try_write(&mut self, data: &[u8]) -> Result<(), WriteError> {
    // 1. Check data size fits in capacity
    if data.len() > self.capacity {
        return Err(WriteError::InvalidSize);
    }

    // 2. Calculate available space
    let producer_head = self.header.producer_head.load(Ordering::Acquire);
    let consumer_head = self.header.consumer_head.load(Ordering::Acquire);
    let used = producer_head.saturating_sub(consumer_head);

    if data.len() > (self.capacity - used as usize) {
        return Err(WriteError::NoSpace);
    }

    // 3. Write data (may wrap around)
    let start = (producer_head % self.capacity) as usize;
    let end = ((producer_head + data.len() as u64) % self.capacity) as usize;

    if start < end {
        // Simple case: no wrap
        self.data[start..end].copy_from_slice(data);
    } else {
        // Wrap case: split write
        let first_chunk = self.capacity - start as u64;
        self.data[start..].copy_from_slice(&data[..first_chunk as usize]);
        self.data[..end].copy_from_slice(&data[first_chunk as usize..]);
    }

    // 4. Update producer head (release semantic)
    self.header.producer_head.store(
        producer_head + data.len() as u64,
        Ordering::Release
    );

    Ok(())
}
```

### try_read (Consumer Domain)

```rust
/// Attempt to read data from the ring buffer
///
/// # Arguments
/// * `buf` - Buffer to read into
///
/// # Returns
/// * `Ok(n)` - Number of bytes read
/// * `Err(ReadError::Empty)` - No data available
///
/// # Safety
/// Caller must be the designated consumer domain
pub fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
    // 1. Calculate available data
    let producer_head = self.header.producer_head.load(Ordering::Acquire);
    let consumer_head = self.header.consumer_head.load(Ordering::Acquire);
    let available = producer_head.saturating_sub(consumer_head) as usize;

    if available == 0 {
        return Err(ReadError::Empty);
    }

    // 2. Clamp to buffer size
    let to_read = buf.len().min(available);

    // 3. Read data (may wrap around)
    let start = (consumer_head % self.capacity) as usize;
    let end = ((consumer_head + to_read as u64) % self.capacity) as usize;

    if start < end {
        // Simple case: no wrap
        buf[..to_read].copy_from_slice(&self.data[start..end]);
    } else {
        // Wrap case: split read
        let first_chunk = self.capacity - start as u64;
        buf[..first_chunk as usize]
            .copy_from_slice(&self.data[start..]);
        buf[first_chunk as usize..to_read]
            .copy_from_slice(&self.data[..end]);
    }

    // 4. Update consumer head (release semantic)
    self.header.consumer_head.store(
        consumer_head + to_read as u64,
        Ordering::Release
    );

    Ok(to_read)
}
```

### Query Operations

```rust
/// Returns the number of bytes available for reading
pub fn available_read(&self) -> usize {
    let producer_head = self.header.producer_head.load(Ordering::Acquire);
    let consumer_head = self.header.consumer_head.load(Ordering::Acquire);
    producer_head.saturating_sub(consumer_head) as usize
}

/// Returns the number of bytes available for writing
pub fn available_write(&self) -> usize {
    self.capacity - self.available_read()
}

/// Returns true if the buffer is empty
pub fn is_empty(&self) -> bool {
    self.available_read() == 0
}

/// Returns true if the buffer is full
pub fn is_full(&self) -> bool {
    self.available_write() == 0
}
```

## Cache Modes

The ring buffer supports different cache modes via the `flags` field:

### x86_64

| Mode           | Value | Page Table Flags        | Behavior                          |
|----------------|-------|-------------------------|-----------------------------------|
| UNCACHED       | 0     | PAT index 0 (UC)        | Bypass CPU caches entirely        |
| WRITE_COMBINE | 1     | PAT index 1 (WC)        | Write merging, no read caching    |
| WRITE_BACK     | 2     | PAT index 6 (WB)        | Standard caching (default)        |

### aarch64

| Mode           | Value | MAIR Index              | Behavior                          |
|----------------|-------|-------------------------|-----------------------------------|
| UNCACHED       | 0     | Device-nGnRnE           | Device memory, no caching         |
| WRITE_COMBINE | 1     | Normal Non-Cacheable    | Outer non-cacheable               |
| WRITE_BACK     | 2     | Normal Write-Back       | Normal cacheable memory (default) |

**Recommendation:** Use UNCACHED for MMIO regions, WRITE_BACK for general data transfer.

## Usage Example

### Setup (Kernel/Superintendent)

```rust
// Create shared memory region with ring buffer capacity
let capacity = 65536; // 64 KiB
let region_size = 32 + capacity; // Header + data

let (region_id, shm_cap) = table.create_region(
    1,                      // content_id
    region_size,            // size
    REGION_FLAG_READABLE | REGION_FLAG_WRITABLE,
    4096,                   // page_size
)?;

// Initialize ring buffer header in the region
let header = unsafe { &mut *(region_vaddr as *mut RingBufferHeader) };
header.producer_head.store(0, Ordering::Release);
header.consumer_head.store(0, Ordering::Release);
header.capacity = capacity;
header.flags = CACHE_MODE_WRITE_BACK;

// Map region into producer domain (domain 0)
table.map_region(region_id, shm_cap, 0, RIGHTS_WRITE, 0)?;

// Map region into consumer domain (domain 1)
table.map_region(region_id, shm_cap, 1, RIGHTS_READ, 0)?;
```

### Producer (Domain 0)

```rust
// Producer writes data directly to ring buffer
let ring_buffer = unsafe {
    &mut *(mapped_vaddr as *mut RingBuffer)
};

loop {
    match ring_buffer.try_write(&data) {
        Ok(()) => break,
        Err(WriteError::NoSpace) => {
            // Wait for consumer to make space
            std::hint::spin_loop();
        }
        Err(e) => panic!("Write error: {:?}", e),
    }
}
```

### Consumer (Domain 1)

```rust
// Consumer reads data directly from ring buffer
let ring_buffer = unsafe {
    &mut *(mapped_vaddr as *mut RingBuffer)
};

let mut buf = [0u8; 4096];
loop {
    match ring_buffer.try_read(&mut buf) {
        Ok(n) => {
            // Process data
            handle_data(&buf[..n]);
        }
        Err(ReadError::Empty) => {
            // Wait for producer to write data
            std::hint::spin_loop();
        }
    }
}
```

## Zero-Copy Property

The ring buffer achieves **zero-copy** data transfer because:
1. Producer writes directly to shared memory (no intermediate kernel buffer)
2. Consumer reads directly from shared memory (no copy on receive)
3. No IPC messages carry payload data (only region_id + offset)

**Contrast with IPC:**
```
Traditional IPC:
Producer → Kernel Copy → IPC Message → Kernel Copy → Consumer
           (copy 1)                    (copy 2)

Ring Buffer:
Producer → Shared Memory → Consumer
           (direct access)
```

## Security Considerations

### Access Control

The ring buffer itself has **no built-in access control**. Security is enforced at the shared memory layer:
- Only domains with valid capabilities can map the region
- Rights (READ/WRITE) are enforced on `map_region`
- Domain isolation prevents unauthorized access

### Atomicity Guarantees

- All index updates use `AtomicU64` with release/acquire semantics
- No locks or mutexes (avoids deadlock and priority inversion)
- Producer/consumer never contend on the same atomic

### Memory Ordering

**Producer (try_write):**
1. Load indices with `Ordering::Acquire`
2. Write data to buffer
3. Store producer_head with `Ordering::Release`

**Consumer (try_read):**
1. Load indices with `Ordering::Acquire`
2. Read data from buffer
3. Store consumer_head with `Ordering::Release`

This ensures:
- Writes to data are visible before producer_head update
- Reads of data see all prior producer_head updates

## Implementation Notes

### Alignment

The entire ring buffer (header + data) should be aligned to at least 8 bytes for atomic operations. In practice, page alignment (4096 bytes) is guaranteed by the MMU.

### Wrap-Around Handling

The ring buffer naturally wraps around at `capacity`. The implementation handles:
- Single contiguous write/read (no wrap)
- Split write/read (wraps around end)

**Example with capacity=8:**
```
Indices: [0] [1] [2] [3] [4] [5] [6] [7]
         |--data--|                 |--wrap data--|
```

### Overflow/Underflow Prevention

**Overflow (write when full):**
- Check `available_write()` before writing
- Return `WriteError::NoSpace` if insufficient space
- Never block or spin-wait in the kernel path

**Underflow (read when empty):**
- Check `available_read()` before reading
- Return `ReadError::Empty` if no data
- Never block or spin-wait in the kernel path

## Testing Strategy

### Unit Tests

1. **Single Producer/Consumer:** Basic correctness
2. **Wrap-Around:** Boundary conditions at capacity edges
3. **Empty/Full:** Edge cases for no space/no data
4. **Atomic Ordering:** Verify release/acquire semantics
5. **Concurrent Access:** Stress tests with multiple threads

### Integration Tests (QEMU)

1. **Zero-Copy Transfer:** Domain 0 → Domain 1 data transfer
2. **Data Integrity:** Byte-by-byte verification
3. **Performance:** Throughput measurements
4. **Cache Modes:** Verify cache mode flags work

## Future Enhancements

### Multi-Producer/Multi-Consumer (MPMC)

Requires:
- Multiple atomic indices (one per producer/consumer)
- More sophisticated synchronization
- Trade-off: Higher contention, lower throughput

### Priority Queue

For prioritized data:
- Multiple ring buffers (one per priority level)
- Consumer checks high-priority buffers first
- Trade-off: Priority inversion for low-priority data

### Persistent Ring Buffers

Survive domain restarts:
- Log indices to persistent storage
- Recover on restart
- Trade-off: Complexity vs. reliability

---

**Document Status:** Implemented v0 reference; enhancements remain future work
**Author:** Claude (RamenOS Architecture)
**Date:** 2026-02-10
**Related:**
- `docs/archive/plans/2026-02-10-s8-phase5-ring-buffer.md` (historical plan)
- `kernel/src/shmem.rs` (Shared memory control plane)
- `kernel_api/src/generated/`
