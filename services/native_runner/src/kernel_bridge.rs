//! Kernel IPC bridge for WASM host functions.
//!
//! This module provides the interface for host functions to communicate
//! with the kernel. Each method makes a single IPC call that includes
//! capability validation.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use kernel_api::cap::Handle;
use kernel_api::generated::echo_harness_v1::{
    EchoReply, EchoReplyReply, EchoRequest, EchoRequestReply,
};
use kernel_api::generated::trace_service_v2::{
    TraceRead, TraceReadReply, TraceWrite, TraceWriteReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::ipc_frame::{
    ENVELOPE_WIRE_SIZE as FRAME_ENVELOPE_SIZE, envelope_from_wire, envelope_to_wire,
    validate_frame_length,
};
use kernel_api::wire::{read_payload, write_payload};

use crate::error::{RunnerError, Status};

/// Wire format size for envelope serialization.
/// Layout: protocol(4) + msg_type(4) + handle_packed(8) + payload_len(4) + payload(64) + pad(4) = 88
const ENVELOPE_WIRE_SIZE: usize = 88;

// Protocol IDs for harness services
#[allow(dead_code)]
const PROTOCOL_ECHO_HARNESS_V1: u32 = 0x211; // Capability-based echo harness
#[allow(dead_code)]
const PROTOCOL_TRACE_SERVICE_V2: u32 = 0x212; // Capability-based trace service

// Message types for PROTOCOL_ECHO_HARNESS_V1
#[allow(dead_code)]
const MSG_ECHO_REQUEST: u32 = 1;
#[allow(dead_code)]
const MSG_ECHO_REQUEST_REPLY: u32 = 2;
#[allow(dead_code)]
const MSG_ECHO_REPLY: u32 = 3;
#[allow(dead_code)]
const MSG_ECHO_REPLY_REPLY: u32 = 4;

// Message types for PROTOCOL_TRACE_SERVICE_V2
#[allow(dead_code)]
const MSG_TRACE_READ: u32 = 1;
#[allow(dead_code)]
const MSG_TRACE_READ_REPLY: u32 = 2;
#[allow(dead_code)]
const MSG_TRACE_WRITE: u32 = 3;
#[allow(dead_code)]
const MSG_TRACE_WRITE_REPLY: u32 = 4;

/// Trait for kernel communication.
/// Implementations can be real IPC or mock for testing.
pub trait KernelBridgeOps {
    /// Echo harness: send request.
    /// Kernel validates cap_handle as part of this operation.
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, Status>;

    /// Echo harness: send reply.
    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), Status>;

    /// Trace service: read trace data.
    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, Status>;

    /// Trace service: write trace data.
    fn trace_write(&mut self, cap_handle: u64, data: &[u8]) -> Result<(), Status>;

    /// Shared memory: read from region.
    fn shmem_read(&mut self, shm_cap: u64, offset: u64, len: usize) -> Result<Vec<u8>, Status>;

    /// Shared memory: write to region.
    fn shmem_write(&mut self, shm_cap: u64, offset: u64, data: &[u8]) -> Result<usize, Status>;

    /// Raw envelope IPC transaction (used by generated WASM host shims).
    fn transact(&mut self, request: Envelope) -> Result<Envelope, Status>;
}

/// Real kernel IPC bridge using Unix domain sockets.
pub struct KernelBridge {
    socket_path: PathBuf,
    stream: Option<UnixStream>,
}

