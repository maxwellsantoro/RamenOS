//! Firmware-active bootstrap ownership admission; separate from RUN0 firmware exit.

use super::address::PAGE_SIZE;
use super::boot_pool::{
    MAX_PHYSICAL_END_EXCLUSIVE, MAX_POOL_FRAMES, POOL_END_EXCLUSIVE, POOL_START,
};
use super::{BitmapAllocator, PhysFrame};
use crate::boot::{BootMemoryMap, BootstrapProvenance, MAX_BOOT_REGIONS, RegionKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    InvalidMap,
    InvalidRegion,
    OverlappingRegions,
    MissingAllocation,
    InvalidAllocation,
    UnbackedAllocation,
    AlreadyInitialized,
}

/// A checked escrow allocation, not a conventional-memory or firmware-exit pool.
#[derive(Debug)]
pub struct BootstrapPool {
    base: PhysFrame,
    frames: usize,
    provenance: BootstrapProvenance,
}

impl BootstrapPool {
    pub const fn base(&self) -> PhysFrame {
        self.base
    }
    pub const fn frame_count(&self) -> usize {
        self.frames
    }
    pub const fn provenance(&self) -> BootstrapProvenance {
        self.provenance
    }
}

fn checked_end(start: u64, len: u64, limit: u64) -> Option<u64> {
    if len == 0 || !start.is_multiple_of(PAGE_SIZE) || !len.is_multiple_of(PAGE_SIZE) {
        return None;
    }
    start.checked_add(len).filter(|end| *end <= limit)
}

/// Validate every descriptor and the complete explicitly owned pool before publish.
/// No physical dereference, firmware reclamation, or allocator mutation occurs here.
pub fn validate(map: &BootMemoryMap) -> Result<BootstrapPool, BootstrapError> {
    if !map.is_complete() || map.count == 0 || map.count > MAX_BOOT_REGIONS {
        return Err(BootstrapError::InvalidMap);
    }
    let mut ends = [0u64; MAX_BOOT_REGIONS];
    let regions = &map.regions[..map.count];
    for (index, region) in regions.iter().enumerate() {
        ends[index] = checked_end(
            region.start.as_u64(),
            region.len_bytes,
            MAX_PHYSICAL_END_EXCLUSIVE,
        )
        .ok_or(BootstrapError::InvalidRegion)?;
    }
    for (index, region) in regions.iter().enumerate() {
        for previous in 0..index {
            if region.start.as_u64() < ends[previous]
                && regions[previous].start.as_u64() < ends[index]
            {
                return Err(BootstrapError::OverlappingRegions);
            }
        }
    }
    let claim = map
        .bootstrap_allocation()
        .ok_or(BootstrapError::MissingAllocation)?;
    let end = checked_end(claim.base, claim.len_bytes, POOL_END_EXCLUSIVE)
        .ok_or(BootstrapError::InvalidAllocation)?;
    let frames = claim.len_bytes / PAGE_SIZE;
    if claim.base < POOL_START || frames > MAX_POOL_FRAMES as u64 {
        return Err(BootstrapError::InvalidAllocation);
    }
    let backing_kind = match claim.provenance {
        BootstrapProvenance::FirmwareAllocation => RegionKind::LoaderData,
        BootstrapProvenance::QemuFixture => RegionKind::Usable,
    };
    if !regions.iter().enumerate().any(|(index, region)| {
        region.kind == backing_kind && region.start.as_u64() <= claim.base && ends[index] >= end
    }) {
        return Err(BootstrapError::UnbackedAllocation);
    }
    Ok(BootstrapPool {
        base: PhysFrame::from_frame_number(claim.base / PAGE_SIZE),
        frames: frames as usize,
        provenance: claim.provenance,
    })
}

