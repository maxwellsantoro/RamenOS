//! V-012 Phase 5: User-space trace service client library
//!
//! This crate provides a client library for services to access kernel trace
//! buffers through the trace_service_v1 IPC protocol. It supports domain-scoped
//! trace reading with capability-based access control.
//!
//! # Architecture
//!
//! The trace client connects to the kernel's trace service via IPC and uses
//! capability handles to authorize trace operations. Each client is bound to
//! a specific domain ID and can only access traces for that domain.
//!
//! # Example
//!
//! ```no_run
//! use trace_client::{TraceClient, TraceClientError};
//!
//! // Connect to trace service for domain 42
//! let mut client = TraceClient::connect(42)?;
//!
//! // Read trace data
//! let mut buf = [0u8; 1024];
//! let n = client.read_trace(&mut buf)?;
//!
//! // Get trace buffer info
//! let info = client.get_info()?;
//! println!("Domain {} has {} bytes of trace data", info.domain_id, info.write_offset - info.read_offset);
//! # Ok::<(), TraceClientError>(())
//! ```

pub mod capability;
pub mod client;
pub mod error;
pub mod ipc;

// Re-export main types at crate root
pub use capability::TraceCapability;
pub use client::{TraceClient, TraceClientBuilder};
pub use error::TraceClientError;
pub use ipc::{MockTransport, TraceTransport};

/// Re-export kernel_api types needed by consumers
pub use kernel_api::cap::Handle;
