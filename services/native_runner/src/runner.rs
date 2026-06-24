//! Native runner execution logic.
//!
//! This module provides the `NativeRunner`, `RunConfig`, `RunResult`,
//! and `RunnerConfig` types for WASM module execution with capability injection.

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn run_config_accepts_granted_handles() {
        let config = RunConfig {
            granted_handles: HashMap::from([("RAMEN_CAP_ECHO_REQUEST".to_string(), 0x1234)]),
        };
        assert_eq!(
            config.granted_handles.get("RAMEN_CAP_ECHO_REQUEST"),
            Some(&0x1234)
        );
    }

    #[test]
    fn runner_config_has_kernel_ipc_path() {
        let config = RunnerConfig {
            kernel_ipc: "/run/ramen/kernel.sock".into(),
            kernel_ipc_transport: KernelIpcTransport::default(),
            trace_output: None,
        };
        assert_eq!(config.kernel_ipc.to_str(), Some("/run/ramen/kernel.sock"));
    }

    #[test]
    fn run_config_default_is_empty() {
        let config = RunConfig::default();
        assert!(config.granted_handles.is_empty());
    }

    #[test]
    fn native_runner_for_testing_creates_instance() {
        let runner = NativeRunner::for_testing();
        assert!(runner.config.kernel_ipc.to_string_lossy().contains("null"));
    }

    #[test]
    fn loaded_module_hides_internal_details() {
        // LoadedModule should be opaque - we can't inspect the module directly
        // This test verifies the type exists and can be created
        let runner = NativeRunner::for_testing();

        // Minimal valid WASM module (empty module)
        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let result = runner.load(&wasm_bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn runner_loads_minimal_wasm() {
        let runner = NativeRunner::for_testing();

        // Minimal WASM module
        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let result = runner.load(&wasm_bytes);

        assert!(result.is_ok());
    }

    #[test]
    fn runner_loads_wasm_with_start() {
        let runner = NativeRunner::for_testing();

        // WASM module with _start function that returns 0
        let wasm_bytes = wat::parse_str(
            r#"(module
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let module = runner.load(&wasm_bytes);
        assert!(module.is_ok());

        let result = runner.run(module.unwrap(), RunConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 0);
    }

    #[test]
    fn runner_injects_capability_globals() {
        let runner = NativeRunner::for_testing();

        // WASM module with capability globals (must be mutable for injection)
        let wasm_bytes = wat::parse_str(
            r#"(module
                (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let module = runner.load(&wasm_bytes).unwrap();

        let config = RunConfig {
            granted_handles: HashMap::from([("RAMEN_CAP_ECHO_REQUEST".to_string(), 0xABCD)]),
        };

        let result = runner.run(module, config);
        assert!(result.is_ok());
    }

    #[test]
    fn runner_fails_on_missing_capability() {
        let runner = NativeRunner::for_testing();

        // WASM module with capability global that we won't provide (must be mutable)
        let wasm_bytes = wat::parse_str(
            r#"(module
                (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let module = runner.load(&wasm_bytes).unwrap();

        // Don't provide the required capability
        let result = runner.run(module, RunConfig::default());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::RunnerError::MissingCapability(_)));
    }

    #[test]
    fn runner_injects_multiple_capabilities() {
        let runner = NativeRunner::for_testing();

        // WASM module with multiple capability globals (must be mutable)
        let wasm_bytes = wat::parse_str(
            r#"(module
                (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
                (global $RAMEN_CAP_ECHO_REPLY (export "RAMEN_CAP_ECHO_REPLY") (mut i64) (i64.const 0))
                (global $RAMEN_CAP_TRACE_WRITE (export "RAMEN_CAP_TRACE_WRITE") (mut i64) (i64.const 0))
                (func (export "_start") (result i32)
                    i32.const 0
                )
            )"#,
        )
        .unwrap();

        let module = runner.load(&wasm_bytes).unwrap();

        let config = RunConfig {
            granted_handles: HashMap::from([
                ("RAMEN_CAP_ECHO_REQUEST".to_string(), 0x1000),
                ("RAMEN_CAP_ECHO_REPLY".to_string(), 0x2000),
                ("RAMEN_CAP_TRACE_WRITE".to_string(), 0x3000),
            ]),
        };

        let result = runner.run(module, config);
        assert!(result.is_ok());
    }

    #[test]
    fn runner_fails_on_wasm_compile_error() {
        let runner = NativeRunner::for_testing();

        // Invalid WASM bytes
        let invalid_bytes = b"\x00invalid wasm";

        let result = runner.load(invalid_bytes);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::RunnerError::WasmCompile(_)));
    }

    #[test]
    fn runner_fails_on_missing_start() {
        let runner = NativeRunner::for_testing();

        // WASM module without _start function
        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let module = runner.load(&wasm_bytes).unwrap();

        let result = runner.run(module, RunConfig::default());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::RunnerError::WasmInstantiate(_)));
    }

    #[test]
    fn load_and_run_combines_operations() {
        let runner = NativeRunner::for_testing();

        let wasm_bytes = wat::parse_str(
            r#"(module
                (func (export "_start") (result i32)
                    i32.const 42
                )
            )"#,
        )
        .unwrap();

        let result = runner.load_and_run(&wasm_bytes, RunConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 42);
    }
}

