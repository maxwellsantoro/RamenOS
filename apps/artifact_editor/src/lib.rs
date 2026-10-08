//! Default-off host composition consumer for the native artifact editor task.
//!
//! The gate-first task assertions precede its implementation. The concrete
//! Desktop and Store owners retain authority; this crate adds no public text,
//! receipt, or Save-state setter. A separate editor process remains UI1.1d.

#[cfg(feature = "host_native_save_v0_dev")]
mod native;
#[cfg(feature = "host_native_save_v0_dev")]
pub use native::{NativeArtifactEditor, NativeEditorError};
