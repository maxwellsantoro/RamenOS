//! RUN0.0 pure assertions. Coordinator registers this module after contract review.
//! The absent boot_pool implementation must fail the first registered run.

use crate::boot::{BootMemoryMap, MemoryRegion, RegionKind};
use crate::mm::address::{PAGE_SIZE, PhysAddr, PhysFrame};
use crate::mm::boot_pool::{
    BootPoolError, FirmwareStage, MAX_BOOT_REGIONS, MAX_PHYSICAL_END_EXCLUSIVE, MAX_POOL_FRAMES,
    MAX_RETAINED_RANGES, POOL_END_EXCLUSIVE, POOL_START, RETENTION_END_EXCLUSIVE, RetainedKind,
    RetainedRange, select_boot_pool,
};

const OWNED: FirmwareStage = FirmwareStage::ExitedInterruptsMasked;
const KINDS: [RetainedKind; 7] = [
    RetainedKind::CurrentStack,
    RetainedKind::InitImage,
    RetainedKind::KernelImage,
    RetainedKind::PageTables,
    RetainedKind::Gdt,
    RetainedKind::Idt,
    RetainedKind::FinalMapBuffer,
];

fn region(start: u64, pages: u64, kind: RegionKind) -> MemoryRegion {
    MemoryRegion {
        // Synthetic numeric fixtures; tests never dereference these addresses.
        start: unsafe { PhysAddr::new(start) },
        len_bytes: pages * PAGE_SIZE,
        kind,
    }
}

fn fixture() -> (BootMemoryMap, [RetainedRange; 7]) {
    let mut map = BootMemoryMap::new();
    map.regions[0] = region(0x1000, 7, RegionKind::Reserved);
    map.regions[1] = region(0x10_0000, 16, RegionKind::Usable);
    map.count = 2;
    let retained = core::array::from_fn(|i| RetainedRange {
        start: 0x1000 + i as u64 * PAGE_SIZE,
        len_bytes: PAGE_SIZE,
        kind: KINDS[i],
    });
    (map, retained)
}

fn frame(address: u64) -> PhysFrame {
    PhysFrame::from_start_address(unsafe { PhysAddr::new(address) })
}

fn add(map: &mut BootMemoryMap, value: MemoryRegion) {
    map.regions[map.count] = value;
    map.count += 1;
}

#[test]
fn boot_pool_valid_final_conventional_pool() {
    let (map, retained) = fixture();
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(pool.base(), frame(0x10_0000));
    assert_eq!(pool.frame_count(), 16);
}

#[test]
fn boot_pool_denies_active_and_unmasked_stages() {
    let (map, retained) = fixture();
    for stage in [
        FirmwareStage::BootServicesActive,
        FirmwareStage::ExitedInterruptsUnverified,
    ] {
        assert_eq!(
            select_boot_pool(&map, stage, &retained).unwrap_err(),
            BootPoolError::FirmwareNotOwned
        );
    }
}

