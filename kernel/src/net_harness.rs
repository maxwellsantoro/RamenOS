//! S11.8 runtime `harness.net` provider for QEMU init integration tests.
//!
//! Validates send payloads and stages receive payloads against the baked Oracle
//! vectors while using real shared-memory regions for the data plane.

// Compiled under `test` cfg via lib.rs for tranche5 `--all-targets`; handlers are
// only dispatched when the `test_protocols` feature is enabled (QEMU init gates).
#![cfg_attr(not(feature = "test_protocols"), allow(dead_code))]

use kernel_api::cap::Handle;
use kernel_api::generated::{
    MSG_NET_V1_RECEIVE_PACKET, MSG_NET_V1_RECEIVE_PACKET_REPLY, MSG_NET_V1_SEND_PACKET,
    MSG_NET_V1_SEND_PACKET_REPLY, NET_V1_PROTOCOL_ID, ReceivePacket, ReceivePacketReply,
    SendPacket, SendPacketReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::net_packet_oracle_vector::{
    NET_STATUS_OK, S11_ORACLE_PACKET_LEN, S11_ORACLE_RECV_PAYLOAD, S11_ORACLE_RECV_REQUEST_ID,
    S11_ORACLE_SEND_PAYLOAD, S11_ORACLE_SEND_REQUEST_ID,
};
use kernel_api::wire::{read_payload, write_payload};

use crate::shmem;
use crate::trace_ring;

pub const NET_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const NET_STATUS_PAYLOAD_MISMATCH: u32 = 2;
pub const NET_STATUS_SHMEM_ERROR: u32 = 3;

const TAG_NET_SEND: u32 = 0x4e53; // 'NS'
const TAG_NET_RECV: u32 = 0x4e52; // 'NR'

pub fn handle_net_v1(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    match env.msg_type {
        MSG_NET_V1_SEND_PACKET => handle_send_packet(writer, shmem_table, env),
        MSG_NET_V1_RECEIVE_PACKET => handle_receive_packet(writer, shmem_table, env),
        _ => Envelope::empty(NET_V1_PROTOCOL_ID, env.msg_type),
    }
}

fn handle_send_packet(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let request = match read_payload::<SendPacket>(env) {
        Ok(request) => request,
        Err(_) => return empty_send_reply(0, NET_STATUS_INVALID_ARGUMENT, 0),
    };

    let reply = match process_send_packet(shmem_table, &request) {
        Ok(bytes_sent) => SendPacketReply {
            request_id: request.request_id,
            status: NET_STATUS_OK,
            bytes_sent,
        },
        Err(status) => SendPacketReply {
            request_id: request.request_id,
            status,
            bytes_sent: 0,
        },
    };

    trace_ring::emit(
        writer,
        TAG_NET_SEND,
        request.request_id,
        reply.status as u64,
    );

    write_send_reply(&reply)
}

fn handle_receive_packet(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let request = match read_payload::<ReceivePacket>(env) {
        Ok(request) => request,
        Err(_) => return empty_recv_reply(0, NET_STATUS_INVALID_ARGUMENT, 0),
    };

    let reply = match process_receive_packet(shmem_table, &request) {
        Ok(bytes_received) => ReceivePacketReply {
            request_id: request.request_id,
            status: NET_STATUS_OK,
            bytes_received,
        },
        Err(status) => ReceivePacketReply {
            request_id: request.request_id,
            status,
            bytes_received: 0,
        },
    };

    trace_ring::emit(
        writer,
        TAG_NET_RECV,
        request.request_id,
        reply.status as u64,
    );

    write_recv_reply(&reply)
}

fn process_send_packet(
    shmem_table: &shmem::ShmemRegionTable,
    request: &SendPacket,
) -> Result<u32, u32> {
    if request.request_id != S11_ORACLE_SEND_REQUEST_ID {
        return Err(NET_STATUS_INVALID_ARGUMENT);
    }
    if request.data_len != S11_ORACLE_PACKET_LEN {
        return Err(NET_STATUS_INVALID_ARGUMENT);
    }

    let shm_cap = Handle::unpack(request.data_shm_cap);
    let observed = read_shmem_slice(shmem_table, shm_cap, request.data_offset, request.data_len)?;
    if observed != S11_ORACLE_SEND_PAYLOAD {
        return Err(NET_STATUS_PAYLOAD_MISMATCH);
    }

    Ok(request.data_len)
}

fn process_receive_packet(
    shmem_table: &shmem::ShmemRegionTable,
    request: &ReceivePacket,
) -> Result<u32, u32> {
    if request.request_id != S11_ORACLE_RECV_REQUEST_ID {
        return Err(NET_STATUS_INVALID_ARGUMENT);
    }
    if request.buffer_len < S11_ORACLE_PACKET_LEN {
        return Err(NET_STATUS_INVALID_ARGUMENT);
    }

    let shm_cap = Handle::unpack(request.buffer_shm_cap);
    write_shmem_slice(
        shmem_table,
        shm_cap,
        request.buffer_offset,
        &S11_ORACLE_RECV_PAYLOAD,
    )?;

    Ok(S11_ORACLE_PACKET_LEN)
}

fn read_shmem_slice(
    shmem_table: &shmem::ShmemRegionTable,
    shm_cap: Handle,
    offset: u64,
    len: u32,
) -> Result<&'static [u8], u32> {
    let phys_addr = shmem_table
        .phys_addr_for_cap(shm_cap)
        .ok_or(NET_STATUS_SHMEM_ERROR)?;
    let end = offset
        .checked_add(len as u64)
        .ok_or(NET_STATUS_SHMEM_ERROR)?;
    if end > shmem_table.region_size_for_cap(shm_cap).unwrap_or(0) {
        return Err(NET_STATUS_SHMEM_ERROR);
    }

    // SAFETY: boot-time init tests run single-threaded in QEMU with freshly
    // allocated physical shmem frames returned by `create_region`.
    unsafe {
        let ptr = phys_addr.wrapping_add(offset) as *const u8;
        Ok(core::slice::from_raw_parts(ptr, len as usize))
    }
}

