//! Private SW0 validator worker. No filesystem, network or process imports in WASM.
use artifact_store_schema::agent_task::{ValidationOutcomeV0, ValidatorJobV0, ValidatorResultV0};
use std::io::{Read, Write};
use std::time::Duration;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
struct Context {
    limits: StoreLimits,
}
fn load_blob(root: &str, id: &str, limit: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let parsed = artifact_store_schema::ContentId::parse(id)?;
    let mut bytes = Vec::new();
    std::fs::File::open(std::path::Path::new(root).join(format!("{}.blob", parsed.hash_hex())))?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit || format!("sha256:{}", hex::encode(Sha256::digest(&bytes))) != id
    {
        return Err("CAS pin mismatch".into());
    }
    Ok(bytes)
}
fn run(job: ValidatorJobV0) -> Result<ValidatorResultV0, Box<dyn std::error::Error>> {
    job.budget.validate()?;
    if job.schema_version != 1 || job.store_root.len() > 4096 {
        return Err("invalid worker job".into());
    }
    let validator = load_blob(&job.store_root, &job.validator_id, 1_048_576)?;
    let candidate = load_blob(&job.store_root, &job.candidate_id, 65_536)?;
    let schema = load_blob(&job.store_root, &job.schema_id, 65_536)?;
    // Finite CPU/memory/process resources apply before compilation. Linux rlimits
    // are containment limits, not the task authority boundary or a full sandbox.
    unsafe {
        let cpu = libc::rlimit {
            rlim_cur: job.budget.wall_ms.div_ceil(1000) + 1,
            rlim_max: job.budget.wall_ms.div_ceil(1000) + 1,
        };
        if libc::setrlimit(libc::RLIMIT_CPU, &cpu) != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        #[cfg(target_os = "linux")]
        {
            let memory = libc::rlimit {
                rlim_cur: 2 * 1024 * 1024 * 1024,
                rlim_max: 2 * 1024 * 1024 * 1024,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &memory) != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            // Wasmtime uses smaller reservations under this bounded address space.
        }
    }
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.static_memory_maximum_size(16 * 1024 * 1024);
    config.memory_guard_size(0);
    config.parallel_compilation(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &validator)?;
    let linker = Linker::new(&engine);
    let candidate = serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(&candidate)?)?;
    let schema = serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(&schema)?)?;
    if candidate.len() > 65_536 || schema.len() > 65_536 {
        return Err("normalized input bound".into());
    }
    let mut store = Store::new(
        &engine,
        Context {
            limits: StoreLimitsBuilder::new()
                .memory_size(16 * 1024 * 1024)
                .instances(1)
                .memories(1)
                .tables(1)
                .table_elements(1024)
                .build(),
        },
    );
    store.limiter(|context| &mut context.limits);
    store.set_epoch_deadline(1);
    let (cancel, wait) = std::sync::mpsc::channel();
    let clock = engine.clone();
    let guest_ms = job.budget.guest_ms;
    let timer = std::thread::spawn(move || {
        if wait.recv_timeout(Duration::from_millis(guest_ms)).is_err() {
            clock.increment_epoch();
        }
    });
    let guest_start = std::time::Instant::now();
    let outcome = (|| -> Result<i32, wasmtime::Error> {
        let instance = linker.instantiate(&mut store, &module)?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("validator requires memory"))?;
        let header = kernel_api::generated::agent_task_v1::ValidatorInputHeader {
            magic: kernel_api::agent_task_protocol::VALIDATOR_MEMORY_MAGIC,
            candidate_len: candidate.len() as u32,
            schema_len: schema.len() as u32,
            reserved: 0,
        };
        let mut env = kernel_api::ipc::Envelope::empty(14, 20);
        kernel_api::wire::write_payload(&mut env, &header)
            .map_err(|_| wasmtime::Error::msg("validator header"))?;
        memory.write(&mut store, 0, &env.payload[..16])?;
        memory.write(&mut store, 16, &candidate)?;
        memory.write(&mut store, 16 + candidate.len(), &schema)?;
        instance
            .get_typed_func::<(), i32>(&mut store, "_start")?
            .call(&mut store, ())
    })();
    let guest_elapsed_ms = guest_start.elapsed().as_millis() as u64;
    let _ = cancel.send(());
    let _ = timer.join();
    let outcome = match outcome {
        Ok(0) => ValidationOutcomeV0::Valid,
        Ok(_) => ValidationOutcomeV0::Invalid,
        Err(e) if e.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::Interrupt) => {
            ValidationOutcomeV0::Timeout
        }
        Err(_) => ValidationOutcomeV0::HostFailure,
    };
    let mut diagnostics = match outcome {
        ValidationOutcomeV0::Valid => Vec::new(),
        ValidationOutcomeV0::Invalid => {
            b"configuration does not satisfy the pinned equality schema".to_vec()
        }
        ValidationOutcomeV0::Timeout => b"validator exceeded guest deadline".to_vec(),
        ValidationOutcomeV0::HostFailure => b"validator could not complete".to_vec(),
    };
    let truncated = diagnostics.len() as u64 > job.budget.max_diagnostics_bytes;
    diagnostics.truncate(job.budget.max_diagnostics_bytes as usize);
    Ok(ValidatorResultV0 {
        schema_version: 1,
        outcome,
        diagnostics,
        truncated,
        guest_elapsed_ms,
    })
}
fn main() {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        std::io::stdin().take(5_000_001).read_to_end(&mut bytes)?;
        if bytes.len() > 5_000_000 {
            return Err("oversized job".into());
        }
        let result = run(serde_json::from_slice(&bytes)?)?;
        std::io::stdout().write_all(&serde_json::to_vec(&result)?)?;
        Ok(())
    })();
    if result.is_err() {
        std::process::exit(1);
    }
}
