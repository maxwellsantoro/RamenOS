#[cfg(test)]
use kernel_api::cap::CapTable;
use kernel_api::cap::Handle;
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

/// Authenticated caller namespace supplied out of band from request bytes.
///
/// There is no untrusted syscall/transport integration yet. A future entrypoint
/// must obtain this context from authenticated execution state, never a requested
/// target domain or a numeric handle copied from an envelope.
pub(crate) struct CallerContext {
    domain: u64,
}

impl CallerContext {
    /// Construct a context for trusted boot and in-kernel test fixtures only.
    ///
    /// # Safety
    /// The caller must independently know the fixture's executing domain. Never
    /// derive `domain` from an envelope, payload, or requested target identity.
    pub(crate) unsafe fn for_trusted_kernel_fixture(domain: u64) -> Self {
        Self { domain }
    }
}

#[cfg(test)]
fn fixture_caller(domain: u64) -> CallerContext {
    // SAFETY: test callers name their independently configured fixture domain.
    unsafe { CallerContext::for_trusted_kernel_fixture(domain) }
}

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
/// Test providers use the same admission checks as ordinary IPC. Enabling an
/// Oracle-vector provider does not grant authority to call it.
fn validate_handle(caller: &CallerContext, table: &StaticCapTable, env: &Envelope) -> bool {
    // V-001: Reject INVALID handles for all protocols (no capability bypass)
    if env.handle == Handle::INVALID {
        return false;
    }
    // V-16/SC-13: Reject handles with wrong kind
    if env.handle.kind != kernel_api::cap::HandleKind::Ipc {
        return false;
    }
    table.domain_for(env.handle) == Some(caller.domain)
}

