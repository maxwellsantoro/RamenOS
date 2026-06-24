use kernel_api::cap::{CapTable, Handle};
use kernel_api::generated::{
    CloseRegion, CloseRegionReply, CreateRegion, CreateRegionReply, CreateTraceBuffer,
    CreateTraceBufferReply, DestroyTraceBuffer, DestroyTraceBufferReply, GetTraceInfo,
    GetTraceInfoReply, MapRegion, MapRegionReply, Ping, Pong, ReadTrace, ReadTraceReply,
    UnmapRegion, UnmapRegionReply,
};
use kernel_api::ipc::{Envelope, MSG_PING, MSG_PONG};
use kernel_api::trace::TAG_IPC;
use kernel_api::wire::{read_payload, write_payload};

use crate::cap_table::StaticCapTable;
use crate::shmem;
use crate::trace_ring;
use crate::trace_service::TraceService;

// Protocol IDs
pub const PROTOCOL_PING: u32 = 1;
pub const PROTOCOL_SHMEM_CONTROL: u32 = 8;
pub const PROTOCOL_TRACE_SERVICE_V1: u32 = 9;

// Message types for PROTOCOL_SHMEM_CONTROL
pub const MSG_CREATE_REGION: u32 = 1;
pub const MSG_CREATE_REGION_REPLY: u32 = 2;
pub const MSG_MAP_REGION: u32 = 3;
pub const MSG_MAP_REGION_REPLY: u32 = 4;
pub const MSG_UNMAP_REGION: u32 = 5;
pub const MSG_UNMAP_REGION_REPLY: u32 = 6;
pub const MSG_CLOSE_REGION: u32 = 7;
pub const MSG_CLOSE_REGION_REPLY: u32 = 8;

// Message types for PROTOCOL_TRACE_SERVICE_V1
pub const MSG_CREATE_TRACE_BUFFER: u32 = 1;
pub const MSG_CREATE_TRACE_BUFFER_REPLY: u32 = 2;
pub const MSG_DESTROY_TRACE_BUFFER: u32 = 3;
pub const MSG_DESTROY_TRACE_BUFFER_REPLY: u32 = 4;
pub const MSG_READ_TRACE: u32 = 5;
pub const MSG_READ_TRACE_REPLY: u32 = 6;
pub const MSG_GET_TRACE_INFO: u32 = 7;
pub const MSG_GET_TRACE_INFO_REPLY: u32 = 8;

// S8 Phase 4: Trace tag constants for shared memory operations
const TAG_SHMEM_CREATE: u32 = 0x1000;
const TAG_SHMEM_MAP: u32 = 0x1001;
const TAG_SHMEM_CLOSE: u32 = 0x1002;

// V-012 Phase 4: Trace tag constants for trace service operations
const TAG_TRACE_CREATE: u32 = 0x2000;
const TAG_TRACE_DESTROY: u32 = 0x2001;
const TAG_TRACE_READ: u32 = 0x2002;
const TAG_TRACE_GET_INFO: u32 = 0x2003;

/// V-001: Validate handle before dispatching control operations.
/// Rejects INVALID handles to prevent capability bypass for privileged operations.
/// V-16/SC-13: Verify handle kind matches IPC type.
///
/// # Additional Hardening: PING Protocol Exemption
///
/// The PING protocol is only exempt from capability validation when the
/// `test_protocols` feature is enabled. In production builds (without the
/// feature), ALL protocols require valid capabilities.
fn validate_handle(table: &StaticCapTable, env: &Envelope) -> bool {
    // Additional hardening: PING protocol only exempt with feature flag
    #[cfg(feature = "test_protocols")]
    {
        if env.protocol == PROTOCOL_PING
            || env.protocol == kernel_api::generated::NET_V1_PROTOCOL_ID
            || env.protocol == kernel_api::generated::BLOCK_V1_PROTOCOL_ID
        {
            return true;
        }
    }

    // V-001: Reject INVALID handles for all protocols (no capability bypass)
    if env.handle == Handle::INVALID {
        return false;
    }
    // V-16/SC-13: Reject handles with wrong kind
    if env.handle.kind != kernel_api::cap::HandleKind::Ipc {
        return false;
    }
    table.validate(env.handle)
}

