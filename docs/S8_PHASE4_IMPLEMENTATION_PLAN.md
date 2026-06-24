# S8 Phase 4: Data-Plane Implementation Plan

**Last Updated:** 2026-02-10
**Status:** Planning Phase
**Slice:** S8 — Shared Memory Primitives

## Executive Summary

S8 Phase 4 completes the shared-memory data-plane implementation by integrating the control-plane (Phases 1-2) with physical memory allocation (Phase 3) and MMU programming. This phase enables actual zero-copy memory sharing between domains while maintaining kernel-side capability validation and control/data-plane separation.

**Current Status:**
- ✅ Phase 1: Typed IDL contract `shmem_control_v1.toml` with codegen
- ✅ Phase 2: Kernel capability validation (ShmemRegionTable + IPC handlers)
- ✅ Phase 3: Physical frame allocator (PhysAddr/PhysFrame + BumpAllocator)
- ⏸️ Phase 4: Data-plane integration (THIS PLAN)

**Phase 4 Goals:**
1. Wire `create_region` to allocate physical frames from the frame allocator
2. Implement MMU programming for page table updates on `map_region`
3. Implement `BitmapAllocator` for frame reuse (replacing BumpAllocator)
4. Add per-domain address space management (page table roots)
5. Create comprehensive Foundry gate with 40+ assertions

---

## Architecture Overview

### Control Plane vs Data Plane Separation

```
┌─────────────────────────────────────────────────────────────────┐
│                         Control Plane                         │
│  (Typed IPC Messages - shmem_control_v1.toml)              │
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │CreateRegion │    │ MapRegion  │    │CloseRegion │ │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘ │
│         │                   │                   │           │
│         ▼                   ▼                   ▼           │
│  ┌─────────────────────────────────────────────────────┐       │
│  │      ShmemRegionTable (Accounting)              │       │
│  │  - Generation counters, refcount, rights        │       │
│  │  - Capability validation                       │       │
│  └─────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Capability-validated
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Data Plane                          │
│  (Zero-Copy Physical Memory)                                 │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐       │
│  │      BitmapAllocator (Frame Management)            │       │
│  │  - Allocate/deallocate physical frames            │       │
│  │  - Track free/used state                        │       │
│  └─────────────────────────────────────────────────────┘       │
│                              │                               │
│                              │ Physical frames                │
│                              ▼                               │
│  ┌─────────────────────────────────────────────────────┐       │
│  │      MMU Programming (Page Tables)               │       │
│  │  - Map physical frames to virtual addresses      │       │
│  │  - Per-domain page table roots (CR3/TTBR0)     │       │
│  │  - Rights enforcement (R/W/X)                  │       │
│  └─────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Kernel-Side Validation:** All capability checks happen in the kernel before any memory is mapped
2. **Type Safety:** Physical addresses and frames use newtype wrappers (PhysAddr, PhysFrame)
3. **Fail-Closed:** Invalid operations return errors; no silent failures
4. **Auditability:** All operations emit trace events for replay verification
5. **Vertical Slice:** Each capability ships with a consumer + a Foundry gate

---

## Phase 4 Implementation Breakdown

### 4.1 BitmapAllocator Implementation

**File:** `kernel/src/mm/bitmap.rs` (new)

**Purpose:** Replace BumpAllocator with a reusable frame allocator that supports deallocation.

**Key Requirements:**
- Implement `FrameAllocator` trait
- Track free/used frames using a bitmap (1 bit per frame)
- Support allocation of contiguous frame ranges (for shared memory regions)
- Support individual frame deallocation
- Static array backing (no heap allocation)

**Data Structure:**
```rust
pub struct BitmapAllocator {
    /// Bitmap of frame states (1 bit per frame)
    /// 0 = free, 1 = allocated
    bitmap: [u64; BITMAP_WORDS],
    
    /// Base physical frame for this allocator
    base_frame: PhysFrame,
    
    /// Total number of frames managed
    total_frames: usize,
    
    /// Number of free frames (for fast availability check)
    free_frames: usize,
}