/// Handle PROTOCOL_SHMEM_CONTROL messages.
fn handle_shmem_control(
    caller: &CallerContext,
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
            if !validate_handle(caller, table, env) {
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
            if !validate_handle(caller, table, env) {
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
            if !validate_handle(caller, table, env) {
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
    table: &StaticCapTable,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    match env.msg_type {
        MSG_CREATE_TRACE_BUFFER => handle_create_trace_buffer(writer, table, trace_service, env),
        MSG_DESTROY_TRACE_BUFFER => handle_destroy_trace_buffer(writer, trace_service, env),
        MSG_READ_TRACE => handle_read_trace(writer, trace_service, env),
        MSG_GET_TRACE_INFO => handle_get_trace_info(writer, trace_service, env),
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

fn handle_create_trace_buffer(
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
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

    // Creation issues a new bearer capability, so it requires live IPC authority
    // bound to the requested domain before consulting or mutating trace state.
    let authority_status = match table.domain_for(env.handle) {
        None => crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
        Some(domain_id) if domain_id != req.domain_id => {
            crate::trace_service::TRACE_STATUS_PERMISSION_DENIED
        }
        Some(_) => crate::trace_service::TRACE_STATUS_OK,
    };
    if authority_status != crate::trace_service::TRACE_STATUS_OK {
        return write_reply_or_error(
            PROTOCOL_TRACE_SERVICE_V1,
            MSG_CREATE_TRACE_BUFFER_REPLY,
            &CreateTraceBufferReply {
                request_id: req.request_id,
                status: authority_status,
                trace_cap: 0,
            },
        );
    }

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

    // The v1 reply carries metadata only and has no data-plane destination.
    // A nonzero read cannot prove delivery, so it must never consume events.
    let (status, bytes_read) = if req.length != 0 {
        (crate::trace_service::TRACE_STATUS_INVALID_SIZE, 0)
    } else {
        trace_service
            .read_trace(trace_cap, req.offset, 0, &mut [])
            .unwrap_or((crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY, 0))
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

pub(crate) fn handle_envelope(
    caller: &CallerContext,
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    shmem_table: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    if env.protocol == kernel_api::generated::SHMEM_CONSENT_V2_PROTOCOL_ID {
        return handle_region_consent(caller, table, shmem_table, env);
    }
    if env.protocol == kernel_api::generated::TRACE_DELIVERY_V3_PROTOCOL_ID {
        // SAFETY: this trusted IPC path serializes access to the kernel service.
        return unsafe {
            handle_trace_delivery_envelope(
                caller,
                writer,
                table,
                shmem_table,
                &mut KERNEL_TRACE_SERVICE,
                env,
            )
        };
    }
    if env.protocol == PROTOCOL_TRACE_SERVICE_V1 {
        // SAFETY: KERNEL_TRACE_SERVICE is only accessed from the kernel IPC path.
        return unsafe {
            handle_trace_service_envelope(caller, writer, table, &mut KERNEL_TRACE_SERVICE, env)
        };
    }

    // V-05/V-06: Validate handle before dispatching control operations
    if !validate_handle(caller, table, env) {
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
        PROTOCOL_SHMEM_CONTROL => handle_shmem_control(caller, writer, table, shmem_table, env),
        #[cfg(feature = "test_protocols")]
        kernel_api::generated::NET_V1_PROTOCOL_ID => {
            crate::net_harness::handle_net_v1(caller.domain, writer, shmem_table, env)
        }
        #[cfg(feature = "test_protocols")]
        kernel_api::generated::BLOCK_V1_PROTOCOL_ID => {
            crate::block_harness::handle_block_v1(caller.domain, writer, shmem_table, env)
        }
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

/// Versioned offer/accept control plane. Execution identity is never a wire field.
fn handle_region_consent(
    caller: &CallerContext,
    caps: &StaticCapTable,
    regions: &mut shmem::ShmemRegionTable,
    env: &Envelope,
) -> Envelope {
    use kernel_api::generated::{
        AcceptRegion, AcceptRegionReply, CancelRegionOffer, CancelRegionOfferReply, OfferRegion,
        OfferRegionReply,
    };
    let authorized = validate_handle(caller, caps, env);
    let header_status = |version: u32, reserved: u32| {
        if !authorized {
            shmem::STATUS_INVALID_CAPABILITY
        } else if version != 2 || reserved != 0 {
            shmem::STATUS_INVALID_SIZE
        } else {
            shmem::STATUS_OK
        }
    };
    match env.msg_type {
        1 => {
            let mut reply = OfferRegionReply {
                request_id: 0,
                offer_id: 0,
                status: shmem::STATUS_INVALID_SIZE,
                reserved: 0,
            };
            if let Ok(req) = read_payload::<OfferRegion>(env) {
                reply.request_id = req.request_id;
                reply.status = header_status(req.version, req.reserved);
                if reply.status == 0 {
                    match regions.offer_region(
                        req.region_id,
                        Handle::unpack(req.shm_cap),
                        caller.domain,
                        req.target_domain_id,
                        req.rights,
                        req.cache_mode,
                    ) {
                        Ok(id) => reply.offer_id = id,
                        Err(status) => reply.status = status,
                    }
                }
            }
            write_reply_or_error(env.protocol, 2, &reply)
        }
        3 => {
            let mut reply = AcceptRegionReply {
                request_id: 0,
                mapping_id: 0,
                status: shmem::STATUS_INVALID_SIZE,
                reserved: 0,
            };
            if let Ok(req) = read_payload::<AcceptRegion>(env) {
                reply.request_id = req.request_id;
                reply.status = header_status(req.version, req.reserved);
                if reply.status == 0 {
                    match regions.accept_region(caller.domain, req.offer_id) {
                        Ok(id) => reply.mapping_id = id,
                        Err(status) => reply.status = status,
                    }
                }
            }
            write_reply_or_error(env.protocol, 4, &reply)
        }
        5 => {
            let mut reply = CancelRegionOfferReply {
                request_id: 0,
                status: shmem::STATUS_INVALID_SIZE,
                reserved: 0,
            };
            if let Ok(req) = read_payload::<CancelRegionOffer>(env) {
                reply.request_id = req.request_id;
                reply.status = header_status(req.version, req.reserved);
                if reply.status == 0 {
                    reply.status = regions
                        .cancel_region_offer(caller.domain, req.offer_id)
                        .err()
                        .unwrap_or(shmem::STATUS_OK);
                }
            }
            write_reply_or_error(env.protocol, 6, &reply)
        }
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

/// Deliver through a caller-owned writable Shmem range, then consume only after
/// explicit acknowledgement. Admission is identical for direct and main entry.
pub(crate) fn handle_trace_delivery_envelope(
    caller: &CallerContext,
    _writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    regions: &mut shmem::ShmemRegionTable,
    service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    use crate::trace_service::{TRACE_STATUS_INVALID_CAPABILITY, TRACE_STATUS_INVALID_SIZE};
    use kernel_api::generated::{
        CommitTraceRead, CommitTraceReadReply, ReadTraceInto, ReadTraceIntoReply,
    };
    if env.protocol != kernel_api::generated::TRACE_DELIVERY_V3_PROTOCOL_ID {
        return Envelope::empty(env.protocol, env.msg_type);
    }
    match env.msg_type {
        1 => {
            let mut reply = ReadTraceIntoReply {
                request_id: 0,
                lease_id: 0,
                next_offset: 0,
                status: TRACE_STATUS_INVALID_SIZE,
                data_len: 0,
            };
            if let Ok(req) = read_payload::<ReadTraceInto>(env) {
                reply.request_id = req.request_id;
                let source = Handle::unpack(req.trace_cap);
                let destination = Handle::unpack(req.destination_shm_cap);
                let result = (|| {
                    if !validate_handle(caller, table, env)
                        || service.cap_domain(source) != Some(caller.domain)
                    {
                        return Err(TRACE_STATUS_INVALID_CAPABILITY);
                    }
                    if req.version != 3
                        || req.reserved != 0
                        || req.length > 1536
                        || !req.length.is_multiple_of(24)
                    {
                        return Err(TRACE_STATUS_INVALID_SIZE);
                    }
                    // Validate all destination authority and bounds before any
                    // trace cursor/event inspection, including for no-op reads.
                    if req.length == 0 {
                        if regions
                            .writable_phys_range_for(destination, caller.domain, 0, 1)
                            .is_none()
                            || regions
                                .region_size_for_cap(destination)
                                .is_none_or(|size| req.destination_offset > size)
                        {
                            return Err(TRACE_STATUS_INVALID_CAPABILITY);
                        }
                        return service.prepare_delivery(
                            source,
                            destination,
                            req.destination_offset,
                            req.trace_offset,
                            0,
                            &mut [],
                        );
                    }
                    let physical = regions
                        .writable_phys_range_for(
                            destination,
                            caller.domain,
                            req.destination_offset,
                            req.length as usize,
                        )
                        .ok_or(TRACE_STATUS_INVALID_CAPABILITY)?;
                    // SAFETY: the exact live, contiguous writable range belongs
                    // to this admitted caller. This mutable table remains held
                    // for the entire copy and receipt installation; no borrowed
                    // physical view escapes and close/reuse cannot interleave.
                    let bytes = unsafe {
                        core::slice::from_raw_parts_mut(physical as *mut u8, req.length as usize)
                    };
                    service.prepare_delivery(
                        source,
                        destination,
                        req.destination_offset,
                        req.trace_offset,
                        req.length,
                        bytes,
                    )
                })();
                match result {
                    Ok((lease, next, len)) => {
                        reply.status = 0;
                        reply.lease_id = lease;
                        reply.next_offset = next;
                        reply.data_len = len;
                    }
                    Err(status) => reply.status = status,
                }
            }
            write_reply_or_error(env.protocol, 2, &reply)
        }
        3 => {
            let mut reply = CommitTraceReadReply {
                request_id: 0,
                next_offset: 0,
                status: TRACE_STATUS_INVALID_SIZE,
                reserved: 0,
            };
            if let Ok(req) = read_payload::<CommitTraceRead>(env) {
                reply.request_id = req.request_id;
                let source = Handle::unpack(req.trace_cap);
                let result = (|| {
                    if !validate_handle(caller, table, env)
                        || service.cap_domain(source) != Some(caller.domain)
                    {
                        return Err(TRACE_STATUS_INVALID_CAPABILITY);
                    }
                    if req.version != 3 || req.reserved != 0 {
                        return Err(TRACE_STATUS_INVALID_SIZE);
                    }
                    match service.delivery_receipt(source, req.lease_id)? {
                        crate::trace_service::TraceDeliveryReceipt::Committed(next) => Ok(next),
                        crate::trace_service::TraceDeliveryReceipt::Pending(receipt) => {
                            regions
                                .writable_phys_range_for(
                                    receipt.destination,
                                    caller.domain,
                                    receipt.destination_offset,
                                    receipt.data_len as usize,
                                )
                                .ok_or(TRACE_STATUS_INVALID_CAPABILITY)?;
                            service.commit_delivery(source, req.lease_id)
                        }
                    }
                })();
                match result {
                    Ok(next) => {
                        reply.status = 0;
                        reply.next_offset = next;
                    }
                    Err(status) => reply.status = status,
                }
            }
            write_reply_or_error(env.protocol, 4, &reply)
        }
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

/// Dispatch trace operations only with live IPC and trace authority in the
/// authenticated caller's namespace. Payload handles cannot select a namespace.
pub(crate) fn handle_trace_service_envelope(
    caller: &CallerContext,
    writer: &trace_ring::TraceWriter,
    table: &StaticCapTable,
    trace_service: &mut TraceService,
    env: &Envelope,
) -> Envelope {
    if env.protocol != PROTOCOL_TRACE_SERVICE_V1 {
        return Envelope::empty(env.protocol, env.msg_type);
    }
    if !validate_handle(caller, table, env) {
        return trace_authority_error(env);
    }
    if env.msg_type == MSG_CREATE_TRACE_BUFFER {
        return handle_create_trace_buffer(writer, table, trace_service, env);
    }

    let trace_cap = match env.msg_type {
        MSG_DESTROY_TRACE_BUFFER => read_payload::<DestroyTraceBuffer>(env)
            .ok()
            .map(|req| Handle::unpack(req.trace_cap)),
        MSG_READ_TRACE => read_payload::<ReadTrace>(env)
            .ok()
            .map(|req| Handle::unpack(req.trace_cap)),
        MSG_GET_TRACE_INFO => read_payload::<GetTraceInfo>(env)
            .ok()
            .map(|req| Handle::unpack(req.trace_cap)),
        _ => None,
    };
    // Reject foreign handles before buffer state, cursors, destruction, or traces.
    if trace_cap.and_then(|cap| trace_service.cap_domain(cap)) != Some(caller.domain) {
        return trace_authority_error(env);
    }
    handle_trace_service(writer, table, trace_service, env)
}

fn trace_authority_error(env: &Envelope) -> Envelope {
    let status = crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY;
    match env.msg_type {
        MSG_CREATE_TRACE_BUFFER => write_reply_or_error(
            env.protocol,
            MSG_CREATE_TRACE_BUFFER_REPLY,
            &CreateTraceBufferReply {
                request_id: read_payload::<CreateTraceBuffer>(env).map_or(0, |r| r.request_id),
                status,
                trace_cap: 0,
            },
        ),
        MSG_DESTROY_TRACE_BUFFER => write_reply_or_error(
            env.protocol,
            MSG_DESTROY_TRACE_BUFFER_REPLY,
            &DestroyTraceBufferReply {
                request_id: read_payload::<DestroyTraceBuffer>(env).map_or(0, |r| r.request_id),
                status,
            },
        ),
        MSG_READ_TRACE => write_reply_or_error(
            env.protocol,
            MSG_READ_TRACE_REPLY,
            &ReadTraceReply {
                request_id: read_payload::<ReadTrace>(env).map_or(0, |r| r.request_id),
                status,
                data_len: 0,
            },
        ),
        MSG_GET_TRACE_INFO => write_reply_or_error(
            env.protocol,
            MSG_GET_TRACE_INFO_REPLY,
            &GetTraceInfoReply {
                request_id: read_payload::<GetTraceInfo>(env).map_or(0, |r| r.request_id),
                status,
                domain_id: 0,
                size: 0,
                read_offset: 0,
                write_offset: 0,
            },
        ),
        _ => Envelope::empty(env.protocol, env.msg_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::generated::{CloseRegion, CreateRegion, MapRegion, UnmapRegion};

    #[test]
    fn validate_handle_rejects_invalid_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let table = StaticCapTable::new();
        // V-001: SHMEM_CONTROL rejects INVALID handles.
        let env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        assert!(!validate_handle(&fixture_caller(0), &table, &env));
    }

    #[test]
    fn validate_handle_accepts_current_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        // V-001: SHMEM_CONTROL accepts live IPC handles.
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = handle;
        assert!(validate_handle(&fixture_caller(0), &table, &env));
    }

    #[test]
    fn validate_handle_rejects_stale_handle() {
        crate::cap_table::reset_smp_state_for_test();
        let mut table = StaticCapTable::new();
        let handle = table.allocate().unwrap();
        table.deallocate(handle).unwrap();
        // V-001: Use SHMEM_CONTROL protocol to test stale handle rejection
        let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        env.handle = handle;
        assert!(!validate_handle(&fixture_caller(0), &table, &env));
    }

    #[test]
    fn handle_create_region_succeeds_with_valid_params() {
        let _memory_guard = crate::mm::test_support::setup();
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

        let reply_env = handle_envelope(
            &fixture_caller(100),
            &writer,
            &cap_table,
            &mut shmem_table,
            &env,
        );

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
        let _memory_guard = crate::mm::test_support::setup();
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

        let reply_env = handle_envelope(
            &fixture_caller(100),
            &writer,
            &cap_table,
            &mut shmem_table,
            &env,
        );

        assert_eq!(reply_env.protocol, PROTOCOL_SHMEM_CONTROL);
        assert_eq!(reply_env.msg_type, MSG_CREATE_REGION_REPLY);

        let reply: CreateRegionReply = read_payload(&reply_env).unwrap();
        assert_eq!(reply.status, shmem::STATUS_INVALID_PAGE_SIZE);
    }

    #[test]
    fn handle_map_region_without_capability_fails() {
        let _memory_guard = crate::mm::test_support::setup();
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

        let create_reply_env = handle_envelope(
            &fixture_caller(100),
            &writer,
            &cap_table,
            &mut shmem_table,
            &create_env,
        );
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

        let map_reply_env = handle_envelope(
            &fixture_caller(100),
            &writer,
            &cap_table,
            &mut shmem_table,
            &map_env,
        );

        // Should fail with invalid capability
        assert_eq!(map_reply_env.protocol, PROTOCOL_SHMEM_CONTROL);
        assert_eq!(map_reply_env.msg_type, MSG_MAP_REGION_REPLY);

        let map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();
        assert_eq!(map_reply.status, shmem::STATUS_INVALID_CAPABILITY);
    }

    #[test]
    fn handle_map_and_unmap_region_succeeds_with_capability() {
        let _memory_guard = crate::mm::test_support::setup();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        // Create a region
        let create_req = CreateRegion {
            request_id: 1,
            owner_domain_id: 1,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 4096,
        };

        // V-001: Allocate valid capability for CREATE_REGION
        let create_cap = cap_table.allocate_for_domain(1).unwrap();
        let mut create_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        create_env.handle = create_cap;
        write_payload(&mut create_env, &create_req).unwrap();

        let create_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &create_env,
        );
        let create_reply: CreateRegionReply = read_payload(&create_reply_env).unwrap();

        // Allocate a valid capability for the map operation
        let map_cap = cap_table.allocate_for_domain(1).unwrap();

        // Map the region
        let map_req = MapRegion {
            request_id: 2,
            caller_domain_id: 1, // Owner of the region
            region_id: create_reply.region_id,
            target_domain_id: 1,
            shm_cap: create_reply.shm_cap,
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };

        let mut map_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION);
        map_env.handle = map_cap; // Use valid capability for envelope
        write_payload(&mut map_env, &map_req).unwrap();

        let map_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &map_env,
        );

        let map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();
        assert_eq!(map_reply.status, shmem::STATUS_OK);
        assert_ne!(map_reply.mapping_id, 0);

        // Unmap the region
        let unmap_req = UnmapRegion {
            request_id: 3,
            caller_domain_id: 1, // P1 fix: owner must unmap
            mapping_id: map_reply.mapping_id,
            target_domain_id: 1,
        };

        let mut unmap_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_UNMAP_REGION);
        unmap_env.handle = map_cap; // Use same capability
        write_payload(&mut unmap_env, &unmap_req).unwrap();

        let unmap_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &unmap_env,
        );

        let unmap_reply: UnmapRegionReply = read_payload(&unmap_reply_env).unwrap();
        assert_eq!(unmap_reply.status, shmem::STATUS_OK);
    }

    #[test]
    fn handle_close_region_fails_with_active_mappings() {
        let _memory_guard = crate::mm::test_support::setup();
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        // Create a region
        let create_req = CreateRegion {
            request_id: 1,
            owner_domain_id: 1,
            size_bytes: 4096,
            flags: shmem::REGION_FLAG_READABLE,
            page_size: 4096,
        };

        // V-001: Allocate valid capability for CREATE_REGION
        let create_cap = cap_table.allocate_for_domain(1).unwrap();
        let mut create_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
        create_env.handle = create_cap;
        write_payload(&mut create_env, &create_req).unwrap();

        let create_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &create_env,
        );
        let create_reply: CreateRegionReply = read_payload(&create_reply_env).unwrap();

        // Allocate capabilities for map and close operations
        let map_cap = cap_table.allocate_for_domain(1).unwrap();
        let close_cap = cap_table.allocate_for_domain(1).unwrap();

        // Map the region
        let map_req = MapRegion {
            request_id: 2,
            caller_domain_id: 1, // Owner of the region
            region_id: create_reply.region_id,
            target_domain_id: 1,
            shm_cap: create_reply.shm_cap,
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };

        let mut map_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_MAP_REGION);
        map_env.handle = map_cap;
        write_payload(&mut map_env, &map_req).unwrap();

        let map_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &map_env,
        );
        let _map_reply: MapRegionReply = read_payload(&map_reply_env).unwrap();

        // Try to close while mapping is active
        let close_req = CloseRegion {
            request_id: 3,
            caller_domain_id: 1, // P1 fix: owner must close
            region_id: create_reply.region_id,
        };

        let mut close_env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CLOSE_REGION);
        close_env.handle = close_cap;
        write_payload(&mut close_env, &close_req).unwrap();

        let close_reply_env = handle_envelope(
            &fixture_caller(1),
            &writer,
            &cap_table,
            &mut shmem_table,
            &close_env,
        );

        let close_reply: CloseRegionReply = read_payload(&close_reply_env).unwrap();
        assert_eq!(close_reply.status, shmem::STATUS_REGION_IN_USE);
    }

    #[test]
    fn ping_protocol_still_works() {
        crate::cap_table::reset_smp_state_for_test();
        let mut cap_table = StaticCapTable::new();
        let mut shmem_table = shmem::ShmemRegionTable::new();
        let _guard = trace_ring::reset_for_test();
        let writer = trace_ring::TraceWriter::claim().expect("writer claim");

        let ping = Ping { nonce: 42 };
        let mut env = Envelope::empty(PROTOCOL_PING, MSG_PING);
        env.handle = cap_table.allocate_for_domain(0).unwrap();
        write_payload(&mut env, &ping).unwrap();

        let reply = handle_envelope(
            &fixture_caller(0),
            &writer,
            &cap_table,
            &mut shmem_table,
            &env,
        );

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

        let reply = handle_envelope(
            &fixture_caller(0),
            &writer,
            &cap_table,
            &mut shmem_table,
            &env,
        );

        // Should return empty envelope (rejected) without test_protocols feature
        assert_eq!(reply.protocol, PROTOCOL_PING);
        assert_eq!(reply.msg_type, MSG_PING); // Not MSG_PONG, request rejected
        assert_eq!(reply.payload_len, 0);
    }
}

#[cfg(test)]
mod review_trace_consumer {
    use super::*;
    #[test]
    fn review_trace_ipc_typed_read_and_metadata_preserve_denied_work() {
        let _guard = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut table = StaticCapTable::new();
        let ipc = table.allocate_for_domain(0).unwrap();
        let mut service = TraceService::new();
        service.init_kernel();
        let (_, cap) = service.create_trace_buffer(0, 4096).unwrap();
        service.trace_ring_mut().emit(0, 10, 20, 30);
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_READ_TRACE);
        env.handle = ipc;
        write_payload(
            &mut env,
            &ReadTrace {
                request_id: 1,
                trace_cap: cap.pack(),
                offset: 0,
                length: 1,
            },
        )
        .unwrap();
        let denied =
            handle_trace_service_envelope(&fixture_caller(0), &writer, &table, &mut service, &env);
        let reply = read_payload::<ReadTraceReply>(&denied).unwrap();
        assert_eq!(
            (reply.request_id, reply.status, reply.data_len),
            (1, crate::trace_service::TRACE_STATUS_INVALID_SIZE, 0)
        );
        let mut info = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_GET_TRACE_INFO);
        info.handle = ipc;
        write_payload(
            &mut info,
            &GetTraceInfo {
                request_id: 2,
                trace_cap: cap.pack(),
            },
        )
        .unwrap();
        let reply =
            handle_trace_service_envelope(&fixture_caller(0), &writer, &table, &mut service, &info);
        let reply = read_payload::<GetTraceInfoReply>(&reply).unwrap();
        assert_eq!((reply.read_offset, reply.write_offset), (0, 24));
        let mut foreign = env;
        foreign.protocol = PROTOCOL_SHMEM_CONTROL;
        assert_eq!(
            handle_trace_service_envelope(
                &fixture_caller(0),
                &writer,
                &table,
                &mut service,
                &foreign
            )
            .payload_len,
            0
        );
        write_payload(
            &mut env,
            &ReadTrace {
                request_id: 3,
                trace_cap: cap.pack(),
                offset: 0,
                length: 24,
            },
        )
        .unwrap();
        let success =
            handle_trace_service_envelope(&fixture_caller(0), &writer, &table, &mut service, &env);
        let success = read_payload::<ReadTraceReply>(&success).unwrap();
        assert_eq!(
            (success.status, success.data_len),
            (crate::trace_service::TRACE_STATUS_INVALID_SIZE, 0)
        );
        let reply =
            handle_trace_service_envelope(&fixture_caller(0), &writer, &table, &mut service, &info);
        let reply = read_payload::<GetTraceInfoReply>(&reply).unwrap();
        assert_eq!((reply.read_offset, reply.write_offset), (0, 24));
    }
}

#[cfg(test)]
mod review_authority {
    use super::*;
    use crate::trace_service::{
        TRACE_STATUS_INVALID_CAPABILITY, TRACE_STATUS_OK, TRACE_STATUS_PERMISSION_DENIED,
    };
    use kernel_api::cap::HandleKind;

    #[test]
    fn review_authority_ping_net_block_require_live_ipc_caps() {
        let _guard = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut table = StaticCapTable::new();
        let stale = table.allocate_for_domain(0).unwrap();
        table.deallocate(stale).unwrap();
        let valid = table.allocate_for_domain(0).unwrap();
        let wrong_kind = Handle {
            kind: HandleKind::Trace,
            ..valid
        };
        let mut shmem = shmem::ShmemRegionTable::new();
        for protocol in [
            PROTOCOL_PING,
            kernel_api::generated::NET_V1_PROTOCOL_ID,
            kernel_api::generated::BLOCK_V1_PROTOCOL_ID,
        ] {
            for denied in [Handle::INVALID, wrong_kind, stale] {
                let mut env = Envelope::empty(protocol, MSG_PING);
                env.handle = denied;
                write_payload(&mut env, &Ping { nonce: 42 }).unwrap();
                assert!(
                    !validate_handle(&fixture_caller(0), &table, &env),
                    "protocol {protocol}"
                );
                let reply = handle_envelope(&fixture_caller(0), &writer, &table, &mut shmem, &env);
                assert_eq!((reply.msg_type, reply.payload_len), (env.msg_type, 0));
            }
            let mut authorized = Envelope::empty(protocol, MSG_PING);
            authorized.handle = valid;
            assert!(validate_handle(&fixture_caller(0), &table, &authorized));
        }
        let mut ping = Envelope::empty(PROTOCOL_PING, MSG_PING);
        ping.handle = valid;
        write_payload(&mut ping, &Ping { nonce: 77 }).unwrap();
        let reply = handle_envelope(&fixture_caller(0), &writer, &table, &mut shmem, &ping);
        assert_eq!(read_payload::<Pong>(&reply).unwrap().nonce, 77);
    }

    #[test]
    #[cfg(feature = "test_protocols")]
    fn review_authority_oracle_providers_reject_then_transfer() {
        use kernel_api::block_oracle_vector::{
            S13_ORACLE_BLOCK_COUNT, S13_ORACLE_BLOCK_SIZE, S13_ORACLE_READ_PAYLOAD,
            S13_ORACLE_READ_REQUEST_ID,
        };
        use kernel_api::generated::{
            BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS, MSG_NET_V1_RECEIVE_PACKET,
            NET_V1_PROTOCOL_ID, ReadBlocks, ReadBlocksReply, ReceivePacket, ReceivePacketReply,
        };
        use kernel_api::net_packet_oracle_vector::{
            S11_ORACLE_RECV_PAYLOAD, S11_ORACLE_RECV_REQUEST_ID,
        };
        let _memory = crate::mm::test_support::setup();
        let _trace = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut caps = StaticCapTable::new();
        let owner = caps.allocate_for_domain(0).unwrap();
        let unrelated = caps.allocate_for_domain(1).unwrap();
        let mut regions = shmem::ShmemRegionTable::new();
        let (_, shm, phys) = regions
            .create_region(
                0,
                4096,
                shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
                4096,
            )
            .unwrap();
        unsafe {
            core::ptr::write_bytes(phys as *mut u8, 0xAA, 4096);
        }
        let mut net = Envelope::empty(NET_V1_PROTOCOL_ID, MSG_NET_V1_RECEIVE_PACKET);
        write_payload(
            &mut net,
            &ReceivePacket {
                request_id: S11_ORACLE_RECV_REQUEST_ID,
                buffer_shm_cap: shm.pack(),
                buffer_offset: 0,
                buffer_len: 1500,
            },
        )
        .unwrap();
        let mut block = Envelope::empty(BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS);
        write_payload(
            &mut block,
            &ReadBlocks {
                request_id: S13_ORACLE_READ_REQUEST_ID,
                lba: 0,
                block_count: S13_ORACLE_BLOCK_COUNT,
                block_size: S13_ORACLE_BLOCK_SIZE,
                buffer_shm_cap: shm.pack(),
                buffer_offset: 2048,
            },
        )
        .unwrap();
        for request in [&net, &block] {
            assert_eq!(
                handle_envelope(&fixture_caller(0), &writer, &caps, &mut regions, request)
                    .payload_len,
                0
            );
        }
        assert_eq!(unsafe { *(phys as *const u8) }, 0xAA);
        assert_eq!(unsafe { *((phys + 2048) as *const u8) }, 0xAA);
        net.handle = owner;
        let reply = handle_envelope(&fixture_caller(0), &writer, &caps, &mut regions, &net);
        assert_eq!(
            read_payload::<ReceivePacketReply>(&reply).unwrap().status,
            0
        );
        block.handle = owner;
        let reply = handle_envelope(&fixture_caller(0), &writer, &caps, &mut regions, &block);
        assert_eq!(read_payload::<ReadBlocksReply>(&reply).unwrap().status, 0);
        assert_eq!(
            unsafe {
                core::slice::from_raw_parts(phys as *const u8, S11_ORACLE_RECV_PAYLOAD.len())
            },
            S11_ORACLE_RECV_PAYLOAD
        );
        assert_eq!(
            unsafe {
                core::slice::from_raw_parts(
                    (phys + 2048) as *const u8,
                    S13_ORACLE_READ_PAYLOAD.len(),
                )
            },
            S13_ORACLE_READ_PAYLOAD
        );
        let mut ping = Envelope::empty(PROTOCOL_PING, MSG_PING);
        ping.handle = unrelated;
        write_payload(&mut ping, &Ping { nonce: 99 }).unwrap();
        let reply = handle_envelope(&fixture_caller(1), &writer, &caps, &mut regions, &ping);
        assert_eq!(read_payload::<Pong>(&reply).unwrap().nonce, 99);
    }

    #[test]
    #[cfg(feature = "test_protocols")]
    fn review_namespace_oracle_payload_caps_deny_then_each_owner_transfers() {
        use kernel_api::block_oracle_vector::{
            S13_ORACLE_BLOCK_COUNT, S13_ORACLE_BLOCK_SIZE, S13_ORACLE_READ_PAYLOAD,
            S13_ORACLE_READ_REQUEST_ID, S13_ORACLE_WRITE_PAYLOAD, S13_ORACLE_WRITE_REQUEST_ID,
        };
        use kernel_api::generated::{
            BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS, MSG_BLOCK_V1_WRITE_BLOCKS,
            MSG_NET_V1_RECEIVE_PACKET, MSG_NET_V1_SEND_PACKET, NET_V1_PROTOCOL_ID, ReadBlocks,
            ReadBlocksReply, ReceivePacket, ReceivePacketReply, SendPacket, SendPacketReply,
            WriteBlocks, WriteBlocksReply,
        };
        use kernel_api::net_packet_oracle_vector::{
            S11_ORACLE_PACKET_LEN, S11_ORACLE_RECV_PAYLOAD, S11_ORACLE_RECV_REQUEST_ID,
            S11_ORACLE_SEND_PAYLOAD, S11_ORACLE_SEND_REQUEST_ID,
        };
        let _memory = crate::mm::test_support::setup();
        let _trace = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut caps = StaticCapTable::new();
        let a = caps.allocate_for_domain(1).unwrap();
        let b = caps.allocate_for_domain(2).unwrap();
        let caller_a = fixture_caller(1);
        let caller_b = fixture_caller(2);
        let mut regions = shmem::ShmemRegionTable::new();
        let (_, shm_a, phys_a) = regions
            .create_region(
                1,
                4096,
                shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
                4096,
            )
            .unwrap();
        let (_, shm_b, phys_b) = regions
            .create_region(
                2,
                4096,
                shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
                4096,
            )
            .unwrap();
        unsafe {
            core::ptr::write_bytes(phys_a as *mut u8, 0xAA, 4096);
            core::ptr::write_bytes(phys_b as *mut u8, 0xBB, 4096);
            core::ptr::copy_nonoverlapping(
                S11_ORACLE_SEND_PAYLOAD.as_ptr(),
                (phys_a + 1536) as *mut u8,
                S11_ORACLE_SEND_PAYLOAD.len(),
            );
            core::ptr::copy_nonoverlapping(
                S13_ORACLE_WRITE_PAYLOAD.as_ptr(),
                (phys_a + 3072) as *mut u8,
                S13_ORACLE_WRITE_PAYLOAD.len(),
            );
        }
        let mut net_recv = Envelope::empty(NET_V1_PROTOCOL_ID, MSG_NET_V1_RECEIVE_PACKET);
        write_payload(
            &mut net_recv,
            &ReceivePacket {
                request_id: S11_ORACLE_RECV_REQUEST_ID,
                buffer_shm_cap: shm_a.pack(),
                buffer_offset: 0,
                buffer_len: 1500,
            },
        )
        .unwrap();
        let mut net_send = Envelope::empty(NET_V1_PROTOCOL_ID, MSG_NET_V1_SEND_PACKET);
        write_payload(
            &mut net_send,
            &SendPacket {
                request_id: S11_ORACLE_SEND_REQUEST_ID,
                data_shm_cap: shm_a.pack(),
                data_offset: 1536,
                data_len: S11_ORACLE_PACKET_LEN,
            },
        )
        .unwrap();
        let mut block_read = Envelope::empty(BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_READ_BLOCKS);
        write_payload(
            &mut block_read,
            &ReadBlocks {
                request_id: S13_ORACLE_READ_REQUEST_ID,
                lba: 0,
                block_count: S13_ORACLE_BLOCK_COUNT,
                block_size: S13_ORACLE_BLOCK_SIZE,
                buffer_shm_cap: shm_a.pack(),
                buffer_offset: 2048,
            },
        )
        .unwrap();
        let mut block_write = Envelope::empty(BLOCK_V1_PROTOCOL_ID, MSG_BLOCK_V1_WRITE_BLOCKS);
        write_payload(
            &mut block_write,
            &WriteBlocks {
                request_id: S13_ORACLE_WRITE_REQUEST_ID,
                lba: 1,
                block_count: S13_ORACLE_BLOCK_COUNT,
                block_size: S13_ORACLE_BLOCK_SIZE,
                data_shm_cap: shm_a.pack(),
                data_offset: 3072,
            },
        )
        .unwrap();
        for request in [
            &mut net_recv,
            &mut net_send,
            &mut block_read,
            &mut block_write,
        ] {
            request.handle = b;
        }
        let denied = handle_envelope(&caller_b, &writer, &caps, &mut regions, &net_recv);
        assert_eq!(
            read_payload::<ReceivePacketReply>(&denied).unwrap().status,
            crate::net_harness::NET_STATUS_SHMEM_ERROR
        );
        let denied = handle_envelope(&caller_b, &writer, &caps, &mut regions, &net_send);
        assert_eq!(
            read_payload::<SendPacketReply>(&denied).unwrap().status,
            crate::net_harness::NET_STATUS_SHMEM_ERROR
        );
        let denied = handle_envelope(&caller_b, &writer, &caps, &mut regions, &block_read);
        assert_eq!(
            read_payload::<ReadBlocksReply>(&denied).unwrap().status,
            crate::block_harness::BLOCK_STATUS_SHMEM_ERROR
        );
        let denied = handle_envelope(&caller_b, &writer, &caps, &mut regions, &block_write);
        assert_eq!(
            read_payload::<WriteBlocksReply>(&denied).unwrap().status,
            crate::block_harness::BLOCK_STATUS_SHMEM_ERROR
        );
        assert!(
            unsafe { core::slice::from_raw_parts(phys_a as *const u8, 1500) }
                .iter()
                .all(|b| *b == 0xAA)
        );
        assert!(
            unsafe { core::slice::from_raw_parts((phys_a + 2048) as *const u8, 512) }
                .iter()
                .all(|b| *b == 0xAA)
        );
        let mut events = [kernel_api::trace::Event {
            tag: 0,
            arg0: 0,
            arg1: 0,
        }; 4];
        assert_eq!(
            trace_ring::read(&mut events),
            0,
            "foreign payload denials must precede trace mutation"
        );
        for request in [
            &mut net_recv,
            &mut net_send,
            &mut block_read,
            &mut block_write,
        ] {
            request.handle = a;
        }
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &net_recv);
        assert_eq!(
            read_payload::<ReceivePacketReply>(&reply).unwrap().status,
            0
        );
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &net_send);
        assert_eq!(read_payload::<SendPacketReply>(&reply).unwrap().status, 0);
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &block_read);
        assert_eq!(read_payload::<ReadBlocksReply>(&reply).unwrap().status, 0);
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &block_write);
        assert_eq!(read_payload::<WriteBlocksReply>(&reply).unwrap().status, 0);
        assert_eq!(
            unsafe {
                core::slice::from_raw_parts(phys_a as *const u8, S11_ORACLE_RECV_PAYLOAD.len())
            },
            S11_ORACLE_RECV_PAYLOAD
        );
        assert_eq!(
            unsafe {
                core::slice::from_raw_parts(
                    (phys_a + 2048) as *const u8,
                    S13_ORACLE_READ_PAYLOAD.len(),
                )
            },
            S13_ORACLE_READ_PAYLOAD
        );
        net_recv.handle = b;
        write_payload(
            &mut net_recv,
            &ReceivePacket {
                request_id: S11_ORACLE_RECV_REQUEST_ID,
                buffer_shm_cap: shm_b.pack(),
                buffer_offset: 0,
                buffer_len: 1500,
            },
        )
        .unwrap();
        let reply = handle_envelope(&caller_b, &writer, &caps, &mut regions, &net_recv);
        assert_eq!(
            read_payload::<ReceivePacketReply>(&reply).unwrap().status,
            0
        );
        assert_eq!(
            unsafe {
                core::slice::from_raw_parts(phys_b as *const u8, S11_ORACLE_RECV_PAYLOAD.len())
            },
            S11_ORACLE_RECV_PAYLOAD
        );
    }

    fn dispatch(
        caller: &CallerContext,
        main_entry: bool,
        writer: &trace_ring::TraceWriter,
        table: &StaticCapTable,
        service: &mut TraceService,
        env: &Envelope,
    ) -> Envelope {
        if main_entry {
            handle_envelope(
                caller,
                writer,
                table,
                &mut shmem::ShmemRegionTable::new(),
                env,
            )
        } else {
            handle_trace_service_envelope(caller, writer, table, service, env)
        }
    }

    fn with_service(
        main_entry: bool,
        service: &mut TraceService,
        f: impl FnOnce(&mut TraceService),
    ) {
        if main_entry {
            // The test holds the global trace test lock for its entire dispatch sequence.
            unsafe { f(&mut KERNEL_TRACE_SERVICE) }
        } else {
            f(service)
        }
    }

    fn trace_creation_requires_domain_authority(main_entry: bool) {
        let _guard = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut service = TraceService::new();
        with_service(main_entry, &mut service, |service| {
            *service = TraceService::new();
            service.init_kernel();
            service.register_domain(1, "unrelated").unwrap();
        });
        let mut table = StaticCapTable::new();
        let stale = table.allocate_for_domain(0).unwrap();
        table.deallocate(stale).unwrap();
        let owner = table.allocate_for_domain(0).unwrap();
        let foreign = table.allocate_for_domain(1).unwrap();
        let wrong_kind = Handle {
            kind: HandleKind::Trace,
            ..owner
        };
        let mut env = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_CREATE_TRACE_BUFFER);
        env.handle = foreign;
        write_payload(
            &mut env,
            &CreateTraceBuffer {
                request_id: 1,
                domain_id: 1,
                size: 4096,
            },
        )
        .unwrap();
        let unrelated = dispatch(
            &fixture_caller(1),
            main_entry,
            &writer,
            &table,
            &mut service,
            &env,
        );
        let unrelated = read_payload::<CreateTraceBufferReply>(&unrelated).unwrap();
        assert_eq!(unrelated.status, TRACE_STATUS_OK);
        with_service(main_entry, &mut service, |service| {
            service.trace_ring_mut().emit(0, 10, 20, 30);
            service.trace_ring_mut().emit(1, 11, 21, 31);
        });
        write_payload(
            &mut env,
            &CreateTraceBuffer {
                request_id: 2,
                domain_id: 0,
                size: 4096,
            },
        )
        .unwrap();
        for (handle, status) in [
            (Handle::INVALID, TRACE_STATUS_INVALID_CAPABILITY),
            (wrong_kind, TRACE_STATUS_INVALID_CAPABILITY),
            (stale, TRACE_STATUS_INVALID_CAPABILITY),
            (foreign, TRACE_STATUS_INVALID_CAPABILITY),
        ] {
            env.handle = handle;
            let denied = dispatch(
                &fixture_caller(0),
                main_entry,
                &writer,
                &table,
                &mut service,
                &env,
            );
            let denied = read_payload::<CreateTraceBufferReply>(&denied).unwrap();
            assert_eq!(
                (denied.request_id, denied.status, denied.trace_cap),
                (2, status, 0)
            );
            with_service(main_entry, &mut service, |service| {
                assert_eq!(service.trace_ring().positions(0), (0, 24));
                assert_eq!(service.trace_ring().positions(1), (0, 24));
            });
        }
        env.handle = owner;
        let denied = dispatch(
            &fixture_caller(1),
            main_entry,
            &writer,
            &table,
            &mut service,
            &env,
        );
        assert_eq!(
            read_payload::<CreateTraceBufferReply>(&denied)
                .unwrap()
                .status,
            TRACE_STATUS_INVALID_CAPABILITY
        );
        env.handle = foreign;
        let denied = dispatch(
            &fixture_caller(1),
            main_entry,
            &writer,
            &table,
            &mut service,
            &env,
        );
        assert_eq!(
            read_payload::<CreateTraceBufferReply>(&denied)
                .unwrap()
                .status,
            TRACE_STATUS_PERMISSION_DENIED
        );
        env.handle = owner;
        let created = dispatch(
            &fixture_caller(0),
            main_entry,
            &writer,
            &table,
            &mut service,
            &env,
        );
        let created = read_payload::<CreateTraceBufferReply>(&created).unwrap();
        assert_eq!(created.status, TRACE_STATUS_OK);
        with_service(main_entry, &mut service, |service| {
            service.trace_ring_mut().emit(0, 12, 22, 32);
        });
        // Legacy metadata-only reads deny even for admitted owners, preserving
        // both streams until a supported consumer receives actual bytes.
        for (domain, ipc, trace_cap) in [
            (0, owner, created.trace_cap),
            (1, foreign, unrelated.trace_cap),
        ] {
            let mut read = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_READ_TRACE);
            read.handle = ipc;
            write_payload(
                &mut read,
                &ReadTrace {
                    request_id: 3,
                    trace_cap,
                    offset: 0,
                    length: 24,
                },
            )
            .unwrap();
            let reply = dispatch(
                &fixture_caller(domain),
                main_entry,
                &writer,
                &table,
                &mut service,
                &read,
            );
            let reply = read_payload::<ReadTraceReply>(&reply).unwrap();
            assert_eq!(
                (reply.request_id, reply.status, reply.data_len),
                (3, crate::trace_service::TRACE_STATUS_INVALID_SIZE, 0)
            );
            with_service(main_entry, &mut service, |service| {
                assert_eq!(service.trace_ring().positions(0), (0, 24));
                assert_eq!(service.trace_ring().positions(1), (0, 24));
            });
        }
        // A trusted Rust consumer with a real output buffer still makes progress;
        // separate v3 tests exercise authenticated IPC delivery into actual Shmem.
        for (domain, ipc, trace_cap, tag, arg0, arg1) in [
            (0, owner, created.trace_cap, 12u32, 22u64, 32u64),
            (1, foreign, unrelated.trace_cap, 11u32, 21u64, 31u64),
        ] {
            with_service(main_entry, &mut service, |service| {
                let mut bytes = [0xA5; 24];
                assert_eq!(
                    service
                        .read_trace(Handle::unpack(trace_cap), 0, 24, &mut bytes)
                        .unwrap(),
                    (TRACE_STATUS_OK, 24)
                );
                assert_eq!(&bytes[..4], &tag.to_le_bytes());
                assert_eq!(&bytes[4..8], &[0; 4]);
                assert_eq!(&bytes[8..16], &arg0.to_le_bytes());
                assert_eq!(&bytes[16..24], &arg1.to_le_bytes());
                assert_eq!(service.trace_ring().positions(domain), (24, 24));
                if domain == 0 {
                    assert_eq!(service.trace_ring().positions(1), (0, 24));
                }
            });
            let mut destroy = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_DESTROY_TRACE_BUFFER);
            destroy.handle = ipc;
            write_payload(
                &mut destroy,
                &DestroyTraceBuffer {
                    request_id: 4,
                    trace_cap,
                },
            )
            .unwrap();
            let reply = dispatch(
                &fixture_caller(domain),
                main_entry,
                &writer,
                &table,
                &mut service,
                &destroy,
            );
            assert_eq!(
                read_payload::<DestroyTraceBufferReply>(&reply)
                    .unwrap()
                    .status,
                TRACE_STATUS_OK
            );
        }
        // Destroy released the owner slot; the same authenticated domain can create again.
        let recreated = dispatch(
            &fixture_caller(0),
            main_entry,
            &writer,
            &table,
            &mut service,
            &env,
        );
        let recreated = read_payload::<CreateTraceBufferReply>(&recreated).unwrap();
        assert_eq!(recreated.status, TRACE_STATUS_OK);
        assert_ne!(recreated.trace_cap, created.trace_cap);
    }

    #[test]
    fn review_authority_trace_direct_creation_denies_then_recovers() {
        trace_creation_requires_domain_authority(false);
    }

    #[test]
    fn review_authority_trace_main_creation_denies_then_recovers() {
        trace_creation_requires_domain_authority(true);
    }
}

