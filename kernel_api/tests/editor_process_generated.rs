//! Gate-first generated DATA type/ID/layout checks only. No serialization, capability or child proof.
//! Proposed maintained path: kernel_api/tests/editor_process_generated.rs.
//! Existing kernel_api target exists; generated module v1 is intentionally missing before Root codegen.
use core::mem::{offset_of, size_of};
use kernel_api::generated::desktop_editor_process_v1 as p;

#[test]
fn process_generated_ids_fields_and_layout_match_frozen_contract() {
    assert_eq!(p::DESKTOP_EDITOR_PROCESS_V1_PROTOCOL_ID, 384);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_BOOTSTRAP, 1);
    let _value = p::Bootstrap {
        request_id: 1,
        channel_generation: 7,
        document_binding_id: 1234605616436508552,
        offer_id: 1,
        selected_text_len: 68,
        width: 640,
        height: 480,
        stride: 2560,
        reserved: [0; 16],
    };
    assert_eq!(size_of::<p::Bootstrap>(), 64);
    assert_eq!(offset_of!(p::Bootstrap, request_id), 0);
    assert_eq!(offset_of!(p::Bootstrap, channel_generation), 8);
    assert_eq!(offset_of!(p::Bootstrap, document_binding_id), 16);
    assert_eq!(offset_of!(p::Bootstrap, offer_id), 24);
    assert_eq!(offset_of!(p::Bootstrap, selected_text_len), 32);
    assert_eq!(offset_of!(p::Bootstrap, width), 36);
    assert_eq!(offset_of!(p::Bootstrap, height), 40);
    assert_eq!(offset_of!(p::Bootstrap, stride), 44);
    assert_eq!(offset_of!(p::Bootstrap, reserved), 48);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_DELIVER_INPUT, 2);
    let _value = p::DeliverInput {
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
    };
    assert_eq!(size_of::<p::DeliverInput>(), 64);
    assert_eq!(offset_of!(p::DeliverInput, request_id), 0);
    assert_eq!(offset_of!(p::DeliverInput, channel_generation), 8);
    assert_eq!(offset_of!(p::DeliverInput, input_sequence), 16);
    assert_eq!(offset_of!(p::DeliverInput, focus_epoch), 24);
    assert_eq!(offset_of!(p::DeliverInput, focus_service_epoch), 32);
    assert_eq!(offset_of!(p::DeliverInput, offer_id), 40);
    assert_eq!(offset_of!(p::DeliverInput, usage), 48);
    assert_eq!(offset_of!(p::DeliverInput, phase), 52);
    assert_eq!(offset_of!(p::DeliverInput, modifiers), 56);
    assert_eq!(offset_of!(p::DeliverInput, flags), 60);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_INPUT_ACK, 3);
    let _value = p::InputAck {
        request_id: 72623859790382856,
        channel_generation: 7,
        input_sequence: 9,
        text_generation: 2,
        view_generation: 3,
        flags: 1,
        status: 0,
        reserved: [0; 16],
    };
    assert_eq!(size_of::<p::InputAck>(), 64);
    assert_eq!(offset_of!(p::InputAck, request_id), 0);
    assert_eq!(offset_of!(p::InputAck, channel_generation), 8);
    assert_eq!(offset_of!(p::InputAck, input_sequence), 16);
    assert_eq!(offset_of!(p::InputAck, text_generation), 24);
    assert_eq!(offset_of!(p::InputAck, view_generation), 32);
    assert_eq!(offset_of!(p::InputAck, flags), 40);
    assert_eq!(offset_of!(p::InputAck, status), 44);
    assert_eq!(offset_of!(p::InputAck, reserved), 48);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_STOP, 4);
    let _value = p::Stop {
        request_id: 72623859790382856,
        channel_generation: 7,
        reason: 1,
        reserved: [0; 44],
    };
    assert_eq!(size_of::<p::Stop>(), 64);
    assert_eq!(offset_of!(p::Stop, request_id), 0);
    assert_eq!(offset_of!(p::Stop, channel_generation), 8);
    assert_eq!(offset_of!(p::Stop, reason), 16);
    assert_eq!(offset_of!(p::Stop, reserved), 20);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH, 5);
    let _value = p::Publish {
        request_id: 72623859790382856,
        channel_generation: 7,
        input_sequence: 9,
        publication_sequence: 3,
        text_generation: 2,
        view_generation: 3,
        offer_id: 2,
        text_state_len: 132,
        surface_len: 1228800,
    };
    assert_eq!(size_of::<p::Publish>(), 64);
    assert_eq!(offset_of!(p::Publish, request_id), 0);
    assert_eq!(offset_of!(p::Publish, channel_generation), 8);
    assert_eq!(offset_of!(p::Publish, input_sequence), 16);
    assert_eq!(offset_of!(p::Publish, publication_sequence), 24);
    assert_eq!(offset_of!(p::Publish, text_generation), 32);
    assert_eq!(offset_of!(p::Publish, view_generation), 40);
    assert_eq!(offset_of!(p::Publish, offer_id), 48);
    assert_eq!(offset_of!(p::Publish, text_state_len), 56);
    assert_eq!(offset_of!(p::Publish, surface_len), 60);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_PUBLISH_ACK, 6);
    let _value = p::PublishAck {
        request_id: 72623859790382856,
        channel_generation: 7,
        publication_sequence: 3,
        input_sequence: 9,
        status: 0,
        reserved: [0; 28],
    };
    assert_eq!(size_of::<p::PublishAck>(), 64);
    assert_eq!(offset_of!(p::PublishAck, request_id), 0);
    assert_eq!(offset_of!(p::PublishAck, channel_generation), 8);
    assert_eq!(offset_of!(p::PublishAck, publication_sequence), 16);
    assert_eq!(offset_of!(p::PublishAck, input_sequence), 24);
    assert_eq!(offset_of!(p::PublishAck, status), 32);
    assert_eq!(offset_of!(p::PublishAck, reserved), 36);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_RECEIPT_NOTICE, 7);
    let _value = p::ReceiptNotice {
        request_id: 72623859790382856,
        channel_generation: 7,
        capture_input_sequence: 9,
        operation_id: 33,
        committed_revision: 2,
        capture_text_generation: 2,
        outcome: 0,
        phase: 0,
        reserved: [0; 8],
    };
    assert_eq!(size_of::<p::ReceiptNotice>(), 64);
    assert_eq!(offset_of!(p::ReceiptNotice, request_id), 0);
    assert_eq!(offset_of!(p::ReceiptNotice, channel_generation), 8);
    assert_eq!(offset_of!(p::ReceiptNotice, capture_input_sequence), 16);
    assert_eq!(offset_of!(p::ReceiptNotice, operation_id), 24);
    assert_eq!(offset_of!(p::ReceiptNotice, committed_revision), 32);
    assert_eq!(offset_of!(p::ReceiptNotice, capture_text_generation), 40);
    assert_eq!(offset_of!(p::ReceiptNotice, outcome), 48);
    assert_eq!(offset_of!(p::ReceiptNotice, phase), 52);
    assert_eq!(offset_of!(p::ReceiptNotice, reserved), 56);
    assert_eq!(p::MSG_DESKTOP_EDITOR_PROCESS_V1_REQUEST_PUBLICATION, 8);
    let _value = p::RequestPublication {
        request_id: 72623859790382856,
        channel_generation: 7,
        last_input_sequence: 9,
        offer_id: 2,
        expected_text_generation: 2,
        expected_view_generation: 3,
        flags: 0,
        reserved: [0; 12],
    };
    assert_eq!(size_of::<p::RequestPublication>(), 64);
    assert_eq!(offset_of!(p::RequestPublication, request_id), 0);
    assert_eq!(offset_of!(p::RequestPublication, channel_generation), 8);
    assert_eq!(offset_of!(p::RequestPublication, last_input_sequence), 16);
    assert_eq!(offset_of!(p::RequestPublication, offer_id), 24);
    assert_eq!(
        offset_of!(p::RequestPublication, expected_text_generation),
        32
    );
    assert_eq!(
        offset_of!(p::RequestPublication, expected_view_generation),
        40
    );
    assert_eq!(offset_of!(p::RequestPublication, flags), 48);
    assert_eq!(offset_of!(p::RequestPublication, reserved), 52);
}