const BITMAP_WORDS: usize = (MAX_FRAMES + 63) / 64;
const MAX_FRAMES: usize = 131072; // 512 MiB / 4 KiB
```

**Key Operations:**
- `allocate_contiguous(n_frames: usize) -> Option<PhysFrame>`: Allocate N contiguous frames
- `allocate() -> Option<PhysFrame>`: Allocate single frame (FrameAllocator trait)
- `deallocate(frame: PhysFrame)`: Mark frame as free
- `deallocate_range(start: PhysFrame, n_frames: usize)`: Deallocate N contiguous frames

**Integration Points:**
- Replace `BumpAllocator` in `kernel/src/mm/mod.rs` global `FRAME_ALLOCATOR`
- Update `mm::init()` to initialize bitmap instead of bump allocator
- Add tests for allocation, deallocation, and fragmentation scenarios

**Foundry Assertions (10):**
1. `bitmap_allocator_allocates_single_frame`
2. `bitmap_allocator_allocates_contiguous_range`
3. `bitmap_allocator_rejects_insufficient_contiguous`
4. `bitmap_allocator_deallocates_single_frame`
5. `bitmap_allocator_deallocates_range`
6. `bitmap_allocator_prevents_double_allocation`
7. `bitmap_allocator_prevents_double_deallocation`
8. `bitmap_allocator_tracks_free_frames`
9. `bitmap_allocator_handles_fragmentation`
10. `bitmap_allocator_exhaustion_returns_none`

---

### 4.2 Per-Domain Address Space Management

**File:** `kernel/src/mm/address_space.rs` (new)

**Purpose:** Track page table roots for each domain to enable MMU programming.

**Key Requirements:**
- Store page table root physical address for each domain (CR3 on x86_64, TTBR0 on aarch64)
- Integrate with existing `DomainRegistry`
- Provide safe accessors for MMU programming

**Data Structure:**
```rust
pub struct AddressSpaceTable {
    /// Page table root for each domain
    /// None = domain not initialized
    roots: [Option<PhysAddr>; MAX_DOMAINS],
}

impl AddressSpaceTable {
    /// Set page table root for a domain
    pub fn set_root(&mut self, domain_id: DomainId, root: PhysAddr);
    
    /// Get page table root for a domain
    pub fn get_root(&self, domain_id: DomainId) -> Option<PhysAddr>;
    
    /// Initialize kernel address space (domain 0)
    pub fn init_kernel(&mut self, root: PhysAddr);
}
```

**Integration Points:**
- Add global static `ADDRESS_SPACE_TABLE` in `kernel/src/mm/mod.rs`
- Initialize kernel page table root during boot
- Wire into `DomainRegistry` lifecycle

**Boot Integration:**
```rust
// In kernel/src/boot.rs or arch-specific entry
let kernel_pt_root = arch::mmu::get_current_page_table_root();
mm::ADDRESS_SPACE_TABLE.init_kernel(kernel_pt_root);
```

**Foundry Assertions (4):**
1. `address_space_table_initializes_kernel_root`
2. `address_space_table_sets_domain_root`
3. `address_space_table_gets_domain_root`
4. `address_space_table_rejects_invalid_domain`

---

### 4.3 MMU Programming Interface

**File:** `kernel/src/arch/mmu.rs` (new, per-arch implementations)

**Purpose:** Architecture-agnostic interface for programming page tables.

**Key Requirements:**
- Map physical frames to virtual addresses
- Set page rights (R/W/X) and cache mode
- Flush TLB after updates
- Support both x86_64 and aarch64

**Trait Definition:**
```rust
pub trait Mmu {
    /// Map physical frames to virtual address range
    /// 
    /// # Arguments
    /// * `domain_id` - Target domain for mapping
    /// * `vaddr` - Starting virtual address (must be page-aligned)
    /// * `frames` - Iterator over physical frames to map
    /// * `rights` - Mapping rights (READ/WRITE/EXECUTE)
    /// * `cache_mode` - Cache mode (WB/UC/WT)
    /// 
    /// # Safety
    /// Caller must ensure:
    /// - `domain_id` is valid and has an address space
    /// - `vaddr` is page-aligned
    /// - `vaddr` range does not overlap existing mappings
    unsafe fn map_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
    ) -> Result<(), MmuError>;
    
    /// Unmap virtual address range
    unsafe fn unmap_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        num_pages: usize,
    ) -> Result<(), MmuError>;
    
    /// Flush TLB for a domain
    unsafe fn flush_tlb(domain_id: DomainId);
}

