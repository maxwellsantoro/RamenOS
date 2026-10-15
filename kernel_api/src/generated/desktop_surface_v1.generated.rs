// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/desktop_surface_v1.toml

// namespace = services.desktop_surface, version = 1

pub const DESKTOP_SURFACE_V1_PROTOCOL_ID: u32 = 833;

pub const MSG_DESKTOP_SURFACE_V1_ACQUIRE: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Acquire {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub buffer_index: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(Acquire {
    request_id: u64,
    surface_id: u64,
    surface_generation: u64,
    buffer_index: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_ACQUIRE_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AcquireReply {
    pub request_id: u64,
    pub mapping_generation: u64,
    pub buffer_shm: u64,
    pub status: u32,
    pub byte_len: u32,
}

crate::impl_wire_payload!(AcquireReply {
    request_id: u64,
    mapping_generation: u64,
    buffer_shm: u64,
    status: u32,
    byte_len: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_CONSUME: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Consume {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub sequence: u64,
}

crate::impl_wire_payload!(Consume {
    request_id: u64,
    surface_id: u64,
    surface_generation: u64,
    sequence: u64,
});

pub const MSG_DESKTOP_SURFACE_V1_CONSUME_REPLY: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ConsumeReply {
    pub request_id: u64,
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ConsumeReply {
    request_id: u64,
    sequence: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_CREATE: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Create {
    pub request_id: u64,
    pub session_id: u64,
    pub session_generation: u64,
    pub instance_id: u64,
    pub instance_generation: u64,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(Create {
    request_id: u64,
    session_id: u64,
    session_generation: u64,
    instance_id: u64,
    instance_generation: u64,
    width: u32,
    height: u32,
    format: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_CREATE_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CreateReply {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub buffer0_shm: u64,
    pub buffer1_shm: u64,
    pub status: u32,
    pub stride: u32,
    pub format: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(CreateReply {
    request_id: u64,
    surface_id: u64,
    surface_generation: u64,
    buffer0_shm: u64,
    buffer1_shm: u64,
    status: u32,
    stride: u32,
    format: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_DESTROY: u32 = 9;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Destroy {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
}

crate::impl_wire_payload!(Destroy {
    request_id: u64,
    surface_id: u64,
    surface_generation: u64,
});

pub const MSG_DESKTOP_SURFACE_V1_DESTROY_REPLY: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DestroyReply {
    pub request_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(DestroyReply {
    request_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_PRESENT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Present {
    pub request_id: u64,
    pub surface_id: u64,
    pub surface_generation: u64,
    pub mapping_generation: u64,
    pub sequence: u64,
    pub buffer_index: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(Present {
    request_id: u64,
    surface_id: u64,
    surface_generation: u64,
    mapping_generation: u64,
    sequence: u64,
    buffer_index: u32,
    reserved: u32,
});

pub const MSG_DESKTOP_SURFACE_V1_PRESENT_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PresentReply {
    pub request_id: u64,
    pub sequence: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(PresentReply {
    request_id: u64,
    sequence: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_desktop_surface_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Acquire>()];
        let value = Acquire::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Acquire>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Acquire>()];
        let offset = core::mem::offset_of!(Acquire, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Acquire, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Acquire, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Acquire, buffer_index);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Acquire, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Acquire initialized bytes");
        let again = Acquire::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
        assert_eq!(value.buffer_index, again.buffer_index);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<AcquireReply>()];
        let value = AcquireReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<AcquireReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<AcquireReply>()];
        let offset = core::mem::offset_of!(AcquireReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcquireReply, mapping_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcquireReply, buffer_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcquireReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(AcquireReply, byte_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "AcquireReply initialized bytes");
        let again = AcquireReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.mapping_generation, again.mapping_generation);
        assert_eq!(value.buffer_shm, again.buffer_shm);
        assert_eq!(value.status, again.status);
        assert_eq!(value.byte_len, again.byte_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Consume>()];
        let value = Consume::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Consume>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Consume>()];
        let offset = core::mem::offset_of!(Consume, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Consume, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Consume, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Consume, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Consume initialized bytes");
        let again = Consume::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
        assert_eq!(value.sequence, again.sequence);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ConsumeReply>()];
        let value = ConsumeReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ConsumeReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ConsumeReply>()];
        let offset = core::mem::offset_of!(ConsumeReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConsumeReply, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConsumeReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ConsumeReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ConsumeReply initialized bytes");
        let again = ConsumeReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Create>()];
        let value = Create::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Create>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Create>()];
        let offset = core::mem::offset_of!(Create, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, session_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, instance_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, instance_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, width);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, height);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, format);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Create, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Create initialized bytes");
        let again = Create::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.session_generation, again.session_generation);
        assert_eq!(value.instance_id, again.instance_id);
        assert_eq!(value.instance_generation, again.instance_generation);
        assert_eq!(value.width, again.width);
        assert_eq!(value.height, again.height);
        assert_eq!(value.format, again.format);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<CreateReply>()];
        let value = CreateReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<CreateReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<CreateReply>()];
        let offset = core::mem::offset_of!(CreateReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, buffer0_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, buffer1_shm);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, stride);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, format);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(CreateReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "CreateReply initialized bytes");
        let again = CreateReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
        assert_eq!(value.buffer0_shm, again.buffer0_shm);
        assert_eq!(value.buffer1_shm, again.buffer1_shm);
        assert_eq!(value.status, again.status);
        assert_eq!(value.stride, again.stride);
        assert_eq!(value.format, again.format);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Destroy>()];
        let value = Destroy::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Destroy>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Destroy>()];
        let offset = core::mem::offset_of!(Destroy, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Destroy, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Destroy, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Destroy initialized bytes");
        let again = Destroy::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<DestroyReply>()];
        let value = DestroyReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<DestroyReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<DestroyReply>()];
        let offset = core::mem::offset_of!(DestroyReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DestroyReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DestroyReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "DestroyReply initialized bytes");
        let again = DestroyReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Present>()];
        let value = Present::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Present>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Present>()];
        let offset = core::mem::offset_of!(Present, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, surface_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, surface_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, mapping_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, buffer_index);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Present, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Present initialized bytes");
        let again = Present::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.surface_id, again.surface_id);
        assert_eq!(value.surface_generation, again.surface_generation);
        assert_eq!(value.mapping_generation, again.mapping_generation);
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.buffer_index, again.buffer_index);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PresentReply>()];
        let value = PresentReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PresentReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PresentReply>()];
        let offset = core::mem::offset_of!(PresentReply, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PresentReply, sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PresentReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PresentReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PresentReply initialized bytes");
        let again = PresentReply::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.sequence, again.sequence);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