/// Handle PROTOCOL_SHMEM_CONTROL messages.
fn handle_shmem_control(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    match env.msg_type {
        MSG_CREATE_REGION => handle_create_region(writer, table, shmem_table, env),
        MSG_MAP_REGION => {
            // Validate capability for MAP_REGION
            if env.handle == Handle::INVALID {
                let reply = MapRegionReply {
                    request_id: 0,
                    region_id: 0,
                    mapping_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply);
            }
            if !validate_handle(table, env) {
                let reply = MapRegionReply {
                    request_id: 0,
                    region_id: 0,
                    mapping_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply);
            }
            handle_map_region(writer, table, shmem_table, env)
        }
        MSG_UNMAP_REGION => {
            // Validate capability for UNMAP_REGION
            if env.handle == Handle::INVALID {
                let reply = UnmapRegionReply {
                    request_id: 0,
                    mapping_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(
                    PROTOCOL_SHMEM_CONTROL,
                    MSG_UNMAP_REGION_REPLY,
                    &reply,
                );
            }
            if !validate_handle(table, env) {
                let reply = UnmapRegionReply {
                    request_id: 0,
                    mapping_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(
                    PROTOCOL_SHMEM_CONTROL,
                    MSG_UNMAP_REGION_REPLY,
                    &reply,
                );
            }
            handle_unmap_region(table, shmem_table, env)
        }
        MSG_CLOSE_REGION => {
            // Validate capability for CLOSE_REGION
            if env.handle == Handle::INVALID {
                let reply = CloseRegionReply {
                    request_id: 0,
                    region_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(
                    PROTOCOL_SHMEM_CONTROL,
                    MSG_CLOSE_REGION_REPLY,
                    &reply,
                );
            }
            if !validate_handle(table, env) {
                let reply = CloseRegionReply {
                    request_id: 0,
                    region_id: 0,
                    status: shmem::STATUS_INVALID_CAPABILITY,
                    reserved: 0,
                };
                return write_reply_or_error(
                    PROTOCOL_SHMEM_CONTROL,
                    MSG_CLOSE_REGION_REPLY,
                    &reply,
                );
            }
            handle_close_region(writer, table, shmem_table, env)
        }
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

fn handle_create_region(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<CreateRegion>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = CreateRegionReply {
                request_id: 0,
                region_id: 0,
                shm_cap: 0,
                phys_addr: 0,
                status: shmem::STATUS_INVALID_SIZE,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION_REPLY, &reply);
        }
    };

    let caller_domain_id = match table.domain_for(env.handle) {
        Some(domain_id) => domain_id,
        None => {
            let reply = CreateRegionReply {
                request_id: req.request_id,
                region_id: 0,
                shm_cap: 0,
                phys_addr: 0,
                status: shmem::STATUS_INVALID_CAPABILITY,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION_REPLY, &reply);
        }
    };
    if req.owner_domain_id != caller_domain_id {
        let reply = CreateRegionReply {
            request_id: req.request_id,
            region_id: 0,
            shm_cap: 0,
            phys_addr: 0,
            status: shmem::STATUS_PERMISSION_DENIED,
            reserved: 0,
        };
        return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION_REPLY, &reply);
    }

    match shmem_table.create_region(caller_domain_id, req.size_bytes, req.flags, req.page_size) {
        Ok((region_id, shm_cap, phys_addr)) => {
            // S8 Phase 4: Emit trace event for successful create_region
            // Pack trace data: arg0 = region_id, arg1 = (size_bytes << 32) | flags
            trace_ring::emit(
                writer,
                TAG_SHMEM_CREATE,
                region_id,
                (req.size_bytes << 32) | (req.flags as u64),
            );

            let reply = CreateRegionReply {
                request_id: req.request_id,
                region_id,
                shm_cap: shm_cap.pack(),
                phys_addr,
                status: shmem::STATUS_OK,
                reserved: 0,
            };
            write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION_REPLY, &reply)
        }
        Err(status) => {
            let reply = CreateRegionReply {
                request_id: req.request_id,
                region_id: 0,
                shm_cap: 0,
                phys_addr: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION_REPLY, &reply)
        }
    }
}

fn handle_map_region(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<MapRegion>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = MapRegionReply {
                request_id: 0,
                region_id: 0,
                mapping_id: 0,
                status: shmem::STATUS_INVALID_RIGHTS,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply);
        }
    };

    let shm_cap = Handle::unpack(req.shm_cap);
    let caller_domain_id = match table.domain_for(env.handle) {
        Some(domain_id) => domain_id,
        None => {
            let reply = MapRegionReply {
                request_id: req.request_id,
                region_id: req.region_id,
                mapping_id: 0,
                status: shmem::STATUS_INVALID_CAPABILITY,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply);
        }
    };
    if req.caller_domain_id != caller_domain_id {
        let reply = MapRegionReply {
            request_id: req.request_id,
            region_id: req.region_id,
            mapping_id: 0,
            status: shmem::STATUS_PERMISSION_DENIED,
            reserved: 0,
        };
        return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply);
    }

    match shmem_table.map_region(
        req.region_id,
        shm_cap,
        caller_domain_id,
        req.target_domain_id,
        req.rights,
        req.cache_mode,
    ) {
        Ok(mapping_id) => {
            // S8 Phase 4: Emit trace event for successful map_region
            // Pack trace data: arg0 = region_id, arg1 = (target_domain_id << 32) | rights
            trace_ring::emit(
                writer,
                TAG_SHMEM_MAP,
                req.region_id,
                (req.target_domain_id << 32) | (req.rights as u64),
            );

            let reply = MapRegionReply {
                request_id: req.request_id,
                region_id: req.region_id,
                mapping_id,
                status: shmem::STATUS_OK,
                reserved: 0,
            };
            write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply)
        }
        Err(status) => {
            let reply = MapRegionReply {
                request_id: req.request_id,
                region_id: req.region_id,
                mapping_id: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION_REPLY, &reply)
        }
    }
}