#[cfg(test)]
mod review_caller_namespace {
    use super::*;

    #[test]
    fn review_namespace_copied_ipc_handle_denies_then_each_owner_progresses() {
        let _trace = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut table = StaticCapTable::new();
        let a = table.allocate_for_domain(1).unwrap();
        let b = table.allocate_for_domain(2).unwrap();
        // Test-only trusted contexts are independent of all envelope bytes.
        let caller_a = unsafe { CallerContext::for_trusted_kernel_fixture(1) };
        let caller_b = unsafe { CallerContext::for_trusted_kernel_fixture(2) };
        let mut regions = shmem::ShmemRegionTable::new();
        let mut env = Envelope::empty(PROTOCOL_PING, MSG_PING);
        env.handle = a;
        write_payload(&mut env, &Ping { nonce: 81 }).unwrap();
        let denied = handle_envelope(&caller_b, &writer, &table, &mut regions, &env);
        assert_eq!(
            denied.payload_len, 0,
            "copying A's complete numeric handle must not confer A's namespace"
        );
        let owner = handle_envelope(&caller_a, &writer, &table, &mut regions, &env);
        assert_eq!(read_payload::<Pong>(&owner).unwrap().nonce, 81);
        env.handle = b;
        let unrelated = handle_envelope(&caller_b, &writer, &table, &mut regions, &env);
        assert_eq!(read_payload::<Pong>(&unrelated).unwrap().nonce, 81);
    }

