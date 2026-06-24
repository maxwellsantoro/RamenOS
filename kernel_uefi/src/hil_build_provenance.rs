//! Build-time provenance baked into the UEFI image at link time.

use kernel::boot::HilBuildProvenance;

pub const BUILD_PROVENANCE: HilBuildProvenance = HilBuildProvenance {
    git_sha: env!("RAMEN_GIT_SHA"),
    machine_id: env!("RAMEN_MACHINE_ID"),
    storage_manifest_sha256: env!("RAMEN_STORAGE_MANIFEST_SHA256"),
    kernel_efi_sha256: env!("RAMEN_KERNEL_EFI_SHA256"),
    init_img_sha256: env!("RAMEN_INIT_IMG_SHA256"),
};
