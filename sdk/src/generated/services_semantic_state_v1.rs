// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/services/semantic_state_v1.toml

// namespace = services.semantic_state, version = 1

//! WASM guest-side imports for SDK
//!
//! These functions are imported from the host runner and provide
//! access to harness endpoints via capability handles.

#![allow(dead_code)]

mod get_snapshot {
    #[link(wasm_import_module = "ramen::services.semantic_state")]
    extern "C" {
        #[link_name = "get_snapshot::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            format: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for services.semantic_state::get_snapshot harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct GetSnapshotClient {
        cap: u64,
    }

    impl GetSnapshotClient {
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
            format: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    format,
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

pub use get_snapshot::GetSnapshotClient;

mod get_snapshot_reply {
    #[link(wasm_import_module = "ramen::services.semantic_state")]
    extern "C" {
        #[link_name = "get_snapshot_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            status: u32,
            shm_cap: u64,
            shm_size: u64,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for services.semantic_state::get_snapshot_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct GetSnapshotReplyClient {
        cap: u64,
    }

    impl GetSnapshotReplyClient {
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
            status: u32,
            shm_cap: u64,
            shm_size: u64,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    status,
                    shm_cap,
                    shm_size,
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

pub use get_snapshot_reply::GetSnapshotReplyClient;

mod state_changed_event {
    #[link(wasm_import_module = "ramen::services.semantic_state")]
    extern "C" {
        #[link_name = "state_changed_event::call"]
        fn harness_call(
            cap: u64,
            subscription_id: u64,
            event_type: u32,
            shm_cap: u64,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for services.semantic_state::state_changed_event harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct StateChangedEventClient {
        cap: u64,
    }

    impl StateChangedEventClient {
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
            subscription_id: u64,
            event_type: u32,
            shm_cap: u64,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    subscription_id,
                    event_type,
                    shm_cap,
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

pub use state_changed_event::StateChangedEventClient;

mod subscribe {
    #[link(wasm_import_module = "ramen::services.semantic_state")]
    extern "C" {
        #[link_name = "subscribe::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            event_mask: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for services.semantic_state::subscribe harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct SubscribeClient {
        cap: u64,
    }

    impl SubscribeClient {
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
            event_mask: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    event_mask,
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

pub use subscribe::SubscribeClient;

mod subscribe_reply {
    #[link(wasm_import_module = "ramen::services.semantic_state")]
    extern "C" {
        #[link_name = "subscribe_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            status: u32,
            subscription_id: u64,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for services.semantic_state::subscribe_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct SubscribeReplyClient {
        cap: u64,
    }

    impl SubscribeReplyClient {
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
            status: u32,
            subscription_id: u64,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    status,
                    subscription_id,
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

pub use subscribe_reply::SubscribeReplyClient;
