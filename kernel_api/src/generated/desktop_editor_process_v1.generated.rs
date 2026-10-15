// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/portals/desktop_editor_process_v1.toml

// namespace = portal.desktop_editor_process, version = 1

pub const DESKTOP_EDITOR_PROCESS_V1_PROTOCOL_ID: u32 = 384;

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_BOOTSTRAP: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Bootstrap {
    pub request_id: u64,
    pub channel_generation: u64,
    pub document_binding_id: u64,
    pub offer_id: u64,
    pub selected_text_len: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub reserved: [u8; 16],
}

crate::impl_wire_payload!(Bootstrap {
    request_id: u64,
    channel_generation: u64,
    document_binding_id: u64,
    offer_id: u64,
    selected_text_len: u32,
    width: u32,
    height: u32,
    stride: u32,
    reserved: [u8; 16],
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_DELIVER_INPUT: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DeliverInput {
    pub request_id: u64,
    pub channel_generation: u64,
    pub input_sequence: u64,
    pub focus_epoch: u64,
    pub focus_service_epoch: u64,
    pub offer_id: u64,
    pub usage: u32,
    pub phase: u32,
    pub modifiers: u32,
    pub flags: u32,
}

crate::impl_wire_payload!(DeliverInput {
    request_id: u64,
    channel_generation: u64,
    input_sequence: u64,
    focus_epoch: u64,
    focus_service_epoch: u64,
    offer_id: u64,
    usage: u32,
    phase: u32,
    modifiers: u32,
    flags: u32,
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_INPUT_ACK: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct InputAck {
    pub request_id: u64,
    pub channel_generation: u64,
    pub input_sequence: u64,
    pub text_generation: u64,
    pub view_generation: u64,
    pub flags: u32,
    pub status: u32,
    pub reserved: [u8; 16],
}

crate::impl_wire_payload!(InputAck {
    request_id: u64,
    channel_generation: u64,
    input_sequence: u64,
    text_generation: u64,
    view_generation: u64,
    flags: u32,
    status: u32,
    reserved: [u8; 16],
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Publish {
    pub request_id: u64,
    pub channel_generation: u64,
    pub input_sequence: u64,
    pub publication_sequence: u64,
    pub text_generation: u64,
    pub view_generation: u64,
    pub offer_id: u64,
    pub text_state_len: u32,
    pub surface_len: u32,
}

crate::impl_wire_payload!(Publish {
    request_id: u64,
    channel_generation: u64,
    input_sequence: u64,
    publication_sequence: u64,
    text_generation: u64,
    view_generation: u64,
    offer_id: u64,
    text_state_len: u32,
    surface_len: u32,
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH_ACK: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PublishAck {
    pub request_id: u64,
    pub channel_generation: u64,
    pub publication_sequence: u64,
    pub input_sequence: u64,
    pub status: u32,
    pub reserved: [u8; 28],
}

crate::impl_wire_payload!(PublishAck {
    request_id: u64,
    channel_generation: u64,
    publication_sequence: u64,
    input_sequence: u64,
    status: u32,
    reserved: [u8; 28],
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_RECEIPT_NOTICE: u32 = 7;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReceiptNotice {
    pub request_id: u64,
    pub channel_generation: u64,
    pub capture_input_sequence: u64,
    pub operation_id: u64,
    pub committed_revision: u64,
    pub capture_text_generation: u64,
    pub outcome: u32,
    pub phase: u32,
    pub reserved: [u8; 8],
}

crate::impl_wire_payload!(ReceiptNotice {
    request_id: u64,
    channel_generation: u64,
    capture_input_sequence: u64,
    operation_id: u64,
    committed_revision: u64,
    capture_text_generation: u64,
    outcome: u32,
    phase: u32,
    reserved: [u8; 8],
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_REQUEST_PUBLICATION: u32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RequestPublication {
    pub request_id: u64,
    pub channel_generation: u64,
    pub last_input_sequence: u64,
    pub offer_id: u64,
    pub expected_text_generation: u64,
    pub expected_view_generation: u64,
    pub flags: u32,
    pub reserved: [u8; 12],
}

crate::impl_wire_payload!(RequestPublication {
    request_id: u64,
    channel_generation: u64,
    last_input_sequence: u64,
    offer_id: u64,
    expected_text_generation: u64,
    expected_view_generation: u64,
    flags: u32,
    reserved: [u8; 12],
});

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_STOP: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Stop {
    pub request_id: u64,
    pub channel_generation: u64,
    pub reason: u32,
    pub reserved: [u8; 44],
}

crate::impl_wire_payload!(Stop {
    request_id: u64,
    channel_generation: u64,
    reason: u32,
    reserved: [u8; 44],
});

#[cfg(test)]
#[test]
fn wire_codec_desktop_editor_process_v1_all_fields_and_padding() {
    use crate::wire::WirePayload;
    {
        let input = [0xa5u8; core::mem::size_of::<Bootstrap>()];
        let value = Bootstrap::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Bootstrap>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Bootstrap>()];
        let offset = core::mem::offset_of!(Bootstrap, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, document_binding_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, selected_text_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, width);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, height);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, stride);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Bootstrap, reserved);
        expected[offset..offset + <[u8; 16] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Bootstrap initialized bytes");
        let again = Bootstrap::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.document_binding_id, again.document_binding_id);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.selected_text_len, again.selected_text_len);
        assert_eq!(value.width, again.width);
        assert_eq!(value.height, again.height);
        assert_eq!(value.stride, again.stride);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<DeliverInput>()];
        let value = DeliverInput::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<DeliverInput>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<DeliverInput>()];
        let offset = core::mem::offset_of!(DeliverInput, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, focus_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, focus_service_epoch);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, usage);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, phase);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, modifiers);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(DeliverInput, flags);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "DeliverInput initialized bytes");
        let again = DeliverInput::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.input_sequence, again.input_sequence);
        assert_eq!(value.focus_epoch, again.focus_epoch);
        assert_eq!(value.focus_service_epoch, again.focus_service_epoch);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.usage, again.usage);
        assert_eq!(value.phase, again.phase);
        assert_eq!(value.modifiers, again.modifiers);
        assert_eq!(value.flags, again.flags);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<InputAck>()];
        let value = InputAck::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<InputAck>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<InputAck>()];
        let offset = core::mem::offset_of!(InputAck, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, text_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, view_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, flags);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(InputAck, reserved);
        expected[offset..offset + <[u8; 16] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "InputAck initialized bytes");
        let again = InputAck::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.input_sequence, again.input_sequence);
        assert_eq!(value.text_generation, again.text_generation);
        assert_eq!(value.view_generation, again.view_generation);
        assert_eq!(value.flags, again.flags);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Publish>()];
        let value = Publish::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Publish>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Publish>()];
        let offset = core::mem::offset_of!(Publish, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, publication_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, text_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, view_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, text_state_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Publish, surface_len);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Publish initialized bytes");
        let again = Publish::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.input_sequence, again.input_sequence);
        assert_eq!(value.publication_sequence, again.publication_sequence);
        assert_eq!(value.text_generation, again.text_generation);
        assert_eq!(value.view_generation, again.view_generation);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(value.text_state_len, again.text_state_len);
        assert_eq!(value.surface_len, again.surface_len);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<PublishAck>()];
        let value = PublishAck::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<PublishAck>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<PublishAck>()];
        let offset = core::mem::offset_of!(PublishAck, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PublishAck, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PublishAck, publication_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PublishAck, input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PublishAck, status);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(PublishAck, reserved);
        expected[offset..offset + <[u8; 28] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "PublishAck initialized bytes");
        let again = PublishAck::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.publication_sequence, again.publication_sequence);
        assert_eq!(value.input_sequence, again.input_sequence);
        assert_eq!(value.status, again.status);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<ReceiptNotice>()];
        let value = ReceiptNotice::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<ReceiptNotice>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<ReceiptNotice>()];
        let offset = core::mem::offset_of!(ReceiptNotice, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, capture_input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, operation_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, committed_revision);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, capture_text_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, outcome);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, phase);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(ReceiptNotice, reserved);
        expected[offset..offset + <[u8; 8] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "ReceiptNotice initialized bytes");
        let again = ReceiptNotice::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.capture_input_sequence, again.capture_input_sequence);
        assert_eq!(value.operation_id, again.operation_id);
        assert_eq!(value.committed_revision, again.committed_revision);
        assert_eq!(value.capture_text_generation, again.capture_text_generation);
        assert_eq!(value.outcome, again.outcome);
        assert_eq!(value.phase, again.phase);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<RequestPublication>()];
        let value = RequestPublication::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<RequestPublication>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<RequestPublication>()];
        let offset = core::mem::offset_of!(RequestPublication, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, last_input_sequence);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, offer_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, expected_text_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, expected_view_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, flags);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(RequestPublication, reserved);
        expected[offset..offset + <[u8; 12] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "RequestPublication initialized bytes");
        let again = RequestPublication::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.last_input_sequence, again.last_input_sequence);
        assert_eq!(value.offer_id, again.offer_id);
        assert_eq!(
            value.expected_text_generation,
            again.expected_text_generation
        );
        assert_eq!(
            value.expected_view_generation,
            again.expected_view_generation
        );
        assert_eq!(value.flags, again.flags);
        assert_eq!(value.reserved, again.reserved);
    }
    {
        let input = [0xa5u8; core::mem::size_of::<Stop>()];
        let value = Stop::decode(&input).unwrap();
        let mut output = [0x5au8; core::mem::size_of::<Stop>()];
        value.encode(&mut output).unwrap();
        let mut expected = [0u8; core::mem::size_of::<Stop>()];
        let offset = core::mem::offset_of!(Stop, request_id);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Stop, channel_generation);
        expected[offset..offset + <u64 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Stop, reason);
        expected[offset..offset + <u32 as WirePayload>::WIRE_SIZE].fill(0xa5);
        let offset = core::mem::offset_of!(Stop, reserved);
        expected[offset..offset + <[u8; 44] as WirePayload>::WIRE_SIZE].fill(0xa5);
        assert_eq!(output, expected, "Stop initialized bytes");
        let again = Stop::decode(&output).unwrap();
        assert_eq!(value.request_id, again.request_id);
        assert_eq!(value.channel_generation, again.channel_generation);
        assert_eq!(value.reason, again.reason);
        assert_eq!(value.reserved, again.reserved);
    }
}
