//! aarch64-specific MMU implementation for shared-memory data-plane.
//!
//! This module implements the Mmu trait for aarch64 architecture using 4-level
//! paging (L0 → L1 → L2 → L3 → Physical Page). All operations are unsafe
//! because they perform direct hardware manipulation of page tables and TLB.
//!
//! # Page Table Structure
//!
//! ```text
//! L0 (Translation Table Level 0) → L1 (Level 1) → L2 (Level 2) → L3 (Level 3) → Physical Page
//! ```
//!
//! # Safety
//!
//! All trait methods are `unsafe` because they perform direct hardware
//! manipulation of page tables and TLB. Callers must ensure:
//!
//! - Page table addresses are valid and properly aligned (4 KiB for granule)
//! - The caller holds any necessary locks for concurrent access
//! - Virtual addresses are within valid ranges for the architecture
//! - Physical addresses point to valid memory or MMIO regions
//! - TLB invalidation (TLBI) and barriers (DSB/ISB) are performed after modifications
#![cfg_attr(not(target_arch = "aarch64"), allow(dead_code, unused_imports))]

use core::arch::asm;

#[cfg(test)]
use crate::arch::mmu::{
    CACHE_MODE_UNCACHED, CACHE_MODE_WRITE_BACK, CACHE_MODE_WRITE_COMBINE, RIGHTS_READ,
};
use crate::arch::mmu::{Mmu, MmuError, VirtAddr};
use crate::domain_registry::{DomainId, MAX_DOMAINS};
use crate::mm::address::PhysAddr;
use crate::mm::address::PhysFrame;
use crate::mm::frame::FrameAllocator;

use crate::arch::aarch64_page_entry::PageTableEntry;

/// Page table structure for aarch64 4-level paging.
///
/// Each page table contains 512 entries (512 * 8 bytes = 4096 bytes).
#[repr(C, align(4096))]
struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new page table with all entries unused.
    #[cfg(test)]
    const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }
}

/// aarch64 MMU implementation.
///
/// This type implements the Mmu trait for aarch64 architecture.
pub struct AArch64Mmu;

#[cfg_attr(test, allow(dead_code))]
impl AArch64Mmu {
    /// Test-only per-call injection into the identical production mapper core.
    #[cfg(all(feature = "test_protocols", not(test)))]
    unsafe fn map_pages_with_table_budget(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
        budget: usize,
        reserved: &mut usize,
    ) -> Result<(), MmuError> {
        let mut fault = crate::arch::mmu::MappingFault {
            budget,
            reserved: 0,
        };
        let result = Self::map_pages_core(
            domain_id,
            vaddr,
            frames,
            rights,
            cache_mode,
            Some(&mut fault),
        );
        *reserved = fault.reserved;
        result
    }

