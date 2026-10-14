//! Pure projection of raw firmware descriptor facts for the active-firmware bootstrap.

use kernel::boot::{BootMemoryMap, MAX_BOOT_REGIONS, RegionKind};

#[derive(Copy, Clone, Debug)]
pub struct DescriptorFacts {
    pub ty: u32,
    pub start: u64,
    pub pages: u64,
    pub attributes: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MapError {
    EmptyMap,
    TooManyDescriptors,
    InvalidDescriptor,
    UnknownType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MapCaptureFailure {
    Released,
    Retained { base: u64 },
}

/// Return a captured map or release the exact newly allocated pool. A failed
/// release returns the still-owned address; the caller must retain it and halt,
/// rather than return to firmware while claiming the allocation was released.
pub fn capture_or_release_pool<T>(
    base: u64,
    capture: impl FnOnce() -> Option<T>,
    release: impl FnOnce(u64) -> bool,
) -> Result<T, MapCaptureFailure> {
    match capture() {
        Some(map) => Ok(map),
        None if release(base) => Err(MapCaptureFailure::Released),
        None => Err(MapCaptureFailure::Retained { base }),
    }
}

/// Preserve each supported firmware descriptor; no boot-services reclamation.
/// Numeric projection is not final-map, mapping-access, or retention evidence.
pub fn convert(facts: impl Iterator<Item = DescriptorFacts>) -> Result<BootMemoryMap, MapError> {
    let mut map = BootMemoryMap::new();
    for fact in facts {
        if map.count == MAX_BOOT_REGIONS {
            return Err(MapError::TooManyDescriptors);
        }
        let len = fact
            .pages
            .checked_mul(4096)
            .ok_or(MapError::InvalidDescriptor)?;
        let end = fact
            .start
            .checked_add(len)
            .ok_or(MapError::InvalidDescriptor)?;
        if len == 0 || !fact.start.is_multiple_of(4096) || end > (1 << 52) {
            return Err(MapError::InvalidDescriptor);
        }
        // Supported raw types follow pinned uefi-raw0.5.2. Runtime-owned entries
        // are always reserved, including a misleading conventional/loader type.
        let kind = match fact.ty {
            0..=14 if fact.attributes & (1 << 63) != 0 => RegionKind::Reserved,
            1 => RegionKind::LoaderCode,
            2 => RegionKind::LoaderData,
            7 => RegionKind::Usable,
            0..=14 => RegionKind::Reserved,
            _ => return Err(MapError::UnknownType),
        };
        map.add(fact.start, len, kind);
    }
    if map.count == 0 {
        return Err(MapError::EmptyMap);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::{DescriptorFacts, MapError, convert};
    use kernel::boot::{MAX_BOOT_REGIONS, RegionKind};

    fn fact(ty: u32, start: u64, pages: u64, attributes: u64) -> DescriptorFacts {
        DescriptorFacts {
            ty,
            start,
            pages,
            attributes,
        }
    }

    #[test]
    fn bootstrap_failed_map_releases_exact_pool_and_next_attempt_succeeds() {
        use core::cell::Cell;
        let available = Cell::new(true);
        let freed = Cell::new(0);
        let allocate = || available.replace(false).then_some(0x100_0000);
        let base = allocate().unwrap();
        let failed = super::capture_or_release_pool(
            base,
            || None::<u32>,
            |released| {
                assert_eq!(released, base);
                assert!(!available.replace(true));
                freed.set(freed.get() + 1);
                true
            },
        );
        assert_eq!(failed, Err(super::MapCaptureFailure::Released));
        let next = allocate().unwrap();
        assert_eq!(next, base);
        assert_eq!(
            super::capture_or_release_pool(next, || Some(42), |_| panic!("live pool freed")),
            Ok(42)
        );
        assert_eq!(freed.get(), 1);
    }

    #[test]
    fn bootstrap_failed_release_returns_retained_owner_without_claiming_cleanup() {
        let base = 0x100_0000;
        let result = super::capture_or_release_pool(
            base,
            || None::<u32>,
            |released| {
                assert_eq!(released, base);
                false
            },
        );
        assert_eq!(result, Err(super::MapCaptureFailure::Retained { base }));
    }

    #[test]
    fn bootstrap_map_preserves_all_descriptors_and_never_reclaims_boot_services() {
        let facts = [
            fact(7, 0x1000, 1, 0),
            fact(1, 0x2000, 1, 0),
            fact(2, 0x3000, 1, 0),
            fact(3, 0x4000, 1, 0),
            fact(4, 0x5000, 1, 0),
            fact(7, 0x6000, 1, 1 << 63),
            fact(11, 0x7000, 1, 0),
        ];
        let map = convert(facts.into_iter()).unwrap();
        assert_eq!(map.count, facts.len());
        assert_eq!(map.regions[0].kind, RegionKind::Usable);
        assert_eq!(map.regions[1].kind, RegionKind::LoaderCode);
        assert_eq!(map.regions[2].kind, RegionKind::LoaderData);
        for region in &map.regions[3..map.count] {
            assert_eq!(region.kind, RegionKind::Reserved);
        }
    }

    #[test]
    fn bootstrap_map_rejects_overflow_truncation_unknown_and_malformed_facts() {
        for raw in [
            fact(7, 0x1000, u64::MAX, 0),
            fact(7, u64::MAX - 4095, 1, 0),
            fact(7, 0x1001, 1, 0),
            fact(7, 0x1000, 0, 0),
            fact(7, 1 << 52, 1, 0),
        ] {
            assert!(matches!(
                convert([raw].into_iter()),
                Err(MapError::InvalidDescriptor)
            ));
        }
        assert!(matches!(
            convert([fact(0xffff, 0x1000, 1, 0)].into_iter()),
            Err(MapError::UnknownType)
        ));
        assert!(matches!(
            convert((0..=MAX_BOOT_REGIONS).map(|i| fact(7, (i as u64 + 1) * 4096, 1, 0))),
            Err(MapError::TooManyDescriptors)
        ));
        assert!(matches!(
            convert(core::iter::empty()),
            Err(MapError::EmptyMap)
        ));
        assert!(convert([fact(7, 0x1000, 1, 0)].into_iter()).is_ok());
    }
}
