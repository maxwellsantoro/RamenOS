//! Baked virtio-blk harness.block Oracle sector bytes for S13.6 QEMU runtime I/O.
//!
//! Init trace provenance: `drivers/reference_vaults/virtio-blk/traces/oracle_init_trace.json`

const fn fill_sector(mul: u8) -> [u8; 512] {
    let mut buf = [0u8; 512];
    let mut i = 0usize;
    while i < 512 {
        buf[i] = (i as u8).wrapping_mul(mul);
        i += 1;
    }
    buf
}

/// Oracle read sector staged by the block harness provider.
pub const S13_ORACLE_READ_PAYLOAD: [u8; 512] = fill_sector(0x13);

/// Oracle write sector validated by the block harness provider.
pub const S13_ORACLE_WRITE_PAYLOAD: [u8; 512] = fill_sector(0x37);

pub const S13_ORACLE_BLOCK_SIZE: u32 = 512;
pub const S13_ORACLE_BLOCK_COUNT: u32 = 1;
pub const S13_ORACLE_READ_LBA: u64 = 0;
pub const S13_ORACLE_WRITE_LBA: u64 = 1;
pub const S13_ORACLE_READ_OFFSET: u64 = 0;
pub const S13_ORACLE_WRITE_OFFSET: u64 = 512;
pub const S13_ORACLE_READ_REQUEST_ID: u64 = 1;
pub const S13_ORACLE_WRITE_REQUEST_ID: u64 = 2;
pub const S13_ORACLE_BLOCK_SHMEM_SIZE: u64 = 4096;

/// Hex prefix of the live `oracle_init_trace.json` SHA-256 digest.
pub const S13_ORACLE_INIT_TRACE_SHA256_PREFIX: &str = "eb816f3657bb5807";

pub const BLOCK_STATUS_OK: u32 = 0;