    /// Caller holds exclusive hierarchy access from preflight through commit.
    /// Existing roots and frames are identity mapped; these walkers do not
    /// provide SMP exclusion. No entry is published until reservation succeeds.
    #[cfg(not(test))]
    unsafe fn map_pages_core(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
        #[cfg(feature = "test_protocols")] mut fault: Option<&mut crate::arch::mmu::MappingFault>,
    ) -> Result<(), MmuError> {
        if domain_id >= MAX_DOMAINS as u64 {
            return Err(MmuError::InvalidDomain);
        }
        let end = crate::arch::mmu::mapping_range_end(vaddr, frames.len())?;
        if vaddr.as_u64() >> 48 != 0 || (end - 1) >> 48 != 0 {
            return Err(MmuError::InvalidAddress);
        }
        if frames
            .iter()
            .any(|frame| frame.start_address().as_u64() & !0x0000_ffff_ffff_f000 != 0)
        {
            return Err(MmuError::InvalidAddress);
        }

        let root = if domain_id == 0 {
            Self::read_ttbr0() & 0x0000_ffff_ffff_f000
        } else {
            let guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
            match guard.as_ref().and_then(|table| table.get_root(domain_id)) {
                Some(root) => root.as_u64() & 0x0000_ffff_ffff_f000,
                None => return Err(MmuError::InvalidDomain),
            }
        } as *mut PageTable;
        const SHIFTS: [u32; 3] = [39, 30, 21];
        let mut plan = crate::arch::mmu::MappingPlan::new();
        // Read-only preflight covers every page, including descendants whose
        // ancestors do not exist yet. A block is rejected before dereference.
        for page in 0..frames.len() {
            let address = vaddr.as_u64() + page as u64 * 4096;
            let mut table = Some(root as *const PageTable);
            for (level, shift) in SHIFTS.iter().copied().enumerate() {
                let prefix = address >> shift;
                if let Some(current) = table {
                    let entry = (*current).entries[(prefix & 511) as usize];
                    if entry.is_valid() {
                        if entry.value & 3 != 3 {
                            return Err(MmuError::PermissionDenied);
                        }
                        table = Some(entry.addr().as_u64() as *const PageTable);
                    } else {
                        plan.record(level, prefix)?;
                        table = None;
                    }
                } else {
                    plan.record(level, prefix)?;
                }
            }
            if let Some(table) = table {
                if (*table).entries[((address >> 12) & 511) as usize].value != 0 {
                    return Err(MmuError::PermissionDenied);
                }
            }
        }
        // Reserve and zero every required frame before mutating any old table.
        // Rollback owns exactly this prefix of reservations, never old entries.
        for index in 0..plan.len {
            #[cfg(feature = "test_protocols")]
            if fault
                .as_ref()
                .is_some_and(|fault| fault.reserved == fault.budget)
            {
                for node in &plan.nodes[..index] {
                    let _ = Self.free_page_table(PhysAddr::new(node.frame));
                }
                return Err(MmuError::AllocationFailed);
            }
            match Self.allocate_page_table() {
                Ok(frame) => {
                    plan.nodes[index].frame = frame.as_u64();
                    #[cfg(feature = "test_protocols")]
                    if let Some(fault) = fault.as_mut() {
                        fault.reserved += 1;
                    }
                }
                Err(error) => {
                    for node in &plan.nodes[..index] {
                        let _ = Self.free_page_table(PhysAddr::new(node.frame));
                    }
                    return Err(error);
                }
            }
        }
        // Discovery orders parents before children. Publishing these reserved
        // tables and then leaves cannot allocate or fail under exclusive access.
        // Existing ancestor addresses and flags are never rewritten.
        for node in &plan.nodes[..plan.len] {
            let address = node.prefix << SHIFTS[node.level];
            let mut table = root;
            for shift in &SHIFTS[..node.level] {
                table = (*table).entries[((address >> shift) & 511) as usize]
                    .addr()
                    .as_u64() as *mut PageTable;
            }
            let entry = &mut (*table).entries[(node.prefix & 511) as usize];
            entry.set_addr(PhysAddr::new(node.frame));
            entry.set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
        }
        let flags = Self::rights_to_flags(rights, cache_mode);
        for (page, frame) in frames.iter().enumerate() {
            let current = VirtAddr::new(vaddr.as_u64() + page as u64 * 4096);
            let mut table = root;
            for shift in SHIFTS {
                table = (*table).entries[((current.as_u64() >> shift) & 511) as usize]
                    .addr()
                    .as_u64() as *mut PageTable;
            }
            let entry = &mut (*table).entries[((current.as_u64() >> 12) & 511) as usize];
            entry.set_addr(frame.start_address());
            entry.set_flags(flags);
            Self::tlbi_vaae1is(current);
        }
        Self::dsb_is();
        Self::isb();
        Ok(())
    }

