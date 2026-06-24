use core::sync::atomic::{AtomicBool, Ordering};

use kernel_api::trace::TAG_BOOT;

use crate::mm::address::PhysAddr;
use crate::{arch, init, kprintln, mm, serial, trace_ring};

#[derive(Copy, Clone)]
pub struct InitImage {
    pub ptr: *const u8,
    pub len: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub len_bytes: u64,
    pub kind: RegionKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegionKind {
    Usable,
    Reserved,
    LoaderCode,
    LoaderData,
}

/// A fixed-size memory map passed from the bootloader.
/// S8 Phase 3 constraint: Static array to avoid heap dependency in bootloader glue.
#[derive(Copy, Clone)]
pub struct BootMemoryMap {
    pub regions: [MemoryRegion; 64],
    pub count: usize,
}

impl BootMemoryMap {
    pub const fn new() -> Self {
        Self {
            regions: [MemoryRegion {
                // SAFETY: PhysAddr::new is safe for 0 (NULL address) as it is
                // used as a sentinel/invalid value for uninitialized regions.
                start: unsafe { PhysAddr::new(0) },
                len_bytes: 0,
                kind: RegionKind::Reserved,
            }; 64],
            count: 0,
        }
    }

    pub fn add(&mut self, start: u64, len_bytes: u64, kind: RegionKind) {
        if self.count < self.regions.len() {
            self.regions[self.count] = MemoryRegion {
                // SAFETY: PhysAddr::new is safe here because:
                // - For valid addresses, UEFI guarantees the memory map entries are correct
                // - The address comes from UEFI boot services which validates memory regions
                start: unsafe { PhysAddr::new(start) },
                len_bytes,
                kind,
            };
            self.count += 1;
        }
    }
}

impl Default for BootMemoryMap {
    fn default() -> Self {
        Self::new()
    }
}

static INIT_SET: AtomicBool = AtomicBool::new(false);
static mut INIT_IMAGE: InitImage = InitImage {
    ptr: core::ptr::null(),
    len: 0,
};

static MEM_MAP_SET: AtomicBool = AtomicBool::new(false);
static mut MEM_MAP: BootMemoryMap = BootMemoryMap::new();

/// Result of a UEFI GOP probe performed in `kernel_uefi` before `boot_main`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GopProbeInfo {
    pub status: u32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,
    pub fill_ok: bool,
}

pub const GOP_PROBE_OK: u32 = 0;
pub const GOP_PROBE_MISSING: u32 = 1;
pub const GOP_PROBE_FILL_FAILED: u32 = 2;

impl GopProbeInfo {
    pub const fn missing() -> Self {
        Self {
            status: GOP_PROBE_MISSING,
            width: 0,
            height: 0,
            pixel_format: 0,
            fill_ok: false,
        }
    }
}

static GOP_PROBE_SET: AtomicBool = AtomicBool::new(false);
static mut GOP_PROBE: GopProbeInfo = GopProbeInfo::missing();

pub fn set_init_image(image: InitImage) {
    // SAFETY: INIT_IMAGE is a static mut that is only written to during
    // single-threaded boot initialization. The Release ordering on INIT_SET
    // ensures the write is visible before the flag is set.
    unsafe {
        INIT_IMAGE = image;
    }
    INIT_SET.store(true, Ordering::Release);
}

pub fn set_gop_probe(info: GopProbeInfo) {
    // SAFETY: GOP_PROBE is written only during single-threaded UEFI boot glue.
    unsafe {
        GOP_PROBE = info;
    }
    GOP_PROBE_SET.store(true, Ordering::Release);
}

pub fn gop_probe_info() -> Option<GopProbeInfo> {
    if GOP_PROBE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering on GOP_PROBE_SET ensures the probe write is visible.
        Some(unsafe { GOP_PROBE })
    } else {
        None
    }
}

/// Result of an ACPI DMAR inventory probe performed in `kernel_uefi` before `boot_main`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IommuProbeInfo {
    pub status: u32,
    pub dmar_present: bool,
}

pub const IOMMU_PROBE_OK: u32 = 0;
pub const IOMMU_PROBE_ACPI_MISSING: u32 = 1;
pub const IOMMU_PROBE_DMAR_MISSING: u32 = 2;

impl IommuProbeInfo {
    pub const fn acpi_missing() -> Self {
        Self {
            status: IOMMU_PROBE_ACPI_MISSING,
            dmar_present: false,
        }
    }

    pub const fn dmar_missing() -> Self {
        Self {
            status: IOMMU_PROBE_DMAR_MISSING,
            dmar_present: false,
        }
    }

