use crate::{Status, context::InstanceContext, kernel_bridge::KernelBridgeOps};
use kernel_api::ipc::Envelope;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use wasmtime::{Engine, Linker, Module, Store};

struct CountingBridge {
    calls: Arc<AtomicUsize>,
    reply_len: usize,
}
impl KernelBridgeOps for CountingBridge {
    fn echo_request(&mut self, _: u64, _: u64, _: &[u8]) -> Result<Vec<u8>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![0x42; self.reply_len])
    }
    fn echo_reply(&mut self, _: u64, _: u64, _: u32, _: &[u8]) -> Result<(), Status> {
        unreachable!()
    }
    fn trace_read(&mut self, _: u64, _: u64, _: usize) -> Result<Vec<u8>, Status> {
        unreachable!()
    }
    fn trace_write(&mut self, _: u64, _: &[u8]) -> Result<(), Status> {
        unreachable!()
    }
    fn shmem_read(&mut self, _: u64, _: u64, _: usize) -> Result<Vec<u8>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![0x42; self.reply_len])
    }
    fn shmem_write(&mut self, _: u64, _: u64, bytes: &[u8]) -> Result<usize, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(bytes.len())
    }
    fn transact(&mut self, request: Envelope) -> Result<Envelope, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut reply = Envelope::empty(request.protocol, request.msg_type + 1);
        reply.payload_len = self.reply_len as u32;
        reply.payload[..self.reply_len].fill(0x42);
        Ok(reply)
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Read,
    Create,
    Write,
}

fn invoke(
    capacity: i32,
    out: u32,
    len_ptr: u32,
    reply_len: usize,
    operation: Operation,
) -> (i32, usize, Vec<u8>, Vec<u8>) {
    let engine = Engine::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut store = Store::new(
        &engine,
        InstanceContext::new(Box::new(CountingBridge {
            calls: calls.clone(),
            reply_len,
        })),
    );
    let mut linker = Linker::new(&engine);
    crate::generated::harness_shmem_control_v1_host::register_shared_memory_control_host(
        &mut linker,
    )
    .unwrap();
    let (import, call) = if matches!(operation, Operation::Create) {
        ("(import \"ramen::shared_memory.control\" \"create_region::call\" (func $call (param i64 i64 i64 i64 i32 i32 i32 i32) (result i32)))".to_string(),
         format!("(call $call (i64.const 1) (i64.const 1) (i64.const 1) (i64.const 4096) (i32.const 1) (i32.const 4096) (i32.const {out}) (i32.const {len_ptr}))"))
    } else if matches!(operation, Operation::Write) {
        ("(import \"ramen::shared_memory.control\" \"shmem_write::call\" (func $call (param i64 i64 i64 i64 i32 i32 i32) (result i32)))".to_string(),
         format!("(call $call (i64.const 1) (i64.const 1) (i64.const 0) (i64.const 512) (i32.const 4) (i32.const {out}) (i32.const {len_ptr}))"))
    } else {
        ("(import \"ramen::shared_memory.control\" \"shmem_read::call\" (func $call (param i64 i64 i64 i32 i32 i32) (result i32)))".to_string(),
         format!("(call $call (i64.const 1) (i64.const 1) (i64.const 0) (i32.const 4) (i32.const {out}) (i32.const {len_ptr}))"))
    };
    let module = Module::new(&engine, format!("(module {import} (memory (export \"memory\") 1) (func (export \"run\") (result i32) {call}))")).unwrap();
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    memory.data_mut(&mut store).fill(0xA5);
    if (len_ptr as usize) <= 65532 {
        memory
            .write(&mut store, len_ptr as usize, &capacity.to_le_bytes())
            .unwrap();
    }
    let before = memory.data(&store).to_vec();
    let status = instance
        .get_typed_func::<(), i32>(&mut store, "run")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    (
        status,
        calls.load(Ordering::SeqCst),
        before,
        memory.data(&store).to_vec(),
    )
}