    /// Exercise the actual mapper against isolated, never-activated page tables.
    /// Raw entries and backing bytes are observed independently of its walker.
    ///
    /// # Safety
    /// Boot-only fixture: allocator and address-space registry must be initialized,
    /// physical memory identity mapped, and all page-table access exclusive.
    #[cfg(all(feature = "test_protocols", not(test)))]
    pub unsafe fn run_mapping_regressions() -> bool {
        unsafe fn read(page: PhysAddr, index: usize) -> u64 {
            (page.as_u64() as *const u64).add(index).read_volatile()
        }
        unsafe fn write(page: PhysAddr, index: usize, value: u64) {
            (page.as_u64() as *mut u64).add(index).write_volatile(value);
        }
        unsafe fn filled(page: PhysAddr, value: u8) -> bool {
            (0..4096).all(|i| (page.as_u64() as *const u8).add(i).read_volatile() == value)
        }
        fn report(case: &str, ok: bool, passed: &mut usize) {
            crate::kprintln!("mmu_review: {} {}", case, if ok { "PASS" } else { "FAIL" });
            *passed += usize::from(ok);
        }
        const TABLE: u64 = 0x3;
        const LEAF: u64 = 0x0060_0000_0000_079f;
        const BLOCK: u64 = 0x401;
        const DOMAIN: DomainId = (MAX_DOMAINS - 1) as DomainId;
        const OTHER: DomainId = (MAX_DOMAINS - 2) as DomainId;
        let mut pages = [PhysAddr::new(0); 10];
        let mut allocated = 0;
        for index in 0..pages.len() {
            match Self.allocate_page_table() {
                Ok(address) => {
                    pages[index] = address;
                    allocated += 1;
                }
                Err(_) => {
                    for address in &pages[..allocated] {
                        let _ = Self.free_page_table(*address);
                    }
                    crate::kprintln!("mmu_review: setup FAIL");
                    crate::kprintln!("mmu_review: COMPLETE 0/17");
                    return false;
                }
            }
        }
        // A 512-frame run contains a 2 MiB-aligned frame. A block descriptor
        // targeting that frame is architecturally aligned; buggy descent can
        // only mutate fixture-owned sentinel bytes, never arbitrary memory.
        let run = {
            let mut guard = crate::mm::FRAME_ALLOCATOR.lock();
            guard
                .as_mut()
                .and_then(|allocator| allocator.allocate_contiguous(512))
        };
        let Some(run) = run else {
            for address in pages {
                let _ = Self.free_page_table(address);
            }
            crate::kprintln!("mmu_review: setup FAIL");
            crate::kprintln!("mmu_review: COMPLETE 0/17");
            return false;
        };
        let sentinel = PhysAddr::new((run.start_address().as_u64() + 0x1f_ffff) & !0x1f_ffff);
        let [
            root,
            upper,
            middle,
            lower,
            other_root,
            other_upper,
            other_middle,
            other_lower,
            old_data,
            new_data,
        ] = pages;
        (old_data.as_u64() as *mut u8).write_bytes(0xa5, 4096);
        (new_data.as_u64() as *mut u8).write_bytes(0x33, 4096);
        for (r, u, m, l) in [
            (root, upper, middle, lower),
            (other_root, other_upper, other_middle, other_lower),
        ] {
            write(r, 0, u.as_u64() | TABLE);
            write(u, 0, m.as_u64() | TABLE);
            write(m, 0, l.as_u64() | TABLE);
        }
        write(other_lower, 7, old_data.as_u64() | LEAF);
        let saved = {
            let mut guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
            let Some(table) = guard.as_mut() else {
                drop(guard);
                for address in pages {
                    let _ = Self.free_page_table(address);
                }
                for i in 0..512 {
                    let address = PhysAddr::new(run.start_address().as_u64() + i * 4096);
                    let _ = Self.free_page_table(address);
                }
                crate::kprintln!("mmu_review: setup FAIL");
                crate::kprintln!("mmu_review: COMPLETE 0/17");
                return false;
            };
            let saved = (table.get_root(DOMAIN), table.get_root(OTHER));
            table.set_root(DOMAIN, root);
            table.set_root(OTHER, other_root);
            saved
        };
        let frame = PhysFrame::from_start_address(new_data);
        let mut passed = 0;
        let mut unrelated = true;
        for (case, start, count, block) in [
            ("leaf_collision", 0x2000, 1, false),
            ("later_leaf_collision_atomic", 0x1000, 2, false),
            ("block_collision", 0x20_0000, 1, true),
            ("later_block_collision_atomic", 0x1f_f000, 2, true),
        ] {
            write(lower, 1, 0);
            write(lower, 2, if block { 0 } else { old_data.as_u64() | LEAF });
            write(lower, 511, 0);
            (sentinel.as_u64() as *mut u8).write_bytes(0x5a, 4096);
            let descriptor = if block { sentinel.as_u64() | BLOCK } else { 0 };
            write(middle, 1, descriptor);
            let result = Self::map_pages(
                DOMAIN,
                VirtAddr::new(start),
                &[frame, frame][..count],
                crate::arch::mmu::RIGHTS_READ,
                crate::arch::mmu::CACHE_MODE_WRITE_BACK,
            );
            let hierarchy = read(root, 0) == upper.as_u64() | TABLE
                && read(upper, 0) == middle.as_u64() | TABLE
                && read(middle, 0) == lower.as_u64() | TABLE
                && read(middle, 1) == descriptor;
            let untouched = hierarchy
                && read(lower, 1) == 0
                && read(lower, 511) == 0
                && read(lower, 2) == if block { 0 } else { old_data.as_u64() | LEAF }
                && filled(old_data, 0xa5)
                && filled(new_data, 0x33)
                && filled(sentinel, 0x5a);
            report(
                case,
                result == Err(MmuError::PermissionDenied) && untouched,
                &mut passed,
            );
            unrelated &= read(other_root, 0) == other_upper.as_u64() | TABLE
                && read(other_upper, 0) == other_middle.as_u64() | TABLE
                && read(other_middle, 0) == other_lower.as_u64() | TABLE
                && read(other_lower, 7) == old_data.as_u64() | LEAF;
        }
        report("unrelated_root_preserved", unrelated, &mut passed);
        write(middle, 1, 0);
        write(lower, 1, 0);
        write(lower, 2, 0);
        write(lower, 511, 0);
        let mapped = Self::map_pages(
            DOMAIN,
            VirtAddr::new(0x4000),
            &[frame],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "successful_map_control",
            mapped.is_ok() && read(lower, 4) == new_data.as_u64() | LEAF && filled(new_data, 0x33),
            &mut passed,
        );
        report(
            "old_mapping_bytes_preserved",
            filled(old_data, 0xa5),
            &mut passed,
        );
        // Authored before the per-call fault hook: it must count real reserved
        // table frames, return all of them on failure, and publish no parent.
        let available = || {
            crate::mm::FRAME_ALLOCATOR
                .lock()
                .as_ref()
                .unwrap()
                .available_frames()
        };
        let before = available();
        let mut reserved = 0;
        let fault_va = VirtAddr::new(4 << 39);
        let failed = Self::map_pages_with_table_budget(
            DOMAIN,
            fault_va,
            &[frame],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
            1,
            &mut reserved,
        );
        let old_entries = read(root, 0) == upper.as_u64() | TABLE
            && read(upper, 0) == middle.as_u64() | TABLE
            && read(middle, 0) == lower.as_u64() | TABLE
            && read(lower, 4) == new_data.as_u64() | LEAF
            && read(other_lower, 7) == old_data.as_u64() | LEAF;
        report(
            "allocation_failure_atomic",
            failed == Err(MmuError::AllocationFailed)
                && reserved == 1
                && available() == before
                && old_entries
                && (1..512).all(|index| read(root, index) == 0),
            &mut passed,
        );
        let recovered = Self::map_pages(
            DOMAIN,
            fault_va,
            &[frame],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        unsafe fn raw_leaf(root: PhysAddr, address: u64) -> u64 {
            let mut table = root;
            for shift in [39, 30, 21] {
                let entry = read(table, ((address >> shift) & 511) as usize);
                if entry & 1 == 0 {
                    return 0;
                }
                table = PhysAddr::new(entry & 0x0000_ffff_ffff_f000);
            }
            read(table, ((address >> 12) & 511) as usize)
        }
        report(
            "allocation_failure_recovery",
            recovered.is_ok()
                && raw_leaf(root, fault_va.as_u64()) == new_data.as_u64() | LEAF
                && filled(old_data, 0xa5),
            &mut passed,
        );
        // Starting one page below a fresh 512 GiB boundary touches two upper,
        // two middle and three lowest tables: the fixed plan's seven-node bound.
        let frames = [frame; 1025];
        let boundary = (3 << 39) - 4096;
        let before = available();
        let mapped = Self::map_pages(
            DOMAIN,
            VirtAddr::new(boundary),
            &frames[..1024],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "boundary_1024_seven_tables",
            mapped.is_ok()
                && before.checked_sub(available()) == Some(7)
                && (0..1024).all(|index| {
                    raw_leaf(root, boundary + index * 4096) == new_data.as_u64() | LEAF
                }),
            &mut passed,
        );
        let before = available();
        let rejected = Self::map_pages(
            DOMAIN,
            VirtAddr::new(6 << 39),
            &frames,
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "oversize_rejected",
            rejected == Err(MmuError::InvalidAddress)
                && read(root, 6) == 0
                && available() == before,
            &mut passed,
        );
        let rejected = Self::map_pages(
            DOMAIN,
            VirtAddr::new(u64::MAX - 4095),
            &frames[..2],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "overflow_rejected",
            rejected == Err(MmuError::InvalidAddress)
                && read(root, 511) == 0
                && available() == before,
            &mut passed,
        );
        let rejected = Self::map_pages(
            DOMAIN,
            VirtAddr::new(1 << 48),
            &[frame],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "architecture_range_rejected",
            rejected == Err(MmuError::InvalidAddress)
                && read(lower, 0) == 0
                && available() == before,
            &mut passed,
        );
        let rejected = Self::map_pages(
            DOMAIN,
            VirtAddr::new(0x4001),
            &[frame],
            crate::arch::mmu::RIGHTS_READ,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "unaligned_rejected",
            rejected == Err(MmuError::InvalidAddress)
                && read(lower, 4) == new_data.as_u64() | LEAF
                && available() == before,
            &mut passed,
        );
        let mapped = Self::map_pages(
            DOMAIN,
            VirtAddr::new(0x5000),
            &[frame],
            crate::arch::mmu::RIGHTS_READ | crate::arch::mmu::RIGHTS_WRITE,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "write_right_preserved",
            mapped.is_ok() && read(lower, 5) == new_data.as_u64() | (LEAF & !0x80),
            &mut passed,
        );
        let mapped = Self::map_pages(
            DOMAIN,
            VirtAddr::new(0x6000),
            &[frame],
            crate::arch::mmu::RIGHTS_READ | crate::arch::mmu::RIGHTS_EXECUTE,
            crate::arch::mmu::CACHE_MODE_WRITE_BACK,
        );
        report(
            "execute_right_preserved",
            mapped.is_ok()
                && read(lower, 6) == new_data.as_u64() | (LEAF & !((1 << 53) | (1 << 54))),
            &mut passed,
        );
        (sentinel.as_u64() as *mut u8).write_bytes(0x5a, 4096);
        write(middle, 1, sentinel.as_u64() | BLOCK);
        let _ = Self::unmap_pages(DOMAIN, VirtAddr::new(0x20_0000), 1);
        report(
            "unmap_block_preserved",
            read(middle, 1) == sentinel.as_u64() | BLOCK && filled(sentinel, 0x5a),
            &mut passed,
        );
        write(middle, 1, 0);
        // Release new table branches from the fault/recovery and boundary cases.
        // The original branch zero and the other root remain separately owned.
        for root_index in 1..512 {
            let entry = read(root, root_index);
            if entry & 1 == 0 {
                continue;
            }
            let upper_page = PhysAddr::new(entry & 0x0000_ffff_ffff_f000);
            for upper_index in 0..512 {
                let entry = read(upper_page, upper_index);
                if entry & 1 == 0 {
                    continue;
                }
                let middle_page = PhysAddr::new(entry & 0x0000_ffff_ffff_f000);
                for middle_index in 0..512 {
                    let entry = read(middle_page, middle_index);
                    if entry & 1 != 0 {
                        let _ = Self.free_page_table(PhysAddr::new(entry & 0x0000_ffff_ffff_f000));
                    }
                }
                let _ = Self.free_page_table(middle_page);
            }
            write(root, root_index, 0);
            let _ = Self.free_page_table(upper_page);
        }
        {
            let mut guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
            let table = guard.as_mut().unwrap();
            for (domain, address) in [(DOMAIN, saved.0), (OTHER, saved.1)] {
                match address {
                    Some(address) => table.set_root(domain, address),
                    None => table.clear_root(domain),
                }
            }
        }
        // These roots were never installed in CR3/TTBR. Clear owned roots before
        // releasing every explicitly allocated table/backing frame.
        (root.as_u64() as *mut u8).write_bytes(0, 4096);
        (other_root.as_u64() as *mut u8).write_bytes(0, 4096);
        for address in pages {
            let _ = Self.free_page_table(address);
        }
        for i in 0..512 {
            let address = PhysAddr::new(run.start_address().as_u64() + i * 4096);
            let _ = Self.free_page_table(address);
        }
        crate::kprintln!("mmu_review: COMPLETE {}/17", passed);
        passed == 17
    }

    /// Convert rights and cache mode to a level-three page descriptor.
    #[must_use]
    fn rights_to_flags(rights: u32, cache_mode: u32) -> u64 {
        PageTableEntry::page_flags(rights, cache_mode)
    }

    /// Get the L0 index from a virtual address.
    #[must_use]
    fn l0_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 39) & 0x1FF) as usize
    }

    /// Get the L1 index from a virtual address.
    #[must_use]
    fn l1_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 30) & 0x1FF) as usize
    }

    /// Get the L2 index from a virtual address.
    #[must_use]
    fn l2_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 21) & 0x1FF) as usize
    }

    /// Get the L3 index from a virtual address.
    #[must_use]
    fn l3_index(vaddr: VirtAddr) -> usize {
        ((vaddr.as_u64() >> 12) & 0x1FF) as usize
    }

    /// Load TTBR0_EL1 with the specified page table root address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ttbr0_value` is a valid physical address
    /// of a page table root.
    unsafe fn load_ttbr0(ttbr0_value: u64) {
        // SAFETY: The caller ensures ttbr0_value is a valid page table root.
        asm!("msr ttbr0_el1, {}", in(reg) ttbr0_value, options(nomem, nostack));
    }

    /// Read the current TTBR0_EL1 value.
    ///
    /// # Safety
    ///
    /// This is safe to call at any time, but the returned value should
    /// only be used in contexts where the caller knows the page table layout.
    #[must_use]
    unsafe fn read_ttbr0() -> u64 {
        let ttbr0_value: u64;
        // SAFETY: Reading TTBR0 is always safe.
        asm!("mrs {}, ttbr0_el1", out(reg) ttbr0_value, options(nomem, nostack));
        ttbr0_value
    }

    /// Invalidate all TLB entries (TLBI VMALLE1IS).
    ///
    /// # Safety
    ///
    /// The caller must ensure that no code is executing from pages
    /// that will be invalidated.
    #[inline(always)]
    unsafe fn tlbi_vmalle1is() {
        // SAFETY: The caller ensures it's safe to invalidate all TLB entries.
        asm!("tlbi vmalle1is", options(nomem, nostack));
    }

    /// Invalidate TLB entry for a specific address (TLBI VAAE1IS).
    ///
    /// # Safety
    ///
    /// The caller must ensure that `vaddr` is a valid virtual address
    /// that may have been mapped or unmapped.
    #[inline(always)]
    unsafe fn tlbi_vaae1is(vaddr: VirtAddr) {
        // SAFETY: The caller ensures vaddr is valid for invalidation.
        asm!("tlbi vaae1is, {}", in(reg) vaddr.as_u64() >> 12, options(nomem, nostack));
    }

    /// Data synchronization barrier inner shareable (DSB IS).
    ///
    /// # Safety
    ///
    /// This is safe to call at any time.
    #[inline(always)]
    unsafe fn dsb_is() {
        // SAFETY: DSB IS is always safe.
        asm!("dsb ish", options(nomem, nostack));
    }

    /// Instruction synchronization barrier (ISB).
    ///
    /// # Safety
    ///
    /// This is safe to call at any time.
    #[inline(always)]
    unsafe fn isb() {
        // SAFETY: ISB is always safe.
        asm!("isb", options(nomem, nostack));
    }
}

