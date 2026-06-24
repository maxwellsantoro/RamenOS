//! Native Runner Binary
//!
//! Command-line interface for executing WASM modules with RamenOS-native semantics.

use clap::Parser;
use native_runner::{KernelIpcTransport, NativeRunner, RunConfig, RunnerConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "native_runner")]
#[command(about = "Execute RamenOS native WASM modules")]
struct Args {
    /// Path to WASM module.
    #[arg(short, long)]
    wasm: PathBuf,

    /// Path to kernel IPC socket.
    #[arg(short, long)]
    kernel_ipc: PathBuf,

    /// Grant capability (repeatable, format: NAME=HANDLE).
    #[arg(short, long = "cap")]
    capabilities: Vec<String>,

    /// Path to write execution trace.
    #[arg(short, long)]
    trace_output: Option<PathBuf>,

    /// Enable verbose output.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.verbose {
        eprintln!("[native_runner] Loading WASM: {:?}", args.wasm);
    }

    let wasm_bytes = match std::fs::read(&args.wasm) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading WASM file: {}", e);
            return ExitCode::from(1);
        }
    };

    let mut granted_handles = HashMap::new();
    for cap in &args.capabilities {
        let parts: Vec<&str> = cap.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("Invalid capability format: {} (expected NAME=HANDLE)", cap);
            return ExitCode::from(1);
        }
        let name = parts[0].to_string();
        let handle: u64 = match u64::from_str_radix(parts[1].trim_start_matches("0x"), 16) {
            Ok(h) => h,
            Err(_) => match parts[1].parse::<u64>() {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("Invalid handle value: {}", parts[1]);
                    return ExitCode::from(1);
                }
            },
        };
        if args.verbose {
            eprintln!("[native_runner] Granting {}: {:#x}", name, handle);
        }
        granted_handles.insert(name, handle);
    }

    let config = RunnerConfig {
        kernel_ipc: args.kernel_ipc,
        kernel_ipc_transport: KernelIpcTransport::default(),
        trace_output: args.trace_output,
    };

    let runner = match NativeRunner::new(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error creating runner: {}", e);
            return ExitCode::from(1);
        }
    };

    let run_config = RunConfig { granted_handles };
    let result = match runner.load_and_run(&wasm_bytes, run_config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error executing WASM: {}", e);
            return ExitCode::from(1);
        }
    };

    if !result.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&result.stdout));
    }

    ExitCode::from(result.exit_code as u8)
}