    fn trace_cap_copy_denies_without_consuming_or_destroying(main_entry: bool) {
        let _trace = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut caps = StaticCapTable::new();
        let a = caps.allocate_for_domain(1).unwrap();
        let b = caps.allocate_for_domain(2).unwrap();
        let caller_a = unsafe { CallerContext::for_trusted_kernel_fixture(1) };
        let caller_b = unsafe { CallerContext::for_trusted_kernel_fixture(2) };
        let mut service = TraceService::new();
        let initialize = |service: &mut TraceService| {
            *service = TraceService::new();
            service.init_kernel();
            service.register_domain(1, "a").unwrap();
            service.register_domain(2, "b").unwrap();
            let (_, trace_a) = service.create_trace_buffer(1, 4096).unwrap();
            let (_, trace_b) = service.create_trace_buffer(2, 4096).unwrap();
            service.trace_ring_mut().emit(1, 91, 92, 93);
            service.trace_ring_mut().emit(2, 71, 72, 73);
            (trace_a, trace_b)
        };
        let (trace_a, trace_b) = if main_entry {
            // SAFETY: the trace test lock serializes the complete global sequence.
            unsafe { initialize(&mut KERNEL_TRACE_SERVICE) }
        } else {
            initialize(&mut service)
        };
        let dispatch = |caller: &CallerContext, service: &mut TraceService, env: &Envelope| {
            if main_entry {
                handle_envelope(
                    caller,
                    &writer,
                    &caps,
                    &mut shmem::ShmemRegionTable::new(),
                    env,
                )
            } else {
                handle_trace_service_envelope(caller, &writer, &caps, service, env)
            }
        };
        let mut info = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_GET_TRACE_INFO);
        info.handle = b;
        write_payload(
            &mut info,
            &GetTraceInfo {
                request_id: 1,
                trace_cap: trace_a.pack(),
            },
        )
        .unwrap();
        let denied = dispatch(&caller_b, &mut service, &info);
        assert_ne!(
            read_payload::<GetTraceInfoReply>(&denied).unwrap().status,
            0
        );
        let mut read = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_READ_TRACE);
        read.handle = b;
        write_payload(
            &mut read,
            &ReadTrace {
                request_id: 2,
                trace_cap: trace_a.pack(),
                offset: 0,
                length: 24,
            },
        )
        .unwrap();
        let denied = dispatch(&caller_b, &mut service, &read);
        assert_ne!(read_payload::<ReadTraceReply>(&denied).unwrap().status, 0);
        let mut destroy = Envelope::empty(PROTOCOL_TRACE_SERVICE_V1, MSG_DESTROY_TRACE_BUFFER);
        destroy.handle = b;
        write_payload(
            &mut destroy,
            &DestroyTraceBuffer {
                request_id: 3,
                trace_cap: trace_a.pack(),
            },
        )
        .unwrap();
        let denied = dispatch(&caller_b, &mut service, &destroy);
        assert_ne!(
            read_payload::<DestroyTraceBufferReply>(&denied)
                .unwrap()
                .status,
            0
        );
        // A valid trace payload cannot bypass a missing or copied IPC authority.
        for invalid_ipc in [Handle::INVALID, a] {
            info.handle = invalid_ipc;
            read.handle = invalid_ipc;
            destroy.handle = invalid_ipc;
            let denied = dispatch(&caller_b, &mut service, &info);
            let denied = read_payload::<GetTraceInfoReply>(&denied).unwrap();
            assert_eq!(
                (
                    denied.request_id,
                    denied.status,
                    denied.domain_id,
                    denied.size,
                    denied.read_offset,
                    denied.write_offset
                ),
                (
                    1,
                    crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY,
                    0,
                    0,
                    0,
                    0
                )
            );
            let denied = dispatch(&caller_b, &mut service, &read);
            let denied = read_payload::<ReadTraceReply>(&denied).unwrap();
            assert_eq!(
                (denied.request_id, denied.status, denied.data_len),
                (2, crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY, 0)
            );
            let denied = dispatch(&caller_b, &mut service, &destroy);
            let denied = read_payload::<DestroyTraceBufferReply>(&denied).unwrap();
            assert_eq!(
                (denied.request_id, denied.status),
                (3, crate::trace_service::TRACE_STATUS_INVALID_CAPABILITY)
            );
        }
        info.handle = a;
        let owner = dispatch(&caller_a, &mut service, &info);
        let owner = read_payload::<GetTraceInfoReply>(&owner).unwrap();
        assert_eq!(
            (
                owner.status,
                owner.domain_id,
                owner.read_offset,
                owner.write_offset
            ),
            (0, 1, 0, 24)
        );
        write_payload(
            &mut info,
            &GetTraceInfo {
                request_id: 4,
                trace_cap: trace_b.pack(),
            },
        )
        .unwrap();
        info.handle = b;
        let unrelated = dispatch(&caller_b, &mut service, &info);
        let unrelated = read_payload::<GetTraceInfoReply>(&unrelated).unwrap();
        assert_eq!(
            (
                unrelated.status,
                unrelated.domain_id,
                unrelated.read_offset,
                unrelated.write_offset
            ),
            (0, 2, 0, 24)
        );
        destroy.handle = a;
        let owner = dispatch(&caller_a, &mut service, &destroy);
        assert_eq!(
            read_payload::<DestroyTraceBufferReply>(&owner)
                .unwrap()
                .status,
            0
        );
    }

    #[test]
    fn review_namespace_trace_cap_copy_denies_without_consuming_or_destroying() {
        trace_cap_copy_denies_without_consuming_or_destroying(false);
    }

    #[test]
    fn review_namespace_trace_main_cap_copy_denies_without_consuming_or_destroying() {
        trace_cap_copy_denies_without_consuming_or_destroying(true);
    }
}

