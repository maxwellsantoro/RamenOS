// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/block_v1.toml

// namespace = harness.block, version = 1

pub const BLOCK_V1_PROTOCOL_ID: u32 = 801;

pub const MSG_BLOCK_V1_READ_BLOCKS: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadBlocks {
    pub request_id: u64,
    pub lba: u64,
    pub block_count: u32,
    pub block_size: u32,
    pub buffer_shm_cap: u64,
    pub buffer_offset: u64,
}

pub const MSG_BLOCK_V1_READ_BLOCKS_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadBlocksReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_read: u32,
}

pub const MSG_BLOCK_V1_WRITE_BLOCKS: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WriteBlocks {
    pub request_id: u64,
    pub lba: u64,
    pub block_count: u32,
    pub block_size: u32,
    pub data_shm_cap: u64,
    pub data_offset: u64,
}

pub const MSG_BLOCK_V1_WRITE_BLOCKS_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WriteBlocksReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_written: u32,
}
