# Multi-Domain Architecture

**Last Updated:** 2026-02-10
**Status:** In Progress (S8 Phase 5)
**Dependencies:** S6 (Domain Manager), S8 Phase 4 (MMU Integration)

## Overview

RamenOS provides hardware-enforced domain isolation through per-domain page tables. Each domain has its own virtual address space, preventing unauthorized memory access between domains. Shared memory regions enable controlled, capability-based communication.

## Domain Model

### Domain Lifecycle

```
CREATE → ACTIVE → DESTROY
  ↓        ↓         ↓
INIT    RUNNING    CLEANUP
```

**States:**
1. **INIT:** Domain allocated, page table root allocated
2. **ACTIVE:** Domain can map/unmap shared regions
3. **RUNNING:** Domain has active mappings and can execute
4. **DESTROY:** Domain cleanup in progress
5. **CLEANUP:** All resources freed, page tables released

### Domain ID Allocation

**Range:** 0-15 (MAX_DOMAINS = 16)

**Special Domains:**
- **Domain 0:** Kernel domain (always exists, contains kernel mappings)
- **Domain 1-15:** User domains (created dynamically)

**Allocation Strategy:**
```
find_free_domain() -> Option<DomainId> {
    for id in 1..MAX_DOMAINS {
        if DOMAIN_TABLE[id].is_none() {
            return Some(id);
        }
    }
    None
}
```

## Page Table Architecture

### x86_64 (4-Level Paging)

```
PML4 (Page Map Level 4)
  └─ PDP (Page Directory Pointer Table)
      └─ PD (Page Directory)
          └─ PT (Page Table)
              └─ Physical Frame (4 KiB)
```

**Virtual Address Split (48-bit):*
```
[ PML4 (9 bits) | PDP (9 bits) | PD (9 bits) | PT (9 bits) | Offset (12 bits) ]
```

**Each Level:**
- 512 entries per table (9 bits index)
- 8 bytes per entry (64-bit PTE/PDE/...)
- 4 KiB per table

**Total Address Range per Domain:** 256 TiB

### aarch64 (4-Level Translation)

```
TTBR0_EL1 (Table Base Address)
  └─ L4 (Level 4 Table)
      └─ L3 (Level 3 Table)
          └─ L2 (Level 2 Table)
              └─ L1 (Level 1 Table)
                  └─ Physical Frame (4 KiB)
```

**Virtual Address Split (48-bit):*
```
[ L4 (9 bits) | L3 (9 bits) | L2 (9 bits) | L1 (9 bits) | Offset (12 bits) ]
```

**Each Level:**
- 512 entries per table (9 bits index)
- 8 bytes per entry (64-bit descriptor)
- 4 KiB per table

## Page Table Allocation

### Current State (S8 Phase 4)

**Limitation:** MMU can only operate on existing page tables
- Assumes PML4/PDP/PD entries are pre-populated
- Returns error if intermediate tables are missing
- Only works for domain 0 (kernel domain)

**Example:**
```rust
// This works (domain 0 has full page tables)
map_pages(domain_id=0, vaddr=0x8000_0000, frames=[...]) → Ok(())

// This fails (domain 1 has no page tables yet)
map_pages(domain_id=1, vaddr=0x8000_0000, frames=[...]) → Err(InvalidDomain)
```

### Phase 5 Enhancement

**Goal:** Allocate missing intermediate tables on-demand

**Flow:**
```
map_pages(domain_id=1, vaddr=0x8000_0000, frames=[...])

1. Look up domain 1's PML4 root from AddressSpaceTable
2. Walk PML4 → PDP → PD → PT
3. PDP entry missing? Allocate new PDP table (4 KiB from FRAME_ALLOCATOR)
4. PD entry missing? Allocate new PD table (4 KiB from FRAME_ALLOCATOR)
5. PT entry missing? Allocate new PT table (4 KiB from FRAME_ALLOCATOR)
6. Set final PT entry to point to physical frame
7. Invalidate TLB for the mapped virtual address
```

**Allocation Strategy:**
- Use global FRAME_ALLOCATOR (BitmapAllocator)
- Return `STATUS_NO_MEMORY` if allocation fails
- Never allocate from domain-specific pools