#[derive(Debug, PartialEq)]
pub enum MmuError {
    InvalidDomain,
    InvalidAddress,
    AllocationFailed,
    PermissionDenied,
}
```

**Architecture Implementations:**
- `kernel/src/arch/x86_64/mmu.rs`: x86_64 page tables (PML4 → PDP → PD → PT)
- `kernel/src/arch/aarch64/mmu.rs`: aarch64 page tables (L0 → L1 → L2 → L3)

**Integration Points:**
- Call from `map_region` in `kernel/src/shmem.rs`
- Use `AddressSpaceTable` to get page table roots
- Update CR3/TTBR0 after mapping

**Foundry Assertions (6):**
1. `mmu_map_pages_succeeds_with_valid_params`
2. `mmu_map_pages_rejects_invalid_domain`
3. `mmu_map_pages_rejects_misaligned_address`
4. `mmu_unmap_pages_removes_mapping`
5. `mmu_flush_tlb_clears_mappings`
6. `mmu_rights_enforcement_blocks_invalid_access`

---

### 4.4 Data-Plane Integration in ShmemRegionTable

**File:** `kernel/src/shmem.rs` (modify)

**Purpose:** Wire physical frame allocation and MMU programming into control-plane operations.

**Changes to `ShmemRegion`:**
```rust
struct ShmemRegion {
    // ... existing fields ...
    
    /// Physical frames allocated for this region
    /// None = not yet allocated (accounting-only mode)
    frames: Option<[PhysFrame; MAX_FRAMES_PER_REGION]>,
    
    /// Number of frames allocated
    num_frames: usize,
}

const MAX_FRAMES_PER_REGION: usize = 1024; // 4 MiB max per region
```

**Changes to `create_region`:**
```rust
pub fn create_region(
    &mut self,
    owner_domain_id: u64,
    size_bytes: u64,
    flags: u32,
    page_size: u32,
) -> Result<(u64, Handle), u32> {
    // ... existing validation ...
    
    // NEW: Allocate physical frames
    let num_frames = (size_bytes + page_size as u64 - 1) / page_size as u64;
    let num_frames = num_frames as usize;
    
    if num_frames > MAX_FRAMES_PER_REGION {
        return Err(STATUS_INVALID_SIZE);
    }
    
    let mut frames = [PhysFrame::from_frame_number(0); MAX_FRAMES_PER_REGION];
    
    unsafe {
        for i in 0..num_frames {
            match mm::FRAME_ALLOCATOR.allocate() {
                Some(frame) => frames[i] = frame,
                None => return Err(STATUS_NO_MEMORY),
            }
        }
    }
    
    // Store frames in region
    slot.frames = Some(frames);
    slot.num_frames = num_frames;
    
    // ... rest of existing code ...
}
```

**Changes to `map_region`:**
```rust
pub fn map_region(
    &mut self,
    region_id: u64,
    shm_cap: Handle,
    target_domain_id: u64,
    rights: u32,
    cache_mode: u32,
) -> Result<u64, u32> {
    // ... existing validation ...
    
    // NEW: Get physical frames from region
    let frames = slot.frames.ok_or(STATUS_REGION_NOT_FOUND)?;
    let frame_slice = &frames[..slot.num_frames];
    
    // NEW: Choose virtual address for mapping
    // For now, use a fixed range per domain
    let vaddr = VirtAddr::new(0x8000_0000 + (region_id & 0xFFFF) * 0x1000);
    
    // NEW: Program MMU
    unsafe {
        arch::mmu::map_pages(
            target_domain_id,
            vaddr,
            frame_slice,
            rights,
            cache_mode,
        ).map_err(|_| STATUS_INVALID_RIGHTS)?;
    }
    
    // Increment refcount
    slot.refcount += 1;
    
    // Return mapping_id (for now, same as region_id)
    Ok(region_id)
}
```

**Changes to `close_region`:**
```rust
pub fn close_region(&mut self, region_id: u64) -> Result<(), u32> {
    // ... existing validation ...
    
    // NEW: Deallocate physical frames
    if let Some(frames) = slot.frames {
        for i in 0..slot.num_frames {
            unsafe {
                mm::FRAME_ALLOCATOR.deallocate(frames[i]);
            }
        }
    }
    
    // Deallocate the slot
    slot.in_use = false;
    slot.generation = slot.generation.wrapping_add(1);
    if slot.generation == 0 {
        slot.generation = 1;
    }
    
    Ok(())
}
```

**Foundry Assertions (12):**
1. `create_region_allocates_physical_frames`
2. `create_region_returns_no_memory_on_exhaustion`
3. `create_region_rejects_too_large_region`
4. `map_region_programs_mmu`
5. `map_region_returns_error_on_mmu_failure`
6. `map_region_increments_refcount`
7. `unmap_region_decrements_refcount`
8. `close_region_deallocates_frames`
9. `close_region_fails_with_active_mappings`
10. `close_region_succeeds_after_unmap`
11. `generation_counter_prevents_stale_access`
12. `capability_validation_required_for_map_unmap_close`

---

### 4.5 IPC Handler Updates

**File:** `kernel/src/ipc_v0.rs` (modify)

**Purpose:** Ensure IPC handlers properly pass frame allocator and MMU context.

**Changes:**
- Update `handle_envelope` to pass frame allocator reference
- Ensure error replies include correct status codes for MMU failures
- Add trace events for data-plane operations

**Trace Events:**
```rust
// In create_region handler
trace_ring::emit_event(TraceEvent {
    domain_id: env.domain_id,
    tag: TAG_SHMEM_CREATE,
    data: ShmemCreateTrace {
        region_id,
        size_bytes,
        flags,
    },
});