#[cfg(test)]
mod review_consent_consumer {
    use super::*;
    use kernel_api::generated::{
        AcceptRegion, AcceptRegionReply, CancelRegionOffer, CancelRegionOfferReply, OfferRegion,
        OfferRegionReply, SHMEM_CONSENT_V2_PROTOCOL_ID,
    };

    #[test]
    fn review_consent_wire_requires_target_context_and_rejects_invalid_headers() {
        let _memory = crate::mm::test_support::setup();
        let _trace = trace_ring::reset_for_test();
        crate::cap_table::reset_smp_state_for_test();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut caps = StaticCapTable::new();
        let a = caps.allocate_for_domain(1).unwrap();
        let b = caps.allocate_for_domain(2).unwrap();
        let other = caps.allocate_for_domain(3).unwrap();
        let caller_a = fixture_caller(1);
        let caller_b = fixture_caller(2);
        let caller_other = fixture_caller(3);
        let mut regions = shmem::ShmemRegionTable::new();
        let (region, cap, _) = regions
            .create_region(1, 4096, shmem::REGION_FLAG_READABLE, 4096)
            .unwrap();
        let mut env = Envelope::empty(SHMEM_CONSENT_V2_PROTOCOL_ID, 1);
        env.handle = a;
        let mut request = OfferRegion {
            request_id: 1,
            version: 2,
            reserved: 0,
            region_id: region,
            shm_cap: cap.pack(),
            target_domain_id: 2,
            rights: shmem::RIGHTS_READ,
            cache_mode: 0,
        };
        for case in 0..3 {
            request.version = if case == 0 { 1 } else { 2 };
            request.reserved = u32::from(case == 1);
            write_payload(&mut env, &request).unwrap();
            if case == 2 {
                env.payload_len -= 1;
            }
            let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &env);
            assert_ne!(read_payload::<OfferRegionReply>(&reply).unwrap().status, 0);
            assert_eq!(regions.regions[(cap.index - 1) as usize].refcount, 0);
        }
        request.version = 2;
        request.reserved = 0;
        write_payload(&mut env, &request).unwrap();
        let reply = handle_envelope(&caller_b, &writer, &caps, &mut regions, &env);
        assert_ne!(
            read_payload::<OfferRegionReply>(&reply).unwrap().status,
            0,
            "copied owner IPC must deny"
        );
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &env);
        let reply = read_payload::<OfferRegionReply>(&reply).unwrap();
        assert_eq!(reply.status, 0);
        assert_ne!(reply.offer_id, 0);
        assert_eq!(regions.regions[(cap.index - 1) as usize].refcount, 0);
        let mut accept = Envelope::empty(SHMEM_CONSENT_V2_PROTOCOL_ID, 3);
        accept.handle = other;
        write_payload(
            &mut accept,
            &AcceptRegion {
                request_id: 2,
                version: 2,
                reserved: 0,
                offer_id: reply.offer_id,
            },
        )
        .unwrap();
        let denied = handle_envelope(&caller_other, &writer, &caps, &mut regions, &accept);
        assert_ne!(
            read_payload::<AcceptRegionReply>(&denied).unwrap().status,
            0
        );
        accept.handle = b;
        let accepted = handle_envelope(&caller_b, &writer, &caps, &mut regions, &accept);
        let accepted = read_payload::<AcceptRegionReply>(&accepted).unwrap();
        assert_eq!((accepted.status, accepted.mapping_id), (0, region));
        regions.unmap_region(2, accepted.mapping_id, 2).unwrap();
        let reply = handle_envelope(&caller_a, &writer, &caps, &mut regions, &env);
        let reply = read_payload::<OfferRegionReply>(&reply).unwrap();
        let mut cancel = Envelope::empty(SHMEM_CONSENT_V2_PROTOCOL_ID, 5);
        cancel.handle = b;
        write_payload(
            &mut cancel,
            &CancelRegionOffer {
                request_id: 3,
                version: 2,
                reserved: 0,
                offer_id: reply.offer_id,
            },
        )
        .unwrap();
        let cancelled = handle_envelope(&caller_b, &writer, &caps, &mut regions, &cancel);
        assert_eq!(
            read_payload::<CancelRegionOfferReply>(&cancelled)
                .unwrap()
                .status,
            0
        );
        write_payload(
            &mut accept,
            &AcceptRegion {
                request_id: 4,
                version: 2,
                reserved: 0,
                offer_id: reply.offer_id,
            },
        )
        .unwrap();
        let denied = handle_envelope(&caller_b, &writer, &caps, &mut regions, &accept);
        assert_ne!(
            read_payload::<AcceptRegionReply>(&denied).unwrap().status,
            0
        );
        regions.close_region(1, region).unwrap();
    }
}

