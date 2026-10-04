# Ring Buffer V0

**Last Updated:** 2026-10-03
**Status:** Implemented SPSC shared-memory primitive; bounded caller contract

[kernel_api/src/ring_buffer.rs](../kernel_api/src/ring_buffer.rs) defines a
single-producer/single-consumer byte ring in shared memory. After mapping, its
read/write operations need no kernel intermediary. The current slice API copies
bytes into and out of the ring; it is not a copy-free caller interface.

## Layout

The `repr(C)` header occupies 32 bytes and requires at least 8-byte alignment.
Data storage follows the header by convention; `from_raw_parts` takes separate
header/data pointers.

| Offset | Field | Meaning |
|--------|-------|---------|
| 0 | `producer_head: AtomicU64` | Published producer position |
| 8 | `consumer_head: AtomicU64` | Released consumer position |
| 16 | `capacity: u64` | Byte capacity |
| 24 | `flags: u64` | Metadata; low 8 bits encode cache mode |

A contiguous allocation needs `32 + capacity` bytes, plus any mapping alignment.
Capacity must be nonzero, a power of two, and at most `isize::MAX`.

## Construction and lifetime

`unsafe RingBuffer::from_raw_parts` asserts non-null pointers and caches capacity
once. Later edits to the shared capacity field do not change its cached bounds.
The caller must still provide a valid aligned header, data storage of that size,
and mappings valid for the ring's entire lifetime.

Exactly one producer calls writes and one consumer calls reads. These role and
lifetime requirements are not established by the pointers or Rust wrapper.
Unmapping, revocation, or destruction must not race continued pointer use.
Malformed peer indices and arbitrary mutation of shared data are outside the
cooperating-peer protocol; this primitive alone is not a hostile-peer boundary.

## Operations

| API | Result |
|-----|--------|
| `try_write(&mut self, data: &[u8])` | Writes all bytes or returns `NoSpace`/`InvalidSize` |
| `try_read(&mut self, buf: &mut [u8])` | Reads available bytes up to buffer length or returns `Empty` |
| `available_read`, `available_write` | Snapshot counts under the SPSC protocol |
| `is_empty`, `is_full`, `capacity`, `cache_mode` | State/metadata queries |

The producer copies bytes before publishing its head with Release ordering.
The consumer reads published positions with Acquire ordering, copies bytes,
then releases its head. Data movement may split at the capacity boundary.
This buffer wrap is distinct from overflow of the monotonic u64 counters; the
current arithmetic is not a proven unlimited-lifetime rollover protocol.

The flags describe cache mode but do not program page tables, PAT, or aarch64
memory attributes. Mapping/cache policy belongs to the shared-memory/MMU layer.
The ring itself has no capability validation or domain access control.

## Evidence and limits

`cargo test -p kernel_api ring_buffer` exercises empty/full behavior, wrap,
partial reads, capacity validation, and named primitive cases. Shared-memory
integration gates cover their own mapping/control assertions; passing them does
not certify arbitrary peer behavior, whole-kernel SMP safety, or performance.

Use [Multi-Domain](MULTI_DOMAIN.md) for mapping ownership,
[Security Status](../SECURITY_STATUS.md) for residual risk, and the
[historical ring plan](archive/plans/2026-02-10-s8-phase5-ring-buffer.md) for
original implementation rationale. Target throughput/latency need separate
measurements against representative consumers.
