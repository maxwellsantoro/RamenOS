//! UEFI boot-epoch nonce for HIL graduation evidence (`RamenBootNonce`).

use kernel::boot::BootEpochNonceInfo;
use uefi::prelude::*;
use uefi::table::runtime::VariableVendor;
use uefi::{cstr16, guid};

const RAMEN_STORAGE_VENDOR: VariableVendor =
    VariableVendor(guid!("a3b8c14e-5f20-4d71-9e62-1308ab080000"));

pub fn probe_boot_nonce(st: &SystemTable<Boot>) -> BootEpochNonceInfo {
    let rt = st.runtime_services();
    let name = cstr16!("RamenBootNonce");
    let mut buf = [0u8; 8];

    let Ok((data, _attrs)) = rt.get_variable(name, &RAMEN_STORAGE_VENDOR, &mut buf) else {
        return BootEpochNonceInfo::missing();
    };

    if data.len() < 8 {
        return BootEpochNonceInfo::missing();
    }

    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&data[..8]);
    let nonce = u64::from_le_bytes(nonce_bytes);
    if nonce == 0 {
        BootEpochNonceInfo::missing()
    } else {
        BootEpochNonceInfo::present(nonce)
    }
}