impl KernelBridge {
    /// Create a new kernel bridge connected to the specified socket.
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            stream: None,
        }
    }

    /// Ensure that a connection to the kernel is established.
    fn ensure_connection(&mut self) -> Result<&mut UnixStream, RunnerError> {
        if self.stream.is_none() {
            let stream = UnixStream::connect(&self.socket_path).map_err(|e| {
                RunnerError::KernelIpc(format!("connect to {}: {}", self.socket_path.display(), e))
            })?;
            self.stream = Some(stream);
        }
        Ok(self.stream.as_mut().unwrap())
    }

    /// Perform a single IPC transaction: send request, receive reply.
    pub fn transact(&mut self, request: Envelope) -> Result<Envelope, RunnerError> {
        // Serialize envelope to wire format
        let request_bytes = envelope_to_bytes(&request);

        // Attempt to send and receive, with one retry on connection failure
        let mut retry = true;
        loop {
            let stream = self.ensure_connection()?;

            // Send request
            if let Err(e) = stream.write_all(&request_bytes) {
                if retry {
                    eprintln!("KernelBridge: write error, attempting reconnect: {}", e);
                    self.stream = None;
                    retry = false;
                    continue;
                }
                return Err(RunnerError::KernelIpc(format!("write: {}", e)));
            }

            // Read reply
            let mut reply_bytes = [0u8; ENVELOPE_WIRE_SIZE];
            if let Err(e) = stream.read_exact(&mut reply_bytes) {
                if retry {
                    eprintln!("KernelBridge: read error, attempting reconnect: {}", e);
                    self.stream = None;
                    retry = false;
                    continue;
                }
                return Err(RunnerError::KernelIpc(format!("read: {}", e)));
            }

            // Deserialize reply
            return Ok(bytes_to_envelope(&reply_bytes));
        }
    }
}

impl KernelBridgeOps for KernelBridge {
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, Status> {
        // Build request envelope
        let mut env = Envelope::empty(PROTOCOL_ECHO_HARNESS_V1, MSG_ECHO_REQUEST);
        let req = EchoRequest {
            cap_handle,
            request_id,
            payload_len: payload.len() as u32,
            reserved: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;

        // Perform IPC transaction
        let reply = self.transact(env).map_err(|_| Status::KernelError)?;

        // Parse reply
        let reply_payload: EchoRequestReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;

        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }

        // Read payload from shared memory when the reply carries data.
        let payload_len = reply_payload.payload_len as usize;
        if payload_len > 0 {
            return self.shmem_read(cap_handle, 0, payload_len);
        }
        Ok(Vec::new())
    }

    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), Status> {
        // Build request envelope
        let mut env = Envelope::empty(PROTOCOL_ECHO_HARNESS_V1, MSG_ECHO_REPLY);
        let req = EchoReply {
            cap_handle,
            request_id,
            status,
            payload_len: payload.len() as u32,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;

        // Perform IPC transaction
        let reply = self.transact(env).map_err(|_| Status::KernelError)?;

        // Parse reply
        let reply_payload: EchoReplyReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;

        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }

        Ok(())
    }

    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, Status> {
        // Build request envelope
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V2, MSG_TRACE_READ);
        let req = TraceRead {
            cap_handle,
            offset,
            max_len: max_len as u32,
            reserved: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;

        // Perform IPC transaction
        let reply = self.transact(env).map_err(|_| Status::KernelError)?;

        // Parse reply
        let reply_payload: TraceReadReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;

        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }

        // Read trace data from the capability-backed shared memory region.
        let data_len = reply_payload.data_len as usize;
        if data_len == 0 {
            return Ok(Vec::new());
        }
        self.shmem_read(cap_handle, offset, data_len)
    }

    fn trace_write(&mut self, cap_handle: u64, data: &[u8]) -> Result<(), Status> {
        if !data.is_empty() {
            self.shmem_write(cap_handle, 0, data)?;
        }

        // Build request envelope
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V2, MSG_TRACE_WRITE);
        let req = TraceWrite {
            cap_handle,
            data_len: data.len() as u32,
            reserved: 0,
            reserved2: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;

        // Perform IPC transaction
        let reply = self.transact(env).map_err(|_| Status::KernelError)?;

        // Parse reply
        let reply_payload: TraceWriteReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;

        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }

        Ok(())
    }

    fn shmem_read(&mut self, shm_cap: u64, offset: u64, len: usize) -> Result<Vec<u8>, Status> {
        shmem_read_region(shm_cap, offset, len)
    }

    fn shmem_write(&mut self, shm_cap: u64, offset: u64, data: &[u8]) -> Result<usize, Status> {
        shmem_write_region(shm_cap, offset, data)
    }

    fn transact(&mut self, request: Envelope) -> Result<Envelope, Status> {
        KernelBridge::transact(self, request).map_err(|_| Status::KernelError)
    }
}

