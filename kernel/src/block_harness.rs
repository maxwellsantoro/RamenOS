//! S13.6 runtime `harness.block` provider for QEMU init integration tests.
//!
//! Validates write payloads and stages read payloads against baked Oracle sector
//! vectors while using real shared-memory regions for the data plane.

// Compiled under `test` cfg via lib.rs for tranche5 `--all-targets`; handlers are
// only dispatched when the `test_protocols` feature is enabled (QEMU init gates).
#![cfg_attr(not(feature = "test_protocols"), allow(dead_code))]

use kernel_api::block_oracle_vector::{
    BLOCK_STATUS_OK, S13_ORACLE_BLOCK_COUNT, S13_ORACLE_BLOCK_SIZE, S13_ORACLE_READ_PAYLOAD,
    S13_ORACLE_READ_REQUEST_ID, S13_ORACLE_WRITE_PAYLOAD, S13_ORACLE_WRITE_REQUEST_ID,
};
use kernel_api::cap::Handle;
use kernel_api::generated::{
    BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS, MSG_BLOCK_V1_READ_BLOCKS_REPLY,
    MSG_BLOCK_V1_WRITE_BLOCKS, MSG_BLOCK_V1_WRITE_BLOCKS_REPLY, ReadBlocks, ReadBlocksReply,
    WriteBlocks, WriteBlocksReply,
};
use kernel_api::ipc::Envelope;
use kernel_api::wire::{read_payload, write_payload};

use crate::shmem;
use crate::trace_ring;

pub const BLOCK_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const BLOCK_STATUS_PAYLOAD_MISMATCH: u32 = 2;
pub const BLOCK_STATUS_SHMEM_ERROR: u32 = 3;

const TAG_BLOCK_READ: u32 = 0x4252; // 'BR'
const TAG_BLOCK_WRITE: u32 = 0x4257; // 'BW'

pub fn handle_block_v1(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    match env.msg_type {
        MSG_BLOCK_V1_READ_BLOCKS => handle_read_blocks(writer, shmem_table, env),
        MSG_BLOCK_V1_WRITE_BLOCKS => handle_write_blocks(writer, shmem_table, env),
        _ => Envelope::empty(BLOCK_V1_PROTOCOL_ID, env.msg_type),
    }
}

fn handle_read_blocks(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let request = match read_payload::<ReadBlocks>(env) {
        Ok(request) => request,
        Err(_) => return empty_read_reply(0, BLOCK_STATUS_INVALID_ARGUMENT, 0),
    };

    let reply = match process_read_blocks(shmem_table, &request) {
        Ok(bytes_read) => ReadBlocksReply {
            request_id: request.request_id,
            status: BLOCK_STATUS_OK,
            bytes_read,
        },
        Err(status) => ReadBlocksReply {
            request_id: request.request_id,
            status,
            bytes_read: 0,
        },
    };

    trace_ring::emit(
        writer,
        TAG_BLOCK_READ,
        request.request_id,
        reply.status as u64,
    );

    write_read_reply(&reply)
}

fn handle_write_blocks(
    writer: &trace_ring::TraceWriter,
    shmem_table: &shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let request = match read_payload::<WriteBlocks>(env) {
        Ok(request) => request,
        Err(_) => return empty_write_reply(0, BLOCK_STATUS_INVALID_ARGUMENT, 0),
    };

    let reply = match process_write_blocks(shmem_table, &request) {
        Ok(bytes_written) => WriteBlocksReply {
            request_id: request.request_id,
            status: BLOCK_STATUS_OK,
            bytes_written,
        },
        Err(status) => WriteBlocksReply {
            request_id: request.request_id,
            status,
            bytes_written: 0,
        },
    };

    trace_ring::emit(
        writer,
        TAG_BLOCK_WRITE,
        request.request_id,
        reply.status as u64,
    );

    write_write_reply(&reply)
}

fn process_read_blocks(
    shmem_table: &shmem::ShmemRegionTable,
    request: &ReadBlocks,
) -> Result<u32, u32> {
    if request.request_id != S13_ORACLE_READ_REQUEST_ID {
        return Err(BLOCK_STATUS_INVALID_ARGUMENT);
    }
    if request.block_size != S13_ORACLE_BLOCK_SIZE || request.block_count != S13_ORACLE_BLOCK_COUNT
    {
        return Err(BLOCK_STATUS_INVALID_ARGUMENT);
    }

    let byte_len = request
        .block_size
        .checked_mul(request.block_count)
        .ok_or(BLOCK_STATUS_INVALID_ARGUMENT)?;
    let shm_cap = Handle::unpack(request.buffer_shm_cap);
    write_shmem_slice(
        shmem_table,
        shm_cap,
        request.buffer_offset,
        &S13_ORACLE_READ_PAYLOAD,
    )?;

    Ok(byte_len)
}

