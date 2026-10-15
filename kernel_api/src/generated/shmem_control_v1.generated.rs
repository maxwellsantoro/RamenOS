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

crate::impl_wire_payload!(CloseRegion {
    request_id: u64,
    caller_domain_id: u64,
    region_id: u64,
});

pub const MSG_SHMEM_CONTROL_V1_CLOSE_REGION_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CloseRegionReply {
    pub request_id: u64,
    pub region_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CloseRegionReply {
    request_id: u64,
    region_id: u64,
    status: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(CreateRegion {
    request_id: u64,
    owner_domain_id: u64,
    size_bytes: u64,
    flags: u32,
    page_size: u32,
});

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

crate::impl_wire_payload!(CreateRegionReply {
    request_id: u64,
    region_id: u64,
    shm_cap: u64,
    phys_addr: u64,
    status: u32,
    reserved: u32,
});

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

crate::impl_wire_payload!(MapRegion {
    request_id: u64,
    caller_domain_id: u64,
    region_id: u64,
    target_domain_id: u64,
    shm_cap: u64,
    rights: u32,
    cache_mode: u32,
});

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

crate::impl_wire_payload!(MapRegionReply {
    request_id: u64,
    region_id: u64,
    mapping_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_SHMEM_CONTROL_V1_SHMEM_READ: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemRead {
    pub shm_cap: u64,
    pub offset: u64,
    pub len: u32,
}

crate::impl_wire_payload!(ShmemRead {
    shm_cap: u64,
    offset: u64,
    len: u32,
});

pub const MSG_SHMEM_CONTROL_V1_SHMEM_READ_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemReadReply {
    pub status: u32,
    pub bytes_read: u32,
}

crate::impl_wire_payload!(ShmemReadReply {
    status: u32,
    bytes_read: u32,
});

pub const MSG_SHMEM_CONTROL_V1_SHMEM_WRITE: u32 = 11;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemWrite {
    pub shm_cap: u64,
    pub offset: u64,
    pub data_offset: u64,
    pub len: u32,
}

crate::impl_wire_payload!(ShmemWrite {
    shm_cap: u64,
    offset: u64,
    data_offset: u64,
    len: u32,
});

pub const MSG_SHMEM_CONTROL_V1_SHMEM_WRITE_REPLY: u32 = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShmemWriteReply {
    pub status: u32,
    pub bytes_written: u32,
}

crate::impl_wire_payload!(ShmemWriteReply {
    status: u32,
    bytes_written: u32,
});

pub const MSG_SHMEM_CONTROL_V1_UNMAP_REGION: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnmapRegion {
    pub request_id: u64,
    pub caller_domain_id: u64,
    pub mapping_id: u64,
    pub target_domain_id: u64,
}

crate::impl_wire_payload!(UnmapRegion {
    request_id: u64,
    caller_domain_id: u64,
    mapping_id: u64,
    target_domain_id: u64,
});

pub const MSG_SHMEM_CONTROL_V1_UNMAP_REGION_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UnmapRegionReply {
    pub request_id: u64,
    pub mapping_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(UnmapRegionReply {
    request_id: u64,
    mapping_id: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_shmem_control_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<CloseRegion>()];
        let value = CloseRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CloseRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CloseRegion>()];
        let offset = core::mem::offset_of!(CloseRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseRegion, caller_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseRegion, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CloseRegion initialized bytes");
        let again = CloseRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.caller_domain_id, again.caller_domain_id);
        assert_eq!(value.region_id, again.region_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CloseRegionReply>()];
        let value = CloseRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CloseRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CloseRegionReply>()];
        let offset = core::mem::offset_of!(CloseRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseRegionReply, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CloseRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CloseRegionReply initialized bytes");
        let again = CloseRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CreateRegion>()];
        let value = CreateRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateRegion>()];
        let offset = core::mem::offset_of!(CreateRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegion, owner_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegion, size_bytes);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegion, flags);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegion, page_size);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateRegion initialized bytes");
        let again = CreateRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.owner_domain_id, again.owner_domain_id);
        assert_eq!(value.size_bytes, again.size_bytes);
        assert_eq!(value.flags, again.flags);
        assert_eq!(value.page_size, again.page_size);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CreateRegionReply>()];
        let value = CreateRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateRegionReply>()];
        let offset = core::mem::offset_of!(CreateRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegionReply, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegionReply, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegionReply, phys_addr);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateRegionReply initialized bytes");
        let again = CreateRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.phys_addr, again.phys_addr);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<MapRegion>()];
        let value = MapRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<MapRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<MapRegion>()];
        let offset = core::mem::offset_of!(MapRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, caller_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, target_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, rights);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegion, cache_mode);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "MapRegion initialized bytes");
        let again = MapRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.caller_domain_id, again.caller_domain_id);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.target_domain_id, again.target_domain_id);
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.rights, again.rights);
        assert_eq!(value.cache_mode, again.cache_mode);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<MapRegionReply>()];
        let value = MapRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<MapRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<MapRegionReply>()];
        let offset = core::mem::offset_of!(MapRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegionReply, region_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegionReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(MapRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "MapRegionReply initialized bytes");
        let again = MapRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.region_id, again.region_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ShmemRead>()];
        let value = ShmemRead::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ShmemRead>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ShmemRead>()];
        let offset = core::mem::offset_of!(ShmemRead, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemRead, offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemRead, len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ShmemRead initialized bytes");
        let again = ShmemRead::decode(&output).unwrap();
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.offset, again.offset);
        assert_eq!(value.len, again.len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ShmemReadReply>()];
        let value = ShmemReadReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ShmemReadReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ShmemReadReply>()];
        let offset = core::mem::offset_of!(ShmemReadReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemReadReply, bytes_read);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ShmemReadReply initialized bytes");
        let again = ShmemReadReply::decode(&output).unwrap();
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_read, again.bytes_read);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ShmemWrite>()];
        let value = ShmemWrite::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ShmemWrite>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ShmemWrite>()];
        let offset = core::mem::offset_of!(ShmemWrite, shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemWrite, offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemWrite, data_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemWrite, len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ShmemWrite initialized bytes");
        let again = ShmemWrite::decode(&output).unwrap();
        assert_eq!(value.shm_cap, again.shm_cap);
        assert_eq!(value.offset, again.offset);
        assert_eq!(value.data_offset, again.data_offset);
        assert_eq!(value.len, again.len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ShmemWriteReply>()];
        let value = ShmemWriteReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ShmemWriteReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ShmemWriteReply>()];
        let offset = core::mem::offset_of!(ShmemWriteReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShmemWriteReply, bytes_written);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ShmemWriteReply initialized bytes");
        let again = ShmemWriteReply::decode(&output).unwrap();
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_written, again.bytes_written);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<UnmapRegion>()];
        let value = UnmapRegion::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<UnmapRegion>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<UnmapRegion>()];
        let offset = core::mem::offset_of!(UnmapRegion, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegion, caller_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegion, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegion, target_domain_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "UnmapRegion initialized bytes");
        let again = UnmapRegion::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.caller_domain_id, again.caller_domain_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.target_domain_id, again.target_domain_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<UnmapRegionReply>()];
        let value = UnmapRegionReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<UnmapRegionReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<UnmapRegionReply>()];
        let offset = core::mem::offset_of!(UnmapRegionReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegionReply, mapping_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegionReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(UnmapRegionReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "UnmapRegionReply initialized bytes");
        let again = UnmapRegionReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.mapping_id, again.mapping_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