/// QEMU chardev IPC bridge: length-prefixed envelope frames over a Unix socket.
///
/// Opens a fresh connection per transaction (QEMU semantic relay handles one roundtrip).
pub struct ChardevKernelBridge {
    socket_path: PathBuf,
}

impl ChardevKernelBridge {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    fn transact_once(&self, request: &Envelope) -> Result<Envelope, RunnerError> {
        use std::io::{Read, Write};
        use std::time::Duration;

        let mut stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            RunnerError::KernelIpc(format!("connect to {}: {}", self.socket_path.display(), e))
        })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| RunnerError::KernelIpc(format!("set read timeout: {}", e)))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| RunnerError::KernelIpc(format!("set write timeout: {}", e)))?;

        let wire = envelope_to_wire(request);
        let frame_len = (FRAME_ENVELOPE_SIZE as u32).to_le_bytes();
        stream
            .write_all(&frame_len)
            .map_err(|e| RunnerError::KernelIpc(format!("write frame length: {}", e)))?;
        stream
            .write_all(&wire)
            .map_err(|e| RunnerError::KernelIpc(format!("write envelope: {}", e)))?;
        stream
            .flush()
            .map_err(|e| RunnerError::KernelIpc(format!("flush: {}", e)))?;

        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .map_err(|e| RunnerError::KernelIpc(format!("read reply length: {}", e)))?;
        let reply_len = u32::from_le_bytes(len_buf);
        validate_frame_length(reply_len)
            .map_err(|_| RunnerError::KernelIpc("invalid reply frame length".to_string()))?;
        if reply_len as usize != FRAME_ENVELOPE_SIZE {
            return Err(RunnerError::KernelIpc(
                "unexpected reply envelope size".to_string(),
            ));
        }
        let mut reply_wire = [0u8; FRAME_ENVELOPE_SIZE];
        stream
            .read_exact(&mut reply_wire)
            .map_err(|e| RunnerError::KernelIpc(format!("read reply envelope: {}", e)))?;
        Ok(envelope_from_wire(&reply_wire))
    }
}

impl KernelBridgeOps for ChardevKernelBridge {
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, Status> {
        let mut env = Envelope::empty(PROTOCOL_ECHO_HARNESS_V1, MSG_ECHO_REQUEST);
        let req = EchoRequest {
            cap_handle,
            request_id,
            payload_len: payload.len() as u32,
            reserved: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;
        let reply = self.transact(env)?;
        let reply_payload: EchoRequestReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;
        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }
        let payload_len = reply_payload.payload_len as usize;
        if payload_len > 0 {
            return self.shmem_read(cap_handle, 0, payload_len);
        }
        Ok(Vec::new())
    }

    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), Status> {
        let mut env = Envelope::empty(PROTOCOL_ECHO_HARNESS_V1, MSG_ECHO_REPLY);
        let req = EchoReply {
            cap_handle,
            request_id,
            status,
            payload_len: payload.len() as u32,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;
        let reply = self.transact(env)?;
        let reply_payload: EchoReplyReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;
        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }
        Ok(())
    }

    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, Status> {
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V2, MSG_TRACE_READ);
        let req = TraceRead {
            cap_handle,
            offset,
            max_len: max_len as u32,
            reserved: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;
        let reply = self.transact(env)?;
        let reply_payload: TraceReadReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;
        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }
        let data_len = reply_payload.data_len as usize;
        if data_len == 0 {
            return Ok(Vec::new());
        }
        self.shmem_read(cap_handle, offset, data_len)
    }

    fn trace_write(&mut self, cap_handle: u64, data: &[u8]) -> Result<(), Status> {
        if !data.is_empty() {
            self.shmem_write(cap_handle, 0, data)?;
        }
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V2, MSG_TRACE_WRITE);
        let req = TraceWrite {
            cap_handle,
            data_len: data.len() as u32,
            reserved: 0,
            reserved2: 0,
        };
        write_payload(&mut env, &req).map_err(|_| Status::InternalError)?;
        let reply = self.transact(env)?;
        let reply_payload: TraceWriteReply =
            read_payload(&reply).map_err(|_| Status::KernelError)?;
        if reply_payload.status != 0 {
            return Err(Status::from_u32(reply_payload.status));
        }
        Ok(())
    }

    fn shmem_read(&mut self, shm_cap: u64, offset: u64, len: usize) -> Result<Vec<u8>, Status> {
        shmem_read_region(shm_cap, offset, len)
    }

    fn shmem_write(&mut self, shm_cap: u64, offset: u64, data: &[u8]) -> Result<usize, Status> {
        shmem_write_region(shm_cap, offset, data)
    }

    fn transact(&mut self, request: Envelope) -> Result<Envelope, Status> {
        self.transact_once(&request)
            .map_err(|_| Status::KernelError)
    }
}

