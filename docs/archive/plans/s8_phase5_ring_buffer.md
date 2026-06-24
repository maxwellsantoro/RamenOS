# S8 Phase 5: Data Plane Ring Buffer

**Last Updated:** 2026-02-10
**Status:** Planning
**Dependencies:** S8 Phase 4 (COMPLETE - MMU Integration)
**Target:** Zero-copy data transfer between domains using shared memory regions

## Overview

S8 Phase 4 demonstrated hardware-enforced isolation via MMU programming. S8 Phase 5 proves **zero-copy data transfer** by implementing a lock-free ring buffer that lives in shared memory and enables direct data exchange between domains without IPC message payloads.

## Objectives

1. **Ring Buffer Implementation**: Lock-free SPSC (Single Producer/Single Consumer) ring buffer in shared memory
2. **Multi-Domain Support**: Enable domain>0 operations with proper page table root tracking
3. **Page Table Allocation**: Dynamic allocation of intermediate page table levels (PML4/PDP/PD)
4. **Cache Coherency**: Validate cache mode flags (UNCACHED, WRITE_COMBINE, WRITE_BACK)
5. **Data Transfer Gate**: QEMU-based integration test proving zero-copy transfer

## Architecture

### Ring Buffer Structure

```rust
// Shared memory layout (must berepr(C) and safe for shared access)
#[repr(C)]
pub struct RingBuffer {
    // Producer index (updated by producer domain)
    producer_head: AtomicU64,

    // Consumer index (updated by consumer domain)
    consumer_head: AtomicU64,

    // Buffer metadata
    capacity: u64,      // Total capacity in bytes
    flags: u64,         // Cache mode flags, etc.

    // Flexible array member for data storage
    data: [u8; 0],      // In practice: mapped physical frames
}

// Ring buffer operations (non-blocking, lock-free)
impl RingBuffer {
    // Write from producer domain
    pub fn try_write(&mut self, data: &[u8]) -> Result<(), WriteError>;

    // Read from consumer domain
    pub fn try_read(&mut self, buf: &mut [u8]) -> Result<(), ReadError>;

    // Query available space
    pub fn available_write(&self) -> usize;
    pub fn available_read(&self) -> usize;
}
```

### Multi-Domain Architecture

**Domain Registration Flow:**
1. Domain manager creates new domain via domain_registry
2. AddressSpaceTable allocates new page table root (4KiB physical frame)
3. MMU registers the root with domain_id
4. Domain can now map/unmap shared regions

**Page Table Allocation:**
```
Current (S8 Phase 4): MMU only operates on existing tables
Phase 5: MMU allocates missing intermediate levels

Example mapping flow:
1. map_pages(domain_id=1, vaddr=0x8000_0000, frames=[...])
2. Look up domain 1's PML4 root from AddressSpaceTable
3. Walk PML4 → PDP → PD → PT
4. If PDP entry absent, allocate new PDP table (4KiB)
5. If PD entry absent, allocate new PD table (4KiB)
6. If PT entry absent, allocate new PT table (4KiB)
7. Set final PT entry to point to physical frame
8. Invalidate TLB for the mapped virtual address
```

### Cache Coherency Strategy

**x86_64 Cache Modes:**
- `CACHE_MODE_UNCACHED` → PAT index 0 (uncacheable)
- `CACHE_MODE_WRITE_COMBINE` → PAT index 1 (write-combining)
- `CACHE_MODE_WRITE_BACK` → PAT index 6 (write-back, standard)

**aarch64 Cache Modes:**
- `CACHE_MODE_UNCACHED` → MAIR_ATTR_DEVICE (Device-nGnRnE)
- `CACHE_MODE_WRITE_COMBINE` → MAIR_ATTR_NC (Normal Non-Cacheable)
- `CACHE_MODE_WRITE_BACK` → MAIR_ATTR_WB (Normal Write-Back)

**Coherency Gates:**
1. Test UC mode: Verify reads bypass CPU caches
2. Test WC mode: Verify write-combining behavior (optional, QEMU may not emulate)
3. Test WB mode: Verify standard cached behavior (default)

## Implementation Plan

### Phase 5.1: Ring Buffer Core (1 week)

**Tasks:**
1. Define `RingBuffer` structure in `kernel_api/src/ring_buffer.rs`
2. Implement SPSC ring buffer operations with AtomicU64 indices
3. Add unit tests for ring buffer logic (no shared memory yet)
4. Create Foundry gate: `foundry_ring_buffer_core_s8_phase5_1.sh`