**Cleanup on Domain Destroy:**
```
destroy_domain(domain_id=1):
1. Unmap all shared regions
2. Walk entire page table tree
3. Free all intermediate tables (PDP, PD, PT)
4. Free PML4 root
5. Mark domain as free in DOMAIN_TABLE
```

## Address Space Table

### Purpose

Track per-domain page table roots and enable domain lifecycle management.

### Structure

```rust
pub struct AddressSpaceTable {
    // Fixed-size array of page table roots (one per domain)
    roots: [Option<PhysAddr>; MAX_DOMAINS],
}

impl AddressSpaceTable {
    // Create new address space table
    pub fn new() -> Self;

    // Set page table root for a domain
    pub fn set_root(&mut self, domain_id: DomainId, root: PhysAddr);

    // Get page table root for a domain
    pub fn get_root(&self, domain_id: DomainId) -> Option<PhysAddr>;

    // Remove domain (on destroy)
    pub fn remove(&mut self, domain_id: DomainId) -> Option<PhysAddr>;
}
```

### Usage

**Domain Creation:**
```rust
// Allocate new page table root (4 KiB frame)
let pml4_frame = FRAME_ALLOCATOR.allocate_frame()?;

// Initialize PML4 with kernel mappings (identity map kernel code/data)
let pml4_vaddr = phys_to_virt(pml4_frame.start_address());
initialize_pml4(pml4_vaddr);

// Register with AddressSpaceTable
ADDRESS_SPACE_TABLE.set_root(domain_id, pml4_frame.start_address());
```

**Domain Destroy:**
```rust
// Remove from AddressSpaceTable
let pml4_root = ADDRESS_SPACE_TABLE.remove(domain_id)?;

// Free entire page table tree
walk_and_free_page_tables(pml4_root);

// Free PML4 root itself
FRAME_ALLOCATOR.deallocate_frame(pml4_root);
```

## Shared Memory Mapping

### Map Region

**Operation:** Map physical frames into domain's virtual address space

**Parameters:**
- `region_id`: Shared memory region identifier
- `shm_cap`: Capability proving ownership
- `target_domain_id`: Domain to map into
- `rights`: Access rights (READ/WRITE/EXECUTE)
- `cache_mode`: Cache mode (UNCACHED/WC/WB)

**Flow:**
```
map_region(region_id, shm_cap, target_domain_id, rights, cache_mode):

1. Validate capability
2. Look up region from ShmemRegionTable
3. Get region's physical frames
4. Choose virtual address (e.g., 0x8000_0000 + region_id * 0x1000)
5. Call MMU::map_pages(target_domain_id, vaddr, frames, rights, cache_mode)
   a. Look up target_domain_id's page table root
   b. Allocate missing intermediate tables (Phase 5)
   c. Walk page table hierarchy
   d. Set final PT entries to point to physical frames
   e. Invalidate TLB
6. Increment region refcount
```

### Unmap Region

**Operation:** Unmap virtual address range from domain's address space

**Parameters:**
- `mapping_id`: Mapping identifier (region_id for now)
- `target_domain_id`: Domain to unmap from

**Flow:**
```
unmap_region(mapping_id, target_domain_id):

1. Look up region from ShmemRegionTable
2. Get virtual address used in map_region
3. Call MMU::unmap_pages(target_domain_id, vaddr, num_pages)
   a. Look up target_domain_id's page table root
   b. Walk page table hierarchy
   c. Clear PT entries (set to not present)
   d. Free empty intermediate tables (Phase 5)
   e. Invalidate TLB
4. Decrement region refcount
```

## Domain Isolation

### Hardware Enforcement

**x86_64:**
- Each domain has its own CR3 value (page table root)
- Switching domains requires updating CR3
- TLB entries tagged with address space ID (PCID on x86_64)

**aarch64:**
- Each domain has its own TTBR0_EL1 value
- Switching domains requires updating TTBR0_EL1
- TLB entries tagged with ASID

### Isolation Properties

**Guarantees:**
1. Domain A cannot access Domain B's mappings
2. Kernel (Domain 0) can access all domains (for supervision)
3. Shared memory regions are explicit (requires capability)
4. Rights enforced per-mapping (READ/WRITE/EXECUTE)

