//! ACPI DMAR inventory probe for S12.3 golden-machine IOMMU visibility.

use kernel::boot::IommuProbeInfo;
use uefi::prelude::*;
use uefi::table::cfg::{ACPI_GUID, ACPI2_GUID};

const ACPI_TABLE_HEADER_LEN: usize = 36;

pub fn probe_iommu(st: &SystemTable<Boot>) -> IommuProbeInfo {
    let Some(rsdp) = find_rsdp(st) else {
        return IommuProbeInfo::acpi_missing();
    };

    let revision = unsafe { core::ptr::read(rsdp.add(15)) };
    if revision >= 2 {
        let xsdt_addr = unsafe { core::ptr::read_unaligned(rsdp.add(24) as *const u64) };
        if xsdt_addr != 0 && acpi_table_has_dmar(xsdt_addr as *const u8, true) {
            return IommuProbeInfo::present();
        }
    }

    let rsdt_addr = unsafe { core::ptr::read_unaligned(rsdp.add(16) as *const u32) } as u64;
    if rsdt_addr != 0 && acpi_table_has_dmar(rsdt_addr as *const u8, false) {
        return IommuProbeInfo::present();
    }

    IommuProbeInfo::dmar_missing()
}

fn find_rsdp(st: &SystemTable<Boot>) -> Option<*const u8> {
    for entry in st.config_table() {
        if entry.guid == ACPI2_GUID || entry.guid == ACPI_GUID {
            return Some(entry.address.cast());
        }
    }
    None
}

fn acpi_table_has_dmar(root: *const u8, xsdt: bool) -> bool {
    if root.is_null() {
        return false;
    }

    let sig = unsafe { core::ptr::read(root as *const [u8; 4]) };
    let expected = if xsdt { *b"XSDT" } else { *b"RSDT" };
    if sig != expected {
        return false;
    }

    let length = unsafe { core::ptr::read_unaligned(root.add(4) as *const u32) } as usize;
    if length < ACPI_TABLE_HEADER_LEN {
        return false;
    }

    let entry_bytes = if xsdt { 8 } else { 4 };
    let entries_len = length.saturating_sub(ACPI_TABLE_HEADER_LEN);
    let entry_count = entries_len / entry_bytes;

    for i in 0..entry_count {
        let table_addr = if xsdt {
            let offset = ACPI_TABLE_HEADER_LEN + i * 8;
            unsafe { core::ptr::read_unaligned(root.add(offset) as *const u64) }
        } else {
            let offset = ACPI_TABLE_HEADER_LEN + i * 4;
            (unsafe { core::ptr::read_unaligned(root.add(offset) as *const u32) }) as u64
        };

        if table_addr == 0 {
            continue;
        }

        let table_sig = unsafe { core::ptr::read(table_addr as *const [u8; 4]) };
        if table_sig == *b"DMAR" {
            return true;
        }
    }

    false
}