**Tests:**
- Single producer/consumer correctness (10/10 scenarios)
- Wrap-around boundary conditions
- Empty/full buffer edge cases
- Atomic index ordering (release/acquire semantics)

### Phase 5.2: Multi-Domain Support (1 week)

**Tasks:**
1. Add page table allocation to MMU trait
2. Implement PML4/PDP/PD allocation in `x86_64/mmu.rs`
3. Implement TTBR0/PUD/PMD allocation in `aarch64/mmu.rs`
4. Domain manager integration with AddressSpaceTable
5. Create Foundry gate: `foundry_multidomain_s8_phase5_2.sh`

**Tests:**
- Create domain 1 with dedicated page table root
- Map region into domain 1 (not domain 0)
- Verify domain isolation (domain 1 cannot access domain 0 mappings)
- Test domain cleanup (free page tables on domain destroy)

### Phase 5.3: Shared Memory Ring Buffer (1 week)

**Tasks:**
1. Extend `ShmemRegion` to include optional `RingBuffer` metadata
2. Add `create_ring_buffer_region()` to `ShmemRegionTable`
3. Implement `ring_buffer_write()` and `ring_buffer_read()` operations
4. Map ring buffer into two domains (producer + consumer)
5. Create Foundry gate: `foundry_shmem_ring_buffer_s8_phase5_3.sh`

**Tests:**
- Create shared region with ring buffer metadata
- Domain 0 writes pattern, domain 1 reads pattern
- Verify data integrity without IPC payload transfer
- Test concurrent access (producer writes while consumer reads)

### Phase 5.4: Cache Coherency Validation (3 days)

**Tasks:**
1. Define cache mode test patterns
2. Create regions with each cache mode
3. Add assertions for cache mode enforcement in MMU
4. Create Foundry gate: `foundry_cache_coherency_s8_phase5_4.sh`

**Tests:**
- UNCACHED mode: Verify reads hit physical memory (not cached)
- WRITE_COMBINE mode: Verify write merging (best-effort)
- WRITE_BACK mode: Verify standard caching (default)
- Cache mode validation: Reject invalid mode values

### Phase 5.5: Integration Test (3 days)

**Tasks:**
1. Create end-to-end data transfer scenario
2. Simulate producer/consumer domains
3. Measure zero-copy benefit (no data copying in kernel)
4. Create Foundry gate: `foundry_ring_buffer_integration_s8_phase5_5.sh`

**Tests:**
- Producer writes 1MiB data through ring buffer
- Consumer reads 1MiB data from ring buffer
- Verify data integrity (byte-by-byte comparison)
- Verify zero-copy (no intermediate buffers)
- Benchmark throughput (optional)

## Gate Specifications

### Gate 1: Ring Buffer Core
```bash
./tools/ci/foundry_ring_buffer_core_s8_phase5_1.sh
```
**Assertions:** 25 tests
- SPSC correctness: 10 tests
- Boundary conditions: 8 tests
- Atomic operations: 5 tests
- Error handling: 2 tests

### Gate 2: Multi-Domain
```bash
./tools/ci/foundry_multidomain_s8_phase5_2.sh
```
**Assertions:** 15 tests
- Domain creation: 3 tests
- Page table allocation: 5 tests
- Domain isolation: 4 tests
- Domain cleanup: 3 tests

### Gate 3: Shared Memory Ring Buffer
```bash
./tools/ci/foundry_shmem_ring_buffer_s8_phase5_3.sh
```
**Assertions:** 20 tests (QEMU-based)
- Create ring buffer region: 4 tests
- Producer/consumer operations: 8 tests
- Data integrity: 4 tests
- Concurrent access: 4 tests

### Gate 4: Cache Coherency
```bash
./tools/ci/foundry_cache_coherency_s8_phase5_4.sh
```
**Assertions:** 12 tests
- UNCACHED mode: 3 tests
- WRITE_COMBINE mode: 3 tests
- WRITE_BACK mode: 3 tests
- Cache mode validation: 3 tests

### Gate 5: Integration
```bash
./tools/ci/foundry_ring_buffer_integration_s8_phase5_5.sh
```
**Assertions:** 10 tests (QEMU-based)
- End-to-end transfer: 3 tests
- Data integrity: 3 tests
- Zero-copy verification: 2 tests
- Performance: 2 tests (optional)

