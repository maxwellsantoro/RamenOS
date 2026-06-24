//! Host-side kernel harness proxy for S10.5.1+.
//!
//! Unix socket transport (S10.5.1) and chardev serial framing (S10.5.2).

pub mod chardev_serial;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use kernel_api::generated::semantic_state_v1::{GetSnapshot, GetSnapshotReply};
use kernel_api::generated::{ShmemRead, ShmemReadReply, ShmemWrite, ShmemWriteReply};
use kernel_api::ipc::Envelope;
use kernel_api::ipc_frame::{ENVELOPE_WIRE_SIZE, envelope_from_wire, envelope_to_wire};
use kernel_api::semantic_snapshot_vector::S10_5_SEMANTIC_SNAPSHOT_BYTES;
use kernel_api::wire::{read_payload, write_payload};
const PROTOCOL_SHMEM_CONTROL: u32 = 8;
const PROTOCOL_SEMANTIC_STATE: u32 = 10;
const MSG_GET_SNAPSHOT: u32 = 1;
const MSG_GET_SNAPSHOT_REPLY: u32 = 2;
const MSG_SHMEM_READ: u32 = 9;
const MSG_SHMEM_READ_REPLY: u32 = 10;
const MSG_SHMEM_WRITE: u32 = 11;
const MSG_SHMEM_WRITE_REPLY: u32 = 12;

const STATUS_OK: u32 = 0;
const STATUS_INVALID_CAPABILITY: u32 = 1;
const STATUS_INVALID_ARGUMENT: u32 = 3;
const STATUS_INTERNAL_ERROR: u32 = 6;

pub const INTERFACE_SHMEM_CONTROL_V1: &str = "shared_memory.control_v1";
pub const INTERFACE_SEMANTIC_STATE_V1: &str = "services.semantic_state_v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantBinding {
    pub domain_id: u64,
    pub interface: &'static str,
}

#[derive(Debug, Default)]
pub struct KernelHarnessProxy {
    grants: HashMap<u64, GrantBinding>,
    shmem: HashMap<u64, Vec<u8>>,
    next_shm_cap: u64,
}

impl KernelHarnessProxy {
    pub fn new() -> Self {
        Self {
            grants: HashMap::new(),
            shmem: HashMap::new(),
            next_shm_cap: 0x5308_0001_0000_0001,
        }
    }

    pub fn with_semantic_grants(domain_id: u64, shmem_handle: u64, semantic_handle: u64) -> Self {
        let mut proxy = Self::new();
        proxy.register_grant(domain_id, shmem_handle, INTERFACE_SHMEM_CONTROL_V1);
        proxy.register_grant(domain_id, semantic_handle, INTERFACE_SEMANTIC_STATE_V1);
        proxy
    }

    pub fn register_grant(&mut self, domain_id: u64, handle: u64, interface: &'static str) {
        self.grants.insert(
            handle,
            GrantBinding {
                domain_id,
                interface,
            },
        );
    }

    pub fn read_shmem(&self, shm_cap: u64) -> Option<&[u8]> {
        self.shmem.get(&shm_cap).map(Vec::as_slice)
    }

    pub fn transact(&mut self, request: Envelope) -> Envelope {
        match (request.protocol, request.msg_type) {
            (PROTOCOL_SEMANTIC_STATE, MSG_GET_SNAPSHOT) => self.handle_get_snapshot(&request),
            (PROTOCOL_SHMEM_CONTROL, MSG_SHMEM_READ) => self.handle_shmem_read(&request),
            (PROTOCOL_SHMEM_CONTROL, MSG_SHMEM_WRITE) => self.handle_shmem_write(&request),
            _ => Envelope::empty(request.protocol, request.msg_type.saturating_add(1)),
        }
    }