fn write_shmem_slice(
    shmem_table: &shmem::ShmemRegionTable,
    shm_cap: Handle,
    offset: u64,
    data: &[u8],
) -> Result<(), u32> {
    let phys_addr = shmem_table
        .phys_addr_for_cap(shm_cap)
        .ok_or(NET_STATUS_SHMEM_ERROR)?;
    let end = offset
        .checked_add(data.len() as u64)
        .ok_or(NET_STATUS_SHMEM_ERROR)?;
    if end > shmem_table.region_size_for_cap(shm_cap).unwrap_or(0) {
        return Err(NET_STATUS_SHMEM_ERROR);
    }

    // SAFETY: same boot-time single-threaded QEMU integration context as reads.
    unsafe {
        let ptr = phys_addr.wrapping_add(offset) as *mut u8;
        core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
    Ok(())
}

fn write_send_reply(reply: &SendPacketReply) -> Envelope {
    let mut out = Envelope::empty(NET_V1_PROTOCOL_ID, MSG_NET_V1_SEND_PACKET_REPLY);
    if write_payload(&mut out, reply).is_err() {
        return empty_send_reply(reply.request_id, NET_STATUS_INVALID_ARGUMENT, 0);
    }
    out
}

fn write_recv_reply(reply: &ReceivePacketReply) -> Envelope {
    let mut out = Envelope::empty(NET_V1_PROTOCOL_ID, MSG_NET_V1_RECEIVE_PACKET_REPLY);
    if write_payload(&mut out, reply).is_err() {
        return empty_recv_reply(reply.request_id, NET_STATUS_INVALID_ARGUMENT, 0);
    }
    out
}

fn empty_send_reply(request_id: u64, status: u32, bytes_sent: u32) -> Envelope {
    write_send_reply(&SendPacketReply {
        request_id,
        status,
        bytes_sent,
    })
}

fn empty_recv_reply(request_id: u64, status: u32, bytes_received: u32) -> Envelope {
    write_recv_reply(&ReceivePacketReply {
        request_id,
        status,
        bytes_received,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::cap::{Handle, HandleKind};
    use kernel_api::net_packet_oracle_vector::S11_ORACLE_PACKET_LEN;

    #[test]
    fn send_rejects_wrong_request_id_without_shmem() {
        let table = shmem::ShmemRegionTable::new();
        let err = process_send_packet(
            &table,
            &SendPacket {
                request_id: 99,
                data_shm_cap: 0,
                data_offset: 0,
                data_len: S11_ORACLE_PACKET_LEN,
            },
        );
        assert_eq!(err, Err(NET_STATUS_INVALID_ARGUMENT));
    }

    #[test]
    fn receive_rejects_short_buffer_without_shmem() {
        let table = shmem::ShmemRegionTable::new();
        let err = process_receive_packet(
            &table,
            &ReceivePacket {
                request_id: S11_ORACLE_RECV_REQUEST_ID,
                buffer_shm_cap: 0,
                buffer_offset: 0,
                buffer_len: 1,
            },
        );
        assert_eq!(err, Err(NET_STATUS_INVALID_ARGUMENT));
    }

    #[test]
    fn phys_addr_for_cap_requires_valid_handle() {
        let table = shmem::ShmemRegionTable::new();
        let invalid = Handle {
            kind: HandleKind::Shmem,
            index: 1,
            generation: 1,
        };
        assert!(table.phys_addr_for_cap(invalid).is_none());
    }
}
