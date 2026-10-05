//! Default-off typed editor host fixture. Volatile, in-process enforcement only.
mod client;
mod codec;
mod host;
pub use client::*;
pub use codec::{EditorMessage, decode_envelope_wire, encode_envelope_wire};
pub use host::*;