    pub const fn present() -> Self {
        Self {
            status: IOMMU_PROBE_OK,
            dmar_present: true,
        }
    }
}

static IOMMU_PROBE_SET: AtomicBool = AtomicBool::new(false);
static mut IOMMU_PROBE: IommuProbeInfo = IommuProbeInfo::acpi_missing();

pub fn set_iommu_probe(info: IommuProbeInfo) {
    // SAFETY: IOMMU_PROBE is written only during single-threaded UEFI boot glue.
    unsafe {
        IOMMU_PROBE = info;
    }
    IOMMU_PROBE_SET.store(true, Ordering::Release);
}

pub fn iommu_probe_info() -> Option<IommuProbeInfo> {
    if IOMMU_PROBE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering on IOMMU_PROBE_SET ensures the probe write is visible.
        Some(unsafe { IOMMU_PROBE })
    } else {
        None
    }
}

/// Result of a UEFI loaded-image device path probe for S13.7 metal NVMe boot.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NvmeBootProbeInfo {
    pub status: u32,
    pub nvme_boot: bool,
}

pub const NVME_BOOT_PROBE_OK: u32 = 0;
pub const NVME_BOOT_PROBE_MISSING: u32 = 1;
pub const NVME_BOOT_PROBE_NOT_NVME: u32 = 2;

impl NvmeBootProbeInfo {
    pub const fn missing() -> Self {
        Self {
            status: NVME_BOOT_PROBE_MISSING,
            nvme_boot: false,
        }
    }

    pub const fn not_nvme() -> Self {
        Self {
            status: NVME_BOOT_PROBE_NOT_NVME,
            nvme_boot: false,
        }
    }

    pub const fn ok() -> Self {
        Self {
            status: NVME_BOOT_PROBE_OK,
            nvme_boot: true,
        }
    }
}

static NVME_BOOT_PROBE_SET: AtomicBool = AtomicBool::new(false);
static mut NVME_BOOT_PROBE: NvmeBootProbeInfo = NvmeBootProbeInfo::missing();

pub fn set_nvme_boot_probe(info: NvmeBootProbeInfo) {
    // SAFETY: NVME_BOOT_PROBE is written only during single-threaded UEFI boot glue.
    unsafe {
        NVME_BOOT_PROBE = info;
    }
    NVME_BOOT_PROBE_SET.store(true, Ordering::Release);
}

pub fn nvme_boot_probe_info() -> Option<NvmeBootProbeInfo> {
    if NVME_BOOT_PROBE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering on NVME_BOOT_PROBE_SET ensures the probe write is visible.
        Some(unsafe { NVME_BOOT_PROBE })
    } else {
        None
    }
}

/// Result of a UEFI A/B slot metadata probe for S13.8 atomic update graduation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AtomicUpdateProbeInfo {
    pub status: u32,
    pub active_slot: u8,
    pub rollback_ready: bool,
}

pub const ATOMIC_UPDATE_PROBE_OK: u32 = 0;
pub const ATOMIC_UPDATE_PROBE_MISSING: u32 = 1;
pub const ATOMIC_UPDATE_PROBE_ROLLBACK_NOT_READY: u32 = 2;

impl AtomicUpdateProbeInfo {
    pub const fn missing() -> Self {
        Self {
            status: ATOMIC_UPDATE_PROBE_MISSING,
            active_slot: 0,
            rollback_ready: false,
        }
    }

    pub const fn rollback_not_ready(active_slot: u8) -> Self {
        Self {
            status: ATOMIC_UPDATE_PROBE_ROLLBACK_NOT_READY,
            active_slot,
            rollback_ready: false,
        }
    }

    pub const fn ok(active_slot: u8) -> Self {
        Self {
            status: ATOMIC_UPDATE_PROBE_OK,
            active_slot,
            rollback_ready: true,
        }
    }
}

static ATOMIC_UPDATE_PROBE_SET: AtomicBool = AtomicBool::new(false);
static mut ATOMIC_UPDATE_PROBE: AtomicUpdateProbeInfo = AtomicUpdateProbeInfo::missing();

pub fn set_atomic_update_probe(info: AtomicUpdateProbeInfo) {
    // SAFETY: ATOMIC_UPDATE_PROBE is written only during single-threaded UEFI boot glue.
    unsafe {
        ATOMIC_UPDATE_PROBE = info;
    }
    ATOMIC_UPDATE_PROBE_SET.store(true, Ordering::Release);
}

