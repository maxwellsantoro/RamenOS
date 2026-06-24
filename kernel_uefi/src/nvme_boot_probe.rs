//! UEFI loaded-image device path probe for S13.7 metal NVMe boot graduation.
//!
//! Walks the image device path for an NVMe namespace node per UEFI §10.3.1.23.

use kernel::boot::NvmeBootProbeInfo;
use uefi::prelude::*;
use uefi::proto::device_path::{DevicePath, DevicePathNodeEnum, LoadedImageDevicePath};
use uefi::proto::loaded_image::LoadedImage;

pub fn probe_nvme_boot(st: &SystemTable<Boot>) -> NvmeBootProbeInfo {
    let bs = st.boot_services();
    let image_handle = bs.image_handle();

    if device_path_has_nvme(bs, image_handle) {
        NvmeBootProbeInfo::ok()
    } else {
        NvmeBootProbeInfo::not_nvme()
    }
}

fn device_path_has_nvme(bs: &uefi::table::boot::BootServices, image_handle: Handle) -> bool {
    if let Ok(path) = bs.open_protocol_exclusive::<LoadedImageDevicePath>(image_handle) {
        if path_contains_nvme(&path) {
            return true;
        }
    }

    if let Ok(loaded) = bs.open_protocol_exclusive::<LoadedImage>(image_handle) {
        if let Some(file_path) = loaded.file_path() {
            if path_contains_nvme(file_path) {
                return true;
            }
        }
    }

    false
}

fn path_contains_nvme(path: &DevicePath) -> bool {
    for instance in path.instance_iter() {
        for node in instance.node_iter() {
            if matches!(
                node.as_enum(),
                Ok(DevicePathNodeEnum::MessagingNvmeNamespace(_))
                    | Ok(DevicePathNodeEnum::MessagingNvmeOfNamespace(_))
            ) {
                return true;
            }
        }
    }
    false
}
