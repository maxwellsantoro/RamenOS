//! Bounded process message data, encoded explicitly in little endian.
//!
//! These values carry no process, Focus, Save or capability authority. The packed
//! handle is a copied reference only. Delivery validation checks a numeric FD
//! count; the actual carrier must derive that count from its owned aliases and
//! independently validate credentials, identity, seals and event lineage.
use kernel_api::generated::desktop_editor_process_v1 as p;

#[derive(Copy, Clone, Debug)]
pub enum ProcessMessage {
    Bootstrap(p::Bootstrap),
    DeliverInput(p::DeliverInput),
    InputAck(p::InputAck),
    Stop(p::Stop),
    Publish(p::Publish),
    PublishAck(p::PublishAck),
    ReceiptNotice(p::ReceiptNotice),
    RequestPublication(p::RequestPublication),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    ParentToChild,
    ChildToParent,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    FrameLength,
    Protocol,
    MessageType,
    PayloadLength,
    Reserved,
    FieldValue,
    Direction,
    FdCount,
}

fn zero(bytes: &[u8]) -> Result<(), WireError> {
    if bytes.iter().all(|byte| *byte == 0) {
        Ok(())
    } else {
        Err(WireError::Reserved)
    }
}

fn field(valid: bool) -> Result<(), WireError> {
    if valid {
        Ok(())
    } else {
        Err(WireError::FieldValue)
    }
}

// One validator serves both directions of conversion. It deliberately does not
// validate actual channel/Focus/offer/capture lineage or grant authority.
fn validate_fields(message: &ProcessMessage) -> Result<(), WireError> {
    match message {
        ProcessMessage::Bootstrap(v) => {
            zero(&v.reserved)?;
            field(
                (64..=4160).contains(&v.selected_text_len)
                    && v.width == 640
                    && v.height == 480
                    && v.stride == 2560,
            )
        }
        ProcessMessage::DeliverInput(v) => field(
            v.offer_id == 0 && v.flags <= 2 && (1..=3).contains(&v.phase) && v.modifiers & !3 == 0,
        ),
        ProcessMessage::InputAck(v) => {
            zero(&v.reserved)?;
            field(v.flags <= 1 && v.status <= 11)?;
            field(
                v.status == 0
                    || (v.channel_generation == 0
                        && v.input_sequence == 0
                        && v.text_generation == 0
                        && v.view_generation == 0
                        && v.flags == 0),
            )
        }
        ProcessMessage::Stop(v) => {
            zero(&v.reserved)?;
            field((1..=6).contains(&v.reason))
        }
        ProcessMessage::Publish(v) => {
            field((128..=4224).contains(&v.text_state_len) && v.surface_len == 1_228_800)
        }
        ProcessMessage::PublishAck(v) => {
            zero(&v.reserved)?;
            field(v.status <= 11)?;
            field(
                v.status == 0
                    || (v.channel_generation == 0
                        && v.publication_sequence == 0
                        && v.input_sequence == 0),
            )
        }
        ProcessMessage::ReceiptNotice(v) => {
            zero(&v.reserved)?;
            field(matches!((v.outcome, v.phase), (0, 0) | (0, 1) | (1, 2)))
        }
        ProcessMessage::RequestPublication(v) => {
            zero(&v.reserved)?;
            field(matches!(v.flags, 0 | 2))
        }
    }
}

// These private helpers are called only with the constant offsets and widths
// listed below. Each caller holds an exact 64-byte payload, so no untrusted
// length or offset participates in indexing or allocation.
fn put32(payload: &mut [u8; 64], offset: usize, value: u32) {
    payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put64(payload: &mut [u8; 64], offset: usize, value: u64) {
    payload[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn get32(payload: &[u8; 64], offset: usize) -> u32 {
    u32::from_le_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ])
}

fn get64(payload: &[u8; 64], offset: usize) -> u64 {
    u64::from_le_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
        payload[offset + 4],
        payload[offset + 5],
        payload[offset + 6],
        payload[offset + 7],
    ])
}

fn array<const N: usize>(payload: &[u8; 64], offset: usize) -> [u8; N] {
    let mut bytes = [0; N];
    bytes.copy_from_slice(&payload[offset..offset + N]);
    bytes
}