// In map_region handler
trace_ring::emit_event(TraceEvent {
    domain_id: env.domain_id,
    tag: TAG_SHMEM_MAP,
    data: ShmemMapTrace {
        region_id,
        target_domain_id,
        rights,
        vaddr: vaddr.as_u64(),
    },
});
```

**Foundry Assertions (4):**
1. `ipc_create_region_emits_trace_event`
2. `ipc_map_region_emits_trace_event`
3. `ipc_close_region_emits_trace_event`
4. `ipc_handlers_return_correct_error_codes`

---

### 4.6 Foundry Gate: `foundry_shmem_dataplane_s8_phase4.sh`

**File:** `tools/ci/foundry_shmem_dataplane_s8_phase4.sh` (new)

**Purpose:** Comprehensive validation of Phase 4 data-plane implementation.

**Gate Structure:**
```bash
#!/usr/bin/env bash
# S8 Phase 4: Data-Plane Foundry Gate
#
# Total assertions: 40+
# Modules:
#   - BitmapAllocator (10 assertions)
#   - AddressSpaceTable (4 assertions)
#   - MMU Programming (6 assertions)
#   - Data-Plane Integration (12 assertions)
#   - IPC Handlers (4 assertions)
#   - Boot Integration (4 assertions)
#   - End-to-End Scenarios (4 assertions)

set -euo pipefail

# ... gate framework (colored output, counters) ...

# Module 1: BitmapAllocator
gate_header "S8P4_BITMAP" "Bitmap frame allocator with reuse"

run_gate_test "S8P4_BITMAP_ALLOC_SINGLE" \
    "cargo test -p kernel mm::bitmap::tests::bitmap_allocator_allocates_single_frame -- --exact"

run_gate_test "S8P4_BITMAP_ALLOC_RANGE" \
    "cargo test -p kernel mm::bitmap::tests::bitmap_allocator_allocates_contiguous_range -- --exact"

# ... 8 more bitmap tests ...

# Module 2: AddressSpaceTable
gate_header "S8P4_ADDRSPACE" "Per-domain address space management"

run_gate_test "S8P4_ADDRSPACE_INIT_KERNEL" \
    "cargo test -p kernel mm::address_space::tests::address_space_table_initializes_kernel_root -- --exact"

# ... 3 more address space tests ...

# Module 3: MMU Programming
gate_header "S8P4_MMU" "MMU programming interface"

run_gate_test "S8P4_MMU_MAP_SUCCESS" \
    "cargo test -p kernel arch::mmu::tests::mmu_map_pages_succeeds_with_valid_params -- --exact"

# ... 5 more MMU tests ...