#[test]
fn review_capacity_preserves_small_slice_and_adjacent_sentinels() {
    let (status, calls, before, after) = invoke(1, 256, 128, 4, Operation::Read);
    assert_eq!(status, Status::InvalidArgument as i32);
    assert_eq!(
        before, after,
        "output, adjacent state, and length must be unchanged"
    );
    assert_eq!(calls, 0, "undersized request must not reach the bridge");
}

#[test]
fn review_capacity_rejects_invalid_ranges_and_aliases_before_calls() {
    for (cap, out, len) in [
        (-1, 256, 128),
        (i32::MIN, 256, 128),
        (0, 256, 128),
        (4, 256, 65534),
        (65536, 256, 128),
        (4, 65534, 128),
        (4, 128, 128),
        (8, 124, 128),
    ] {
        let (status, calls, before, after) = invoke(cap, out, len, 4, Operation::Read);
        assert_eq!(status, Status::InvalidArgument as i32);
        assert_eq!(calls, 0);
        assert_eq!(before, after);
    }
}

#[test]
fn review_capacity_checks_typed_reply_before_side_effects() {
    let (status, calls, before, after) = invoke(1, 256, 128, 40, Operation::Create);
    assert_eq!(status, Status::InvalidArgument as i32);
    assert_eq!(
        calls, 0,
        "create_region must not execute with a too-small reply buffer"
    );
    assert_eq!(before, after);
}

#[test]
fn review_capacity_accepts_exact_and_larger_buffers() {
    for (cap, reply_len, generic) in [
        (4, 4, Operation::Read),
        (8, 4, Operation::Read),
        (40, 40, Operation::Create),
    ] {
        let (status, calls, mut expected, after) = invoke(cap, 256, 128, reply_len, generic);
        assert_eq!(status, Status::Ok as i32);
        assert_eq!(calls, 1);
        expected[256..256 + reply_len].fill(0x42);
        expected[128..132].copy_from_slice(&(reply_len as i32).to_le_bytes());
        assert_eq!(expected, after);
    }
}

#[test]
fn review_capacity_rechecks_unexpected_oversized_reply() {
    let (status, calls, before, after) = invoke(4, 256, 128, 8, Operation::Read);
    assert_eq!(status, Status::InvalidArgument as i32);
    assert_eq!(calls, 1);
    assert_eq!(before, after);
}

#[test]
fn review_capacity_write_preflight_and_zero_length_success() {
    for cap in [-1, 0, 4] {
        let (status, calls, mut expected, after) = invoke(cap, 256, 128, 0, Operation::Write);
        if cap < 0 {
            assert_eq!(status, Status::InvalidArgument as i32);
            assert_eq!(calls, 0);
        } else {
            assert_eq!(status, Status::Ok as i32);
            assert_eq!(calls, 1);
            expected[128..132].copy_from_slice(&0i32.to_le_bytes());
        }
        assert_eq!(expected, after);
    }
}

#[test]
fn review_capacity_legacy_echo_uses_same_output_boundary() {
    for capacity in [1i32, 4] {
        let engine = Engine::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut store = Store::new(
            &engine,
            InstanceContext::new(Box::new(CountingBridge {
                calls: calls.clone(),
                reply_len: 4,
            })),
        );
        let memory = wasmtime::Memory::new(&mut store, wasmtime::MemoryType::new(1, None)).unwrap();
        memory.data_mut(&mut store).fill(0xA5);
        memory
            .write(&mut store, 128, &capacity.to_le_bytes())
            .unwrap();
        let mut expected = memory.data(&store).to_vec();
        let func = crate::harness::echo::create_echo_request_host(&mut store, memory);
        let func = func
            .typed::<(u64, u64, u32, u32, u32, u32), i32>(&store)
            .unwrap();
        let status = func.call(&mut store, (1, 1, 512, 4, 256, 128)).unwrap();
        if capacity == 1 {
            assert_eq!(status, Status::InvalidArgument as i32);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(status, Status::Ok as i32);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            expected[256..260].fill(0x42);
        }
        assert_eq!(expected, memory.data(&store));
    }
}