impl Mmu for AArch64Mmu {
    unsafe fn map_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        frames: &[PhysFrame],
        rights: u32,
        cache_mode: u32,
    ) -> Result<(), MmuError> {
        // Validate domain_id and get page table root
        // Note: For now, we use a simple approach - assume domain 0 has a known page table
        // In a full implementation, we would use AddressSpaceTable to look up the root
        if domain_id >= MAX_DOMAINS as u64 {
            return Err(MmuError::InvalidDomain);
        }

        // Validate virtual address alignment
        if !vaddr.is_page_aligned() {
            return Err(MmuError::InvalidAddress);
        }

        // Validate frames slice is not empty
        if frames.is_empty() {
            return Err(MmuError::InvalidAddress);
        }

        #[cfg(test)]
        {
            // Host tests validate control-plane behavior, not real page table mutation.
            // Avoid dereferencing physical addresses or issuing privileged instructions.
            if domain_id != 0 {
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                if let Some(ref table) = *table_guard {
                    if table
                        .get_root(domain_id as crate::domain_registry::DomainId)
                        .is_none()
                    {
                        return Err(MmuError::InvalidDomain);
                    }
                } else {
                    return Err(MmuError::InvalidDomain);
                }
                drop(table_guard);
            }
            let _ = (vaddr, rights, cache_mode);
            Ok(())
        }

        #[cfg(not(test))]
        {
            Self::map_pages_core(
                domain_id,
                vaddr,
                frames,
                rights,
                cache_mode,
                #[cfg(feature = "test_protocols")]
                None,
            )
        }
    }

    unsafe fn unmap_pages(
        domain_id: DomainId,
        vaddr: VirtAddr,
        num_pages: usize,
    ) -> Result<(), MmuError> {
        // Validate domain_id
        if domain_id >= MAX_DOMAINS as u64 {
            return Err(MmuError::InvalidDomain);
        }

        // Validate virtual address alignment
        if !vaddr.is_page_aligned() {
            return Err(MmuError::InvalidAddress);
        }

        // Validate num_pages is not zero
        if num_pages == 0 {
            return Err(MmuError::InvalidAddress);
        }

        #[cfg(test)]
        {
            if domain_id != 0 {
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                if let Some(ref table) = *table_guard {
                    if table
                        .get_root(domain_id as crate::domain_registry::DomainId)
                        .is_none()
                    {
                        return Err(MmuError::InvalidDomain);
                    }
                } else {
                    return Err(MmuError::InvalidDomain);
                }
                drop(table_guard);
            }
            let _ = (vaddr, num_pages);
            Ok(())
        }

        #[cfg(not(test))]
        {
            // Get page table root for the domain
            let l0_phys = if domain_id == 0 {
                Self::read_ttbr0() & 0x0000_FFFF_FFFF_F000
            } else {
                // Look up from AddressSpaceTable
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                let result = match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x0000_FFFF_FFFF_F000,
                            None => return Err(MmuError::InvalidDomain),
                        }
                    }
                    None => return Err(MmuError::InvalidDomain),
                };
                drop(table_guard);
                result
            };

            // Convert physical address to virtual address for direct access
            let l0_virt = l0_phys as *mut PageTable;

            // Unmap each page
            for i in 0..num_pages {
                // SAFETY: Calculated virtual address is within the valid range for this mapping
                let current_vaddr = unsafe { VirtAddr::new(vaddr.as_u64() + (i as u64 * 4096)) };

                // Walk the page table hierarchy
                let l0 = &mut *l0_virt;
                let l0_idx = Self::l0_index(current_vaddr);

                // Check if L1 entry is present
                if !l0.entries[l0_idx].is_valid() || l0.entries[l0_idx].value & 3 != 3 {
                    // Page not mapped - continue to next page
                    continue;
                }

                let l1_phys = l0.entries[l0_idx].addr();
                let l1 = &mut *(l1_phys.as_u64() as *mut PageTable);
                let l1_idx = Self::l1_index(current_vaddr);

                // Check if L2 entry is present
                if !l1.entries[l1_idx].is_valid() || l1.entries[l1_idx].value & 3 != 3 {
                    continue;
                }

                let l2_phys = l1.entries[l1_idx].addr();
                let l2 = &mut *(l2_phys.as_u64() as *mut PageTable);
                let l2_idx = Self::l2_index(current_vaddr);

                // Check if L3 entry is present
                if !l2.entries[l2_idx].is_valid() || l2.entries[l2_idx].value & 3 != 3 {
                    continue;
                }

                let l3_phys = l2.entries[l2_idx].addr();
                let l3 = &mut *(l3_phys.as_u64() as *mut PageTable);
                let l3_idx = Self::l3_index(current_vaddr);

                // Clear the page table entry
                let entry = &mut l3.entries[l3_idx];
                *entry = PageTableEntry::new();

                // Invalidate the TLB entry for this page
                Self::tlbi_vaae1is(current_vaddr);
            }

            // Ensure TLB invalidation is complete
            Self::dsb_is();
            Self::isb();

            Ok(())
        }
    }

    unsafe fn flush_tlb(domain_id: DomainId) {
        #[cfg(not(test))]
        {
            // Validate domain_id
            if domain_id >= 16 {
                return;
            }

            // Get page table root for the domain
            let l0_phys = if domain_id == 0 {
                Self::read_ttbr0() & 0x0000_FFFF_FFFF_F000
            } else {
                // Look up from AddressSpaceTable (protected by Mutex)
                let table_guard = crate::mm::ADDRESS_SPACE_TABLE.lock();
                match *table_guard {
                    Some(ref table) => {
                        match table.get_root(domain_id as crate::domain_registry::DomainId) {
                            Some(root) => root.as_u64() & 0x0000_FFFF_FFFF_F000,
                            None => return,
                        }
                    }
                    None => return,
                }
            };

            // Invalidate all TLB entries for this address space
            Self::tlbi_vmalle1is();

            // Ensure TLB invalidation is complete
            Self::dsb_is();
            Self::isb();

            // Reload TTBR0 to ensure the page table is active
            Self::load_ttbr0(l0_phys);
        }

        #[cfg(test)]
        let _ = domain_id;
    }

    fn allocate_page_table(&self) -> Result<PhysAddr, MmuError> {
        // Allocate a single 4 KiB frame from the global allocator
        // SAFETY: The mutex ensures exclusive access to the allocator
        let mut allocator_guard = crate::mm::FRAME_ALLOCATOR.lock();
        let frame_opt = match *allocator_guard {
            Some(ref mut allocator) => FrameAllocator::allocate(allocator),
            None => return Err(MmuError::AllocationFailed),
        };
        drop(allocator_guard);

        match frame_opt {
            Some(frame) => {
                let phys_addr = PhysFrame::start_address(frame);

                // Zero-initialize the page table frame
                // SAFETY: The physical address is valid and page-aligned (from allocator).
                // We assume identity mapping for converting physical to virtual addresses.
                // We zero the entire 4 KiB page table (512 entries).
                unsafe {
                    let virt_ptr = phys_addr.as_u64() as *mut u8;
                    virt_ptr.write_bytes(0, crate::mm::address::PAGE_SIZE as usize);
                }

                Ok(phys_addr)
            }
            None => Err(MmuError::AllocationFailed),
        }
    }

    unsafe fn free_page_table(&self, phys_addr: PhysAddr) -> Result<(), MmuError> {
        // Convert physical address to frame
        let frame = PhysFrame::from_start_address(phys_addr);

        // Deallocate the frame back to the global allocator
        // SAFETY: The caller ensures the page table is no longer in use
        // SAFETY: The mutex ensures exclusive access to the allocator
        let mut allocator_guard = crate::mm::FRAME_ALLOCATOR.lock();
        match *allocator_guard {
            Some(ref mut allocator) => {
                FrameAllocator::deallocate(allocator, frame);
                Ok(())
            }
            None => Err(MmuError::AllocationFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::mmu::{RIGHTS_EXECUTE, RIGHTS_WRITE};

    #[test]
    fn page_table_entry_new_creates_unused_entry() {
        let entry = PageTableEntry::new();
        assert!(entry.is_unused());
        assert!(!entry.is_valid());
    }

    #[test]
    fn page_table_entry_is_valid_detects_valid() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID);
        assert!(entry.is_valid());
    }

    #[test]
    fn page_table_entry_set_addr_requires_alignment() {
        let mut entry = PageTableEntry::new();
        let aligned = unsafe { PhysAddr::new(0x1000) };
        entry.set_addr(aligned);
        assert_eq!(entry.addr().as_u64(), 0x1000);
    }

    #[test]
    #[should_panic(expected = "must be page-aligned")]
    fn page_table_entry_set_addr_panics_on_misalignment() {
        let mut entry = PageTableEntry::new();
        let misaligned = unsafe { PhysAddr::new(0x1001) };
        entry.set_addr(misaligned);
    }

    #[test]
    fn page_table_entry_flags_preserved() {
        let mut entry = PageTableEntry::new();
        let flags = PageTableEntry::VALID | PageTableEntry::AF;
        entry.set_flags(flags);
        assert_eq!(entry.flags(), flags);
    }

    #[test]
    fn page_table_entry_is_table_detects_table() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID | PageTableEntry::TABLE);
        assert!(entry.is_table());
        assert!(!entry.is_block());
    }

    #[test]
    fn page_table_entry_is_block_detects_block() {
        let mut entry = PageTableEntry::new();
        entry.set_flags(PageTableEntry::VALID | PageTableEntry::BLOCK);
        assert!(entry.is_block());
        assert!(!entry.is_table());
    }

    #[test]
    fn page_table_new_creates_all_unused() {
        let pt = PageTable::new();
        for entry in pt.entries.iter() {
            assert!(entry.is_unused());
        }
    }

    #[test]
    fn rights_to_flags_includes_valid_and_af() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::VALID != 0);
        assert!(flags & PageTableEntry::AF != 0);
    }

    #[test]
    fn rights_to_flags_sets_rw_for_write_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert_eq!(flags & (0b11 << 6), PageTableEntry::AP_RW);
    }

    #[test]
    fn rights_to_flags_sets_ro_without_write_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_READ, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::AP_RO != 0);
    }

    #[test]
    fn rights_to_flags_sets_execute_never_without_execute_rights() {
        let flags = AArch64Mmu::rights_to_flags(RIGHTS_READ | RIGHTS_WRITE, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::PXN != 0);
        assert!(flags & PageTableEntry::UXN != 0);
    }

    #[test]
    fn rights_to_flags_clears_execute_never_with_execute_rights() {
        let flags = AArch64Mmu::rights_to_flags(
            RIGHTS_READ | RIGHTS_WRITE | RIGHTS_EXECUTE,
            CACHE_MODE_WRITE_BACK,
        );
        assert!(flags & PageTableEntry::PXN == 0);
        assert!(flags & PageTableEntry::UXN == 0);
    }

    #[test]
    fn rights_to_flags_sets_device_for_uncached() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_UNCACHED);
        assert_eq!(flags & (0b111 << 2), PageTableEntry::ATTRINDX_DEVICE);
    }

    #[test]
    fn rights_to_flags_sets_nc_for_write_combine() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_COMBINE);
        assert!(flags & PageTableEntry::ATTRINDX_NC != 0);
    }

    #[test]
    fn rights_to_flags_sets_normal_for_write_back() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::ATTRINDX_NORMAL != 0);
    }

    #[test]
    fn rights_to_flags_sets_inner_shareable() {
        let flags = AArch64Mmu::rights_to_flags(0, CACHE_MODE_WRITE_BACK);
        assert!(flags & PageTableEntry::SH_INNER != 0);
    }

    #[test]
    fn l0_index_extracted_correctly() {
        // Test with various addresses
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l0_index(vaddr);
        assert_eq!(idx, 0x100); // Higher half canonical address space split

        let vaddr = unsafe { VirtAddr::new(0x0000_0000_1234_5678) };
        let idx = AArch64Mmu::l0_index(vaddr);
        assert_eq!(idx, 0x000); // User space
    }

    #[test]
    fn l1_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l1_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn l2_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l2_index(vaddr);
        assert_eq!(idx, 0x000);
    }

    #[test]
    fn l3_index_extracted_correctly() {
        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_0000) };
        let idx = AArch64Mmu::l3_index(vaddr);
        assert_eq!(idx, 0x000);

        let vaddr = unsafe { VirtAddr::new(0xFFFF_8000_0000_1000) };
        let idx = AArch64Mmu::l3_index(vaddr);
        assert_eq!(idx, 0x001);
    }
}