#[test]
fn boot_pool_fixed_capacity_and_checked_counts() {
    let (mut map, retained) = fixture();
    assert_eq!(MAX_BOOT_REGIONS, 256);
    assert_eq!(map.regions.len(), MAX_BOOT_REGIONS);
    assert_eq!(MAX_RETAINED_RANGES, 256);
    for count in [0, MAX_BOOT_REGIONS + 1, usize::MAX] {
        map.count = count;
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::InvalidMapCount
        );
    }
    // Exercise the maintained insertion boundary, not just synthetic counts.
    let (original, retained) = fixture();
    let mut inserted = BootMemoryMap::new();
    for value in &original.regions[..original.count] {
        inserted.add(value.start.as_u64(), value.len_bytes, value.kind);
    }
    while inserted.count < MAX_BOOT_REGIONS {
        inserted.add(
            0x1000_0000 + inserted.count as u64 * PAGE_SIZE,
            PAGE_SIZE,
            RegionKind::Reserved,
        );
    }
    assert!(inserted.is_complete());
    assert!(select_boot_pool(&inserted, OWNED, &retained).is_ok());
    inserted.add(0x2000_0000, PAGE_SIZE, RegionKind::Reserved);
    assert_eq!(inserted.count, MAX_BOOT_REGIONS);
    assert!(!inserted.is_complete());
    assert_eq!(
        select_boot_pool(&inserted, OWNED, &retained).unwrap_err(),
        BootPoolError::InvalidMapCount
    );
    // Public count changes cannot erase the sticky loss-of-descriptor evidence.
    inserted.count = original.count;
    assert!(!inserted.is_complete());
    assert_eq!(
        select_boot_pool(&inserted, OWNED, &retained).unwrap_err(),
        BootPoolError::InvalidMapCount
    );
    for malformed_count in [MAX_BOOT_REGIONS + 1, usize::MAX] {
        let mut malformed = original;
        malformed.count = malformed_count;
        malformed.add(0x2000_0000, PAGE_SIZE, RegionKind::Reserved);
        assert_eq!(malformed.count, malformed_count);
        assert!(!malformed.is_complete());
        assert_eq!(
            select_boot_pool(&malformed, OWNED, &retained).unwrap_err(),
            BootPoolError::InvalidMapCount
        );
    }
}

#[test]
fn boot_pool_validates_every_descriptor_at_capacity() {
    let (mut map, retained) = fixture();
    while map.count < MAX_BOOT_REGIONS {
        let start = 0x1000_0000 + map.count as u64 * PAGE_SIZE;
        add(&mut map, region(start, 1, RegionKind::Reserved));
    }
    assert_eq!(
        select_boot_pool(&map, OWNED, &retained)
            .unwrap()
            .frame_count(),
        16
    );
    map.regions[MAX_BOOT_REGIONS - 1].len_bytes = 0;
    assert_eq!(
        select_boot_pool(&map, OWNED, &retained).unwrap_err(),
        BootPoolError::InvalidRegion
    );
}

#[test]
fn boot_pool_rejects_zero_unaligned_overflow_and_physical_width() {
    let (original, retained) = fixture();
    for bad in [
        (0x10_0000, 0),
        (0x10_0001, PAGE_SIZE),
        (0x10_0000, PAGE_SIZE + 1),
        (!(PAGE_SIZE - 1), PAGE_SIZE),
        (MAX_PHYSICAL_END_EXCLUSIVE, PAGE_SIZE),
    ] {
        for kind in [RegionKind::Usable, RegionKind::Reserved] {
            let mut map = original;
            map.regions[1].kind = kind;
            map.regions[1].start = unsafe { PhysAddr::new(bad.0) };
            map.regions[1].len_bytes = bad.1;
            assert_eq!(
                select_boot_pool(&map, OWNED, &retained).unwrap_err(),
                BootPoolError::InvalidRegion
            );
        }
    }
    let mut map = original;
    add(&mut map, region(0x3000_0001, 1, RegionKind::Reserved));
    assert_eq!(
        select_boot_pool(&map, OWNED, &retained).unwrap_err(),
        BootPoolError::InvalidRegion
    );
    let mut map = original;
    add(
        &mut map,
        region(
            MAX_PHYSICAL_END_EXCLUSIVE - PAGE_SIZE,
            1,
            RegionKind::Reserved,
        ),
    );
    assert!(select_boot_pool(&map, OWNED, &retained).is_ok());
}

#[test]
fn boot_pool_rejects_duplicate_and_overlapping_descriptors() {
    let (original, retained) = fixture();
    for value in [
        region(0x1000, 7, RegionKind::Reserved),
        region(0x2000, 1, RegionKind::Reserved),
        region(0x10_0000, 16, RegionKind::Usable),
        region(0x10_1000, 1, RegionKind::Reserved),
        region(0x0f_f000, 2, RegionKind::LoaderData),
    ] {
        let mut map = original;
        add(&mut map, value);
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::OverlappingRegions
        );
    }
}

