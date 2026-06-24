// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/echo_harness_v0.toml

// namespace = harness.echo, version = 0

//! WASM guest-side imports for SDK
//!
//! These functions are imported from the host runner and provide
//! access to harness endpoints via capability handles.

#![allow(dead_code)]

mod echo_reply {
    #[link(wasm_import_module = "ramen::harness.echo")]
    extern "C" {
        #[link_name = "echo_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            payload_len: u32,
            status: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for harness.echo::echo_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct EchoReplyClient {
        cap: u64,
    }

    impl EchoReplyClient {
        /// Create client from capability handle
        #[inline]
        pub const fn from_cap(cap: u64) -> Self {
            Self { cap }
        }

        /// Call the harness endpoint
        ///
        /// Returns status plus the reply length written by the host.
        /// The reply bytes are written to the provided output buffer.
        #[inline]
        pub fn call(
            &self,
            request_id: u64,
            payload_len: u32,
            status: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    payload_len,
                    status,
                    out_buf.as_mut_ptr(),
                    &mut out_len,
                )
            };
            let reply_len = if out_len < 0 {
                0
            } else {
                core::cmp::min(out_len as usize, out_buf.len())
            };
            (crate::Status::from_raw(status), reply_len)
        }
    }
}

pub use echo_reply::EchoReplyClient;

mod echo_request {
    #[link(wasm_import_module = "ramen::harness.echo")]
    extern "C" {
        #[link_name = "echo_request::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            payload_len: u32,
            reserved: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for harness.echo::echo_request harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct EchoRequestClient {
        cap: u64,
    }

    impl EchoRequestClient {
        /// Create client from capability handle
        #[inline]
        pub const fn from_cap(cap: u64) -> Self {
            Self { cap }
        }

        /// Call the harness endpoint
        ///
        /// Returns status plus the reply length written by the host.
        /// The reply bytes are written to the provided output buffer.
        #[inline]
        pub fn call(
            &self,
            request_id: u64,
            payload_len: u32,
            reserved: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    payload_len,
                    reserved,
                    out_buf.as_mut_ptr(),
                    &mut out_len,
                )
            };
            let reply_len = if out_len < 0 {
                0
            } else {
                core::cmp::min(out_len as usize, out_buf.len())
            };
            (crate::Status::from_raw(status), reply_len)
        }
    }
}

pub use echo_request::EchoRequestClient;