use crate::RunnerError;
use crate::context::InstanceContext;
use std::collections::HashMap;
use std::path::PathBuf;
use wasmtime::*;

/// Host↔kernel IPC transport (S10.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KernelIpcTransport {
    /// Raw 88-byte envelopes over a Unix domain socket (KernelHarnessProxy path).
    #[default]
    UnixSocket,
    /// Length-prefixed envelopes over a QEMU chardev socket.
    ChardevSerial,
}

/// Configuration for the NativeRunner itself.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Path to the kernel IPC socket.
    pub kernel_ipc: PathBuf,

    /// Wire framing for `kernel_ipc` (default: Unix socket).
    pub kernel_ipc_transport: KernelIpcTransport,

    /// Optional path for trace output.
    pub trace_output: Option<PathBuf>,
}

/// Configuration for a specific run invocation.
///
/// This contains the capability handles granted by the broker.
/// The runner is executor-only: it does not decide what to grant.
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Capability handles granted by the broker.
    /// Key is the global name (e.g., "RAMEN_CAP_ECHO_REQUEST").
    /// Value is the capability handle.
    pub granted_handles: HashMap<String, u64>,
}

/// Result of running a WASM module.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Exit code returned by the WASM module.
    pub exit_code: i32,

    /// Captured stdout from execution.
    pub stdout: Vec<u8>,

    /// Optional trace data from execution.
    pub trace: Option<Vec<u8>>,
}

/// A loaded WASM module ready for execution.
#[derive(Debug)]
pub struct LoadedModule {
    module: Module,
}

/// Native WASM runner with capability injection.
///
/// This is the core executor for RamenOS native workloads.
/// It loads WASM modules, injects capabilities via exported globals,
/// and executes the module's _start function.
pub struct NativeRunner {
    /// Runner configuration (used for real kernel IPC, not mock bridge).
    #[allow(dead_code)]
    config: RunnerConfig,
    engine: Engine,
}

impl NativeRunner {
    /// Create a new runner with the given configuration.
    pub fn new(config: RunnerConfig) -> Result<Self, RunnerError> {
        Ok(Self {
            config,
            engine: Engine::default(),
        })
    }

    /// Create a runner for testing purposes.
    ///
    /// Uses /dev/null for kernel IPC path since tests don't need real IPC.
    pub fn for_testing() -> Self {
        Self {
            config: RunnerConfig {
                kernel_ipc: "/dev/null".into(),
                kernel_ipc_transport: KernelIpcTransport::default(),
                trace_output: None,
            },
            engine: Engine::default(),
        }
    }