fn handle_unmap_region(
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<UnmapRegion>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = UnmapRegionReply {
                request_id: 0,
                mapping_id: 0,
                status: shmem::STATUS_REGION_NOT_FOUND,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION_REPLY, &reply);
        }
    };

    let caller_domain_id = match table.domain_for(env.handle) {
        Some(domain_id) => domain_id,
        None => {
            let reply = UnmapRegionReply {
                request_id: req.request_id,
                mapping_id: req.mapping_id,
                status: shmem::STATUS_INVALID_CAPABILITY,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION_REPLY, &reply);
        }
    };
    if req.caller_domain_id != caller_domain_id {
        let reply = UnmapRegionReply {
            request_id: req.request_id,
            mapping_id: req.mapping_id,
            status: shmem::STATUS_PERMISSION_DENIED,
            reserved: 0,
        };
        return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION_REPLY, &reply);
    }

    // P1 fix: Pass caller_domain_id for authorization
    let status =
        match shmem_table.unmap_region(caller_domain_id, req.mapping_id, req.target_domain_id) {
            Ok(()) => shmem::STATUS_OK,
            Err(status) => status,
        };

    let reply = UnmapRegionReply {
        request_id: req.request_id,
        mapping_id: req.mapping_id,
        status,
        reserved: 0,
    };
    write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION_REPLY, &reply)
}

fn handle_close_region(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<CloseRegion>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = CloseRegionReply {
                request_id: 0,
                region_id: 0,
                status: shmem::STATUS_REGION_NOT_FOUND,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION_REPLY, &reply);
        }
    };

    let caller_domain_id = match table.domain_for(env.handle) {
        Some(domain_id) => domain_id,
        None => {
            let reply = CloseRegionReply {
                request_id: req.request_id,
                region_id: req.region_id,
                status: shmem::STATUS_INVALID_CAPABILITY,
                reserved: 0,
            };
            return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION_REPLY, &reply);
        }
    };
    if req.caller_domain_id != caller_domain_id {
        let reply = CloseRegionReply {
            request_id: req.request_id,
            region_id: req.region_id,
            status: shmem::STATUS_PERMISSION_DENIED,
            reserved: 0,
        };
        return write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION_REPLY, &reply);
    }

    // P1 fix: Pass caller_domain_id for authorization
    let status = match shmem_table.close_region(caller_domain_id, req.region_id) {
        Ok(()) => {
            // S8 Phase 4: Emit trace event for successful close_region
            // Pack trace data: arg0 = region_id, arg1 = 0 (reserved)
            trace_ring::emit(writer, TAG_SHMEM_CLOSE, req.region_id, 0);
            shmem::STATUS_OK
        }
        Err(status) => status,
    };

    let reply = CloseRegionReply {
        request_id: req.request_id,
        region_id: req.region_id,
        status,
        reserved: 0,
    };
    write_reply_or_error(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION_REPLY, &reply)
}

