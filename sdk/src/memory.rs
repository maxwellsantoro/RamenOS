//! Linear memory helpers for WASM modules
//!
//! These utilities help with reading and writing data in WASM linear memory
//! when communicating with harness imports.

#![allow(dead_code)]

use core::slice;

/// A borrowed slice in linear memory
#[repr(C)]
pub struct LinearSlice {
    pub ptr: *const u8,
    pub len: usize,
}

impl LinearSlice {
    /// Create from a byte slice
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
        }
    }

    /// Get pointer for WASM import
    #[inline]
    pub fn ptr(&self) -> i32 {
        self.ptr as i32
    }

    /// Get length for WASM import
    #[inline]
    pub fn len(&self) -> i32 {
        self.len as i32
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A mutable slice in linear memory
#[repr(C)]
pub struct LinearSliceMut {
    pub ptr: *mut u8,
    pub len: usize,
}

impl LinearSliceMut {
    /// Create from a mutable byte slice
    #[inline]
    pub fn from_slice(slice: &mut [u8]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
        }
    }

    /// Get pointer for WASM import
    #[inline]
    pub fn ptr(&self) -> i32 {
        self.ptr as i32
    }

    /// Get length/capacity for WASM import
    #[inline]
    pub fn len(&self) -> i32 {
        self.len as i32
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Access as slice
    ///
    /// # Safety
    /// The memory must be valid for the lifetime of the returned slice.
    #[inline]
    pub unsafe fn as_slice(&self) -> &[u8] {
        slice::from_raw_parts(self.ptr, self.len)
    }

    /// Access as mutable slice
    ///
    /// # Safety
    /// The memory must be valid and exclusive for the lifetime of the returned slice.
    #[inline]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

/// Helper trait for types that can be written to linear memory
pub trait IntoLinearMemory {
    /// Write self to linear memory, returning (ptr, len)
    fn write_linear(&self, out: &mut [u8]) -> Option<(i32, i32)>;
}

impl IntoLinearMemory for &[u8] {
    fn write_linear(&self, out: &mut [u8]) -> Option<(i32, i32)> {
        if self.len() > out.len() {
            return None;
        }
        out[..self.len()].copy_from_slice(self);
        Some((out.as_mut_ptr() as i32, self.len() as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_slice_from_slice() {
        let data = [1u8, 2, 3, 4, 5];
        let slice = LinearSlice::from_slice(&data);
        assert_eq!(slice.len(), 5);
        assert!(!slice.is_empty());
    }

    #[test]
    fn linear_slice_empty() {
        let data: [u8; 0] = [];
        let slice = LinearSlice::from_slice(&data);
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn linear_slice_mut_from_slice() {
        let mut data = [0u8; 10];
        let slice = LinearSliceMut::from_slice(&mut data);
        assert_eq!(slice.len(), 10);
    }

    #[test]
    fn into_linear_memory_bytes() {
        let input: &[u8] = &[1, 2, 3];
        let mut out = [0u8; 10];
        let result = input.write_linear(&mut out);
        assert!(result.is_some());
        let (_ptr, len) = result.unwrap();
        assert_eq!(len, 3);
        assert_eq!(&out[..3], &[1, 2, 3]);
    }

    #[test]
    fn into_linear_memory_too_small() {
        let input: &[u8] = &[1, 2, 3, 4, 5];
        let mut out = [0u8; 3];
        let result = input.write_linear(&mut out);
        assert!(result.is_none());
    }
}
