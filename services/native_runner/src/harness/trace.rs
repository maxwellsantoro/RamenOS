//! Trace harness host functions for WASM modules.
//!
//! These functions provide access to trace data while maintaining security bounds:
//! - Capability validation via kernel bridge
//! - Bounds-checked memory access
//! - Single kernel IPC crossing

use crate::context::InstanceContext;
use crate::error::Status;
use wasmtime::*;

pub fn create_trace_read_host(store: &mut Store<InstanceContext>, memory: Memory) -> Func {
    Func::wrap(
        store,
        move |mut caller: Caller<'_, InstanceContext>,
              cap_handle: u64,
              offset: u64,
              out_ptr: u32,
              out_cap: u32,
              out_len_ptr: u32|
              -> i32 {
            let result =
                caller
                    .data_mut()
                    .kernel_bridge
                    .trace_read(cap_handle, offset, out_cap as usize);
            match result {
                Ok(data) => {
                    let mem_data = memory.data_mut(&mut caller);
                    let copy_len = data.len().min(out_cap as usize);
                    if out_ptr as usize + copy_len <= mem_data.len() {
                        mem_data[out_ptr as usize..out_ptr as usize + copy_len]
                            .copy_from_slice(&data[..copy_len]);
                    }
                    if out_len_ptr as usize + 4 <= mem_data.len() {
                        mem_data[out_len_ptr as usize..out_len_ptr as usize + 4]
                            .copy_from_slice(&(copy_len as u32).to_le_bytes());
                    }
                    Status::Ok as i32
                }
                Err(status) => status as i32,
            }
        },
    )
}

pub fn create_trace_write_host(store: &mut Store<InstanceContext>, memory: Memory) -> Func {
    Func::wrap(
        store,
        move |mut caller: Caller<'_, InstanceContext>,
              cap_handle: u64,
              data_ptr: u32,
              data_len: u32|
              -> i32 {
            let data = memory.data(&caller);
            let data_end = match data_ptr.checked_add(data_len) {
                Some(e) => e as usize,
                None => return Status::InvalidArgument as i32,
            };
            if data_end > data.len() {
                return Status::InvalidArgument as i32;
            }
            let trace_data = data[data_ptr as usize..data_end].to_vec();
            match caller
                .data_mut()
                .kernel_bridge
                .trace_write(cap_handle, &trace_data)
            {
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
    fn trace_read_host_function_returns_data() {
        let engine = Engine::default();
        let mut store = Store::new(&engine, InstanceContext::with_mock());

        // Create a mock memory
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type).unwrap();

        // Create host function
        let func = create_trace_read_host(&mut store, memory);

        // Call the function using typed API
        let func_typed: TypedFunc<(u64, u64, u32, u32, u32), i32> = func.typed(&store).unwrap();

        let result = func_typed
            .call(&mut store, (0x1234, 0u64, 0u32, 64u32, 68u32))
            .unwrap();

        // Should return OK (0) because mock returns data
        assert_eq!(result, 0);
    }

    #[test]
    fn trace_write_host_function_returns_ok() {
        let engine = Engine::default();
        let mut store = Store::new(&engine, InstanceContext::with_mock());

        // Create a mock memory
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type).unwrap();

        // Write test data to memory
        let data = memory.data_mut(&mut store);
        let test_data = b"trace data";
        data[0..test_data.len()].copy_from_slice(test_data);

        // Create host function
        let func = create_trace_write_host(&mut store, memory);

        // Call the function using typed API
        let func_typed: TypedFunc<(u64, u32, u32), i32> = func.typed(&store).unwrap();

        let result = func_typed.call(&mut store, (0x1234, 0u32, 9u32)).unwrap();

        // Should return OK (0)
        assert_eq!(result, 0);
    }
}