fn write_reply_or_error<T: Copy>(protocol: u32, msg_type: u32, reply: &T) -> Envelope {
    let mut out = Envelope::empty(protocol, msg_type);
    match write_payload(&mut out, reply) {
        Ok(()) => out,
        Err(_) => Envelope::empty(protocol, msg_type),
    }
}

/// Construct an error reply for shared memory operations.
///
/// This helper reduces code duplication by centralizing the error reply
/// construction logic for all shmem control operations.
fn shmem_error_reply(env: &Envelope, status: u32) -> Envelope {
    let reply_msg_type = env.msg_type + 1;

    match env.msg_type {
        MSG_CREATE_REGION => {
            let reply = CreateRegionReply {
                request_id: 0,
                region_id: 0,
                shm_cap: 0,
                phys_addr: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(env.protocol, reply_msg_type, &reply)
        }
        MSG_MAP_REGION => {
            let reply = MapRegionReply {
                request_id: 0,
                region_id: 0,
                mapping_id: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(env.protocol, reply_msg_type, &reply)
        }
        MSG_UNMAP_REGION => {
            let reply = UnmapRegionReply {
                request_id: 0,
                mapping_id: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(env.protocol, reply_msg_type, &reply)
        }
        MSG_CLOSE_REGION => {
            let reply = CloseRegionReply {
                request_id: 0,
                region_id: 0,
                status,
                reserved: 0,
            };
            write_reply_or_error(env.protocol, reply_msg_type, &reply)
        }
        _ => {
            // Unknown message type; return empty envelope
            Envelope::empty(env.protocol, reply_msg_type)
        }
    }
}

/// Handle PROTOCOL_TRACE_SERVICE_V1 messages.
fn handle_trace_service(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    match env.msg_type {
        MSG_CREATE_TRACE_BUFFER => handle_create_trace_buffer(writer, trace_service, env),
        MSG_DESTROY_TRACE_BUFFER => handle_destroy_trace_buffer(writer, trace_service, env),
        MSG_READ_TRACE => handle_read_trace(writer, trace_service, env),
        MSG_GET_TRACE_INFO => handle_get_trace_info(writer, trace_service, env),
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

fn handle_create_trace_buffer(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<CreateTraceBuffer>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = CreateTraceBufferReply {
                request_id: 0,
                status: crate::trace_service::TRACE_STATUS_INVALID_SIZE,
                trace_cap: 0,
            };
            return write_reply_or_error(
                PROTOCOL_TRACE_SERVICE_V1,
                MSG_CREATE_TRACE_BUFFER_REPLY,
                &reply,
            );
        }
    };

    let (status, trace_cap) = match trace_service.create_trace_buffer(req.domain_id, req.size) {
        Ok((s, c)) => (s, c.pack()),
        Err(_) => (crate::trace_service::TRACE_STATUS_INVALID_DOMAIN, 0),
    };

    trace_ring::emit(writer, TAG_TRACE_CREATE, req.domain_id, status as u64);

    let reply = CreateTraceBufferReply {
        request_id: req.request_id,
        status,
        trace_cap,
    };
    write_reply_or_error(
        PROTOCOL_TRACE_SERVICE_V1,
        MSG_CREATE_TRACE_BUFFER_REPLY,
        &reply,
    )
}

fn handle_destroy_trace_buffer(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<DestroyTraceBuffer>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = DestroyTraceBufferReply {
                request_id: 0,
                status: crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
            };
            return write_reply_or_error(
                PROTOCOL_TRACE_SERVICE_V1,
                MSG_DESTROY_TRACE_BUFFER_REPLY,
                &reply,
            );
        }
    };

    let trace_cap = Handle::unpack(req.trace_cap);

    let status = match trace_service.destroy_trace_buffer(trace_cap) {
        Ok(s) => s,
        Err(_) => crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
    };

    trace_ring::emit(
        writer,
        TAG_TRACE_DESTROY,
        trace_cap.index as u64,
        status as u64,
    );

    let reply = DestroyTraceBufferReply {
        request_id: req.request_id,
        status,
    };
    write_reply_or_error(
        PROTOCOL_TRACE_SERVICE_V1,
        MSG_DESTROY_TRACE_BUFFER_REPLY,
        &reply,
    )
}

fn handle_read_trace(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<ReadTrace>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = ReadTraceReply {
                request_id: 0,
                status: crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
                data_len: 0,
            };
            return write_reply_or_error(PROTOCOL_TRACE_SERVICE_V1, MSG_READ_TRACE_REPLY, &reply);
        }
    };

    let trace_cap = Handle::unpack(req.trace_cap);

    // Allocate temporary buffer for trace data
    let mut data = [0u8; 4096];
    let (status, bytes_read) =
        match trace_service.read_trace(trace_cap, req.offset, req.length, &mut data) {
            Ok((s, b)) => (s, b),
            Err(_) => (crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY, 0),
        };

    trace_ring::emit(writer, TAG_TRACE_READ, req.offset, bytes_read as u64);

    let reply = ReadTraceReply {
        request_id: req.request_id,
        status,
        data_len: bytes_read,
    };
    write_reply_or_error(PROTOCOL_TRACE_SERVICE_V1, MSG_READ_TRACE_REPLY, &reply)
}

