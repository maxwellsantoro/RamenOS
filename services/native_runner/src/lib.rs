// services/native_runner/src/lib.rs
//! Native Runner - WASM executor for RamenOS native workloads.
//!
//! This crate provides the execution layer for WebAssembly modules
//! with RamenOS-native semantics. It is executor-only: no policy
//! decisions, no capability grants. Those are handled by the broker.

#[cfg(test)]
mod capacity_tests;
pub mod context;
pub mod error;
pub mod generated;
mod guest_memory;
pub mod harness;
pub mod kernel_bridge;
pub mod runner;

pub use context::InstanceContext;
pub use error::{RunnerError, Status};
pub use kernel_bridge::{
    ChardevKernelBridge, KernelBridge, KernelBridgeOps, KernelCall, MockKernelBridge,
};
pub use runner::{
    KernelIpcTransport, LoadedModule, NativeRunner, RunConfig, RunResult, RunnerConfig,
};
