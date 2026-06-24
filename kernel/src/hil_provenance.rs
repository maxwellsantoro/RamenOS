//! HIL graduation provenance markers printed to serial for evidence bundles.

use crate::boot;
use crate::kprintln;

/// Emit build-time and boot-time provenance for HIL evidence collection.
pub fn print_boot_evidence(init_profile: &str) {
    let Some(p) = boot::hil_build_provenance() else {
        kprintln!("hil_evidence: provenance=missing");
        return;
    };

    kprintln!("hil_evidence: git_sha={}", p.git_sha);
    kprintln!("hil_evidence: init_profile={}", init_profile);
    kprintln!("hil_evidence: machine_id={}", p.machine_id);
    kprintln!(
        "hil_evidence: storage_manifest_sha256={}",
        p.storage_manifest_sha256
    );
    kprintln!("hil_evidence: kernel_efi_sha256={}", p.kernel_efi_sha256);
    kprintln!("hil_evidence: init_img_sha256={}", p.init_img_sha256);

    if let Some(info) = boot::boot_epoch_nonce_info() {
        if info.present {
            kprintln!("hil_evidence: boot_epoch_nonce={:016x}", info.nonce);
        } else {
            kprintln!("hil_evidence: boot_epoch_nonce=0");
        }
    } else {
        kprintln!("hil_evidence: boot_epoch_nonce=0");
    }
}