fn handle_get_trace_info(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    let req = match read_payload::<GetTraceInfo>(env) {
        Ok(req) => req,
        Err(_) => {
            let reply = GetTraceInfoReply {
                request_id: 0,
                status: crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
                domain_id: 0,
                size: 0,
                read_offset: 0,
                write_offset: 0,
            };
            return write_reply_or_error(
                PROTOCOL_TRACE_SERVICE_V1,
                MSG_GET_TRACE_INFO_REPLY,
                &reply,
            );
        }
    };

    let trace_cap = Handle::unpack(req.trace_cap);

    let (status, info) = match trace_service.get_trace_info(trace_cap) {
        Ok((s, i)) => (s, i),
        Err(_) => (
            crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
            crate::trace_service::TraceBufferInfo {
                domain_id: 0,
                size: 0,
                read_offset: 0,
                write_offset: 0,
            },
        ),
    };

    trace_ring::emit(writer, TAG_TRACE_GET_INFO, info.domain_id, info.size as u64);

    let reply = GetTraceInfoReply {
        request_id: req.request_id,
        status,
        domain_id: info.domain_id,
        size: info.size,
        read_offset: info.read_offset,
        write_offset: info.write_offset,
    };
    write_reply_or_error(PROTOCOL_TRACE_SERVICE_V1, MSG_GET_TRACE_INFO_REPLY, &reply)
}

static mut KERNEL_TRACE_SERVICE: TraceService = TraceService::new();

