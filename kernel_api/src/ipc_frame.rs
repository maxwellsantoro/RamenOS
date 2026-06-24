//! Length-prefixed IPC frame helpers for host↔target bridges (S10.5.2).

use crate::cap::Handle;
use crate::ipc::Envelope;

/// On-wire envelope size: protocol(4) + msg_type(4) + handle(8) + payload_len(4) + payload(64) + pad(4).
pub const ENVELOPE_WIRE_SIZE: usize = 88;

/// Maximum payload bytes allowed in a single IPC frame (fail-closed).
pub const MAX_IPC_FRAME_SIZE: u32 = 4096;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    InvalidLength,
}

/// Validate a frame length prefix before reading body bytes.
pub fn validate_frame_length(len: u32) -> Result<usize, FrameError> {
    if len == 0 || len > MAX_IPC_FRAME_SIZE {
        return Err(FrameError::InvalidLength);
    }
    Ok(len as usize)
}

/// Serialize an envelope to the fixed 88-byte wire form.
pub fn envelope_to_wire(envelope: &Envelope) -> [u8; ENVELOPE_WIRE_SIZE] {
    let mut buf = [0u8; ENVELOPE_WIRE_SIZE];
    buf[0..4].copy_from_slice(&envelope.protocol.to_le_bytes());
    buf[4..8].copy_from_slice(&envelope.msg_type.to_le_bytes());
    buf[8..16].copy_from_slice(&envelope.handle.pack().to_le_bytes());
    buf[16..20].copy_from_slice(&envelope.payload_len.to_le_bytes());
    buf[20..84].copy_from_slice(&envelope.payload);
    buf
}

/// Deserialize an envelope from the fixed 88-byte wire form.
pub fn envelope_from_wire(bytes: &[u8; ENVELOPE_WIRE_SIZE]) -> Envelope {
    let protocol = u32::from_le_bytes(bytes[0..4].try_into().expect("protocol"));
    let msg_type = u32::from_le_bytes(bytes[4..8].try_into().expect("msg_type"));
    let handle_raw = u64::from_le_bytes(bytes[8..16].try_into().expect("handle"));
    let payload_len = u32::from_le_bytes(bytes[16..20].try_into().expect("payload_len"));
    let mut payload = [0u8; 64];
    payload.copy_from_slice(&bytes[20..84]);
    Envelope {
        protocol,
        msg_type,
        handle: Handle::unpack(handle_raw),
        payload_len,
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::semantic_state_v1::GetSnapshot;
    use crate::wire::write_payload;

    #[test]
    fn envelope_wire_roundtrip() {
        let mut env = Envelope::empty(10, 1);
        write_payload(
            &mut env,
            &GetSnapshot {
                cap_handle: 0x5310_0000_0000_0002,
                request_id: 7,
                format: 0,
            },
        )
        .unwrap();
        let wire = envelope_to_wire(&env);
        let decoded = envelope_from_wire(&wire);
        assert_eq!(decoded.protocol, 10);
        assert_eq!(decoded.msg_type, 1);
        assert_eq!(decoded.payload_len, env.payload_len);
    }

    #[test]
    fn frame_length_rejects_oversize() {
        assert!(validate_frame_length(MAX_IPC_FRAME_SIZE + 1).is_err());
        assert!(validate_frame_length(0).is_err());
        assert_eq!(
            validate_frame_length(ENVELOPE_WIRE_SIZE as u32).unwrap(),
            ENVELOPE_WIRE_SIZE
        );
    }
}
