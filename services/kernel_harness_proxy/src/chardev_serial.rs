//! QEMU chardev socket transport for framed kernel IPC (S10.5.2).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use kernel_api::generated::semantic_state_v1::GetSnapshot;
use kernel_api::ipc::Envelope;
use kernel_api::ipc_frame::{
    ENVELOPE_WIRE_SIZE, envelope_from_wire, envelope_to_wire, validate_frame_length,
};
use kernel_api::wire::write_payload;

const PROTOCOL_SEMANTIC_STATE: u32 = 10;
const MSG_GET_SNAPSHOT: u32 = 1;

/// Connect to a QEMU `chardev socket` path and perform one framed transact roundtrip.
pub fn transact_envelope(socket_path: &Path, request: &Envelope) -> std::io::Result<Envelope> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    transact_on_stream(&mut stream, request)
}

pub fn transact_on_stream<S: Read + Write>(
    stream: &mut S,
    request: &Envelope,
) -> std::io::Result<Envelope> {
    let wire = envelope_to_wire(request);
    let frame_len = (ENVELOPE_WIRE_SIZE as u32).to_le_bytes();
    stream.write_all(&frame_len)?;
    stream.write_all(&wire)?;
    stream.flush()?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let reply_len = u32::from_le_bytes(len_buf);
    validate_frame_length(reply_len).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid reply frame length",
        )
    })?;
    if reply_len as usize != ENVELOPE_WIRE_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected reply envelope size",
        ));
    }
    let mut reply_wire = [0u8; ENVELOPE_WIRE_SIZE];
    stream.read_exact(&mut reply_wire)?;
    Ok(envelope_from_wire(&reply_wire))
}

/// Send an oversize length prefix (negative test helper).
pub fn send_invalid_frame_length(socket_path: &Path, invalid_len: u32) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(&invalid_len.to_le_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Build a semantic-state `get_snapshot` request envelope for bridge tests.
pub fn build_get_snapshot_request(cap_handle: u64, request_id: u64, format: u32) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_SEMANTIC_STATE, MSG_GET_SNAPSHOT);
    write_payload(
        &mut env,
        &GetSnapshot {
            cap_handle,
            request_id,
            format,
        },
    )
    .expect("get_snapshot payload fits");
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::generated::semantic_state_v1::GetSnapshotReply;
    use kernel_api::wire::read_payload;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn chardev_frame_roundtrip_local_socket() {
        let dir = std::env::temp_dir().join(format!(
            "ramen-ipc-bridge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let socket_path = dir.join("ipc.sock");
        std::fs::create_dir_all(&dir).unwrap();
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("skip chardev_frame_roundtrip_local_socket: {err}");
                return;
            }
        };
        let (tx, rx) = mpsc::channel();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).unwrap();
            let frame_len = u32::from_le_bytes(len_buf) as usize;
            let mut wire = vec![0u8; frame_len];
            stream.read_exact(&mut wire).unwrap();
            let mut wire_arr = [0u8; ENVELOPE_WIRE_SIZE];
            wire_arr.copy_from_slice(&wire);
            let request = envelope_from_wire(&wire_arr);
            let mut reply = Envelope::empty(request.protocol, 2);
            write_payload(
                &mut reply,
                &GetSnapshotReply {
                    request_id: 7,
                    status: 0,
                    shm_cap: 0x100,
                    shm_size: 64,
                },
            )
            .unwrap();
            let reply_wire = envelope_to_wire(&reply);
            stream
                .write_all(&(ENVELOPE_WIRE_SIZE as u32).to_le_bytes())
                .unwrap();
            stream.write_all(&reply_wire).unwrap();
            tx.send(()).unwrap();
        });

        let request = build_get_snapshot_request(0x5310_0000_0000_0002, 7, 0);
        let reply = transact_envelope(&socket_path, &request).unwrap();
        let payload: GetSnapshotReply = read_payload(&reply).unwrap();
        assert_eq!(payload.request_id, 7);
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validate_rejects_oversize_frame() {
        assert!(validate_frame_length(5000).is_err());
    }
}