#[cfg(test)]
mod review_trace_delivery {
    use super::*;
    use kernel_api::generated::{
        CommitTraceRead, CommitTraceReadReply, ReadTraceInto, ReadTraceIntoReply,
    };

    struct Fixture {
        caps: StaticCapTable,
        regions: shmem::ShmemRegionTable,
        service: TraceService,
        ipc: Handle,
        foreign_ipc: Handle,
        source: Handle,
        unrelated: Handle,
        destination: Handle,
        region: u64,
        physical: u64,
    }

    impl Fixture {
        fn new() -> Self {
            crate::cap_table::reset_smp_state_for_test();
            let mut caps = StaticCapTable::new();
            let ipc = caps.allocate_for_domain(0).unwrap();
            let foreign_ipc = caps.allocate_for_domain(1).unwrap();
            let mut service = TraceService::new();
            service.init_kernel();
            service.register_domain(1, "unrelated").unwrap();
            let (_, source) = service.create_trace_buffer(0, 4096).unwrap();
            let (_, unrelated) = service.create_trace_buffer(1, 4096).unwrap();
            service
                .trace_ring()
                .emit(0, 0x12345678, 0x1122334455667788, 0x8877665544332211);
            service.trace_ring().emit(1, 99, 98, 97);
            let mut regions = shmem::ShmemRegionTable::new();
            let (region, destination, physical) = regions
                .create_region(
                    0,
                    4096,
                    shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
                    4096,
                )
                .unwrap();
            unsafe { core::ptr::write_bytes(physical as *mut u8, 0xA5, 4096) };
            Self {
                caps,
                regions,
                service,
                ipc,
                foreign_ipc,
                source,
                unrelated,
                destination,
                region,
                physical,
            }
        }

