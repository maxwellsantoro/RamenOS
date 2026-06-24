// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/shmem_control_v1.toml

// namespace = shared_memory.control, version = 1

pub const SHMEM_CONTROL_V1_PROTOCOL_ID: u32 = 8;

pub const MSG_SHMEM_CONTROL_V1_CLOSE_REGION: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseRegion {
    pub request_id: u64,
    pub caller_domain_id: u64,
    pub region_id: u64,
}

pub const MSG_SHMEM_CONTROL_V1_CLOSE_REGION_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseRegionReply {
    pub request_id: u64,
    pub region_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_SHMEM_CONTROL_V1_CREATE_REGION: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateRegion {
    pub request_id: u64,
    pub owner_domain_id: u64,
    pub size_bytes: u64,
    pub flags: u32,
    pub page_size: u32,
}

pub const MSG_SHMEM_CONTROL_V1_CREATE_REGION_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateRegionReply {
    pub request_id: u64,
    pub region_id: u64,
    pub shm_cap: u64,
    pub phys_addr: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_SHMEM_CONTROL_V1_MAP_REGION: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MapRegion {
    pub request_id: u64,
    pub caller_domain_id: u64,
    pub region_id: u64,
    pub target_domain_id: u64,
    pub shm_cap: u64,
    pub rights: u32,
    pub cache_mode: u32,
}

pub const MSG_SHMEM_CONTROL_V1_MAP_REGION_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MapRegionReply {
    pub request_id: u64,
    pub region_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub reserved: u32,
}

pub const MSG_SHMEM_CONTROL_V1_SHMEM_READ: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemRead {
    pub shm_cap: u64,
    pub offset: u64,
    pub len: u32,
}

pub const MSG_SHMEM_CONTROL_V1_SHMEM_READ_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemReadReply {
    pub status: u32,
    pub bytes_read: u32,
}

pub const MSG_SHMEM_CONTROL_V1_SHMEM_WRITE: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemWrite {
    pub shm_cap: u64,
    pub offset: u64,
    pub data_offset: u64,
    pub len: u32,
}

pub const MSG_SHMEM_CONTROL_V1_SHMEM_WRITE_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemWriteReply {
    pub status: u32,
    pub bytes_written: u32,
}

pub const MSG_SHMEM_CONTROL_V1_UNMAP_REGION: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnmapRegion {
    pub request_id: u64,
    pub caller_domain_id: u64,
    pub mapping_id: u64,
    pub target_domain_id: u64,
}

pub const MSG_SHMEM_CONTROL_V1_UNMAP_REGION_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnmapRegionReply {
    pub request_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub reserved: u32,
}