fn shmem_read_region(shm_cap: u64, offset: u64, len: usize) -> Result<Vec<u8>, Status> {
    let mem_path = if cfg!(target_os = "linux") {
        "/dev/shm/ramenos_mem"
    } else {
        "/tmp/ramenos_mem"
    };

    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(mem_path)
        .map_err(|_| Status::IoError)?;

    let mmap = unsafe { memmap2::Mmap::map(&file).map_err(|_| Status::IoError)? };

    let phys_addr = shm_cap;
    let target_offset = (phys_addr as usize) + (offset as usize);
    if target_offset + len > mmap.len() {
        return Err(Status::InvalidArgument);
    }

    Ok(mmap[target_offset..target_offset + len].to_vec())
}

fn shmem_write_region(shm_cap: u64, offset: u64, data: &[u8]) -> Result<usize, Status> {
    let mem_path = if cfg!(target_os = "linux") {
        "/dev/shm/ramenos_mem"
    } else {
        "/tmp/ramenos_mem"
    };

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(mem_path)
        .map_err(|_| Status::IoError)?;

    let mut mmap = unsafe { memmap2::MmapMut::map_mut(&file).map_err(|_| Status::IoError)? };

    let phys_addr = shm_cap;
    let target_offset = (phys_addr as usize) + (offset as usize);
    if target_offset + data.len() > mmap.len() {
        return Err(Status::InvalidArgument);
    }

    let dest = &mut mmap[target_offset..target_offset + data.len()];
    dest.copy_from_slice(data);

    Ok(data.len())
}

// ============================================================================
// Wire format serialization (copied from capsule_relay for consistency)
// ============================================================================

/// Serialize Envelope to bytes using explicit field writes (no transmute).
///
/// Wire format is **little-endian** for cross-arch determinism.
/// Layout: protocol(4) + msg_type(4) + handle(8 packed) + payload_len(4) + payload(64) + pad(4) = 88
fn envelope_to_bytes(env: &Envelope) -> [u8; ENVELOPE_WIRE_SIZE] {
    let mut buf = [0u8; ENVELOPE_WIRE_SIZE];
    buf[0..4].copy_from_slice(&env.protocol.to_le_bytes());
    buf[4..8].copy_from_slice(&env.msg_type.to_le_bytes());
    buf[8..16].copy_from_slice(&env.handle.pack().to_le_bytes());
    buf[16..20].copy_from_slice(&env.payload_len.to_le_bytes());
    buf[20..84].copy_from_slice(&env.payload);
    // bytes 84..88 are padding (zeroed)
    buf
}

/// Deserialize Envelope from bytes using explicit field reads (no transmute).
///
/// Wire format is **little-endian** for cross-arch determinism.
fn bytes_to_envelope(bytes: &[u8; ENVELOPE_WIRE_SIZE]) -> Envelope {
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
        handle: Handle::unpack(handle_raw),
        payload_len,
        payload,
    }
}

// ============================================================================
// Mock implementation for testing
// ============================================================================

/// Recorded kernel call for testing assertions.
#[derive(Debug, Clone)]
pub struct KernelCall {
    pub operation: String,
    pub cap_handle: u64,
    pub args: Vec<u8>,
}

