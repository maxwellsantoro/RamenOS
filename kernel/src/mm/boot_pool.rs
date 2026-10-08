//! Allocation-free admission of one bounded boot frame pool.
//!
//! The supplied firmware stage and typed memory kinds are trusted boot
//! instrumentation. This validates numeric ranges and retention records; it
//! does not attest firmware exit, CPU mappings, RAM backing or access rights.
//! No physical pointers, registers, globals or allocator state are accessed.

use crate::boot::{BootMemoryMap, RegionKind};
use crate::mm::address::{PAGE_SIZE, PhysAddr, PhysFrame};

pub use crate::boot::MAX_BOOT_REGIONS;

pub const MAX_RETAINED_RANGES: usize = 256;
pub const MAX_POOL_FRAMES: usize = 131072;
pub const POOL_START: u64 = PAGE_SIZE;
pub const POOL_END_EXCLUSIVE: u64 = 0xc000_0000;
pub const MAX_PHYSICAL_END_EXCLUSIVE: u64 = 1 << 52;
pub const RETENTION_END_EXCLUSIVE: u64 = 1 << 32;

/// Trusted boot-stage observation, not a remote authentication token.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FirmwareStage {
    BootServicesActive,
    ExitedInterruptsUnverified,
    ExitedInterruptsMasked,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RetainedKind {
    CurrentStack,
    InitImage,
    KernelImage,
    PageTables,
    Gdt,
    Idt,
    FinalMapBuffer,
}

impl RetainedKind {
    const fn mask(self) -> u8 {
        match self {
            Self::CurrentStack => 1 << 0,
            Self::InitImage => 1 << 1,
            Self::KernelImage => 1 << 2,
            Self::PageTables => 1 << 3,
            Self::Gdt => 1 << 4,
            Self::Idt => 1 << 5,
            Self::FinalMapBuffer => 1 << 6,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RetainedRange {
    pub start: u64,
    pub len_bytes: u64,
    pub kind: RetainedKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BootPoolError {
    FirmwareNotOwned,
    InvalidMapCount,
    InvalidRegion,
    OverlappingRegions,
    InvalidRetentionCount,
    InvalidRetainedRange,
    MissingRetention,
    UnreservedRetention,
    NoEligiblePool,
}

/// A numerically admitted pool. CPU access validation remains a caller boundary.
#[derive(Debug)]
pub struct BootPool {
    base: PhysFrame,
    frame_count: usize,
}

impl BootPool {
    #[must_use]
    pub const fn base(&self) -> PhysFrame {
        self.base
    }

    #[must_use]
    pub const fn frame_count(&self) -> usize {
        self.frame_count
    }

    #[must_use]
    pub const fn contains(&self, frame: PhysFrame) -> bool {
        let address = frame.as_u64();
        let base = self.base.as_u64();
        address >= base && (address - base) / PAGE_SIZE < self.frame_count as u64
    }
}

fn checked_end(start: u64, len_bytes: u64, limit: u64) -> Option<u64> {
    if len_bytes == 0 || !start.is_multiple_of(PAGE_SIZE) || !len_bytes.is_multiple_of(PAGE_SIZE) {
        return None;
    }
    let end = start.checked_add(len_bytes)?;
    (end <= limit).then_some(end)
}

/// Admit one conventional interval after validating the entire bounded input.
///
/// Repeated retention reasons and overlapping retained records are allowed, but
/// each record must fit inside one non-usable descriptor. The supplied typed
/// `Usable` classification must originate from the separately validated final
/// firmware map. This function neither reclaims firmware memory nor installs
/// the pool into the live allocator.
pub fn select_boot_pool(
    map: &BootMemoryMap,
    stage: FirmwareStage,
    retained: &[RetainedRange],
) -> Result<BootPool, BootPoolError> {
    if stage != FirmwareStage::ExitedInterruptsMasked {
        return Err(BootPoolError::FirmwareNotOwned);
    }
    if !map.is_complete() || map.count == 0 || map.count > MAX_BOOT_REGIONS {
        return Err(BootPoolError::InvalidMapCount);
    }

    // Validate all populated ranges before overlap checks or pool selection.
    let regions = &map.regions[..map.count];
    let mut region_ends = [0u64; MAX_BOOT_REGIONS];
    for (index, region) in regions.iter().enumerate() {
        region_ends[index] = checked_end(
            region.start.as_u64(),
            region.len_bytes,
            MAX_PHYSICAL_END_EXCLUSIVE,
        )
        .ok_or(BootPoolError::InvalidRegion)?;
    }
    for (index, region) in regions.iter().enumerate() {
        for previous in 0..index {
            if region.start.as_u64() < region_ends[previous]
                && regions[previous].start.as_u64() < region_ends[index]
            {
                return Err(BootPoolError::OverlappingRegions);
            }
        }
    }

    if retained.is_empty() || retained.len() > MAX_RETAINED_RANGES {
        return Err(BootPoolError::InvalidRetentionCount);
    }
    let mut retained_ends = [0u64; MAX_RETAINED_RANGES];
    let mut kinds = 0u8;
    for (index, range) in retained.iter().enumerate() {
        retained_ends[index] = checked_end(range.start, range.len_bytes, RETENTION_END_EXCLUSIVE)
            .ok_or(BootPoolError::InvalidRetainedRange)?;
        kinds |= range.kind.mask();
    }
    if kinds != 0x7f {
        return Err(BootPoolError::MissingRetention);
    }
    for (index, range) in retained.iter().enumerate() {
        let covered = regions.iter().enumerate().any(|(region_index, region)| {
            region.kind != RegionKind::Usable
                && region.start.as_u64() <= range.start
                && region_ends[region_index] >= retained_ends[index]
        });
        if !covered {
            return Err(BootPoolError::UnreservedRetention);
        }
    }

    // Compare complete in-profile intervals before applying allocator capacity.
    let mut best: Option<(u64, u64)> = None;
    for (index, region) in regions.iter().enumerate() {
        if region.kind != RegionKind::Usable {
            continue;
        }
        let start = region.start.as_u64().max(POOL_START);
        let end = region_ends[index].min(POOL_END_EXCLUSIVE);
        if start >= end {
            continue;
        }
        let length = end - start;
        if best.is_none_or(|(base, best_length)| {
            length > best_length || (length == best_length && start < base)
        }) {
            best = Some((start, length));
        }
    }
    let (start, length) = best.ok_or(BootPoolError::NoEligiblePool)?;
    let frame_count = (length / PAGE_SIZE).min(MAX_POOL_FRAMES as u64) as usize;
    // SAFETY: validated aligned descriptors intersect an aligned supported
    // physical window. Construction stores an address; it does not access it.
    let base = PhysFrame::from_start_address(unsafe { PhysAddr::new(start) });
    Ok(BootPool { base, frame_count })
}
