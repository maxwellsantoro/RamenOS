//! UEFI A/B boot-slot metadata probe for S13.8 atomic update graduation.
//!
//! Reads vendor variable `RamenAbSlot` (see `hardware/storage_contract_v0.toml`).

use kernel::boot::AtomicUpdateProbeInfo;
use uefi::prelude::*;
use uefi::table::runtime::VariableVendor;
use uefi::{cstr16, guid};

/// RamenOS persistent-storage vendor namespace (pinned in storage contract).
const RAMEN_STORAGE_VENDOR: VariableVendor =
    VariableVendor(guid!("a3b8c14e-5f20-4d71-9e62-1308ab080000"));

const AB_SLOT_SCHEMA_V1: u8 = 1;

pub fn probe_ab_slot(st: &SystemTable<Boot>) -> AtomicUpdateProbeInfo {
    let rt = st.runtime_services();
    let name = cstr16!("RamenAbSlot");
    let mut buf = [0u8; 4];

    let Ok((data, _attrs)) = rt.get_variable(name, &RAMEN_STORAGE_VENDOR, &mut buf) else {
        return AtomicUpdateProbeInfo::missing();
    };

    if data.len() < 3 {
        return AtomicUpdateProbeInfo::missing();
    }

    let schema = data[0];
    let active_slot = data[1];
    let rollback_ready = data[2] == 1;

    if schema != AB_SLOT_SCHEMA_V1 || active_slot > 1 {
        return AtomicUpdateProbeInfo::missing();
    }

    if rollback_ready {
        AtomicUpdateProbeInfo::ok(active_slot)
    } else {
        AtomicUpdateProbeInfo::rollback_not_ready(active_slot)
    }
}
