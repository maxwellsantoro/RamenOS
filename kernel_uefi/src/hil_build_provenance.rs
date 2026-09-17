//! Build-time provenance baked into the UEFI image at link time.

use kernel::boot::HilBuildProvenance;

pub const BUILD_PROVENANCE: HilBuildProvenance = HilBuildProvenance {
    git_sha: env!("RAMEN_GIT_SHA"),
    machine_id: env!("RAMEN_MACHINE_ID"),
    storage_manifest_sha256: env!("RAMEN_STORAGE_MANIFEST_SHA256"),
    kernel_build_id: env!("RAMEN_KERNEL_BUILD_ID"),
    init_img_sha256: env!("RAMEN_INIT_IMG_SHA256"),
};