pub fn handle_envelope(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    if env.protocol == PROTOCOL_TRACE_SERVICE_V1 {
        // SAFETY: KERNEL_TRACE_SERVICE is only accessed from the kernel IPC path.
        return unsafe { handle_trace_service_envelope(writer, &mut KERNEL_TRACE_SERVICE, env) };
    }

    // V-05/V-06: Validate handle before dispatching control operations
    if !validate_handle(table, env) {
        // V-001: Return proper error reply for SHMEM_CONTROL messages
        if env.protocol == PROTOCOL_SHMEM_CONTROL {
            return shmem_error_reply(env, shmem::STATUS_INVALID_CAPABILITY);
        }

        // For other protocols, return empty envelope with request msg_type
        return Envelope::empty(env.protocol, env.msg_type);
    }

    match env.protocol {
        PROTOCOL_PING => {
            if env.msg_type != MSG_PING {
                return Envelope::empty(env.protocol, env.msg_type);
            }

            let ping = match read_payload::<Ping>(env) {
                Ok(ping) => ping,
                Err(_) => return Envelope::empty(PROTOCOL_PING, MSG_PONG),
            };
            trace_ring::emit(writer, TAG_IPC, ping.nonce, 0);

            let pong = Pong { nonce: ping.nonce };
            let mut out = Envelope::empty(PROTOCOL_PING, MSG_PONG);
            if write_payload(&mut out, &pong).is_err() {
                return Envelope::empty(PROTOCOL_PING, MSG_PONG);
            }
            out
        }
        PROTOCOL_SHMEM_CONTROL => handle_shmem_control(writer, table, shmem_table, env),
        #[cfg(feature = "test_protocols")]
        kernel_api::generated::NET_V1_PROTOCOL_ID => {
            crate::net_harness::handle_net_v1(writer, shmem_table, env)
        }
        #[cfg(feature = "test_protocols")]
        kernel_api::generated::BLOCK_V1_PROTOCOL_ID => {
            crate::block_harness::handle_block_v1(writer, shmem_table, env)
        }
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

/// Handle PROTOCOL_TRACE_SERVICE_V1 messages.
///
/// This function is separate from handle_envelope to avoid changing its signature.
/// It can be called directly when trace service integration is needed.
pub fn handle_trace_service_envelope(
    writer: &trace_ring::TraceWriter,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    // For trace service, we validate the trace capability directly
    if env.msg_type == MSG_CREATE_TRACE_BUFFER {
        // CREATE_TRACE_BUFFER doesn't require a valid trace capability
        // It creates a new capability
        return handle_create_trace_buffer(writer, trace_service, env);
    }

    // For other operations, validate the trace capability
    let trace_cap = match env.msg_type {
        MSG_DESTROY_TRACE_BUFFER | MSG_READ_TRACE | MSG_GET_TRACE_INFO => {
            match read_payload::<DestroyTraceBuffer>(env) {
                Ok(req) => Some(Handle::unpack(req.trace_cap)),
                Err(_) => None,
            }
        }
        _ => None,
    };

    // Validate trace capability
    if let Some(cap) = trace_cap {
        if cap.kind != kernel_api::cap::HandleKind::Trace {
            return Envelope::empty(env.protocol, env.msg_type);
        }
    } else {
        return Envelope::empty(env.protocol, env.msg_type);
    }

    handle_trace_service(writer, trace_service, env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};
    use kernel_api::generated::{CloseRegion, CreateRegion, MapRegion, UnmapRegion};

    fn setup_test_mm() {
        static INIT: AtomicBool = AtomicBool::new(false);
        if INIT.load(Ordering::SeqCst) {
            return;
        }
        let base = crate::mm::PhysFrame::from_frame_number(0x2000);
        *crate::mm::FRAME_ALLOCATOR.lock() = Some(crate::mm::BitmapAllocator::new(base, 2048));

        let mut table = crate::mm::AddressSpaceTable::new();
        unsafe {
            table.init_kernel(crate::mm::PhysAddr::new(0x5000));
        }
        for domain_id in [1u64, 2, 3] {
            let domain_root = unsafe { crate::mm::PhysAddr::new(0x10000 + (domain_id * 0x1000)) };
            table.set_root(domain_id as crate::domain_registry::DomainId, domain_root);
        }
        *crate::mm::ADDRESS_SPACE_TABLE.lock() = Some(table);
        INIT.store(true, Ordering::SeqCst);
    }

    #[test]
    fn validate_handle_rejects_invalid_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let table = StaticCapTable::new();
        // V-001: Use SHMEM_CONTROL protocol to test INVALID handle rejection
        // (PING protocol is exempt from capability requirements)
        let env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        assert!(!validate_handle(&table, &env));
    }

    #[test]
    fn validate_handle_accepts_current_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        // V-001: Use SHMEM_CONTROL protocol to test valid handle acceptance
        // (PING protocol is exempt from capability requirements)
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = handle;
        assert!(validate_handle(&table, &env));
    }

    #[test]
    fn validate_handle_rejects_stale_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        table.deallocate(handle).unwrap();
        // V-001: Use SHMEM_CONTROL protocol to test stale handle rejection
        // (PING protocol is exempt from capability requirements)
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = handle;
        assert!(!validate_handle(&table, &env));
    }

    #[test]
    fn handle_create_region_succeeds_with_valid_params() {
        setup_test_mm();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();

        // Reset trace ring for test
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        let req = CreateRegion {
            request_id: 1,
            owner_domain_id: 100,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 4096,
        };

        // V-001: Allocate valid capability for CREATE_REGION
        let cap = cap_table.allocate_for_domain(100).unwrap();
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = cap;
        write_payload(&mut env, &req).unwrap();

        let reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &env);

        assert_eq!(reply_env.protocol, PROTOCOL_SHMEM_CONTROL);
        assert_eq!(reply_env.msg_type, MSG_CREATE_REGION_REPLY);

        let reply: CreateRegionReply = read_payload(&reply_env).unwrap();
        assert_eq!(reply.request_id, 1);
        assert_eq!(reply.status, shmem::STATUS_OK);
        assert_ne!(reply.region_id, 0);
        assert_ne!(reply.shm_cap, 0);
    }

    #[test]
    fn handle_create_region_rejects_invalid_page_size() {
        setup_test_mm();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        let req = CreateRegion {
            request_id: 2,
            owner_domain_id: 100,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 100, // Not power of two
        };

        // V-001: Use valid capability instead of INVALID
        let cap = cap_table.allocate_for_domain(100).unwrap();
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = cap;
        write_payload(&mut env, &req).unwrap();

        let reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &env);

        assert_eq!(reply_env.protocol, PROTOCOL_SHMEM_CONTROL);
        assert_eq!(reply_env.msg_type, MSG_CREATE_REGION_REPLY);

        let reply: CreateRegionReply = read_payload(&reply_env).unwrap();
        assert_eq!(reply.status, shmem::STATUS_INVALID_PAGE_SIZE);
    }

    #[test]
    fn handle_map_region_without_capability_fails() {
        setup_test_mm();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        // V-001: Create region with valid capability (CREATE_REGION no longer accepts INVALID)
        let create_req = CreateRegion {
            request_id: 1,
            owner_domain_id: 100,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
            page_size: 4096,
        };

        // Allocate valid capability for CREATE_REGION
        let create_cap = cap_table.allocate_for_domain(100).unwrap();
        let mut create_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        create_env.handle = create_cap;
        write_payload(&mut create_env, &create_req).unwrap();

        let create_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &create_env);
        let create_reply: CreateRegionReply = read_payload(&create_reply_env).unwrap();

        // Try to map without the capability (use INVALID handle)
        let map_req = MapRegion {
            request_id: 2,
            caller_domain_id: 100, // Owner of the region
            region_id: create_reply.region_id,
            target_domain_id: 200,
            shm_cap: 0, // Invalid capability
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };

        let mut map_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION);
        map_env.handle = Handle::INVALID; // No capability envelope
        write_payload(&mut map_env, &map_req).unwrap();

        let map_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &map_env);

        // Should fail with invalid capability
        assert_eq!(map_reply_env.protocol, PROTOCOL_SHMEM_CONTROL);
        assert_eq!(map_reply_env.msg_type, MSG_MAP_REGION_REPLY);

        let map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();
        assert_eq!(map_reply.status, shmem::STATUS_INVALID_CAPABILITY);
    }

    #[test]
    fn handle_map_and_unmap_region_succeeds_with_capability() {
        setup_test_mm();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        // Create a region
        let create_req = CreateRegion {
            request_id: 1,
            owner_domain_id: 100,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 4096,
        };

        // V-001: Allocate valid capability for CREATE_REGION
        let create_cap = cap_table.allocate_for_domain(100).unwrap();
        let mut create_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        create_env.handle = create_cap;
        write_payload(&mut create_env, &create_req).unwrap();

        let create_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &create_env);
        let create_reply: CreateRegionReply = read_payload(&create_reply_env).unwrap();

        // Allocate a valid capability for the map operation
        let map_cap = cap_table.allocate_for_domain(100).unwrap();

        // Map the region
        let map_req = MapRegion {
            request_id: 2,
            caller_domain_id: 100, // Owner of the region
            region_id: create_reply.region_id,
            target_domain_id: 1,
            shm_cap: create_reply.shm_cap,
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };

        let mut map_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION);
        map_env.handle = map_cap; // Use valid capability for envelope
        write_payload(&mut map_env, &map_req).unwrap();

        let map_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &map_env);

        let map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();
        assert_eq!(map_reply.status, shmem::STATUS_OK);
        assert_ne!(map_reply.mapping_id, 0);

        // Unmap the region
        let unmap_req = UnmapRegion {
            request_id: 3,
            caller_domain_id: 100, // P1 fix: owner must unmap
            mapping_id: map_reply.mapping_id,
            target_domain_id: 1,
        };

        let mut unmap_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION);
        unmap_env.handle = map_cap; // Use same capability
        write_payload(&mut unmap_env, &unmap_req).unwrap();

        let unmap_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &unmap_env);

        let unmap_reply: UnmapRegionReply = read_payload(&unmap_reply_env).unwrap();
        assert_eq!(unmap_reply.status, shmem::STATUS_OK);
    }

    #[test]
    fn handle_close_region_fails_with_active_mappings() {
        setup_test_mm();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        // Create a region
        let create_req = CreateRegion {
            request_id: 1,
            owner_domain_id: 100,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 4096,
        };

        // V-001: Allocate valid capability for CREATE_REGION
        let create_cap = cap_table.allocate_for_domain(100).unwrap();
        let mut create_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        create_env.handle = create_cap;
        write_payload(&mut create_env, &create_req).unwrap();

        let create_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &create_env);
        let create_reply: CreateRegionReply = read_payload(&create_reply_env).unwrap();

        // Allocate capabilities for map and close operations
        let map_cap = cap_table.allocate_for_domain(100).unwrap();
        let close_cap = cap_table.allocate_for_domain(100).unwrap();

        // Map the region
        let map_req = MapRegion {
            request_id: 2,
            caller_domain_id: 100, // Owner of the region
            region_id: create_reply.region_id,
            target_domain_id: 1,
            shm_cap: create_reply.shm_cap,
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };

        let mut map_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION);
        map_env.handle = map_cap;
        write_payload(&mut map_env, &map_req).unwrap();

        let map_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &map_env);
        let _map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();

        // Try to close while mapping is active
        let close_req = CloseRegion {
            request_id: 3,
            caller_domain_id: 100, // P1 fix: owner must close
            region_id: create_reply.region_id,
        };

        let mut close_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION);
        close_env.handle = close_cap;
        write_payload(&mut close_env, &close_req).unwrap();

        let close_reply_env = handle_envelope(&writer, &cap_table, &mut shmem_table, &close_env);

        let close_reply: CloseRegionReply = read_payload(&close_reply_env).unwrap();
        assert_eq!(close_reply.status, shmem::STATUS_REGION_IN_USE);
    }

    #[test]
    #[cfg(feature = "test_protocols")]
    fn ping_protocol_still_works() {
        let cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        let ping = Ping { nonce: 42 };
        let mut env = Envelope::empty(PROTOCOL_PING, MSG_PING);
        write_payload(&mut env, &ping).unwrap();

        let reply = handle_envelope(&writer, &cap_table, &mut shmem_table, &env);

        assert_eq!(reply.protocol, PROTOCOL_PING);
        assert_eq!(reply.msg_type, MSG_PONG);

        let pong: Pong = read_payload(&reply).unwrap();
        assert_eq!(pong.nonce, 42);
    }

    // Additional hardening: Test that PING protocol requires capabilities in production
    #[test]
    #[cfg(not(feature = "test_protocols"))]
    fn ping_protocol_rejects_invalid_handle_without_feature() {
        let cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        let ping = Ping { nonce: 42 };
        let mut env = Envelope::empty(PROTOCOL_PING, MSG_PING);
        // Default handle is INVALID
        write_payload(&mut env, &ping).unwrap();

        let reply = handle_envelope(&writer, &cap_table, &mut shmem_table, &env);

        // Should return empty envelope (rejected) without test_protocols feature
        assert_eq!(reply.protocol, PROTOCOL_PING);
        assert_eq!(reply.msg_type, MSG_PING); // Not MSG_PONG, request rejected
        assert_eq!(reply.payload_len, 0);
    }
}
