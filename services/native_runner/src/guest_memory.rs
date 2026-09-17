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

/// A validated SDK in/out descriptor. Capture capacity before any bridge call.
pub struct ReplyBuffer {
    out: usize,
    len_word: usize,
    capacity: usize,
}

impl ReplyBuffer {
    pub fn new(data: &[u8], out: u32, len_word: u32, required: usize) -> Result<Self, Status> {
        if !valid_range(data.len(), len_word, 4) {
            return Err(Status::InvalidArgument);
        }
        let len_word = len_word as usize;
        let capacity = i32::from_le_bytes(data[len_word..len_word + 4].try_into().unwrap());
        if capacity < 0 || !valid_range(data.len(), out, capacity as usize) {
            return Err(Status::InvalidArgument);
        }
        let out = out as usize;
        let capacity = capacity as usize;
        // The SDK's mutable output slice and stack length word are disjoint.
        if capacity < required || (capacity > 0 && out < len_word + 4 && len_word < out + capacity)
        {
            return Err(Status::InvalidArgument);
        }
        Ok(Self {
            out,
            len_word,
            capacity,
        })
    }

    pub fn write(
        self,
        memory: Memory,
        caller: &mut Caller<'_, InstanceContext>,
        bytes: &[u8],
    ) -> i32 {
        let data = memory.data_mut(caller);
        // Recheck the actual reply before touching either destination, even if
        // the bridge violates the expected IDL size or requested read length.
        if bytes.len() > self.capacity
            || !valid_range(data.len(), self.out as u32, self.capacity)
            || !valid_range(data.len(), self.len_word as u32, 4)
        {
            return Status::InvalidArgument as i32;
        }
        data[self.out..self.out + bytes.len()].copy_from_slice(bytes);
        data[self.len_word..self.len_word + 4].copy_from_slice(&(bytes.len() as i32).to_le_bytes());
        Status::Ok as i32
    }
}
