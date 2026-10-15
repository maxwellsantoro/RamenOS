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

crate::impl_wire_payload!(ReadBlocks {
    request_id: u64,
    lba: u64,
    block_count: u32,
    block_size: u32,
    buffer_shm_cap: u64,
    buffer_offset: u64,
});

pub const MSG_BLOCK_V1_READ_BLOCKS_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReadBlocksReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_read: u32,
}

crate::impl_wire_payload!(ReadBlocksReply {
    request_id: u64,
    status: u32,
    bytes_read: u32,
});

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

crate::impl_wire_payload!(WriteBlocks {
    request_id: u64,
    lba: u64,
    block_count: u32,
    block_size: u32,
    data_shm_cap: u64,
    data_offset: u64,
});

pub const MSG_BLOCK_V1_WRITE_BLOCKS_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WriteBlocksReply {
    pub request_id: u64,
    pub status: u32,
    pub bytes_written: u32,
}

crate::impl_wire_payload!(WriteBlocksReply {
    request_id: u64,
    status: u32,
    bytes_written: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_block_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<ReadBlocks>()];
        let value = ReadBlocks::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadBlocks>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadBlocks>()];
        let offset = core::mem::offset_of!(ReadBlocks, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocks, lba);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocks, block_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocks, block_size);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocks, buffer_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocks, buffer_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadBlocks initialized bytes");
        let again = ReadBlocks::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.lba, again.lba);
        assert_eq!(value.block_count, again.block_count);
        assert_eq!(value.block_size, again.block_size);
        assert_eq!(value.buffer_shm_cap, again.buffer_shm_cap);
        assert_eq!(value.buffer_offset, again.buffer_offset);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReadBlocksReply>()];
        let value = ReadBlocksReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReadBlocksReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReadBlocksReply>()];
        let offset = core::mem::offset_of!(ReadBlocksReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocksReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReadBlocksReply, bytes_read);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReadBlocksReply initialized bytes");
        let again = ReadBlocksReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_read, again.bytes_read);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<WriteBlocks>()];
        let value = WriteBlocks::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<WriteBlocks>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<WriteBlocks>()];
        let offset = core::mem::offset_of!(WriteBlocks, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocks, lba);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocks, block_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocks, block_size);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocks, data_shm_cap);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocks, data_offset);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "WriteBlocks initialized bytes");
        let again = WriteBlocks::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.lba, again.lba);
        assert_eq!(value.block_count, again.block_count);
        assert_eq!(value.block_size, again.block_size);
        assert_eq!(value.data_shm_cap, again.data_shm_cap);
        assert_eq!(value.data_offset, again.data_offset);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<WriteBlocksReply>()];
        let value = WriteBlocksReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<WriteBlocksReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<WriteBlocksReply>()];
        let offset = core::mem::offset_of!(WriteBlocksReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocksReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(WriteBlocksReply, bytes_written);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "WriteBlocksReply initialized bytes");
        let again = WriteBlocksReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.bytes_written, again.bytes_written);
    }
}
