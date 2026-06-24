// V-007 Phase 2: Store Service Library
//
// Public API for store service client library.
// Other services (domain_manager, runtime_supervisor, store_cli) use this
// to communicate with the store service via Unix domain sockets.
//
// V-007 Phase 3: Added audit logging and access control modules (internal use).
// V-007 Phase 5: Added domain-scoped artifact visibility module.

pub mod client;
pub mod frame;
pub mod status;

// Export modules so bin and other crates share one implementation.
pub mod access_control;
pub mod audit;
pub mod capability;
pub mod dev_mode;
pub mod domain_visibility;
pub mod projection_cow;
pub mod projection_index;
pub mod projection_vfs;

// Re-export common types for convenience
pub use client::{StoreClient, StoreClientError};

// Re-export status codes for convenience
pub use status::*;

// V-007 Phase 5: Export domain-scoped artifact visibility types
// Note: DomainArtifactRegistry is scaffold-only in Task 2.
// Full integration with request handlers will happen in Task 5.
pub use domain_visibility::{ArtifactOwner, DomainArtifactRegistry};