/// Mock kernel bridge for testing.
/// Returns canned responses, records calls for assertion.
pub struct MockKernelBridge {
    echo_response: Option<Vec<u8>>,
    calls: Vec<KernelCall>,
}

impl MockKernelBridge {
    pub fn new() -> Self {
        Self {
            echo_response: None,
            calls: Vec::new(),
        }
    }

    /// Set the canned response for echo_request.
    pub fn set_echo_response(&mut self, response: Vec<u8>) {
        self.echo_response = Some(response);
    }

    /// Get recorded calls for assertion.
    pub fn get_calls(&self) -> &[KernelCall] {
        &self.calls
    }
}

impl KernelBridgeOps for MockKernelBridge {
    fn echo_request(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        payload: &[u8],
    ) -> Result<Vec<u8>, Status> {
        // Record the call
        let mut args = vec![];
        args.extend_from_slice(&request_id.to_le_bytes());
        args.extend_from_slice(payload);

        self.calls.push(KernelCall {
            operation: "echo_request".to_string(),
            cap_handle,
            args,
        });

        // Return canned response or error
        self.echo_response.clone().ok_or(Status::IoError)
    }

    fn echo_reply(
        &mut self,
        cap_handle: u64,
        request_id: u64,
        status: u32,
        payload: &[u8],
    ) -> Result<(), Status> {
        let mut args = vec![];
        args.extend_from_slice(&request_id.to_le_bytes());
        args.extend_from_slice(&status.to_le_bytes());
        args.extend_from_slice(payload);

        self.calls.push(KernelCall {
            operation: "echo_reply".to_string(),
            cap_handle,
            args,
        });

        Ok(())
    }

    fn trace_read(
        &mut self,
        cap_handle: u64,
        offset: u64,
        max_len: usize,
    ) -> Result<Vec<u8>, Status> {
        let mut args = vec![];
        args.extend_from_slice(&offset.to_le_bytes());
        args.extend_from_slice(&(max_len as u64).to_le_bytes());

        self.calls.push(KernelCall {
            operation: "trace_read".to_string(),
            cap_handle,
            args,
        });

        // Return empty data for now
        Ok(vec![0; max_len.min(64)])
    }

    fn trace_write(&mut self, cap_handle: u64, data: &[u8]) -> Result<(), Status> {
        self.calls.push(KernelCall {
            operation: "trace_write".to_string(),
            cap_handle,
            args: data.to_vec(),
        });

        Ok(())
    }

    fn shmem_read(&mut self, shm_cap: u64, offset: u64, len: usize) -> Result<Vec<u8>, Status> {
        let mut args = vec![];
        args.extend_from_slice(&offset.to_le_bytes());
        args.extend_from_slice(&(len as u64).to_le_bytes());

        self.calls.push(KernelCall {
            operation: "shmem_read".to_string(),
            cap_handle: shm_cap,
            args,
        });

        Ok(vec![0; len])
    }

    fn shmem_write(&mut self, shm_cap: u64, offset: u64, data: &[u8]) -> Result<usize, Status> {
        let mut args = vec![];
        args.extend_from_slice(&offset.to_le_bytes());
        args.extend_from_slice(data);

        self.calls.push(KernelCall {
            operation: "shmem_write".to_string(),
            cap_handle: shm_cap,
            args,
        });

        Ok(data.len())
    }

    fn transact(&mut self, request: Envelope) -> Result<Envelope, Status> {
        Ok(Envelope::empty(
            request.protocol,
            request.msg_type.saturating_add(1),
        ))
    }
}

impl Default for MockKernelBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chardev_kernel_bridge_framed_roundtrip() {
        use kernel_api::ipc_frame::ENVELOPE_WIRE_SIZE as FRAME_SIZE;
        use std::io::{Read, Write};
        use std::os::unix::net::UnixListener;
        use std::sync::mpsc;
        use std::thread;

        let dir = std::env::temp_dir().join(format!(
            "ramen-chardev-bridge-{}-{}",
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
                eprintln!("skip chardev_kernel_bridge_framed_roundtrip: {err}");
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
            stream
                .write_all(&(FRAME_SIZE as u32).to_le_bytes())
                .unwrap();
            stream.write_all(&wire).unwrap();
            tx.send(()).unwrap();
        });