        fn request(&self) -> ReadTraceInto {
            ReadTraceInto {
                request_id: 101,
                trace_cap: self.source.pack(),
                destination_shm_cap: self.destination.pack(),
                destination_offset: 8,
                trace_offset: 0,
                length: 24,
                version: 3,
                reserved: 0,
            }
        }

        fn dispatch_read(
            &mut self,
            writer: &trace_ring::TraceWriter,
            caller: u64,
            ipc: Handle,
            req: ReadTraceInto,
            length_delta: i32,
        ) -> ReadTraceIntoReply {
            let mut env = Envelope::empty(532, 1);
            env.handle = ipc;
            write_payload(&mut env, &req).unwrap();
            env.payload_len = (env.payload_len as i32 + length_delta) as u32;
            let reply = handle_trace_delivery_envelope(
                &fixture_caller(caller),
                writer,
                &self.caps,
                &mut self.regions,
                &mut self.service,
                &env,
            );
            assert_eq!((reply.protocol, reply.msg_type), (532, 2));
            read_payload::<ReadTraceIntoReply>(&reply).expect("v3 must return typed delivery reply")
        }

        fn read(&mut self, writer: &trace_ring::TraceWriter) -> ReadTraceIntoReply {
            let req = self.request();
            self.dispatch_read(writer, 0, self.ipc, req, 0)
        }

        fn commit(&mut self, writer: &trace_ring::TraceWriter, lease: u64) -> CommitTraceReadReply {
            self.commit_request(
                writer,
                0,
                self.ipc,
                CommitTraceRead {
                    request_id: 102,
                    trace_cap: self.source.pack(),
                    lease_id: lease,
                    version: 3,
                    reserved: 0,
                },
                0,
            )
        }

        fn commit_request(
            &mut self,
            writer: &trace_ring::TraceWriter,
            caller: u64,
            ipc: Handle,
            req: CommitTraceRead,
            length_delta: i32,
        ) -> CommitTraceReadReply {
            let mut env = Envelope::empty(532, 3);
            env.handle = ipc;
            write_payload(&mut env, &req).unwrap();
            env.payload_len = (env.payload_len as i32 + length_delta) as u32;
            let reply = handle_trace_delivery_envelope(
                &fixture_caller(caller),
                writer,
                &self.caps,
                &mut self.regions,
                &mut self.service,
                &env,
            );
            assert_eq!((reply.protocol, reply.msg_type), (532, 4));
            read_payload::<CommitTraceReadReply>(&reply).unwrap()
        }

        fn bytes(&self) -> &[u8] {
            unsafe { core::slice::from_raw_parts(self.physical as *const u8, 64) }
        }
        fn cursors(&self) -> (u64, u64) {
            self.service.trace_ring().positions(0)
        }
        fn unrelated_unchanged(&mut self) {
            assert_eq!(self.service.trace_ring().positions(1), (0, 24));
            let mut bytes = [0; 24];
            assert_eq!(
                self.service
                    .read_trace(self.unrelated, 0, 24, &mut bytes)
                    .unwrap(),
                (0, 24)
            );
            assert_eq!(&bytes[..4], &99u32.to_le_bytes());
        }
    }

