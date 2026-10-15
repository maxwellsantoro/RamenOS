//! Explicit native-layout payload codecs. No Rust object representation is read.
//! Integers retain native byte order and repr(C) offsets for existing peers.
//! Implicit padding is ignored on decode and initialized to zero on encode.

use crate::ipc::Envelope;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    PayloadTooLarge,
    PayloadTooSmall,
    PayloadLenMismatch,
    PayloadLenInvalid,
}

/// A value with an explicit safe field codec. `Copy` alone is insufficient.
/// Implementations construct valid Rust values from fields, never raw object bytes.
pub trait WirePayload: Sized {
    const WIRE_SIZE: usize;
    fn encode(&self, out: &mut [u8]) -> Result<(), WireError>;
    fn decode(input: &[u8]) -> Result<Self, WireError>;
}

macro_rules! integer_codec {
    ($($ty:ty),* $(,)?) => {$ (
        impl WirePayload for $ty {
            const WIRE_SIZE: usize = core::mem::size_of::<Self>();
            fn encode(&self, out: &mut [u8]) -> Result<(), WireError> {
                if out.len() != Self::WIRE_SIZE { return Err(WireError::PayloadLenMismatch); }
                out.copy_from_slice(&self.to_ne_bytes());
                Ok(())
            }
            fn decode(input: &[u8]) -> Result<Self, WireError> {
                let bytes = input.try_into().map_err(|_| WireError::PayloadLenMismatch)?;
                Ok(Self::from_ne_bytes(bytes))
            }
        }
    )*};
}
integer_codec!(u8, u16, u32, u64);

impl<const N: usize> WirePayload for [u8; N] {
    const WIRE_SIZE: usize = N;
    fn encode(&self, out: &mut [u8]) -> Result<(), WireError> {
        if out.len() != N {
            return Err(WireError::PayloadLenMismatch);
        }
        out.copy_from_slice(self);
        Ok(())
    }
    fn decode(input: &[u8]) -> Result<Self, WireError> {
        input.try_into().map_err(|_| WireError::PayloadLenMismatch)
    }
}

/// Implements field codecs for IDL-generated records without touching padding.
/// Each declared field must itself have an explicit codec. Offsets and total
/// lengths preserve the existing native ABI; fields, widths and versions remain
/// owned by IDL. This macro contains no unsafe object-representation conversion.
#[macro_export]
macro_rules! impl_wire_payload {
    ($name:ty { $($field:ident : $ty:ty),* $(,)? }) => {
        impl $crate::wire::WirePayload for $name {
            const WIRE_SIZE: usize = core::mem::size_of::<Self>();
            fn encode(&self, out: &mut [u8]) -> Result<(), $crate::wire::WireError> {
                if out.len() != Self::WIRE_SIZE {
                    return Err($crate::wire::WireError::PayloadLenMismatch);
                }
                out.fill(0);
                $(
                    let offset = core::mem::offset_of!(Self, $field);
                    let end = offset + <$ty as $crate::wire::WirePayload>::WIRE_SIZE;
                    <$ty as $crate::wire::WirePayload>::encode(&self.$field, &mut out[offset..end])?;
                )*
                Ok(())
            }
            fn decode(input: &[u8]) -> Result<Self, $crate::wire::WireError> {
                if input.len() != Self::WIRE_SIZE {
                    return Err($crate::wire::WireError::PayloadLenMismatch);
                }
                Ok(Self {
                    $( $field: {
                        let offset = core::mem::offset_of!(Self, $field);
                        let end = offset + <$ty as $crate::wire::WirePayload>::WIRE_SIZE;
                        <$ty as $crate::wire::WirePayload>::decode(&input[offset..end])?
                    }, )*
                })
            }
        }
    };
}

pub fn write_payload<T: WirePayload>(env: &mut Envelope, value: &T) -> Result<(), WireError> {
    let len = T::WIRE_SIZE;
    if len > env.payload.len() {
        return Err(WireError::PayloadTooLarge);
    }
    // Publish only a complete encoding. A failing codec preserves the envelope.
    let mut payload = [0u8; 64];
    value.encode(&mut payload[..len])?;
    env.payload = payload;
    env.payload_len = len as u32;
    Ok(())
}

/// Decode only explicitly implemented wire values; arbitrary `Copy` values are
/// not evidence of a valid representation for attacker-supplied bytes.
///
/// ```compile_fail
/// use kernel_api::{ipc::Envelope, wire::read_payload};
/// let _: bool = read_payload(&Envelope::empty(1, 1)).unwrap();
/// ```
///
/// ```compile_fail
/// use kernel_api::{ipc::Envelope, wire::read_payload};
/// #[derive(Copy, Clone)]
/// enum Untrusted { One, Two }
/// let _: Untrusted = read_payload(&Envelope::empty(1, 1)).unwrap();
/// ```
///
/// ```compile_fail
/// use kernel_api::{ipc::Envelope, wire::read_payload};
/// let _: &'static u8 = read_payload(&Envelope::empty(1, 1)).unwrap();
/// ```
///
/// ```compile_fail
/// use kernel_api::{ipc::Envelope, wire::write_payload};
/// #[derive(Copy, Clone)]
/// struct Arbitrary { tag: u8, value: u64 }
/// write_payload(&mut Envelope::empty(1, 1), &Arbitrary { tag: 1, value: 2 }).unwrap();
/// ```
pub fn read_payload<T: WirePayload>(env: &Envelope) -> Result<T, WireError> {
    let len = T::WIRE_SIZE;
    let payload_len = env.payload_len as usize;
    if payload_len > env.payload.len() {
        return Err(WireError::PayloadLenInvalid);
    }
    if payload_len < len {
        return Err(WireError::PayloadTooSmall);
    }
    if payload_len > len {
        return Err(WireError::PayloadLenMismatch);
    }
    T::decode(&env.payload[..len])
}

pub fn payload_len_ok(env: &Envelope) -> bool {
    (env.payload_len as usize) <= env.payload.len()
}