    /// Load a WASM module from bytes.
    ///
    /// This compiles the WASM but does not instantiate it.
    /// The returned `LoadedModule` can be run multiple times with different configs.
    pub fn load(&self, wasm_bytes: &[u8]) -> Result<LoadedModule, RunnerError> {
        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| RunnerError::WasmCompile(e.to_string()))?;
        Ok(LoadedModule { module })
    }

    /// Run a loaded WASM module with the given configuration.
    ///
    /// This:
    /// 1. Creates an instance context with either real or mock kernel bridge
    /// 2. Instantiates the module with host functions
    /// 3. Injects capability handles into exported globals
    /// 4. Calls the _start function
    /// 5. Returns the exit code and captured output
    pub fn run(&self, module: LoadedModule, config: RunConfig) -> Result<RunResult, RunnerError> {
        // Create appropriate kernel bridge (real or mock)
        let bridge: Box<dyn crate::KernelBridgeOps> =
            if self.config.kernel_ipc.to_string_lossy() == "/dev/null" {
                Box::new(crate::kernel_bridge::MockKernelBridge::new())
            } else if self.config.kernel_ipc_transport == KernelIpcTransport::ChardevSerial {
                Box::new(crate::kernel_bridge::ChardevKernelBridge::new(
                    self.config.kernel_ipc.clone(),
                ))
            } else {
                Box::new(crate::kernel_bridge::KernelBridge::new(
                    self.config.kernel_ipc.clone(),
                ))
            };

        let context = InstanceContext::new(bridge);
        let mut store = Store::new(&self.engine, context);
        let mut linker = Linker::new(&self.engine);

        // Create memory for the instance
        let memory_type = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;

        // Register generated host functions (bridged to kernel)
        crate::generated::harness_echo_v1_host::register_harness_echo_host(&mut linker, memory)
            .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;
        crate::generated::harness_trace_v2_host::register_harness_trace_host(&mut linker, memory)
            .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;
        crate::generated::harness_shmem_control_v1_host::register_shared_memory_control_host(
            &mut linker,
            memory,
        )
        .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;
        crate::generated::services_semantic_state_v1_host::register_services_semantic_state_host(
            &mut linker,
            memory,
        )
        .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &module.module)
            .map_err(|e| RunnerError::WasmInstantiate(e.to_string()))?;

        // Inject capabilities into exported globals
        inject_capabilities(&mut store, &instance, &config.granted_handles)?;

        // Find and call _start
        let start = instance
            .get_export(&mut store, "_start")
            .and_then(|e| e.into_func())
            .ok_or_else(|| RunnerError::WasmInstantiate("_start function not found".to_string()))?;

        // Call _start which returns i32 exit code
        let start_typed: TypedFunc<(), i32> = start
            .typed(&store)
            .map_err(|e| RunnerError::WasmInstantiate(format!("_start has wrong type: {}", e)))?;

        let exit_code = start_typed
            .call(&mut store, ())
            .map_err(|e| RunnerError::HarnessCall(e.to_string()))?;

        // Extract captured output from context
        let context = store.into_data();
        Ok(RunResult {
            exit_code,
            stdout: context.stdout,
            trace: None,
        })
    }

    /// Load and run a WASM module in one step.
    ///
    /// Convenience method for when you don't need to reuse the module.
    pub fn load_and_run(
        &self,
        wasm_bytes: &[u8],
        config: RunConfig,
    ) -> Result<RunResult, RunnerError> {
        let module = self.load(wasm_bytes)?;
        self.run(module, config)
    }
}

/// Inject capability handles into exported globals.
///
/// This is the fail-closed capability injection: if a global is exported
/// but not provided in granted_handles, we return an error.
fn inject_capabilities(
    store: &mut Store<InstanceContext>,
    instance: &Instance,
    granted_handles: &HashMap<String, u64>,
) -> Result<(), RunnerError> {
    // Collect the exports first to avoid borrow issues
    let exports: Vec<_> = instance
        .exports(&mut *store)
        .filter_map(|export| {
            let name = export.name().to_string();
            if !name.starts_with("RAMEN_CAP_") {
                return None;
            }
            let global = export.into_global()?;
            Some((name, global))
        })
        .collect();

    for (name, global) in exports {
        // Look up the granted handle - fail closed if missing
        let handle = granted_handles.get(&name).ok_or_else(|| {
            RunnerError::MissingCapability(format!("Required capability '{}' not granted", name))
        })?;

        // Set the global value
        global
            .set(&mut *store, Val::I64(*handle as i64))
            .map_err(|e| RunnerError::GlobalSet(format!("{}: {}", name, e)))?;
    }

    Ok(())
}
