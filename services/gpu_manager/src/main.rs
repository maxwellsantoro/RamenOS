//! GPU Manager service for quarantine domain operations.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;

use clap::Parser;
use kernel_api::ipc::Envelope;

const STATUS_NOT_IMPLEMENTED: u32 = 2;

#[derive(Parser, Debug)]
#[command(name = "gpu_manager")]
struct Args {
    /// Socket path for IPC communication.
    #[arg(short, long, default_value = "/tmp/gpu_manager.sock")]
    socket: String,

    /// Run a single self-check without listening on the socket.
    #[arg(long)]
    self_check: bool,
}

fn main() {
    let args = Args::parse();

    if args.self_check {
        let env = Envelope::empty(0x310, 1);
        let reply = handle_envelope(&env);
        println!(
            "GPU_MANAGER: self_check protocol={} msg_type={} -> reply msg_type={}",
            env.protocol, env.msg_type, reply.msg_type
        );
        println!("GPU_MANAGER: ok");
        return;
    }

    let socket_path = args.socket.clone();
    if Path::new(&socket_path).exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path).expect("bind gpu_manager socket");
    eprintln!("gpu_manager: listening on {socket_path}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                thread::spawn(move || {
                    let _ = serve_client(&mut stream);
                });
            }
            Err(err) => eprintln!("gpu_manager: accept error: {err}"),
        }
    }
}

fn serve_client(stream: &mut UnixStream) -> std::io::Result<()> {
    let mut buf = [0u8; 88];
    stream.read_exact(&mut buf)?;
    let request = decode_envelope(&buf);
    let reply = handle_envelope(&request);
    stream.write_all(&encode_envelope(&reply))?;
    Ok(())
}

fn handle_envelope(env: &Envelope) -> Envelope {
    if env.protocol != 0x310 {
        return Envelope::empty(env.protocol, env.msg_type.saturating_add(1));
    }

    let mut reply = Envelope::empty(env.protocol, env.msg_type.saturating_add(1));
    reply.payload[0..4].copy_from_slice(&STATUS_NOT_IMPLEMENTED.to_le_bytes());
    reply.payload_len = 4;
    reply
}

fn encode_envelope(env: &Envelope) -> [u8; 88] {
    let mut buf = [0u8; 88];
    buf[0..4].copy_from_slice(&env.protocol.to_le_bytes());
    buf[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    buf[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    buf[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    buf[20..84].copy_from_slice(&env.payload);
    buf
}

fn decode_envelope(bytes: &[u8; 88]) -> Envelope {
    let protocol = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let msg_type = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let handle_raw = u64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    let payload_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let mut payload = [0u8; 64];
    payload.copy_from_slice(&bytes[20..84]);
    Envelope {
        protocol,
        msg_type,
        handle: kernel_api::cap::Handle::unpack(handle_raw),
        payload_len,
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_protocol_returns_not_implemented_status() {
        let env = Envelope::empty(0x310, 1);
        let reply = handle_envelope(&env);
        assert_eq!(reply.msg_type, 2);
        assert_eq!(
            u32::from_le_bytes([
                reply.payload[0],
                reply.payload[1],
                reply.payload[2],
                reply.payload[3]
            ]),
            STATUS_NOT_IMPLEMENTED
        );
    }
}
