//! Default-off typed editor host fixture. Volatile, in-process enforcement only.
mod client;
mod codec;
mod host;
#[cfg(feature = "editor_native_read_v0_dev")]
mod native_authority;
pub use client::*;
pub use codec::{EditorMessage, decode_envelope_wire, encode_envelope_wire};
pub use host::*;
#[cfg(feature = "editor_native_read_v0_dev")]
pub use native_authority::*;