## IDL Contracts

### Optional: Ring Buffer Control Protocol

**If needed for service communication:**

```toml
# idl/harness/ring_buffer_control_v1.toml
name = "ring_buffer_control_v1"
version = "1"

[[messages]]
name = "CreateRingBuffer"
request_id = 1
[[messages.fields]]
name = "region_id"
type = "u64"
[[messages.fields]]
name = "capacity"
type = "u64"

[[messages]]
name = "RingBufferWrite"
request_id = 2
[[messages.fields]]
name = "region_id"
type = "u64"
[[messages.fields]]
name = "data"
type = "blob"  # Inline for small messages, reference for large

[[messages]]
name = "RingBufferRead"
request_id = 3
[[messages.fields]]
name = "region_id"
type = "u64"
[[messages.fields]]
name = "max_bytes"
type = "u64"
```

**Note:** Ring buffer operations may be pure memory access (no IPC) after initial setup, for true zero-copy performance.

## Technical Risks and Mitigations

### Risk 1: Page Table Allocation Exhaustion
**Risk:** Aggressive mapping could exhaust physical memory for page tables
**Mitigation:**
- Limit max page table depth (4 levels already max for x86_64/aarch64)
- Add max mappings per domain limit
- Return `STATUS_NO_MEMORY` when allocation fails

### Risk 2: Cache Coherency on Real Hardware
**Risk:** QEMU may not accurately emulate cache behavior
**Mitigation:**
- Document that cache coherency gates are best-effort in QEMU
- Add hardware testing notes for production deployment
- Focus on correctness (no crashes) over performance characteristics

### Risk 3: Ring Buffer Overflow/Underflow
**Risk:** Producer/consumer desynchronization could corrupt data
**Mitigation:**
- Use atomic indices with release/acquire ordering
- Return errors on overflow/underflow (never block)
- Add generation counters to detect stale indices

### Risk 4: Domain>0 Testing Complexity
**Risk:** Multi-domain tests harder to set up in QEMU environment
**Mitigation:**
- Use init process to simulate multiple domains
- Create domain lifecycle helpers (create/destroy/cleanup)
- Add comprehensive domain isolation assertions

## Success Criteria

### Must Have (P0):
- ✅ Ring buffer implementation passes unit tests
- ✅ Multi-domain support with page table allocation
- ✅ Zero-copy data transfer verified in QEMU
- ✅ All 5 Foundry gates green

### Should Have (P1):
- ✅ Cache mode validation (at least compiles and runs)
- ✅ Domain isolation verified
- ✅ Concurrent access tested

### Nice to Have (P2):
- ✅ Performance benchmarks (throughput measurements)
- ✅ Multiple ring buffers per domain
- ✅ Adaptive cache mode selection

## Documentation Updates

1. **CURRENT_STATUS.md**: Add S8 Phase 5 section with progress tracking
2. **SLICES.md**: Update S8 section with Phase 5 deliverables
3. **CHANGELOG.md**: Add S8 Phase 5 entry on completion
4. **NEW**: `docs/RING_BUFFER_V0.md` - Ring buffer specification
5. **NEW**: `docs/MULTI_DOMAIN.md` - Multi-domain architecture guide

## Dependencies

**Internal:**
- ✅ S8 Phase 4 (MMU Integration) - COMPLETE
- ✅ Domain Registry (from S6)
- ✅ Capability System (from S7)

**External:**
- None (pure kernel development)

## Timeline

**Total:** 4-5 weeks
- Phase 5.1: 1 week (Ring buffer core)
- Phase 5.2: 1 week (Multi-domain support)
- Phase 5.3: 1 week (Shared memory integration)
- Phase 5.4: 3 days (Cache coherency)
- Phase 5.5: 3 days (Integration testing)

## Post-Phase 5: S8 Phase 6 (Optional)

**Potential Future Work:**
- Multi-producer/consumer ring buffers (MPSC)
- Priority-based ring buffers
- Zero-copy IPC message passing over ring buffers
- DMA engine integration for device access
- Persistent shared memory regions (survive domain restart)

---

**Document Status:** Planning
**Author:** Claude (RamenOS Architecture)
**Date:** 2026-02-10
**Related:**
- `plans/security_remediation_v006_v007_v012.md`
- `CLAUDE.md` (Constitutional Non-Negotiables)
- `SLICES.md` (Slice S8 definition)