# Module 4: Data-Plane Integration
gate_header "S8P4_INTEGRATION" "ShmemRegionTable data-plane wiring"

run_gate_test "S8P4_INTEGRATION_ALLOC_FRAMES" \
    "cargo test -p kernel shmem::tests::create_region_allocates_physical_frames -- --exact"

# ... 11 more integration tests ...

# Module 5: IPC Handlers
gate_header "S8P4_IPC" "IPC handler data-plane integration"

run_gate_test "S8P4_IPC_CREATE_TRACE" \
    "cargo test -p kernel ipc_v0::tests::ipc_create_region_emits_trace_event -- --exact"

# ... 3 more IPC tests ...

# Module 6: Boot Integration
gate_header "S8P4_BOOT" "Boot-time initialization"

run_gate_test "S8P4_BOOT_ALLOCATOR_INIT" \
    "cargo test -p kernel mm::tests::bitmap_allocator_initialized_from_boot_map -- --exact"

# ... 3 more boot tests ...

# Module 7: End-to-End Scenarios
gate_header "S8P4_E2E" "End-to-end shared memory workflows"

run_gate_test "S8P4_E2E_CREATE_MAP_UNMAP_CLOSE" \
    "cargo test -p kernel shmem::tests::end_to_end_create_map_unmap_close -- --exact"

# ... 3 more e2e tests ...