#[test]
fn boot_pool_accepts_adjacency_unsorted_input_and_ignores_unused_slots() {
    let (mut map, retained) = fixture();
    add(&mut map, region(0x11_0000, 1, RegionKind::Usable));
    map.regions.swap(0, 2);
    map.regions[map.count].len_bytes = u64::MAX;
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(pool.base(), frame(0x10_0000));
    assert_eq!(pool.frame_count(), 16);
}

#[test]
fn boot_pool_requires_all_retention_reasons() {
    let (map, retained) = fixture();
    for missing in 0..KINDS.len() {
        let mut incomplete = retained;
        incomplete[missing].kind = KINDS[(missing + 1) % KINDS.len()];
        assert_eq!(
            select_boot_pool(&map, OWNED, &incomplete).unwrap_err(),
            BootPoolError::MissingRetention
        );
    }
}

#[test]
fn boot_pool_retention_count_is_bounded() {
    let (map, retained) = fixture();
    assert_eq!(
        select_boot_pool(&map, OWNED, &[]).unwrap_err(),
        BootPoolError::InvalidRetentionCount
    );
    let mut full = [retained[3]; MAX_RETAINED_RANGES + 1];
    full[..retained.len()].copy_from_slice(&retained);
    assert!(select_boot_pool(&map, OWNED, &full[..MAX_RETAINED_RANGES]).is_ok());
    assert_eq!(
        select_boot_pool(&map, OWNED, &full).unwrap_err(),
        BootPoolError::InvalidRetentionCount
    );
}

#[test]
fn boot_pool_rejects_invalid_retained_ranges() {
    let (map, original) = fixture();
    for bad in [
        (0x1000, 0),
        (0x1001, PAGE_SIZE),
        (0x1000, PAGE_SIZE + 1),
        (!(PAGE_SIZE - 1), PAGE_SIZE),
        (RETENTION_END_EXCLUSIVE, PAGE_SIZE),
    ] {
        let mut retained = original;
        retained[0].start = bad.0;
        retained[0].len_bytes = bad.1;
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::InvalidRetainedRange
        );
    }
    let mut map = map;
    add(
        &mut map,
        region(RETENTION_END_EXCLUSIVE - PAGE_SIZE, 1, RegionKind::Reserved),
    );
    let mut retained = original;
    retained[0].start = RETENTION_END_EXCLUSIVE - PAGE_SIZE;
    assert!(select_boot_pool(&map, OWNED, &retained).is_ok());
}

#[test]
fn boot_pool_rejects_uncovered_and_conventional_retention() {
    let (map, original) = fixture();
    for bad in [
        (0x9000, PAGE_SIZE),
        (0x7000, 2 * PAGE_SIZE),
        (0x10_0000, PAGE_SIZE),
    ] {
        let mut retained = original;
        retained[0].start = bad.0;
        retained[0].len_bytes = bad.1;
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::UnreservedRetention
        );
    }

    // Union coverage across adjacent descriptors is insufficient for one
    // record: the collector must split the record at the descriptor boundary.
    let mut split_map = map;
    split_map.regions[0] = region(0x1000, 3, RegionKind::Reserved);
    add(&mut split_map, region(0x4000, 4, RegionKind::Reserved));
    let mut unsplit = original;
    unsplit[0].start = 0x3000;
    unsplit[0].len_bytes = 2 * PAGE_SIZE;
    assert_eq!(
        select_boot_pool(&split_map, OWNED, &unsplit).unwrap_err(),
        BootPoolError::UnreservedRetention
    );
}

#[test]
fn boot_pool_accepts_conservative_loader_retention_and_shared_reasons() {
    let (mut map, mut retained) = fixture();
    for kind in [
        RegionKind::Reserved,
        RegionKind::LoaderCode,
        RegionKind::LoaderData,
    ] {
        map.regions[0].kind = kind;
        for range in &mut retained {
            range.start = 0x1000;
            range.len_bytes = 7 * PAGE_SIZE;
        }
        assert!(select_boot_pool(&map, OWNED, &retained).is_ok());
    }
}