    pub fn serve_once<P: AsRef<Path>>(&mut self, socket_path: P) -> std::io::Result<()> {
        let socket_path = socket_path.as_ref();
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path)?;
        let (stream, _) = listener.accept()?;
        self.handle_stream(stream)
    }

    fn handle_stream(&mut self, mut stream: UnixStream) -> std::io::Result<()> {
        let mut request_bytes = [0u8; ENVELOPE_WIRE_SIZE];
        stream.read_exact(&mut request_bytes)?;
        let request = envelope_from_wire(&request_bytes);
        let reply = self.transact(request);
        stream.write_all(&envelope_to_wire(&reply))
    }

    fn handle_get_snapshot(&mut self, request: &Envelope) -> Envelope {
        let Ok(payload) = read_payload::<GetSnapshot>(request) else {
            return semantic_reply(0, STATUS_INTERNAL_ERROR, 0, 0);
        };
        if !self.has_interface(payload.cap_handle, INTERFACE_SEMANTIC_STATE_V1) {
            return semantic_reply(payload.request_id, STATUS_INVALID_CAPABILITY, 0, 0);
        }
        if payload.format != 0 {
            return semantic_reply(payload.request_id, STATUS_INVALID_ARGUMENT, 0, 0);
        }

        let shm_cap = self.allocate_snapshot_shmem();
        semantic_reply(
            payload.request_id,
            STATUS_OK,
            shm_cap,
            S10_5_SEMANTIC_SNAPSHOT_BYTES.len() as u64,
        )
    }

    fn handle_shmem_read(&self, request: &Envelope) -> Envelope {
        let Ok(payload) = read_payload::<ShmemRead>(request) else {
            return shmem_read_reply(STATUS_INTERNAL_ERROR, 0);
        };
        let Some(bytes) = self.shmem.get(&payload.shm_cap) else {
            return shmem_read_reply(STATUS_INVALID_CAPABILITY, 0);
        };
        let offset = payload.offset as usize;
        let len = payload.len as usize;
        if offset > bytes.len() {
            return shmem_read_reply(STATUS_INVALID_ARGUMENT, 0);
        }
        let bytes_read = len.min(bytes.len() - offset);
        shmem_read_reply(STATUS_OK, bytes_read as u32)
    }

    fn handle_shmem_write(&mut self, request: &Envelope) -> Envelope {
        let Ok(payload) = read_payload::<ShmemWrite>(request) else {
            return shmem_write_reply(STATUS_INTERNAL_ERROR, 0);
        };
        if !self.shmem.contains_key(&payload.shm_cap) {
            return shmem_write_reply(STATUS_INVALID_CAPABILITY, 0);
        }
        let Some(region) = self.shmem.get(&payload.shm_cap) else {
            return shmem_write_reply(STATUS_INVALID_CAPABILITY, 0);
        };
        let offset = payload.offset as usize;
        let len = payload.len as usize;
        if offset > region.len() || offset.saturating_add(len) > region.len() {
            return shmem_write_reply(STATUS_INVALID_ARGUMENT, 0);
        }
        shmem_write_reply(STATUS_OK, payload.len)
    }

    fn has_interface(&self, handle: u64, interface: &str) -> bool {
        self.grants
            .get(&handle)
            .map(|grant| grant.interface == interface)
            .unwrap_or(false)
    }

    fn allocate_snapshot_shmem(&mut self) -> u64 {
        let shm_cap = self.next_shm_cap;
        self.next_shm_cap = self.next_shm_cap.wrapping_add(1);
        self.shmem
            .insert(shm_cap, S10_5_SEMANTIC_SNAPSHOT_BYTES.to_vec());
        shm_cap
    }
}

fn semantic_reply(request_id: u64, status: u32, shm_cap: u64, shm_size: u64) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_SEMANTIC_STATE, MSG_GET_SNAPSHOT_REPLY);
    let payload = GetSnapshotReply {
        request_id,
        status,
        shm_cap,
        shm_size,
    };
    let _ = write_payload(&mut env, &payload);
    env
}

fn shmem_read_reply(status: u32, bytes_read: u32) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_SHMEM_READ_REPLY);
    let payload = ShmemReadReply { status, bytes_read };
    let _ = write_payload(&mut env, &payload);
    env
}

fn shmem_write_reply(status: u32, bytes_written: u32) -> Envelope {
    let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_SHMEM_WRITE_REPLY);
    let payload = ShmemWriteReply {
        status,
        bytes_written,
    };
    let _ = write_payload(&mut env, &payload);
    env
}

pub fn envelope_to_bytes(env: &Envelope) -> [u8; ENVELOPE_WIRE_SIZE] {
    envelope_to_wire(env)
}

pub fn bytes_to_envelope(bytes: &[u8; ENVELOPE_WIRE_SIZE]) -> Envelope {
    envelope_from_wire(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::semantic_snapshot_vector::S10_5_SEMANTIC_SNAPSHOT_SHA256_PREFIX;
    use kernel_api::wire::read_payload;

    const DOMAIN_ID: u64 = 42;
    const SHMEM_HANDLE: u64 = 0x5308_0000_002a_0001;
    const SEMANTIC_HANDLE: u64 = 0x5310_0000_002a_0002;

    #[test]
    fn proxy_get_snapshot_roundtrip() {
        let mut proxy =
            KernelHarnessProxy::with_semantic_grants(DOMAIN_ID, SHMEM_HANDLE, SEMANTIC_HANDLE);
        let mut request = Envelope::empty(PROTOCOL_SEMANTIC_STATE, MSG_GET_SNAPSHOT);
        write_payload(
            &mut request,
            &GetSnapshot {
                cap_handle: SEMANTIC_HANDLE,
                request_id: 7,
                format: 0,
            },
        )
        .unwrap();

        let reply = proxy.transact(request);
        let payload: GetSnapshotReply = read_payload(&reply).unwrap();

        assert_eq!(payload.request_id, 7);
        assert_eq!(payload.status, STATUS_OK);
        assert_eq!(payload.shm_size, S10_5_SEMANTIC_SNAPSHOT_BYTES.len() as u64);
        assert_eq!(
            proxy.read_shmem(payload.shm_cap).unwrap(),
            S10_5_SEMANTIC_SNAPSHOT_BYTES
        );
        println!(
            "snapshot_sha256_prefix={}",
            S10_5_SEMANTIC_SNAPSHOT_SHA256_PREFIX
        );
    }
}