/// Encode copied message data. Every returned byte is initialized; reserved
/// payloads and the four-byte outer tail are zero. No capability is constructed.
pub fn encode_frame(message: &ProcessMessage, packed_handle: u64) -> Result<[u8; 88], WireError> {
    validate_fields(message)?;
    let mut payload = [0; 64];
    let msg_type = match message {
        ProcessMessage::Bootstrap(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.document_binding_id);
            put64(&mut payload, 24, v.offer_id);
            put32(&mut payload, 32, v.selected_text_len);
            put32(&mut payload, 36, v.width);
            put32(&mut payload, 40, v.height);
            put32(&mut payload, 44, v.stride);
            payload[48..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_BOOTSTRAP
        }
        ProcessMessage::DeliverInput(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.input_sequence);
            put64(&mut payload, 24, v.focus_epoch);
            put64(&mut payload, 32, v.focus_service_epoch);
            put64(&mut payload, 40, v.offer_id);
            put32(&mut payload, 48, v.usage);
            put32(&mut payload, 52, v.phase);
            put32(&mut payload, 56, v.modifiers);
            put32(&mut payload, 60, v.flags);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_DELIVER_INPUT
        }
        ProcessMessage::InputAck(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.input_sequence);
            put64(&mut payload, 24, v.text_generation);
            put64(&mut payload, 32, v.view_generation);
            put32(&mut payload, 40, v.flags);
            put32(&mut payload, 44, v.status);
            payload[48..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_INPUT_ACK
        }
        ProcessMessage::Stop(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put32(&mut payload, 16, v.reason);
            payload[20..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_STOP
        }
        ProcessMessage::Publish(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.input_sequence);
            put64(&mut payload, 24, v.publication_sequence);
            put64(&mut payload, 32, v.text_generation);
            put64(&mut payload, 40, v.view_generation);
            put64(&mut payload, 48, v.offer_id);
            put32(&mut payload, 56, v.text_state_len);
            put32(&mut payload, 60, v.surface_len);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH
        }
        ProcessMessage::PublishAck(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.publication_sequence);
            put64(&mut payload, 24, v.input_sequence);
            put32(&mut payload, 32, v.status);
            payload[36..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH_ACK
        }
        ProcessMessage::ReceiptNotice(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.capture_input_sequence);
            put64(&mut payload, 24, v.operation_id);
            put64(&mut payload, 32, v.committed_revision);
            put64(&mut payload, 40, v.capture_text_generation);
            put32(&mut payload, 48, v.outcome);
            put32(&mut payload, 52, v.phase);
            payload[56..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_RECEIPT_NOTICE
        }
        ProcessMessage::RequestPublication(v) => {
            put64(&mut payload, 0, v.request_id);
            put64(&mut payload, 8, v.channel_generation);
            put64(&mut payload, 16, v.last_input_sequence);
            put64(&mut payload, 24, v.offer_id);
            put64(&mut payload, 32, v.expected_text_generation);
            put64(&mut payload, 40, v.expected_view_generation);
            put32(&mut payload, 48, v.flags);
            payload[52..64].copy_from_slice(&v.reserved);
            p::MSG_DESKTOP_EDITOR_PROCESS_V1_REQUEST_PUBLICATION
        }
    };
    let mut frame = [0; 88];
    frame[0..4].copy_from_slice(&p::DESKTOP_EDITOR_PROCESS_V1_PROTOCOL_ID.to_le_bytes());
    frame[4..8].copy_from_slice(&msg_type.to_le_bytes());
    frame[8..16].copy_from_slice(&packed_handle.to_le_bytes());
    frame[16..20].copy_from_slice(&64u32.to_le_bytes());
    frame[20..84].copy_from_slice(&payload);
    Ok(frame)
}