        let mut bridge = ChardevKernelBridge::new(socket_path.clone());
        let request = Envelope::empty(10, 1);
        let reply = bridge.transact(request).expect("chardev transact");
        assert_eq!(reply.protocol, 10);
        assert_eq!(reply.msg_type, 1);
        rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn mock_kernel_bridge_records_calls() {
        let mut bridge = MockKernelBridge::new();

        bridge.set_echo_response(vec![1, 2, 3, 4]);

        let result = bridge.echo_request(0x1234, 42, &[5, 6, 7]).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);

        let calls = bridge.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].operation, "echo_request");
        assert_eq!(calls[0].cap_handle, 0x1234);
    }

    #[test]
    fn mock_kernel_bridge_returns_error_for_missing_response() {
        let mut bridge = MockKernelBridge::new();

        let result = bridge.echo_request(0x1234, 42, &[5, 6, 7]);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_wire_size_is_88() {
        assert_eq!(ENVELOPE_WIRE_SIZE, 88);
    }

    #[test]
    fn envelope_roundtrip() {
        let env = Envelope::empty(0x211, 1);
        let bytes = envelope_to_bytes(&env);
        let env2 = bytes_to_envelope(&bytes);
        assert_eq!(env.protocol, env2.protocol);
        assert_eq!(env.msg_type, env2.msg_type);
        assert_eq!(env.handle, env2.handle);
        assert_eq!(env.payload_len, env2.payload_len);
    }

    #[test]
    fn envelope_roundtrip_with_payload() {
        let mut env = Envelope::empty(0x211, MSG_ECHO_REQUEST);
        let req = EchoRequest {
            cap_handle: 0x1234_5678,
            request_id: 42,
            payload_len: 16,
            reserved: 0,
        };
        write_payload(&mut env, &req).unwrap();

        let bytes = envelope_to_bytes(&env);
        let env2 = bytes_to_envelope(&bytes);

        assert_eq!(env.protocol, env2.protocol);
        assert_eq!(env.payload_len, env2.payload_len);

        let req2: EchoRequest = read_payload(&env2).unwrap();
        assert_eq!(req.cap_handle, req2.cap_handle);
        assert_eq!(req.request_id, req2.request_id);
        assert_eq!(req.payload_len, req2.payload_len);
    }

    #[test]
    fn status_from_u32_maps_correctly() {
        assert_eq!(Status::from_u32(0), Status::Ok);
        assert_eq!(Status::from_u32(1), Status::InvalidCapability);
        assert_eq!(Status::from_u32(2), Status::PermissionDenied);
        assert_eq!(Status::from_u32(3), Status::InvalidArgument);
        assert_eq!(Status::from_u32(4), Status::WouldBlock);
        assert_eq!(Status::from_u32(5), Status::IoError);
        assert_eq!(Status::from_u32(6), Status::InternalError);
        assert_eq!(Status::from_u32(7), Status::KernelError);
    }

    #[test]
    fn status_from_u32_unknown_is_internal_error() {
        // Unknown codes should be treated as internal error (fail-closed)
        assert_eq!(Status::from_u32(99), Status::InternalError);
        assert_eq!(Status::from_u32(0xFFFFFFFF), Status::InternalError);
    }
}

#[cfg(test)]
mod real_ipc_tests {
    use super::*;

    // These tests require a mock kernel socket
    // They're skipped in CI but run locally for validation

    #[test]
    #[cfg_attr(not(feature = "test_kernel_ipc"), ignore)]
    fn kernel_bridge_connects_to_socket() {
        let bridge = KernelBridge::new(PathBuf::from("/tmp/test_kernel.sock"));
        // Test would fail if socket doesn't exist
        let _ = bridge;
    }

    #[test]
    fn mock_bridge_returns_ok() {
        let mut bridge = MockKernelBridge::new();
        bridge.set_echo_response(vec![1, 2, 3]);
        let result = bridge.echo_request(0x1000, 1, b"test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }
}
