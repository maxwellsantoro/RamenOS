//! Real generated payloads: implicit padding must never become message bytes.
use core::mem::{MaybeUninit, offset_of, size_of};
use kernel_api::generated::CreateTraceBufferReply;
use kernel_api::ipc::Envelope;
use kernel_api::wire::{WireError, read_payload, write_payload};

#[test]
fn review_wire_padded_reply_initializes_only_declared_fields_and_zeroes_gaps() {
    let mut storage = MaybeUninit::<CreateTraceBufferReply>::uninit();
    // Initialize every byte, including padding, to a known sentinel. This fixture
    // never observes uninitialized memory and never moves the padded value.
    let value = unsafe {
        storage
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<CreateTraceBufferReply>());
        let ptr = storage.as_mut_ptr();
        core::ptr::addr_of_mut!((*ptr).request_id).write(0x0102_0304_0506_0708);
        core::ptr::addr_of_mut!((*ptr).status).write(0x1122_3344);
        core::ptr::addr_of_mut!((*ptr).trace_cap).write(0x2132_4354_6576_8798);
        storage.assume_init_ref()
    };
    let mut env = Envelope::empty(9, 2);
    env.payload.fill(0x5a);
    write_payload(&mut env, value).unwrap();
    let mut expected = [0u8; 64];
    let request = offset_of!(CreateTraceBufferReply, request_id);
    expected[request..request + 8].copy_from_slice(&value.request_id.to_ne_bytes());
    let status = offset_of!(CreateTraceBufferReply, status);
    expected[status..status + 4].copy_from_slice(&value.status.to_ne_bytes());
    let cap = offset_of!(CreateTraceBufferReply, trace_cap);
    expected[cap..cap + 8].copy_from_slice(&value.trace_cap.to_ne_bytes());
    assert_eq!(
        env.payload_len as usize,
        size_of::<CreateTraceBufferReply>()
    );
    assert_eq!(
        env.payload, expected,
        "implicit padding and envelope tail must be initialized to zero"
    );
    let decoded: CreateTraceBufferReply = read_payload(&env).unwrap();
    assert_eq!(
        (decoded.request_id, decoded.status, decoded.trace_cap),
        (value.request_id, value.status, value.trace_cap)
    );
}

#[test]
fn review_wire_legacy_padding_is_ignored_but_never_reemitted() {
    let mut env = Envelope::empty(9, 2);
    env.payload.fill(0xa5);
    env.payload_len = size_of::<CreateTraceBufferReply>() as u32;
    let request = offset_of!(CreateTraceBufferReply, request_id);
    env.payload[request..request + 8].copy_from_slice(&7u64.to_ne_bytes());
    let status = offset_of!(CreateTraceBufferReply, status);
    env.payload[status..status + 4].copy_from_slice(&3u32.to_ne_bytes());
    let cap = offset_of!(CreateTraceBufferReply, trace_cap);
    env.payload[cap..cap + 8].copy_from_slice(&11u64.to_ne_bytes());
    let decoded: CreateTraceBufferReply = read_payload(&env).unwrap();
    assert_eq!(
        (decoded.request_id, decoded.status, decoded.trace_cap),
        (7, 3, 11)
    );
    // A new operation still succeeds after malformed wire-length rejection.
    env.payload_len = 65;
    assert_eq!(
        read_payload::<CreateTraceBufferReply>(&env).unwrap_err(),
        WireError::PayloadLenInvalid
    );
    env.payload_len = size_of::<CreateTraceBufferReply>() as u32;
    write_payload(&mut env, &decoded).unwrap();
    assert!(env.payload[12..16].iter().all(|b| *b == 0));
    assert_eq!(
        read_payload::<CreateTraceBufferReply>(&env)
            .unwrap()
            .trace_cap,
        11
    );
}

#[test]
fn review_wire_scalar_array_and_failed_encode_preserve_next_operation() {
    use kernel_api::wire::WirePayload;
    struct Failed;
    impl WirePayload for Failed {
        const WIRE_SIZE: usize = 4;
        fn encode(&self, out: &mut [u8]) -> Result<(), WireError> {
            out.fill(0xff);
            Err(WireError::PayloadLenMismatch)
        }
        fn decode(_: &[u8]) -> Result<Self, WireError> {
            Err(WireError::PayloadLenMismatch)
        }
    }
    let mut env = Envelope::empty(1, 1);
    write_payload(&mut env, &0x0102_0304u32).unwrap();
    let original = env.payload;
    let original_len = env.payload_len;
    assert_eq!(
        write_payload(&mut env, &Failed),
        Err(WireError::PayloadLenMismatch)
    );
    assert_eq!((env.payload, env.payload_len), (original, original_len));
    assert_eq!(
        write_payload(&mut env, &[0u8; 65]),
        Err(WireError::PayloadTooLarge)
    );
    assert_eq!((env.payload, env.payload_len), (original, original_len));
    for byte in 0..=u8::MAX {
        write_payload(&mut env, &[byte; 64]).unwrap();
        assert_eq!(read_payload::<[u8; 64]>(&env).unwrap(), [byte; 64]);
    }
    write_payload(&mut env, &0x0102_0304u32).unwrap();
    assert_eq!(read_payload::<u32>(&env).unwrap(), 0x0102_0304);
    assert_eq!(env.payload[..4], 0x0102_0304u32.to_ne_bytes());
    assert!(env.payload[4..].iter().all(|b| *b == 0));
}