#[test]
fn boot_pool_no_reclamation_and_no_eligible_memory_is_explicit() {
    let (mut map, retained) = fixture();
    for kind in [
        RegionKind::Reserved,
        RegionKind::LoaderCode,
        RegionKind::LoaderData,
    ] {
        map.regions[1].kind = kind;
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::NoEligiblePool
        );
    }
}

#[test]
fn boot_pool_intersects_window_and_excludes_page_zero_and_high_memory() {
    let (mut original, mut retained) = fixture();
    original.regions[0] = region(0x8000_0000, 7, RegionKind::Reserved);
    for (i, range) in retained.iter_mut().enumerate() {
        range.start = 0x8000_0000 + i as u64 * PAGE_SIZE;
    }
    assert_eq!(POOL_START, PAGE_SIZE);
    assert_eq!(POOL_END_EXCLUSIVE, 0xc000_0000);
    for candidate in [
        region(0, 1, RegionKind::Usable),
        region(POOL_END_EXCLUSIVE, 1, RegionKind::Usable),
    ] {
        let mut map = original;
        map.regions[1] = candidate;
        assert_eq!(
            select_boot_pool(&map, OWNED, &retained).unwrap_err(),
            BootPoolError::NoEligiblePool
        );
    }
    for (candidate, expected_base) in [
        (region(0, 2, RegionKind::Usable), POOL_START),
        (
            region(POOL_END_EXCLUSIVE - PAGE_SIZE, 2, RegionKind::Usable),
            POOL_END_EXCLUSIVE - PAGE_SIZE,
        ),
        (
            region(POOL_END_EXCLUSIVE - PAGE_SIZE, 1, RegionKind::Usable),
            POOL_END_EXCLUSIVE - PAGE_SIZE,
        ),
    ] {
        let mut map = original;
        map.regions[1] = candidate;
        let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
        assert_eq!(pool.base(), frame(expected_base));
        assert_eq!(pool.frame_count(), 1);
    }
}

#[test]
fn boot_pool_capacity_policy_is_explicit_and_does_not_round_up() {
    let (mut map, retained) = fixture();
    assert_eq!(MAX_POOL_FRAMES, 131072);
    map.regions[1] = region(0x10_0000, MAX_POOL_FRAMES as u64 + 1, RegionKind::Usable);
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(pool.base(), frame(0x10_0000));
    assert_eq!(pool.frame_count(), MAX_POOL_FRAMES);
    assert!(!pool.contains(frame(0x10_0000 + MAX_POOL_FRAMES as u64 * PAGE_SIZE)));
}

#[test]
fn boot_pool_selection_is_stable_and_never_combines_descriptors() {
    let (mut map, retained) = fixture();
    add(&mut map, region(0x20_0000, 32, RegionKind::Usable));
    add(&mut map, region(0x18_0000, 32, RegionKind::Usable));
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(pool.base(), frame(0x18_0000));
    assert_eq!(pool.frame_count(), 32);
    map.regions.swap(1, 3);
    let reordered = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(reordered.base(), pool.base());
    assert_eq!(reordered.frame_count(), pool.frame_count());

    // Choose the larger eligible interval BEFORE clipping; both outputs would
    // otherwise tie at the allocator limit and incorrectly prefer the low base.
    let (mut map, retained) = fixture();
    map.regions[1] = region(0x10_0000, MAX_POOL_FRAMES as u64 + 1, RegionKind::Usable);
    add(
        &mut map,
        region(0x3000_0000, MAX_POOL_FRAMES as u64 + 2, RegionKind::Usable),
    );
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert_eq!(pool.base(), frame(0x3000_0000));
    assert_eq!(pool.frame_count(), MAX_POOL_FRAMES);
}

#[test]
fn boot_pool_contains_exactly_selected_frames() {
    let (map, retained) = fixture();
    let pool = select_boot_pool(&map, OWNED, &retained).unwrap();
    assert!(!pool.contains(frame(0x10_0000 - PAGE_SIZE)));
    for i in 0..16 {
        assert!(pool.contains(frame(0x10_0000 + i * PAGE_SIZE)));
    }
    assert!(!pool.contains(frame(0x11_0000)));
    assert!(!pool.contains(frame(0)));
}
