#![no_std]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

#[cfg(not(feature = "std"))]
pub use alloc::{string::String, string::ToString, vec::Vec};

#[cfg(feature = "std")]
pub use std::{string::String, string::ToString, vec::Vec};

#[cfg(not(feature = "std"))]
use alloc::string::ToString as _; // Ensure trait is in scope for .to_string()

/// Shared imports for schema modules (std and no_std).
pub mod prelude {
    pub use crate::{String, Vec};

    #[cfg(not(feature = "std"))]
    pub use alloc::{format, string::ToString, vec};

    #[cfg(feature = "std")]
    pub use std::{boxed::Box, format, string::ToString};
}

use core::fmt;
use serde::{Deserialize, Serialize};

pub const CONTENT_ID_PREFIX: &str = "sha256:";
const CONTENT_ID_HEX_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentId(String);

impl ContentId {
    pub fn parse(raw: &str) -> Result<Self, ContentIdError> {
        if !raw.starts_with(CONTENT_ID_PREFIX) {
            return Err(ContentIdError::new("content id must start with sha256:"));
        }
        let hex = &raw[CONTENT_ID_PREFIX.len()..];
        if hex.len() != CONTENT_ID_HEX_LEN {
            return Err(ContentIdError::new(
                "content id must be sha256 + 64 lowercase hex chars",
            ));
        }
        if !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ContentIdError::new("content id must be lowercase hex"));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hash_hex(&self) -> &str {
        &self.0[CONTENT_ID_PREFIX.len()..]
    }
}

impl core::str::FromStr for ContentId {
    type Err = ContentIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentIdError {
    msg: &'static str,
}

impl ContentIdError {
    pub fn new(msg: &'static str) -> Self {
        Self { msg }
    }
}

impl fmt::Display for ContentIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ContentIdError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub content_id: String,
    pub size_bytes: u64,
    pub kind: String,
    pub channels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<String>,
}

// Schema modules (types and validation only, no IO)
pub mod block_sector_trace; // S13.4: harness.block sector oracle trace schema
pub mod claim;
pub mod crash_context;
pub mod driver_protocol_trace; // S11.1: Oracle MMIO/PCI trace schema
pub mod evidence_policy;
pub mod execution_fabric; // S10.4: Execution fabric contracts
pub mod graduation;
pub mod minimal_policy;
pub mod native_wasm; // S10.1: Native WASM v0 manifest schema
pub mod net_packet_trace; // S11.5: harness.net packet oracle trace schema
pub mod observed_caps;
pub mod path; // V-007: Read-only path construction helpers
pub mod prereq_graph;
pub mod projection_storage; // S10.3: Semantic index + VFS projection schemas
pub mod queue_item;
pub mod semantic_state;
pub mod signature; // V-007 Phase 3: Manifest signature validation
pub mod trace;

// Re-export commonly used path helpers for convenience
#[cfg(feature = "std")]
pub use path::{blob_path_for, manifest_path_for};

#[cfg(test)]
mod tests;