**Attacks Mitigated:**
- ❌ Unauthorized memory read (wrong domain)
- ❌ Unauthorized memory write (wrong domain)
- ❌ Privilege escalation (domain escape)
- ❌ Pointer smuggling (passing pointers across domains)

### Shared Memory Exception

Shared memory regions are the **only controlled exception** to domain isolation:
- Requires valid capability
- Rights checked on map_region
- Both domains must agree to share
- Refcount tracks active mappings

**Example:**
```
Domain 0 creates region (CAP_CREATE)
Domain 0 maps region with READ/WRITE → OK
Domain 1 requests map with CAP_SHARE → OK
Domain 1 maps region with READ → OK
Domain 1 attempts WRITE → FAIL (wrong rights)
Domain 2 attempts map (no cap) → FAIL (no capability)
```

## Testing Strategy

### Unit Tests

1. **AddressSpaceTable:** Set/get/remove roots
2. **Domain Lifecycle:** Create/destroy domains
3. **Page Table Allocation:** Allocate intermediate tables
4. **Mapping/Unmapping:** Map/unmap regions

### Integration Tests (QEMU)

1. **Multi-Domain Transfer:** Domain 0 → Domain 1
2. **Domain Isolation:** Verify no cross-domain access
3. **Refcount Tracking:** Verify correct refcount management
4. **Cleanup:** Verify all resources freed on destroy

### Security Tests

1. **Unauthorized Access:** Attempt wrong domain access (should fail)
2. **Rights Enforcement:** Attempt write on read-only mapping (should fail)
3. **Capability Validation:** Attempt map without capability (should fail)
4. **Privilege Escalation:** Attempt to access kernel memory (should fail)

## Performance Considerations

### Page Table Walk

**Cost:** 4 memory accesses per translation (PML4 → PDP → PD → PT)

**Optimizations:**
- TLB caches recent translations (hardware)
- Huge pages (2MiB/1GiB) reduce levels (future work)
- Shared page tables for similar domains (future work)

### Context Switch

**Cost:** Update CR3/TTBR0_EL1 on domain switch

**Optimizations:**
- PCID/ASID to avoid TLB flush on switch
- Lazy TLB invalidation (batch operations)
- Pin domains to CPUs (NUMA awareness)

### Allocation Overhead

**Cost:** 4 KiB per intermediate table

**Optimizations:**
- Pre-allocate common mappings (kernel code/data)
- Share page tables between domains (future work)
- Free empty tables aggressively

## Implementation Checklist

### Phase 5.2: Multi-Domain Support

- [ ] Add page table allocation to MMU trait
- [ ] Implement PML4/PDP/PD allocation in x86_64/mmu.rs
- [ ] Implement TTBR0/PUD/PMD allocation in aarch64/mmu.rs
- [ ] Domain manager integration with AddressSpaceTable
- [ ] Create domain 1 with dedicated page table root
- [ ] Map region into domain 1 (not domain 0)
- [ ] Verify domain isolation (domain 1 cannot access domain 0 mappings)
- [ ] Test domain cleanup (free page tables on domain destroy)
- [ ] Create Foundry gate: `foundry_multidomain_s8_phase5_2.sh`

## Future Work

### Huge Pages

**2 MiB Pages:**
- Skip PT level (direct PD → frame)
- Reduce TLB pressure
- Better performance for large regions

**1 GiB Pages:**
- Skip PD and PT levels (direct PDP → frame)
- For very large regions (framebuffer, etc.)

### Shared Page Tables

**Copy-on-Write:**
- Multiple domains share read-only mappings
- Fork on write (allocate new page tables)
- Memory efficiency

### Namespace Isolation

**Per-Domain Namespaces:**
- Separate file descriptor tables
- Separate capability tables
- Separate resource tracking

---

**Document Status:** Draft (S8 Phase 5)
**Author:** Claude (RamenOS Architecture)
**Date:** 2026-02-10
**Related:**
- `docs/archive/plans/2026-02-10-s8-phase5-ring-buffer.md` (historical plan)
- `kernel/src/mm/address_space.rs`
- `kernel/src/arch/x86_64/mmu.rs`
