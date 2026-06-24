use core::mem::{MaybeUninit, size_of};

use crate::ipc::Envelope;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    PayloadTooLarge,
    PayloadTooSmall,
    PayloadLenMismatch,
    PayloadLenInvalid,
}

pub fn write_payload<T: Copy>(env: &mut Envelope, value: &T) -> Result<(), WireError> {
    let len = size_of::<T>();
    if len > env.payload.len() {
        return Err(WireError::PayloadTooLarge);
    }

    let src = unsafe { core::slice::from_raw_parts((value as *const T).cast::<u8>(), len) };
    env.payload[..len].copy_from_slice(src);
    if len < env.payload.len() {
        env.payload[len..].fill(0);
    }

    env.payload_len = len as u32;
    Ok(())
}

pub fn read_payload<T: Copy>(env: &Envelope) -> Result<T, WireError> {
    let len = size_of::<T>();
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

    let mut out = MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(env.payload.as_ptr(), out.as_mut_ptr().cast::<u8>(), len);
        Ok(out.assume_init())
    }
}

pub fn payload_len_ok(env: &Envelope) -> bool {
    (env.payload_len as usize) <= env.payload.len()
}
