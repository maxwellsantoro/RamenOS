//! Bounds-checked access to the calling WASM instance's exported memory.
use crate::{context::InstanceContext, error::Status};
use wasmtime::{Caller, Memory};

pub fn memory(caller: &mut Caller<'_, InstanceContext>) -> Result<Memory, Status> {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or(Status::InvalidArgument)
}

pub fn valid_range(size: usize, start: u32, len: usize) -> bool {
    (start as usize)
        .checked_add(len)
        .is_some_and(|end| end <= size)
}

pub fn write_reply(
    memory: Memory,
    caller: &mut Caller<'_, InstanceContext>,
    out_ptr: u32,
    out_len_ptr: u32,
    bytes: &[u8],
) -> i32 {
    let data = memory.data_mut(caller);
    if !valid_range(data.len(), out_ptr, bytes.len()) || !valid_range(data.len(), out_len_ptr, 4) {
        return Status::InvalidArgument as i32;
    }
    data[out_ptr as usize..out_ptr as usize + bytes.len()].copy_from_slice(bytes);
    data[out_len_ptr as usize..out_len_ptr as usize + 4]
        .copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    Status::Ok as i32
}