    #[test]
    fn review_trace_delivery_bytes_are_visible_before_explicit_commit_and_retry() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let first = f.read(&writer);
        assert_eq!(
            (
                first.request_id,
                first.status,
                first.data_len,
                first.next_offset
            ),
            (101, 0, 24, 24)
        );
        assert_ne!(first.lease_id, 0);
        assert_eq!(f.cursors(), (0, 24));
        assert_eq!(&f.bytes()[8..12], &0x12345678u32.to_le_bytes());
        assert_eq!(&f.bytes()[12..16], &[0; 4]);
        assert_eq!(&f.bytes()[16..24], &0x1122334455667788u64.to_le_bytes());
        assert_eq!(&f.bytes()[24..32], &0x8877665544332211u64.to_le_bytes());
        assert!(f.bytes()[..8].iter().all(|b| *b == 0xA5));
        assert!(f.bytes()[32..].iter().all(|b| *b == 0xA5));
        // Lost reply: no Commit. The next read returns the same event with a new lease.
        let next = f.read(&writer);
        assert_ne!(next.lease_id, first.lease_id);
        assert_ne!(f.commit(&writer, first.lease_id).status, 0);
        assert_eq!(f.cursors(), (0, 24));
        let committed = f.commit(&writer, next.lease_id);
        assert_eq!(
            (
                committed.request_id,
                committed.status,
                committed.next_offset,
                committed.reserved
            ),
            (102, 0, 24, 0)
        );
        f.service.trace_ring().emit(0, 7, 8, 9);
        let ack = f.commit(&writer, next.lease_id);
        assert_eq!((ack.status, ack.next_offset, ack.reserved), (0, 24, 0));
        assert_eq!(f.cursors(), (24, 48));
        let mut request = f.request();
        request.trace_offset = 24;
        let read = f.dispatch_read(&writer, 0, f.ipc, request, 0);
        assert_eq!(read.status, 0);
        assert_eq!(f.commit(&writer, read.lease_id).next_offset, 48);
        assert_ne!(f.commit(&writer, next.lease_id).status, 0);
        f.unrelated_unchanged();
    }

    #[test]
    fn review_trace_delivery_committed_retry_survives_closed_destination_and_recovers() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let delivered = f.read(&writer);
        assert_eq!(delivered.status, 0);
        let committed = f.commit(&writer, delivered.lease_id);
        assert_eq!((committed.status, committed.next_offset), (0, 24));
        assert_eq!(f.cursors(), (24, 24));
        // The original successful acknowledgement can be lost. Retiring its
        // destination must not prevent reconciliation of the completed result.
        f.regions.close_region(0, f.region).unwrap();
        f.service.trace_ring().emit(0, 7, 8, 9);
        let retried = f.commit(&writer, delivered.lease_id);
        assert_eq!(
            (
                retried.request_id,
                retried.status,
                retried.next_offset,
                retried.reserved
            ),
            (102, 0, 24, 0)
        );
        assert_eq!(f.cursors(), (24, 48));
        let (region, destination, physical) = f
            .regions
            .create_region(
                0,
                4096,
                shmem::REGION_FLAG_READABLE | shmem::REGION_FLAG_WRITABLE,
                4096,
            )
            .unwrap();
        assert_ne!(destination, f.destination);
        f.region = region;
        f.destination = destination;
        f.physical = physical;
        let mut req = f.request();
        req.trace_offset = 24;
        let next = f.dispatch_read(&writer, 0, f.ipc, req, 0);
        assert_eq!((next.status, next.data_len), (0, 24));
        assert_eq!(&f.bytes()[8..12], &7u32.to_le_bytes());
        let committed = f.commit(&writer, next.lease_id);
        assert_eq!((committed.status, committed.next_offset), (0, 48));
        assert_eq!(f.cursors(), (48, 48));
        f.unrelated_unchanged();
    }

    #[test]
    fn review_trace_delivery_read_denials_preserve_bytes_cursor_and_pending_lease() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let pending = f.read(&writer);
        assert_eq!(pending.status, 0);
        let before: [u8; 64] = f.bytes().try_into().unwrap();
        let (_, readonly, _) = f
            .regions
            .create_region(0, 4096, shmem::REGION_FLAG_READABLE, 4096)
            .unwrap();
        let (_, foreign, _) = f
            .regions
            .create_region(1, 4096, shmem::REGION_FLAG_WRITABLE, 4096)
            .unwrap();
        for case in 0..19 {
            let mut req = f.request();
            let (mut caller, mut ipc, mut delta) = (0, f.ipc, 0);
            match case {
                0 => caller = 1,
                1 => ipc = f.foreign_ipc,
                2 => ipc = Handle::INVALID,
                3 => req.trace_cap = f.destination.pack(),
                4 => {
                    let mut h = f.source;
                    h.generation += 1;
                    req.trace_cap = h.pack();
                }
                5 => req.trace_cap = f.unrelated.pack(),
                6 => req.destination_shm_cap = f.source.pack(),
                7 => {
                    let mut h = f.destination;
                    h.generation += 1;
                    req.destination_shm_cap = h.pack();
                }
                8 => req.destination_shm_cap = readonly.pack(),
                9 => req.destination_shm_cap = foreign.pack(),
                10 => req.destination_offset = 4090,
                11 => req.destination_offset = u64::MAX,
                12 => req.trace_offset = 24,
                13 => req.length = 1,
                14 => req.length = 1560,
                15 => req.version = 2,
                16 => req.reserved = 1,
                17 => delta = -1,
                _ => delta = 1,
            }
            let reply = f.dispatch_read(&writer, caller, ipc, req, delta);
            assert_ne!(reply.status, 0, "denial case {case}");
            assert_eq!(
                (reply.lease_id, reply.next_offset, reply.data_len),
                (0, 0, 0)
            );
            assert_eq!(reply.request_id, if delta == 0 { 101 } else { 0 });
            assert_eq!(f.bytes(), before);
            assert_eq!(f.cursors(), (0, 24));
        }
        f.service
            .cap_table_mut()
            .revoke_rights(f.source.index as usize, crate::trace_cap::TRACE_RIGHT_READ)
            .unwrap();
        let req = f.request();
        assert_ne!(f.dispatch_read(&writer, 0, f.ipc, req, 0).status, 0);
        f.service
            .cap_table_mut()
            .grant_rights(f.source.index as usize, crate::trace_cap::TRACE_RIGHT_READ)
            .unwrap();
        assert_eq!(f.commit(&writer, pending.lease_id).status, 0);
        f.unrelated_unchanged();
    }

    #[test]
    fn review_trace_delivery_commit_denials_recovery_and_destination_reuse() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let pending = f.read(&writer);
        for case in 0..9 {
            let mut req = CommitTraceRead {
                request_id: 102,
                trace_cap: f.source.pack(),
                lease_id: pending.lease_id,
                version: 3,
                reserved: 0,
            };
            let (mut caller, mut ipc, mut delta) = (0, f.ipc, 0);
            match case {
                0 => req.lease_id = 0,
                1 => req.lease_id = pending.lease_id + 1,
                2 => req.version = 2,
                3 => req.reserved = 1,
                4 => req.trace_cap = f.destination.pack(),
                5 => caller = 1,
                6 => ipc = f.foreign_ipc,
                7 => delta = -1,
                _ => delta = 1,
            }
            let reply = f.commit_request(&writer, caller, ipc, req, delta);
            assert_ne!(reply.status, 0);
            assert_eq!((reply.next_offset, reply.reserved), (0, 0));
            assert_eq!(f.cursors(), (0, 24));
        }
        f.regions.close_region(0, f.region).unwrap();
        assert_ne!(f.commit(&writer, pending.lease_id).status, 0);
        let (id, cap, physical) = f
            .regions
            .create_region(0, 4096, shmem::REGION_FLAG_WRITABLE, 4096)
            .unwrap();
        assert_ne!(cap, f.destination);
        f.region = id;
        f.destination = cap;
        f.physical = physical;
        assert_ne!(f.commit(&writer, pending.lease_id).status, 0);
        let next = f.read(&writer);
        assert_eq!(next.status, 0);
        assert_eq!(f.commit(&writer, next.lease_id).status, 0);
        assert_eq!(f.cursors(), (24, 24));
        f.unrelated_unchanged();
    }

    #[test]
    fn review_trace_delivery_overflow_resync_and_source_generation_recover() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let pending = f.read(&writer);
        for n in 0..64 {
            f.service.trace_ring().emit(0, n, n as u64, 0);
        }
        assert_eq!(f.cursors(), (24, 1560));
        assert_ne!(f.commit(&writer, pending.lease_id).status, 0);
        let mut req = f.request();
        req.trace_offset = f.service.get_trace_info(f.source).unwrap().1.read_offset;
        let fresh = f.dispatch_read(&writer, 0, f.ipc, req, 0);
        assert_eq!(fresh.status, 0);
        assert_eq!(f.commit(&writer, fresh.lease_id).next_offset, 48);
        assert_eq!(f.service.destroy_trace_buffer(f.source).unwrap(), 0);
        let old = f.source;
        f.source = f.service.create_trace_buffer(0, 4096).unwrap().1;
        assert_ne!(old, f.source);
        f.service.trace_ring().emit(0, 77, 88, 99);
        assert_ne!(f.commit(&writer, fresh.lease_id).status, 0);
        let next = f.read(&writer);
        assert_eq!(next.status, 0);
        assert_ne!(next.lease_id, fresh.lease_id);
        assert_eq!(f.commit(&writer, next.lease_id).next_offset, 24);
        f.unrelated_unchanged();
    }

    #[test]
    fn review_trace_delivery_main_entry_enforces_caller_and_delivers_actual_bytes() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        // SAFETY: the trace test guard owns this complete global-service sequence.
        unsafe {
            KERNEL_TRACE_SERVICE = core::mem::take(&mut f.service);
        }
        let mut env = Envelope::empty(532, 1);
        env.handle = f.ipc;
        write_payload(&mut env, &f.request()).unwrap();
        let denied = handle_envelope(&fixture_caller(1), &writer, &f.caps, &mut f.regions, &env);
        let denied = read_payload::<ReadTraceIntoReply>(&denied).unwrap();
        assert_eq!(
            (
                denied.request_id,
                denied.lease_id,
                denied.next_offset,
                denied.data_len
            ),
            (101, 0, 0, 0)
        );
        assert_ne!(denied.status, 0);
        assert!(f.bytes().iter().all(|byte| *byte == 0xA5));
        let delivered = handle_envelope(&fixture_caller(0), &writer, &f.caps, &mut f.regions, &env);
        let delivered = read_payload::<ReadTraceIntoReply>(&delivered).unwrap();
        assert_eq!((delivered.status, delivered.data_len), (0, 24));
        assert_eq!(&f.bytes()[8..12], &0x12345678u32.to_le_bytes());
        assert_eq!(&f.bytes()[12..16], &[0; 4]);
        assert_eq!(&f.bytes()[16..24], &0x1122334455667788u64.to_le_bytes());
        assert_eq!(&f.bytes()[24..32], &0x8877665544332211u64.to_le_bytes());
        unsafe {
            assert_eq!(KERNEL_TRACE_SERVICE.trace_ring().positions(0), (0, 24));
        }
        let mut commit = Envelope::empty(532, 3);
        commit.handle = f.ipc;
        write_payload(
            &mut commit,
            &CommitTraceRead {
                request_id: 102,
                trace_cap: f.source.pack(),
                lease_id: delivered.lease_id,
                version: 3,
                reserved: 0,
            },
        )
        .unwrap();
        let denied = handle_envelope(
            &fixture_caller(1),
            &writer,
            &f.caps,
            &mut f.regions,
            &commit,
        );
        assert_ne!(
            read_payload::<CommitTraceReadReply>(&denied)
                .unwrap()
                .status,
            0
        );
        let committed = handle_envelope(
            &fixture_caller(0),
            &writer,
            &f.caps,
            &mut f.regions,
            &commit,
        );
        let committed = read_payload::<CommitTraceReadReply>(&committed).unwrap();
        assert_eq!(
            (committed.status, committed.next_offset, committed.reserved),
            (0, 24, 0)
        );
        unsafe {
            assert_eq!(KERNEL_TRACE_SERVICE.trace_ring().positions(0), (24, 24));
            assert_eq!(KERNEL_TRACE_SERVICE.trace_ring().positions(1), (0, 24));
        }
    }

    #[test]
    fn review_trace_delivery_zero_length_preserves_pending_without_physical_zero_span() {
        let _trace = trace_ring::reset_for_test();
        let _memory = crate::mm::test_support::setup();
        let writer = trace_ring::TraceWriter::claim().unwrap();
        let mut f = Fixture::new();
        let pending = f.read(&writer);
        let mut req = f.request();
        req.length = 0;
        req.destination_offset = 4097;
        assert_ne!(f.dispatch_read(&writer, 0, f.ipc, req, 0).status, 0);
        req.destination_offset = 4096;
        req.trace_offset = 24;
        assert_ne!(f.dispatch_read(&writer, 0, f.ipc, req, 0).status, 0);
        req.trace_offset = 0;
        let empty = f.dispatch_read(&writer, 0, f.ipc, req, 0);
        assert_eq!(
            (
                empty.status,
                empty.data_len,
                empty.lease_id,
                empty.next_offset
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(f.cursors(), (0, 24));
        assert_eq!(f.commit(&writer, pending.lease_id).status, 0);
    }
}
