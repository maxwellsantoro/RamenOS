//! Explicitly synthetic D03 and D05 DATA precursors. No actual Linux ancillary/kernel claim.
//! Proposed maintained path services/desktop_editor_core/tests/process_wire.rs.
//! Only Root registers core/test graph; missing crate is NOT an API RED.
use desktop_editor_core::wire::{
    Direction, ProcessMessage as M, WireError, decode_frame, encode_frame, validate_delivery,
};
use kernel_api::generated::desktop_editor_process_v1 as p;
const HANDLE: u64 = 0x0200_0001_0000_0001; // serialized reference only; not an authenticated capability

fn hex88(s: &str) -> [u8; 88] {
    assert_eq!(s.len(), 176);
    let mut out = [0; 88];
    for (i, pair) in s.as_bytes().chunks_exact(2).enumerate() {
        let digit = |b: u8| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => panic!("fixture hex"),
        };
        out[i] = (digit(pair[0]) << 4) | digit(pair[1]);
    }
    out
}
fn fixtures() -> Vec<(M, [u8; 88], Direction, usize)> {
    vec![
        (
            M::Bootstrap(p::Bootstrap {
                request_id: 1,
                channel_generation: 7,
                document_binding_id: 1234605616436508552,
                offer_id: 1,
                selected_text_len: 68,
                width: 640,
                height: 480,
                stride: 2560,
                reserved: [0; 16],
            }),
            hex88(
                "800100000100000001000000010000024000000001000000000000000700000000000000887766554433221101000000000000004400000080020000e0010000000a00000000000000000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            3,
        ),
        (
            M::DeliverInput(p::DeliverInput {
                request_id: 72623859790382856,
                channel_generation: 7,
                input_sequence: 9,
                focus_epoch: 11,
                focus_service_epoch: 13,
                offer_id: 0,
                usage: 4,
                phase: 1,
                modifiers: 0,
                flags: 0,
            }),
            hex88(
                "80010000020000000100000001000002400000000807060504030201070000000000000009000000000000000b000000000000000d0000000000000000000000000000000400000001000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            0,
        ),
        (
            M::InputAck(p::InputAck {
                request_id: 72623859790382856,
                channel_generation: 7,
                input_sequence: 9,
                text_generation: 2,
                view_generation: 3,
                flags: 1,
                status: 0,
                reserved: [0; 16],
            }),
            hex88(
                "80010000030000000100000001000002400000000807060504030201070000000000000009000000000000000200000000000000030000000000000001000000000000000000000000000000000000000000000000000000",
            ),
            Direction::ChildToParent,
            0,
        ),
        (
            M::Stop(p::Stop {
                request_id: 72623859790382856,
                channel_generation: 7,
                reason: 1,
                reserved: [0; 44],
            }),
            hex88(
                "80010000040000000100000001000002400000000807060504030201070000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            0,
        ),
        (
            M::Publish(p::Publish {
                request_id: 72623859790382856,
                channel_generation: 7,
                input_sequence: 9,
                publication_sequence: 3,
                text_generation: 2,
                view_generation: 3,
                offer_id: 2,
                text_state_len: 132,
                surface_len: 1228800,
            }),
            hex88(
                "800100000500000001000000010000024000000008070605040302010700000000000000090000000000000003000000000000000200000000000000030000000000000002000000000000008400000000c0120000000000",
            ),
            Direction::ChildToParent,
            2,
        ),
        (
            M::PublishAck(p::PublishAck {
                request_id: 72623859790382856,
                channel_generation: 7,
                publication_sequence: 3,
                input_sequence: 9,
                status: 0,
                reserved: [0; 28],
            }),
            hex88(
                "80010000060000000100000001000002400000000807060504030201070000000000000003000000000000000900000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            0,
        ),
        (
            M::ReceiptNotice(p::ReceiptNotice {
                request_id: 72623859790382856,
                channel_generation: 7,
                capture_input_sequence: 9,
                operation_id: 33,
                committed_revision: 2,
                capture_text_generation: 2,
                outcome: 0,
                phase: 0,
                reserved: [0; 8],
            }),
            hex88(
                "80010000070000000100000001000002400000000807060504030201070000000000000009000000000000002100000000000000020000000000000002000000000000000000000000000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            0,
        ),
        (
            M::RequestPublication(p::RequestPublication {
                request_id: 72623859790382856,
                channel_generation: 7,
                last_input_sequence: 9,
                offer_id: 2,
                expected_text_generation: 2,
                expected_view_generation: 3,
                flags: 0,
                reserved: [0; 12],
            }),
            hex88(
                "80010000080000000100000001000002400000000807060504030201070000000000000009000000000000000200000000000000020000000000000003000000000000000000000000000000000000000000000000000000",
            ),
            Direction::ParentToChild,
            2,
        ),
    ]
}
fn same_fields(left: &M, right: &M) {
    match (left, right) {
        (M::Bootstrap(a), M::Bootstrap(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.document_binding_id, b.document_binding_id);
            assert_eq!(a.offer_id, b.offer_id);
            assert_eq!(a.selected_text_len, b.selected_text_len);
            assert_eq!(a.width, b.width);
            assert_eq!(a.height, b.height);
            assert_eq!(a.stride, b.stride);
            assert_eq!(a.reserved, b.reserved);
        }
        (M::DeliverInput(a), M::DeliverInput(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.input_sequence, b.input_sequence);
            assert_eq!(a.focus_epoch, b.focus_epoch);
            assert_eq!(a.focus_service_epoch, b.focus_service_epoch);
            assert_eq!(a.offer_id, b.offer_id);
            assert_eq!(a.usage, b.usage);
            assert_eq!(a.phase, b.phase);
            assert_eq!(a.modifiers, b.modifiers);
            assert_eq!(a.flags, b.flags);
        }
        (M::InputAck(a), M::InputAck(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.input_sequence, b.input_sequence);
            assert_eq!(a.text_generation, b.text_generation);
            assert_eq!(a.view_generation, b.view_generation);
            assert_eq!(a.flags, b.flags);
            assert_eq!(a.status, b.status);
            assert_eq!(a.reserved, b.reserved);
        }
        (M::Stop(a), M::Stop(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.reason, b.reason);
            assert_eq!(a.reserved, b.reserved);
        }
        (M::Publish(a), M::Publish(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.input_sequence, b.input_sequence);
            assert_eq!(a.publication_sequence, b.publication_sequence);
            assert_eq!(a.text_generation, b.text_generation);
            assert_eq!(a.view_generation, b.view_generation);
            assert_eq!(a.offer_id, b.offer_id);
            assert_eq!(a.text_state_len, b.text_state_len);
            assert_eq!(a.surface_len, b.surface_len);
        }
        (M::PublishAck(a), M::PublishAck(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.publication_sequence, b.publication_sequence);
            assert_eq!(a.input_sequence, b.input_sequence);
            assert_eq!(a.status, b.status);
            assert_eq!(a.reserved, b.reserved);
        }
        (M::ReceiptNotice(a), M::ReceiptNotice(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.capture_input_sequence, b.capture_input_sequence);
            assert_eq!(a.operation_id, b.operation_id);
            assert_eq!(a.committed_revision, b.committed_revision);
            assert_eq!(a.capture_text_generation, b.capture_text_generation);
            assert_eq!(a.outcome, b.outcome);
            assert_eq!(a.phase, b.phase);
            assert_eq!(a.reserved, b.reserved);
        }
        (M::RequestPublication(a), M::RequestPublication(b)) => {
            assert_eq!(a.request_id, b.request_id);
            assert_eq!(a.channel_generation, b.channel_generation);
            assert_eq!(a.last_input_sequence, b.last_input_sequence);
            assert_eq!(a.offer_id, b.offer_id);
            assert_eq!(a.expected_text_generation, b.expected_text_generation);
            assert_eq!(a.expected_view_generation, b.expected_view_generation);
            assert_eq!(a.flags, b.flags);
            assert_eq!(a.reserved, b.reserved);
        }
        _ => panic!("wrong decoded variant"),
    }
}
fn witness(message: &M, expected: &[u8; 88]) {
    assert_eq!(&encode_frame(message, HANDLE).unwrap(), expected);
    let (handle, decoded) = decode_frame(expected).unwrap();
    assert_eq!(handle, HANDLE);
    same_fields(message, &decoded);
}
fn u32_at(frame: &mut [u8; 88], offset: usize, value: u32) {
    frame[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

// Typed negatives bypass decode: a decoder rejection cannot hide an encoder gap.
// Offsets below are payload offsets from the independently frozen field inventory.
fn set_u32_field(message: &mut M, offset: usize, value: u32) {
    let field = match (message, offset) {
        (M::Bootstrap(v), 32) => &mut v.selected_text_len,
        (M::Bootstrap(v), 36) => &mut v.width,
        (M::Bootstrap(v), 40) => &mut v.height,
        (M::Bootstrap(v), 44) => &mut v.stride,
        (M::DeliverInput(v), 52) => &mut v.phase,
        (M::DeliverInput(v), 56) => &mut v.modifiers,
        (M::DeliverInput(v), 60) => &mut v.flags,
        (M::InputAck(v), 40) => &mut v.flags,
        (M::InputAck(v), 44) => &mut v.status,
        (M::Stop(v), 16) => &mut v.reason,
        (M::Publish(v), 56) => &mut v.text_state_len,
        (M::Publish(v), 60) => &mut v.surface_len,
        (M::PublishAck(v), 32) => &mut v.status,
        (M::ReceiptNotice(v), 48) => &mut v.outcome,
        (M::ReceiptNotice(v), 52) => &mut v.phase,
        (M::RequestPublication(v), 48) => &mut v.flags,
        _ => panic!("unlisted typed fixture field"),
    };
    *field = value;
}
fn field_negative(message: &M, bytes: &[u8; 88], offset: usize, value: u32) {
    let mut bad = *bytes;
    u32_at(&mut bad, 20 + offset, value);
    assert!(matches!(decode_frame(&bad), Err(WireError::FieldValue)));
    let mut typed = *message;
    set_u32_field(&mut typed, offset, value);
    assert!(matches!(
        encode_frame(&typed, HANDLE),
        Err(WireError::FieldValue)
    ));
    witness(message, bytes);
}
fn nonzero_error_byte(message: &mut M, offset: usize) {
    // Canonical error fixtures have every field below zero before this mutation.
    // Every selected payload byte is represented by one exact named typed field.
    match message {
        M::InputAck(v) => match offset {
            8..=15 => v.channel_generation = 1u64 << (8 * (offset - 8)),
            16..=23 => v.input_sequence = 1u64 << (8 * (offset - 16)),
            24..=31 => v.text_generation = 1u64 << (8 * (offset - 24)),
            32..=39 => v.view_generation = 1u64 << (8 * (offset - 32)),
            40..=43 => v.flags = 1u32 << (8 * (offset - 40)),
            48..=63 => v.reserved[offset - 48] = 1,
            _ => panic!("request/status bytes are not error-zero controls"),
        },
        M::PublishAck(v) => match offset {
            8..=15 => v.channel_generation = 1u64 << (8 * (offset - 8)),
            16..=23 => v.publication_sequence = 1u64 << (8 * (offset - 16)),
            24..=31 => v.input_sequence = 1u64 << (8 * (offset - 24)),
            36..=63 => v.reserved[offset - 36] = 1,
            _ => panic!("request/status bytes are not error-zero controls"),
        },
        _ => panic!("error-byte fixture requires acknowledgment variant"),
    }
}

#[test]
fn d03_all_eight_golden_vectors_encode_decode_every_field_explicit_le() {
    for (message, bytes, direction, count) in fixtures() {
        witness(&message, &bytes);
        assert_eq!(validate_delivery(&message, direction, count), Ok(()));
    }
}
#[test]
fn d03_every_truncated_frame_and_oversized_record_is_denied() {
    for (message, bytes, _, _) in fixtures() {
        for len in 0..88 {
            assert!(matches!(
                decode_frame(&bytes[..len]),
                Err(WireError::FrameLength)
            ));
            witness(&message, &bytes);
        }
        for len in [89usize, 96] {
            let mut extra = vec![0u8; len];
            extra[..88].copy_from_slice(&bytes);
            assert!(matches!(decode_frame(&extra), Err(WireError::FrameLength)));
            witness(&message, &bytes);
        }
    }
}
#[test]
fn d03_protocol_type_payload_and_every_outer_tail_byte_fail_closed() {
    for (message, bytes, _, _) in fixtures() {
        for value in [0u32, 383, 385, u32::MAX] {
            let mut bad = bytes;
            u32_at(&mut bad, 0, value);
            assert!(matches!(decode_frame(&bad), Err(WireError::Protocol)));
            witness(&message, &bytes);
        }
        for value in [0u32, 9, u32::MAX] {
            let mut bad = bytes;
            u32_at(&mut bad, 4, value);
            assert!(matches!(decode_frame(&bad), Err(WireError::MessageType)));
            witness(&message, &bytes);
        }
        for value in (0u32..64).chain([65, u32::MAX]) {
            let mut bad = bytes;
            u32_at(&mut bad, 16, value);
            assert!(matches!(decode_frame(&bad), Err(WireError::PayloadLength)));
            witness(&message, &bytes);
        }
        for offset in 84..88 {
            let mut bad = bytes;
            bad[offset] = 1;
            assert!(matches!(decode_frame(&bad), Err(WireError::Reserved)));
            witness(&message, &bytes);
        }
    }
}
#[test]
fn d03_every_reserved_payload_byte_denies_decode_and_encode() {
    for (mut message, bytes, _, _) in fixtures() {
        let (start, len) = match &message {
            M::Bootstrap(..) => (48, 16),
            M::InputAck(..) => (48, 16),
            M::Stop(..) => (20, 44),
            M::PublishAck(..) => (36, 28),
            M::ReceiptNotice(..) => (56, 8),
            M::RequestPublication(..) => (52, 12),
            M::DeliverInput(..) | M::Publish(..) => (0, 0),
        };
        for i in 0..len {
            let mut bad = bytes;
            bad[20 + start + i] = 1;
            assert!(matches!(decode_frame(&bad), Err(WireError::Reserved)));
            match &mut message {
                M::Bootstrap(v) => v.reserved[i] = 1,
                M::InputAck(v) => v.reserved[i] = 1,
                M::Stop(v) => v.reserved[i] = 1,
                M::PublishAck(v) => v.reserved[i] = 1,
                M::ReceiptNotice(v) => v.reserved[i] = 1,
                M::RequestPublication(v) => v.reserved[i] = 1,
                _ => unreachable!(),
            }
            assert!(matches!(
                encode_frame(&message, HANDLE),
                Err(WireError::Reserved)
            ));
            match &mut message {
                M::Bootstrap(v) => v.reserved[i] = 0,
                M::InputAck(v) => v.reserved[i] = 0,
                M::Stop(v) => v.reserved[i] = 0,
                M::PublishAck(v) => v.reserved[i] = 0,
                M::ReceiptNotice(v) => v.reserved[i] = 0,
                M::RequestPublication(v) => v.reserved[i] = 0,
                _ => unreachable!(),
            }
            witness(&message, &bytes);
        }
    }
}
#[test]
fn d03_closed_enum_and_status_values_reject_without_losing_valid_witness() {
    // (fixture index, payload field offset, finite invalid values).
    let cases: &[(usize, usize, &[u32])] = &[
        (1, 60, &[3, 4, u32::MAX]), // DeliverInput flags: 0/1/2 only
        (2, 40, &[2, u32::MAX]),    // InputAck flags: 0/1 only
        (2, 44, &[12, u32::MAX]),   // existing statuses0..11
        (3, 16, &[0, 7, u32::MAX]), // Stop reasons1..6
        (5, 32, &[12, u32::MAX]),
        (6, 48, &[2, u32::MAX]),    // own receipt outcome0/1
        (6, 52, &[3, u32::MAX]),    // own receipt phase0/1/2
        (7, 48, &[1, 3, u32::MAX]), // publication NORMAL0/SAVE_CAPTURE2
    ];
    let vectors = fixtures();
    for &(index, offset, invalid) in cases {
        let (message, bytes, _, _) = &vectors[index];
        for &value in invalid {
            field_negative(message, bytes, offset, value);
        }
    }
}
#[test]
fn d03_error_records_only_keep_request_id_and_known_status() {
    // Error InputAck/PublishAck: every other payload byte must be zero; no FDs.
    let vectors = fixtures();
    for (index, status_offset) in [(2usize, 44usize), (5, 32)] {
        let (_, base, direction, _) = &vectors[index];
        for status in 1u32..=11 {
            let mut valid = *base;
            valid[20..84].fill(0);
            valid[20..28].copy_from_slice(&1u64.to_le_bytes());
            u32_at(&mut valid, 20 + status_offset, status);
            let (_, message) = decode_frame(&valid).unwrap();
            assert_eq!(encode_frame(&message, HANDLE).unwrap(), valid);
            assert_eq!(validate_delivery(&message, *direction, 0), Ok(()));
            for offset in 8..64 {
                if (status_offset..status_offset + 4).contains(&offset) {
                    continue;
                }
                let mut bad = valid;
                bad[20 + offset] = 1;
                assert!(decode_frame(&bad).is_err());
                let mut typed = message;
                nonzero_error_byte(&mut typed, offset);
                assert!(encode_frame(&typed, HANDLE).is_err());
                assert_eq!(encode_frame(&message, HANDLE).unwrap(), valid);
                assert!(decode_frame(&valid).is_ok());
            }
        }
    }
}
#[test]
fn d05_data_only_exact_fd_count_and_direction_for_every_message() {
    for (message, bytes, direction, count) in fixtures() {
        for received in (0usize..=4).chain([usize::MAX]) {
            let got = validate_delivery(&message, direction, received);
            if received == count {
                assert_eq!(got, Ok(()));
            } else {
                assert_eq!(got, Err(WireError::FdCount));
            }
            witness(&message, &bytes);
        }
        let wrong = match direction {
            Direction::ParentToChild => Direction::ChildToParent,
            Direction::ChildToParent => Direction::ParentToChild,
        };
        assert_eq!(
            validate_delivery(&message, wrong, count),
            Err(WireError::Direction)
        );
        assert_eq!(validate_delivery(&message, direction, count), Ok(()));
    }
}

#[test]
fn d03_valid_closed_enum_values_and_stateless_shape_boundaries() {
    let vectors = fixtures();
    // Valid field variants are data witnesses, never actual Focus/capture authority.
    for (index, offset, values) in [
        (2usize, 40usize, vec![0u32, 1]),
        (3, 16, vec![1, 2, 3, 4, 5, 6]),
        (7, 48, vec![0, 2]),
        (0, 32, vec![64, 4160]),
        (4, 56, vec![128, 4224]),
    ] {
        for value in values {
            let mut frame = vectors[index].1;
            u32_at(&mut frame, 20 + offset, value);
            let (_, decoded) = decode_frame(&frame).unwrap();
            assert_eq!(encode_frame(&decoded, HANDLE).unwrap(), frame);
        }
    }
    for (outcome, phase, revision) in [(0u32, 0u32, 2u64), (0, 1, 2), (1, 2, 0)] {
        let mut frame = vectors[6].1;
        u32_at(&mut frame, 68, outcome);
        u32_at(&mut frame, 72, phase);
        frame[52..60].copy_from_slice(&revision.to_le_bytes());
        let (_, decoded) = decode_frame(&frame).unwrap();
        assert_eq!(encode_frame(&decoded, HANDLE).unwrap(), frame);
    }
    for (usage, phase, mods, flags) in [
        (4u32, 1u32, 0u32, 0u32),
        (4, 2, 0, 0),
        (0, 3, 0, 1),
        (22, 1, 1, 2),
    ] {
        let mut frame = vectors[1].1;
        for (offset, value) in [(48, usage), (52, phase), (56, mods), (60, flags)] {
            u32_at(&mut frame, 20 + offset, value);
        }
        let (_, decoded) = decode_frame(&frame).unwrap();
        assert_eq!(encode_frame(&decoded, HANDLE).unwrap(), frame);
    }
}
#[test]
fn d03_stateless_fixed_shape_and_unknown_phase_modifiers_fail_closed() {
    let vectors = fixtures();
    let cases: &[(usize, usize, &[u32])] = &[
        (0, 32, &[0, 63, 4161, u32::MAX]),
        (0, 36, &[0, 639, 641, u32::MAX]),
        (0, 40, &[0, 479, 481, u32::MAX]),
        (0, 44, &[0, 2559, 2561, u32::MAX]),
        (1, 52, &[0, 4, u32::MAX]),
        (1, 56, &[4, u32::MAX]),
        (4, 56, &[0, 127, 4225, u32::MAX]),
        (4, 60, &[0, 1228799, 1228801, u32::MAX]),
    ];
    for &(index, offset, values) in cases {
        for &value in values {
            let (message, bytes, _, _) = &vectors[index];
            field_negative(message, bytes, offset, value);
        }
    }
    let mut offered = vectors[1].1;
    offered[60..68].copy_from_slice(&1u64.to_le_bytes());
    assert!(matches!(decode_frame(&offered), Err(WireError::FieldValue))); // DeliverInput.offer_id=0
    let mut typed_offer = vectors[1].0;
    if let M::DeliverInput(v) = &mut typed_offer {
        v.offer_id = 1;
    } else {
        unreachable!();
    }
    assert!(matches!(
        encode_frame(&typed_offer, HANDLE),
        Err(WireError::FieldValue)
    ));
    witness(&vectors[1].0, &vectors[1].1);
    for (outcome, phase) in [(0u32, 2u32), (1, 0), (1, 1)] {
        let mut bad = vectors[6].1;
        u32_at(&mut bad, 68, outcome);
        u32_at(&mut bad, 72, phase);
        assert!(matches!(decode_frame(&bad), Err(WireError::FieldValue)));
        let mut typed = vectors[6].0;
        set_u32_field(&mut typed, 48, outcome);
        set_u32_field(&mut typed, 52, phase);
        assert!(matches!(
            encode_frame(&typed, HANDLE),
            Err(WireError::FieldValue)
        ));
        witness(&vectors[6].0, &vectors[6].1);
    }
}