/// Decode exactly one 88-byte record. The returned packed handle and payload
/// fields are data references only; acceptance establishes no caller authority.
pub fn decode_frame(bytes: &[u8]) -> Result<(u64, ProcessMessage), WireError> {
    if bytes.len() != 88 {
        return Err(WireError::FrameLength);
    }
    let protocol = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if protocol != p::DESKTOP_EDITOR_PROCESS_V1_PROTOCOL_ID {
        return Err(WireError::Protocol);
    }
    let msg_type = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let payload_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    if payload_len != 64 {
        return Err(WireError::PayloadLength);
    }
    zero(&bytes[84..88])?;
    let packed_handle = u64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    let mut payload = [0; 64];
    payload.copy_from_slice(&bytes[20..84]);
    let message = match msg_type {
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_BOOTSTRAP => ProcessMessage::Bootstrap(p::Bootstrap {
            request_id: get64(&payload, 0),
            channel_generation: get64(&payload, 8),
            document_binding_id: get64(&payload, 16),
            offer_id: get64(&payload, 24),
            selected_text_len: get32(&payload, 32),
            width: get32(&payload, 36),
            height: get32(&payload, 40),
            stride: get32(&payload, 44),
            reserved: array(&payload, 48),
        }),
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_DELIVER_INPUT => {
            ProcessMessage::DeliverInput(p::DeliverInput {
                request_id: get64(&payload, 0),
                channel_generation: get64(&payload, 8),
                input_sequence: get64(&payload, 16),
                focus_epoch: get64(&payload, 24),
                focus_service_epoch: get64(&payload, 32),
                offer_id: get64(&payload, 40),
                usage: get32(&payload, 48),
                phase: get32(&payload, 52),
                modifiers: get32(&payload, 56),
                flags: get32(&payload, 60),
            })
        }
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_INPUT_ACK => ProcessMessage::InputAck(p::InputAck {
            request_id: get64(&payload, 0),
            channel_generation: get64(&payload, 8),
            input_sequence: get64(&payload, 16),
            text_generation: get64(&payload, 24),
            view_generation: get64(&payload, 32),
            flags: get32(&payload, 40),
            status: get32(&payload, 44),
            reserved: array(&payload, 48),
        }),
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_STOP => ProcessMessage::Stop(p::Stop {
            request_id: get64(&payload, 0),
            channel_generation: get64(&payload, 8),
            reason: get32(&payload, 16),
            reserved: array(&payload, 20),
        }),
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH => ProcessMessage::Publish(p::Publish {
            request_id: get64(&payload, 0),
            channel_generation: get64(&payload, 8),
            input_sequence: get64(&payload, 16),
            publication_sequence: get64(&payload, 24),
            text_generation: get64(&payload, 32),
            view_generation: get64(&payload, 40),
            offer_id: get64(&payload, 48),
            text_state_len: get32(&payload, 56),
            surface_len: get32(&payload, 60),
        }),
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH_ACK => ProcessMessage::PublishAck(p::PublishAck {
            request_id: get64(&payload, 0),
            channel_generation: get64(&payload, 8),
            publication_sequence: get64(&payload, 16),
            input_sequence: get64(&payload, 24),
            status: get32(&payload, 32),
            reserved: array(&payload, 36),
        }),
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_RECEIPT_NOTICE => {
            ProcessMessage::ReceiptNotice(p::ReceiptNotice {
                request_id: get64(&payload, 0),
                channel_generation: get64(&payload, 8),
                capture_input_sequence: get64(&payload, 16),
                operation_id: get64(&payload, 24),
                committed_revision: get64(&payload, 32),
                capture_text_generation: get64(&payload, 40),
                outcome: get32(&payload, 48),
                phase: get32(&payload, 52),
                reserved: array(&payload, 56),
            })
        }
        p::MSG_DESKTOP_EDITOR_PROCESS_V1_REQUEST_PUBLICATION => {
            ProcessMessage::RequestPublication(p::RequestPublication {
                request_id: get64(&payload, 0),
                channel_generation: get64(&payload, 8),
                last_input_sequence: get64(&payload, 16),
                offer_id: get64(&payload, 24),
                expected_text_generation: get64(&payload, 32),
                expected_view_generation: get64(&payload, 40),
                flags: get32(&payload, 48),
                reserved: array(&payload, 52),
            })
        }
        _ => return Err(WireError::MessageType),
    };
    validate_fields(&message)?;
    Ok((packed_handle, message))
}

/// Validate only direction and exact numeric FD inventory. The caller must
/// derive this count from actual owned received aliases, never sender data.
pub fn validate_delivery(
    message: &ProcessMessage,
    direction: Direction,
    received_fd_count: usize,
) -> Result<(), WireError> {
    let (expected_direction, expected_fds) = match message {
        ProcessMessage::Bootstrap(_) => (Direction::ParentToChild, 3),
        ProcessMessage::DeliverInput(_) => (Direction::ParentToChild, 0),
        ProcessMessage::InputAck(_) => (Direction::ChildToParent, 0),
        ProcessMessage::Stop(_) => (Direction::ParentToChild, 0),
        ProcessMessage::Publish(_) => (Direction::ChildToParent, 2),
        ProcessMessage::PublishAck(_) => (Direction::ParentToChild, 0),
        ProcessMessage::ReceiptNotice(_) => (Direction::ParentToChild, 0),
        ProcessMessage::RequestPublication(_) => (Direction::ParentToChild, 2),
    };
    if direction != expected_direction {
        return Err(WireError::Direction);
    }
    if received_fd_count != expected_fds {
        return Err(WireError::FdCount);
    }
    Ok(())
}