fn process_write_blocks(
    shmem_table: &shmem::ShmemRegionTable,
    request: &WriteBlocks,
) -> Result<u32, u32> {
    if request.request_id != S13_ORACLE_WRITE_REQUEST_ID {
        return Err(BLOCK_STATUS_INVALID_ARGUMENT);
    }
    if request.block_size != S13_ORACLE_BLOCK_SIZE || request.block_count != S13_ORACLE_BLOCK_COUNT
    {
        return Err(BLOCK_STATUS_INVALID_ARGUMENT);
    }

    let byte_len = request
        .block_size
        .checked_mul(request.block_count)
        .ok_or(BLOCK_STATUS_INVALID_ARGUMENT)?;
    let shm_cap = Handle::unpack(request.data_shm_cap);
    let observed = read_shmem_slice(shmem_table, shm_cap, request.data_offset, byte_len)?;
    if observed != S13_ORACLE_WRITE_PAYLOAD {
        return Err(BLOCK_STATUS_PAYLOAD_MISMATCH);
    }

    Ok(byte_len)
}

fn read_shmem_slice(
    shmem_table: &shmem::ShmemRegionTable,
    shm_cap: Handle,
    offset: u64,
    len: u32,
) -> Result<&'static [u8], u32> {
    let phys_addr = shmem_table
        .phys_addr_for_cap(shm_cap)
        .ok_or(BLOCK_STATUS_SHMEM_ERROR)?;
    let end = offset
        .checked_add(len as u64)
        .ok_or(BLOCK_STATUS_SHMEM_ERROR)?;
    if end > shmem_table.region_size_for_cap(shm_cap).unwrap_or(0) {
        return Err(BLOCK_STATUS_SHMEM_ERROR);
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
        .ok_or(BLOCK_STATUS_SHMEM_ERROR)?;
    let end = offset
        .checked_add(data.len() as u64)
        .ok_or(BLOCK_STATUS_SHMEM_ERROR)?;
    if end > shmem_table.region_size_for_cap(shm_cap).unwrap_or(0) {
        return Err(BLOCK_STATUS_SHMEM_ERROR);
    }

    // SAFETY: same boot-time single-threaded QEMU integration context as reads.
    unsafe {
        let ptr = phys_addr.wrapping_add(offset) as *mut u8;
        core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
    Ok(())
}

fn write_read_reply(reply: &ReadBlocksReply) -> Envelope {
    let mut out = Envelope::empty(BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS_REPLY);
    if write_payload(&mut out, reply).is_err() {
        return empty_read_reply(reply.request_id, BLOCK_STATUS_INVALID_ARGUMENT, 0);
    }
    out
}

fn write_write_reply(reply: &WriteBlocksReply) -> Envelope {
    let mut out = Envelope::empty(BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_WRITE_BLOCKS_REPLY);
    if write_payload(&mut out, reply).is_err() {
        return empty_write_reply(reply.request_id, BLOCK_STATUS_INVALID_ARGUMENT, 0);
    }
    out
}

fn empty_read_reply(request_id: u64, status: u32, bytes_read: u32) -> Envelope {
    write_read_reply(&ReadBlocksReply {
        request_id,
        status,
        bytes_read,
    })
}

fn empty_write_reply(request_id: u64, status: u32, bytes_written: u32) -> Envelope {
    write_write_reply(&WriteBlocksReply {
        request_id,
        status,
        bytes_written,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::block_oracle_vector::S13_ORACLE_BLOCK_SIZE;

    #[test]
    fn read_rejects_wrong_request_id_without_shmem() {
        let table = shmem::ShmemRegionTable::new();
        let err = process_read_blocks(
            &table,
            &ReadBlocks {
                request_id: 99,
                lba: 0,
                block_count: 1,
                block_size: S13_ORACLE_BLOCK_SIZE,
                buffer_shm_cap: 0,
                buffer_offset: 0,
            },
        );
        assert_eq!(err, Err(BLOCK_STATUS_INVALID_ARGUMENT));
    }

    #[test]
    fn write_rejects_wrong_block_size_without_shmem() {
        let table = shmem::ShmemRegionTable::new();
        let err = process_write_blocks(
            &table,
            &WriteBlocks {
                request_id: S13_ORACLE_WRITE_REQUEST_ID,
                lba: 1,
                block_count: 1,
                block_size: 1,
                data_shm_cap: 0,
                data_offset: 0,
            },
        );
        assert_eq!(err, Err(BLOCK_STATUS_INVALID_ARGUMENT));
    }
}
