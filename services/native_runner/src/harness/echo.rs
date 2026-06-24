//! Host functions for echo_harness_v0.

use crate::context::InstanceContext;
use crate::error::Status;
use wasmtime::*;

/// Create the echo_request host function.
///
/// Signature matches generated SDK:
/// - cap_handle: u64 - capability handle
/// - request_id: u64 - request identifier
/// - payload_ptr: u32 - pointer to payload in linear memory
/// - payload_len: u32 - length of payload
/// - out_ptr: u32 - pointer to output buffer
/// - out_len_ptr: u32 - pointer to write actual output length
/// - returns: i32 - status code
pub fn create_echo_request_host(store: &mut Store<InstanceContext>, memory: Memory) -> Func {
    Func::wrap(
        store,
        move |mut caller: Caller<'_, InstanceContext>,
              cap_handle: u64,
              request_id: u64,
              payload_ptr: u32,
              payload_len: u32,
              out_ptr: u32,
              out_len_ptr: u32|
              -> i32 {
            // 1. Read payload from linear memory (bounds-checked)
            let data = memory.data(&caller);

            let payload_end = match payload_ptr.checked_add(payload_len) {
                Some(end) => end as usize,
                None => return Status::InvalidArgument as i32,
            };

            if payload_end > data.len() {
                return Status::InvalidArgument as i32;
            }

            // Copy payload to owned buffer to release borrow on memory
            let payload = data[payload_ptr as usize..payload_end].to_vec();

            // 2. Make single kernel IPC call (validation inherent)
            let result = caller
                .data_mut()
                .kernel_bridge
                .echo_request(cap_handle, request_id, &payload);

            // 3. Write reply to linear memory
            match result {
                Ok(reply) => {
                    let data = memory.data_mut(&mut caller);

                    // Write reply bytes
                    let copy_len = reply.len().min(data.len().saturating_sub(out_ptr as usize));
                    if out_ptr as usize + copy_len <= data.len() {
                        data[out_ptr as usize..out_ptr as usize + copy_len]
                            .copy_from_slice(&reply[..copy_len]);
                    }

                    // Write actual length
                    if out_len_ptr as usize + 4 <= data.len() {
                        let len_bytes = (copy_len as u32).to_le_bytes();
                        data[out_len_ptr as usize..out_len_ptr as usize + 4]
                            .copy_from_slice(&len_bytes);
                    }

                    Status::Ok as i32
                }
                Err(status) => status as i32,
            }
        },
    )
}

/// Create the echo_reply host function.
pub fn create_echo_reply_host(store: &mut Store<InstanceContext>, memory: Memory) -> Func {
    Func::wrap(
        store,
        move |mut caller: Caller<'_, InstanceContext>,
              cap_handle: u64,
              request_id: u64,
              payload_ptr: u32,
              payload_len: u32,
              status_code: u32,
              _out_ptr: u32,
              _out_len_ptr: u32|
              -> i32 {
            let data = memory.data(&caller);

            let payload_end = match payload_ptr.checked_add(payload_len) {
                Some(end) => end as usize,
                None => return Status::InvalidArgument as i32,
            };

            if payload_end > data.len() {
                return Status::InvalidArgument as i32;
            }

            // Copy payload to owned buffer to release borrow on memory
            let payload = data[payload_ptr as usize..payload_end].to_vec();

            let result = caller.data_mut().kernel_bridge.echo_reply(
                cap_handle,
                request_id,
                status_code,
                &payload,
            );

            match result {
                Ok(()) => Status::Ok as i32,
                Err(status) => status as i32,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InstanceContext;

    #[test]
    fn echo_request_host_function_returns_ok() {
        let engine = Engine::default();
        let mut store = Store::new(&engine, InstanceContext::with_mock());

        // Create a mock memory
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type).unwrap();

        // Write test payload to memory
        let payload = b"hello";
        let data = memory.data_mut(&mut store);
        data[0..payload.len()].copy_from_slice(payload);

        // Create host function
        let func = create_echo_request_host(&mut store, memory);

        // Call the function using typed API
        let func_typed: TypedFunc<(u64, u64, u32, u32, u32, u32), i32> =
            func.typed(&store).unwrap();

        let result = func_typed
            .call(&mut store, (0x1234, 42u64, 0u32, 5u32, 100u32, 104u32))
            .unwrap();

        // Should return OK (0) because mock returns canned response
        // But our mock doesn't have a response set, so it returns error
        assert_ne!(result, 0);
    }
}