pub fn atomic_update_probe_info() -> Option<AtomicUpdateProbeInfo> {
    if ATOMIC_UPDATE_PROBE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering on ATOMIC_UPDATE_PROBE_SET ensures the probe write is visible.
        Some(unsafe { ATOMIC_UPDATE_PROBE })
    } else {
        None
    }
}

/// Build-time provenance forwarded from `kernel_uefi` before `boot_main`.
#[derive(Copy, Clone, Debug)]
pub struct HilBuildProvenance {
    pub git_sha: &'static str,
    pub machine_id: &'static str,
    pub storage_manifest_sha256: &'static str,
    pub kernel_efi_sha256: &'static str,
    pub init_img_sha256: &'static str,
}

static HIL_PROVENANCE_SET: AtomicBool = AtomicBool::new(false);
static mut HIL_BUILD_PROVENANCE: Option<HilBuildProvenance> = None;

pub fn set_hil_build_provenance(info: HilBuildProvenance) {
    // SAFETY: written only during single-threaded UEFI boot glue.
    unsafe {
        HIL_BUILD_PROVENANCE = Some(info);
    }
    HIL_PROVENANCE_SET.store(true, Ordering::Release);
}

pub fn hil_build_provenance() -> Option<HilBuildProvenance> {
    if HIL_PROVENANCE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering ensures the provenance write is visible.
        unsafe { HIL_BUILD_PROVENANCE }
    } else {
        None
    }
}

/// Boot-epoch nonce from UEFI variable `RamenBootNonce` (HIL graduation).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BootEpochNonceInfo {
    pub present: bool,
    pub nonce: u64,
}

impl BootEpochNonceInfo {
    pub const fn missing() -> Self {
        Self {
            present: false,
            nonce: 0,
        }
    }

    pub const fn present(nonce: u64) -> Self {
        Self {
            present: true,
            nonce,
        }
    }
}

static BOOT_NONCE_SET: AtomicBool = AtomicBool::new(false);
static mut BOOT_EPOCH_NONCE: BootEpochNonceInfo = BootEpochNonceInfo::missing();

pub fn set_boot_epoch_nonce(info: BootEpochNonceInfo) {
    // SAFETY: written only during single-threaded UEFI boot glue.
    unsafe {
        BOOT_EPOCH_NONCE = info;
    }
    BOOT_NONCE_SET.store(true, Ordering::Release);
}

pub fn boot_epoch_nonce_info() -> Option<BootEpochNonceInfo> {
    if BOOT_NONCE_SET.load(Ordering::Acquire) {
        // SAFETY: Acquire ordering ensures the nonce write is visible.
        Some(unsafe { BOOT_EPOCH_NONCE })
    } else {
        None
    }
}

pub fn set_memory_map(map: BootMemoryMap) {
    // SAFETY: MEM_MAP is a static mut that is only written to during
    // single-threaded boot initialization. The Release ordering on MEM_MAP_SET
    // ensures the write is visible before the flag is set.
    unsafe {
        MEM_MAP = map;
    }
    MEM_MAP_SET.store(true, Ordering::Release);
}

pub fn boot_main() -> ! {
    serial::init();
    let writer = trace_ring::TraceWriter::claim().expect("trace writer already claimed");
    kprintln!("RAMEN OS S0 boot");
    trace_ring::emit(&writer, TAG_BOOT, 0, 0);

    // S8 Phase 4: Initialize address space table with kernel page table root
    let kernel_pt_root = arch::get_current_page_table_root();
    kprintln!("boot: kernel page table root = {:?}", kernel_pt_root);
    mm::init_address_space_table(kernel_pt_root);

    // S8 Phase 3: Initialize physical memory allocator
    if MEM_MAP_SET.load(Ordering::Acquire) {
        // SAFETY: MEM_MAP is a static mut populated by UEFI boot code before
        // this function is called. The pointer is valid for the lifetime of
        // the kernel because UEFI reserves this memory. Acquire ordering on
        // MEM_MAP_SET ensures we see all writes from set_memory_map().
        let map = unsafe { &MEM_MAP };
        kprintln!("boot: memory map has {} regions", map.count);
        mm::init(map);
    } else {
        kprintln!("boot: warning: no memory map provided");
    }

    if INIT_SET.load(Ordering::Acquire) {
        // SAFETY: INIT_IMAGE is a static mut populated by UEFI boot code before
        // this function is called. The image pointer and length are valid because
        // UEFI guarantees the memory is reserved. Acquire ordering on INIT_SET
        // ensures we see all writes from set_init_image().
        let image = unsafe { INIT_IMAGE };
        init::run(&writer, image);
    } else {
        kprintln!("init: missing image");
    }

    kprintln!("kernel: idle");
    loop {
        arch::halt();
    }
}
