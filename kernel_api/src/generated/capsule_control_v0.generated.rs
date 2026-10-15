// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/capsule_control_v0.toml

// namespace = capsule.control, version = 0

pub const CAPSULE_CONTROL_V0_PROTOCOL_ID: u32 = 512;

pub const MSG_CAPSULE_CONTROL_V0_HEALTH: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Health {
    pub session_id: u64,
}

crate::impl_wire_payload!(Health { session_id: u64 });

pub const MSG_CAPSULE_CONTROL_V0_HEALTH_REPLY: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HealthReply {
    pub session_id: u64,
    pub status: u32,
    pub error_count: u32,
}

crate::impl_wire_payload!(HealthReply {
    session_id: u64,
    status: u32,
    error_count: u32,
});

pub const MSG_CAPSULE_CONTROL_V0_HELLO: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Hello {
    pub capsule_id: u64,
    pub backend_caps: u32,
    pub version_major: u16,
    pub version_minor: u16,
}

crate::impl_wire_payload!(Hello {
    capsule_id: u64,
    backend_caps: u32,
    version_major: u16,
    version_minor: u16,
});

pub const MSG_CAPSULE_CONTROL_V0_HELLO_REPLY: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct HelloReply {
    pub session_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(HelloReply {
    session_id: u64,
    status: u32,
    reserved: u32,
});

pub const MSG_CAPSULE_CONTROL_V0_SHUTDOWN: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Shutdown {
    pub session_id: u64,
    pub reason: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(Shutdown {
    session_id: u64,
    reason: u32,
    reserved: u32,
});

pub const MSG_CAPSULE_CONTROL_V0_SHUTDOWN_REPLY: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShutdownReply {
    pub session_id: u64,
    pub status: u32,
    pub reserved: u32,
}

crate::impl_wire_payload!(ShutdownReply {
    session_id: u64,
    status: u32,
    reserved: u32,
});

#[cfg(test)]
#[test]
fn wire_codec_capsule_control_v0_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Health>()];
        let value = Health::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Health>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Health>()];
        let offset = core::mem::offset_of!(Health, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Health initialized bytes");
        let again = Health::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<HealthReply>()];
        let value = HealthReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<HealthReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<HealthReply>()];
        let offset = core::mem::offset_of!(HealthReply, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(HealthReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(HealthReply, error_count);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "HealthReply initialized bytes");
        let again = HealthReply::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.error_count, again.error_count);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Hello>()];
        let value = Hello::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Hello>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Hello>()];
        let offset = core::mem::offset_of!(Hello, capsule_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Hello, backend_caps);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Hello, version_major);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Hello, version_minor);
        expected[offset..offset + <u16 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Hello initialized bytes");
        let again = Hello::decode(&output).unwrap();
        assert_eq!(value.capsule_id, again.capsule_id);
        assert_eq!(value.backend_caps, again.backend_caps);
        assert_eq!(value.version_major, again.version_major);
        assert_eq!(value.version_minor, again.version_minor);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<HelloReply>()];
        let value = HelloReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<HelloReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<HelloReply>()];
        let offset = core::mem::offset_of!(HelloReply, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(HelloReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(HelloReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "HelloReply initialized bytes");
        let again = HelloReply::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Shutdown>()];
        let value = Shutdown::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Shutdown>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Shutdown>()];
        let offset = core::mem::offset_of!(Shutdown, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Shutdown, reason);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Shutdown, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Shutdown initialized bytes");
        let again = Shutdown::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.reason, again.reason);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ShutdownReply>()];
        let value = ShutdownReply::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ShutdownReply>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ShutdownReply>()];
        let offset = core::mem::offset_of!(ShutdownReply, session_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShutdownReply, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ShutdownReply, reserved);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ShutdownReply initialized bytes");
        let again = ShutdownReply::decode(&output).unwrap();
        assert_eq!(value.session_id, again.session_id);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
}