/// The one installer used by mm::init. Denial never replaces an existing owner.
pub fn install_into(
    state: &mut Option<BitmapAllocator>,
    map: &BootMemoryMap,
) -> Result<(), BootstrapError> {
    if state.is_some() {
        return Err(BootstrapError::AlreadyInitialized);
    }
    let pool = validate(map)?;
    *state = Some(BitmapAllocator::new(pool.base, pool.frames));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BootstrapError, install_into, validate};
    use crate::boot::{BootMemoryMap, BootstrapProvenance, MAX_BOOT_REGIONS, RegionKind};
    use crate::mm::{BitmapAllocator, FrameAllocator, PhysFrame};

    const BASE: u64 = 0x100_0000;
    const LEN: u64 = 16 * 1024 * 1024;

    fn firmware_map() -> BootMemoryMap {
        let mut map = BootMemoryMap::new();
        map.add(0x1000, 0x2000, RegionKind::Reserved);
        map.add(0x4000, 0x4000, RegionKind::LoaderCode);
        map.add(BASE, LEN, RegionKind::LoaderData);
        map.add(BASE + LEN, LEN * 2, RegionKind::Usable);
        // SAFETY: Synthetic ownership assertion is used for numeric host tests only.
        unsafe {
            map.claim_bootstrap_allocation(BASE, LEN, BootstrapProvenance::FirmwareAllocation)
        };
        map
    }

    #[test]
    fn bootstrap_uses_only_owned_loader_allocation_and_preserves_unrelated_frames() {
        let map = firmware_map();
        let mut state = None;
        install_into(&mut state, &map).unwrap();
        let alloc = state.as_mut().unwrap();
        assert_eq!(alloc.total_frames(), (LEN / 4096) as usize);
        for i in 0..LEN / 4096 {
            assert_eq!(alloc.allocate().unwrap().as_u64(), BASE + i * 4096);
        }
        assert!(alloc.allocate().is_none());
        alloc.deallocate(PhysFrame::from_frame_number(BASE / 4096));
        assert_eq!(alloc.allocate().unwrap().as_u64(), BASE);
        // The other consumer owns a completely different region and remains usable.
        let mut unrelated = BitmapAllocator::new(PhysFrame::from_frame_number(0x4000 / 4096), 2);
        assert_eq!(unrelated.allocate().unwrap().as_u64(), 0x4000);
    }

    #[test]
    fn bootstrap_rejects_missing_claim_then_accepts_owned_allocation() {
        let mut map = firmware_map();
        let mut absent = BootMemoryMap::new();
        absent.add(BASE, LEN, RegionKind::Usable);
        let mut state = None;
        assert_eq!(
            install_into(&mut state, &absent),
            Err(BootstrapError::MissingAllocation)
        );
        assert!(state.is_none());
        install_into(&mut state, &map).unwrap();
        assert_eq!(state.as_mut().unwrap().allocate().unwrap().as_u64(), BASE);
        // A conventional interval alone never supplies firmware allocation ownership.
        map.regions[2].kind = RegionKind::Usable;
        assert_eq!(
            validate(&map).err(),
            Some(BootstrapError::UnbackedAllocation)
        );
    }

    #[test]
    fn bootstrap_rejects_incomplete_invalid_and_overlapping_maps() {
        let mut map = firmware_map();
        map.count = MAX_BOOT_REGIONS + 1;
        assert_eq!(validate(&map).err(), Some(BootstrapError::InvalidMap));
        let mut map = firmware_map();
        for i in map.count..=MAX_BOOT_REGIONS {
            map.add(
                0x100_0000_0000 + i as u64 * 4096,
                4096,
                RegionKind::Reserved,
            );
        }
        map.count = 4;
        assert_eq!(validate(&map).err(), Some(BootstrapError::InvalidMap));
        for (start, len) in [
            (0, 0),
            (1, 4096),
            (4096, 1),
            (u64::MAX - 4095, 4096),
            (1 << 52, 4096),
        ] {
            let mut map = firmware_map();
            map.add(start, len, RegionKind::Reserved);
            assert_eq!(validate(&map).err(), Some(BootstrapError::InvalidRegion));
        }
        let mut map = firmware_map();
        map.add(BASE + 4096, 4096, RegionKind::Reserved);
        assert_eq!(
            validate(&map).err(),
            Some(BootstrapError::OverlappingRegions)
        );
    }

    #[test]
    fn bootstrap_rejects_unbacked_cross_region_and_reserved_pool_claims() {
        for kind in [
            RegionKind::Reserved,
            RegionKind::LoaderCode,
            RegionKind::Usable,
        ] {
            let mut map = firmware_map();
            map.regions[2].kind = kind;
            assert_eq!(
                validate(&map).err(),
                Some(BootstrapError::UnbackedAllocation)
            );
        }
        let mut map = firmware_map();
        // A claim may be smaller than its containing LoaderData allocation descriptor.
        unsafe {
            map.claim_bootstrap_allocation(
                BASE + 4096,
                4096,
                BootstrapProvenance::FirmwareAllocation,
            )
        };
        assert!(validate(&map).is_ok());
        unsafe {
            map.claim_bootstrap_allocation(
                BASE + LEN - 4096,
                8192,
                BootstrapProvenance::FirmwareAllocation,
            )
        };
        assert_eq!(
            validate(&map).err(),
            Some(BootstrapError::UnbackedAllocation)
        );
    }

    #[test]
    fn bootstrap_checks_capacity_alignment_arithmetic_and_supported_window() {
        for (base, len) in [
            (0, 4096),
            (BASE + 1, 4096),
            (BASE, 0),
            (BASE, 4097),
            (BASE, 131073 * 4096),
            (0xc000_0000, 4096),
            (u64::MAX - 4095, 8192),
        ] {
            let mut map = firmware_map();
            unsafe {
                map.claim_bootstrap_allocation(base, len, BootstrapProvenance::FirmwareAllocation)
            };
            assert_eq!(
                validate(&map).err(),
                Some(BootstrapError::InvalidAllocation)
            );
        }
    }

    #[test]
    fn bootstrap_refuses_reinstall_without_losing_live_allocations_or_reuse() {
        let map = firmware_map();
        let mut state = None;
        install_into(&mut state, &map).unwrap();
        let held = state.as_mut().unwrap().allocate().unwrap();
        assert_eq!(
            install_into(&mut state, &map),
            Err(BootstrapError::AlreadyInitialized)
        );
        let alloc = state.as_mut().unwrap();
        assert_eq!(alloc.available_frames(), LEN as usize / 4096 - 1);
        let next = alloc.allocate().unwrap();
        assert_eq!(next.as_u64(), held.as_u64() + 4096);
        alloc.deallocate(held);
        assert_eq!(alloc.allocate().unwrap(), held);
    }

    #[test]
    fn bootstrap_qemu_fixture_provenance_requires_explicit_usable_backing() {
        let mut map = BootMemoryMap::new();
        map.add(0x5000_0000, LEN, RegionKind::Usable);
        unsafe {
            map.claim_bootstrap_allocation(0x5000_0000, LEN, BootstrapProvenance::QemuFixture)
        };
        assert!(validate(&map).is_ok());
        map.regions[0].kind = RegionKind::LoaderData;
        assert_eq!(
            validate(&map).err(),
            Some(BootstrapError::UnbackedAllocation)
        );
    }
}
