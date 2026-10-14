#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use core::slice;
#[cfg(not(test))]
use kernel::boot::{BootMemoryMap, BootstrapProvenance};
#[cfg(not(test))]
use uefi::cstr16;
#[cfg(not(test))]
use uefi::prelude::*;
#[cfg(not(test))]
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, FileType};
#[cfg(not(test))]
use uefi::proto::media::fs::SimpleFileSystem;
#[cfg(not(test))]
use uefi::table::boot::{AllocateType, MemoryType};

#[cfg(not(test))]
mod ab_slot_probe;
#[cfg(not(test))]
mod boot_nonce_probe;
mod bootstrap;
#[cfg(not(test))]
mod gop_probe;
#[cfg(not(test))]
mod hil_build_provenance;
#[cfg(not(test))]
mod iommu_probe;
#[cfg(not(test))]
mod nvme_boot_probe;

#[cfg(not(test))]
#[entry]
fn efi_main(_image: Handle, st: SystemTable<Boot>) -> Status {
    if let Some(init) = load_init_image(&st) {
        kernel::boot::set_init_image(init);
    }

    // S12.1: probe GOP while UEFI boot services are still available.
    kernel::boot::set_gop_probe(gop_probe::probe_gop(&st));

    // S12.3: inventory ACPI DMAR while firmware tables are still available.
    kernel::boot::set_iommu_probe(iommu_probe::probe_iommu(&st));

    // S13.7: detect NVMe namespace in the loaded-image device path.
    kernel::boot::set_nvme_boot_probe(nvme_boot_probe::probe_nvme_boot(&st));

    // S13.8: read A/B slot metadata published by Store install + rollback prep.
    kernel::boot::set_atomic_update_probe(ab_slot_probe::probe_ab_slot(&st));

    kernel::boot::set_boot_epoch_nonce(boot_nonce_probe::probe_boot_nonce(&st));
    kernel::boot::set_hil_build_provenance(hil_build_provenance::BUILD_PROVENANCE);

    // Keep one real firmware allocation owned for the complete bootstrap lifetime.
    // Boot services remain active; this does not reclaim any firmware memory.
    const BOOTSTRAP_PAGES: usize = 4096; // 16 MiB, bounded below the 3 GiB profile ceiling.
    let pool_base = match st.boot_services().allocate_pages(
        AllocateType::MaxAddress(0xbfff_ffff),
        MemoryType::LOADER_DATA,
        BOOTSTRAP_PAGES,
    ) {
        Ok(base) => base,
        Err(_) => return Status::OUT_OF_RESOURCES,
    };
    // Capture after all probes and allocation; scoped protocol owners have ended.
    let mut map = match bootstrap::capture_or_release_pool(
        pool_base,
        || get_memory_map(&st),
        |base| {
            // SAFETY: this is the exact still-owned allocation above; no
            // allocator or kernel consumer has been installed on this path.
            unsafe { st.boot_services().free_pages(base, BOOTSTRAP_PAGES) }.is_ok()
        },
    ) {
        Ok(map) => map,
        Err(bootstrap::MapCaptureFailure::Released) => return Status::LOAD_ERROR,
        Err(bootstrap::MapCaptureFailure::Retained { base }) => {
            kernel::serial::init();
            kernel::kprintln!("boot: map failed; bootstrap pool retained at {:#x}", base);
            loop {
                kernel::arch::halt();
            }
        }
    };
    // SAFETY: pool_base is the exact successful AllocatePages allocation above.
    // It is never freed, and boot_main never returns to firmware on success.
    unsafe {
        map.claim_bootstrap_allocation(
            pool_base,
            BOOTSTRAP_PAGES as u64 * 4096,
            BootstrapProvenance::FirmwareAllocation,
        );
    }
    kernel::boot::set_memory_map(map);

    kernel::boot::boot_main();
}

#[panic_handler]
#[cfg(not(test))]
fn panic(info: &PanicInfo) -> ! {
    kernel::serial::init();
    kernel::kprintln!("PANIC: {}", info);
    loop {
        kernel::arch::halt();
    }
}

#[cfg(not(test))]
fn get_memory_map(st: &SystemTable<Boot>) -> Option<BootMemoryMap> {
    let bs = st.boot_services();
    // The public UEFI API requires descriptor alignment. A larger fixed scratch
    // accommodates descriptor stride and the complete <=256-record profile;
    // oversized firmware maps fail, rather than truncating or filtering records.
    #[repr(align(8))]
    struct MapScratch([u8; 32768]);
    static mut SCRATCH: MapScratch = MapScratch([0; 32768]);
    // SAFETY: the entry calls this helper once, synchronously, before kernel
    // consumers. Keep this fixed scratch in the loaded image rather than
    // requiring an extra 32 KiB of unqualified firmware-stack headroom.
    let scratch = unsafe { &mut (*core::ptr::addr_of_mut!(SCRATCH)).0 };
    let mmap = bs.memory_map(scratch).ok()?;
    bootstrap::convert(mmap.entries().map(|desc| bootstrap::DescriptorFacts {
        ty: desc.ty.0,
        start: desc.phys_start,
        pages: desc.page_count,
        attributes: desc.att.bits(),
    }))
    .ok()
}

#[cfg(not(test))]
fn load_init_image(st: &SystemTable<Boot>) -> Option<kernel::boot::InitImage> {
    let bs = st.boot_services();
    let sfs_handle = bs.get_handle_for_protocol::<SimpleFileSystem>().ok()?;
    let mut sfs = bs
        .open_protocol_exclusive::<SimpleFileSystem>(sfs_handle)
        .ok()?;
    let mut root = sfs.open_volume().ok()?;
    let path = cstr16!("\\EFI\\BOOT\\init.img");
    let file = root
        .open(path, FileMode::Read, FileAttribute::empty())
        .ok()?;
    let mut file = match file.into_type().ok()? {
        FileType::Regular(f) => f,
        _ => return None,
    };
    let mut info_buf = [0u8; 512];
    let info = file.get_info::<FileInfo>(&mut info_buf).ok()?;
    let size = usize::try_from(info.file_size()).ok()?;
    if size == 0 {
        return None;
    }
    let pages = size.checked_add(0xfff)? / 0x1000;
    let allocation_len = pages.checked_mul(0x1000)?;
    let ptr = bs
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .ok()?;
    let buf = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, allocation_len) };
    let read = file.read(buf).ok()?;
    if read == 0 {
        return None;
    }
    Some(kernel::boot::InitImage {
        ptr: ptr as *const u8,
        len: read,
    })
}

#[cfg(test)]
fn main() {}
