//! RamenOS SDK for Native WASM Development
//!
//! This crate provides the SDK for building RamenOS native WASM modules.
//! It includes:
//! - Capability handle types
//! - Status codes for WASM imports
//! - Linear memory helpers
//! - IDL-generated harness bindings (via codegen)
//!
//! # Example
//!
//! ```ignore
//! use ramen_sdk::{CapabilityHandle, Status};
//! use ramen_sdk::generated::harness_echo_v1::EchoV1Client;
//!
//! #[no_mangle]
//! pub extern "C" fn _start() {
//!     // Capability handles are provided at initialization
//!     let echo_cap = unsafe { RAMEN_CAP_ECHO_V1 };
//!     let client = EchoV1Client::from_cap(echo_cap);
//!
//!     match client.send(b"hello") {
//!         Ok(reply) => { /* handle reply */ }
//!         Err(Status::InvalidCapability) => { /* handle error */ }
//!         _ => {}
//!     }
//! }
//!
//! // Capability handles injected by runner
//! #[no_mangle]
//! pub static mut RAMEN_CAP_ECHO_V1: ramen_sdk::CapabilityHandle = ramen_sdk::CapabilityHandle::INVALID;
//! ```

#![no_std]
#![allow(clippy::too_many_arguments)] // generated harness clients mirror IDL field arity

pub mod allocator;
pub mod capability;
pub mod memory;
pub mod status;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: allocator::BumpAllocator = allocator::BumpAllocator::new();

// Re-export main types
pub use capability::CapabilityHandle;
pub use status::Status;

/// Generated IDL bindings (populated by codegen)
pub mod generated;