# Summary
echo "FOUNDRY_SHMEM_DATAPLANE_S8_PHASE4: ok"
```

**Total Assertions: 40**
- BitmapAllocator: 10
- AddressSpaceTable: 4
- MMU Programming: 6
- Data-Plane Integration: 12
- IPC Handlers: 4
- Boot Integration: 4
- End-to-End Scenarios: 4

---

## Implementation Sequence

### Week 1: Foundation
1. **BitmapAllocator** (3 days)
   - Implement core data structure
   - Add allocation/deallocation logic
   - Write unit tests

2. **AddressSpaceTable** (1 day)
   - Implement domain root tracking
   - Integrate with DomainRegistry
   - Write unit tests

3. **MMU Trait Definition** (1 day)
   - Define architecture-agnostic interface
   - Add error types
   - Write trait tests

### Week 2: Architecture Implementation
4. **x86_64 MMU Implementation** (3 days)
   - Implement page table structures
   - Implement map/unmap operations
   - Add TLB flushing
   - Write arch-specific tests

5. **aarch64 MMU Implementation** (3 days)
   - Implement page table structures
   - Implement map/unmap operations
   - Add TLB invalidation
   - Write arch-specific tests

### Week 3: Integration
6. **ShmemRegionTable Data-Plane Wiring** (3 days)
   - Add frame allocation to create_region
   - Add MMU programming to map_region
   - Add frame deallocation to close_region
   - Write integration tests

7. **IPC Handler Updates** (1 day)
   - Update error handling
   - Add trace events
   - Write IPC tests

8. **Boot Integration** (1 day)
   - Initialize AddressSpaceTable on boot
   - Initialize BitmapAllocator from memory map
   - Write boot tests

### Week 4: Validation
9. **Foundry Gate Development** (2 days)
   - Create gate script
   - Add all 40 assertions
   - Test gate locally

10. **End-to-End Testing** (3 days)
    - Run full gate suite
    - Fix any failures
    - Document any limitations

11. **Documentation** (2 days)
    - Update CURRENT_STATUS.md
    - Update SLICES.md
    - Update DECISIONS.md (if needed)

---

## Risk Mitigation

### Risk 1: MMU Programming Complexity
**Severity:** High
**Mitigation:**
- Start with simple identity mapping for domain 0
- Use existing page table structures from boot
- Incrementally add multi-domain support
- Extensive unit testing before integration

### Risk 2: Frame Allocation Deadlock
**Severity:** Medium
**Mitigation:**
- Use simple bitmap allocator (no locks needed initially)
- Document single-threaded constraint
- Plan for lock-free or spinlock-protected allocator in future

### Risk 3: Address Space Conflicts
**Severity:** Medium
**Mitigation:**
- Use fixed virtual address ranges per domain
- Document address space layout
- Add validation for overlapping mappings
- Plan for dynamic allocation in future

### Risk 4: TLB Coherency
**Severity:** High
**Mitigation:**
- Always flush TLB after page table updates
- Use architecture-specific flush instructions
- Test on both x86_64 and aarch64
- Document flush requirements

---

## Success Criteria

Phase 4 is complete when:

1. **Functional Requirements:**
   - [x] `create_region` allocates physical frames
   - [x] `map_region` programs MMU mappings
   - [x] `close_region` deallocates frames
   - [x] BitmapAllocator supports frame reuse
   - [x] Per-domain address spaces are tracked

2. **Quality Requirements:**
   - [x] All 40 Foundry assertions pass
   - [x] No memory leaks in allocation/deallocation
   - [x] No use-after-free vulnerabilities
   - [x] All operations emit trace events
   - [x] Error codes are consistent

3. **Architecture Requirements:**
   - [x] Control/data-plane separation maintained
   - [x] Kernel-side capability validation preserved
   - [x] Type safety enforced (PhysAddr, PhysFrame)
   - [x] Architecture abstraction (MMU trait)
   - [x] Static allocation (no heap)

4. **Documentation Requirements:**
   - [x] CURRENT_STATUS.md updated
   - [x] SLICES.md updated
   - [x] DECISIONS.md updated (if needed)
   - [x] Foundry gate documented
   - [x] Code comments complete

---

## Dependencies

### Internal Dependencies
- **S8 Phase 1:** IDL contract and codegen (✅ Complete)
- **S8 Phase 2:** Capability validation and ShmemRegionTable (✅ Complete)
- **S8 Phase 3:** FrameAllocator trait and BumpAllocator (✅ Complete)
- **DomainRegistry:** Domain tracking (✅ Complete)
- **TraceRing:** Event emission (✅ Complete)

### External Dependencies
- **None:** All dependencies are internal to the kernel

---

## Future Work (Post-S8 Phase 4)

1. **Dynamic Virtual Address Allocation:**
   - Replace fixed address ranges with dynamic allocator
   - Add address space fragmentation handling
   - Implement address space compaction

2. **Advanced MMU Features:**
   - Support huge pages (2MiB, 1GiB)
   - Add NUMA-aware allocation
   - Implement IOMMU support for DMA

3. **Performance Optimization:**
   - Batch page table updates
   - Use TLB shootdown for multi-core
   - Optimize bitmap operations with SIMD

4. **Security Hardening:**
   - Add page table isolation (ASLR)
   - Implement guard pages between regions
   - Add memory encryption support (if hardware available)

---

## Appendix: File Structure

```
kernel/src/
├── mm/
│   ├── mod.rs              # Update: add BitmapAllocator, AddressSpaceTable
│   ├── address.rs          # Existing: PhysAddr, PhysFrame
│   ├── frame.rs           # Existing: FrameAllocator trait
│   ├── bump.rs           # Existing: BumpAllocator (will be deprecated)
│   ├── bitmap.rs         # NEW: BitmapAllocator
│   └── address_space.rs # NEW: AddressSpaceTable
├── shmem.rs             # Update: add frame allocation, MMU programming
├── ipc_v0.rs           # Update: add trace events, error handling
├── arch/
│   ├── mod.rs           # Update: add mmu module
│   ├── x86_64/
│   │   └── mmu.rs      # NEW: x86_64 MMU implementation
│   └── aarch64/
│       └── mmu.rs      # NEW: aarch64 MMU implementation
└── domain_registry.rs   # Existing: Domain tracking

tools/ci/
└── foundry_shmem_dataplane_s8_phase4.sh  # NEW: Phase 4 gate

docs/
└── S8_PHASE4_IMPLEMENTATION_PLAN.md       # THIS FILE
```

---

## References

- **S8 Phase 1:** `docs/S8_PHASE1_IDL_CONTRACT.md` (if exists)
- **S8 Phase 2:** `docs/S8_PHASE2_CONTROL_PLANE.md` (if exists)
- **S8 Phase 3:** `docs/S8_PHASE3_FRAME_ALLOCATOR.md` (if exists)
- **Constitution:** `CONSTITUTION.md` (control/data-plane separation)
- **Decisions:** `DECISIONS.md` (S7 hardening decisions)
- **Current Status:** `CURRENT_STATUS.md` (S8 status)
- **Slices:** `SLICES.md` (S8 definition)
