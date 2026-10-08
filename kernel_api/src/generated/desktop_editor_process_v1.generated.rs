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

pub const MSG_DESKTOP_EDITOR_PROCESS_V1_STOP: u32 = 4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Stop {
    pub request_id: u64,
    pub channel_generation: u64,
    pub reason: u32,
    pub reserved: [u8; 44],
}
