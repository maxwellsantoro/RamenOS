// services/native_runner/src/harness/mod.rs
//! Harness host functions for WASM modules.

pub mod echo;
pub mod trace;

pub use echo::{create_echo_reply_host, create_echo_request_host};
pub use trace::{create_trace_read_host, create_trace_write_host};
