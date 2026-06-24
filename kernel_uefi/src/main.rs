#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
use core::slice;
#[cfg(not(test))]
use kernel::boot::{BootMemoryMap, RegionKind};
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

    // S8 Phase 3: Retrieve memory map and pass to kernel
    if let Some(map) = get_memory_map(&st) {
        kernel::boot::set_memory_map(map);
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
    // Allocate a buffer for the memory map.
    // 64 regions * 48 bytes approx per desc + slack
    let mut mmap_buf = [0u8; 8192];

    let mmap = bs.memory_map(&mut mmap_buf).ok()?;
    let mut boot_map = BootMemoryMap::new();

    for desc in mmap.entries() {
        let kind = match desc.ty {
            MemoryType::CONVENTIONAL => RegionKind::Usable,
            MemoryType::LOADER_CODE => RegionKind::LoaderCode,
            MemoryType::LOADER_DATA => RegionKind::LoaderData,
            MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA => RegionKind::Usable, // Available after ExitBootServices
            _ => RegionKind::Reserved,
        };

        // We only care about usable memory for the frame allocator for now.
        if kind == RegionKind::Usable {
            boot_map.add(desc.phys_start, desc.page_count * 4096, kind);
        }
    }

    Some(boot_map)
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
    let size = info.file_size() as usize;
    if size == 0 {
        return None;
    }
    let pages = (size + 0xfff) / 0x1000;
    let ptr = bs
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .ok()?;
    let buf = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, pages * 0x1000) };
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
