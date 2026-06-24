// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: idl/harness/shmem_control_v1.toml

// namespace = shared_memory.control, version = 1

//! WASM guest-side imports for SDK
//!
//! These functions are imported from the host runner and provide
//! access to harness endpoints via capability handles.

#![allow(dead_code)]

mod close_region {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "close_region::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            caller_domain_id: u64,
            region_id: u64,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::close_region harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct CloseRegionClient {
        cap: u64,
    }

    impl CloseRegionClient {
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
            caller_domain_id: u64,
            region_id: u64,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    caller_domain_id,
                    region_id,
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

pub use close_region::CloseRegionClient;

mod close_region_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "close_region_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            region_id: u64,
            status: u32,
            reserved: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::close_region_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct CloseRegionReplyClient {
        cap: u64,
    }

    impl CloseRegionReplyClient {
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
            region_id: u64,
            status: u32,
            reserved: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    region_id,
                    status,
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

pub use close_region_reply::CloseRegionReplyClient;

mod create_region {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "create_region::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            owner_domain_id: u64,
            size_bytes: u64,
            flags: u32,
            page_size: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::create_region harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct CreateRegionClient {
        cap: u64,
    }

    impl CreateRegionClient {
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
            owner_domain_id: u64,
            size_bytes: u64,
            flags: u32,
            page_size: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    owner_domain_id,
                    size_bytes,
                    flags,
                    page_size,
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

pub use create_region::CreateRegionClient;

mod create_region_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "create_region_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            region_id: u64,
            shm_cap: u64,
            phys_addr: u64,
            status: u32,
            reserved: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::create_region_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct CreateRegionReplyClient {
        cap: u64,
    }

    impl CreateRegionReplyClient {
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
            region_id: u64,
            shm_cap: u64,
            phys_addr: u64,
            status: u32,
            reserved: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    region_id,
                    shm_cap,
                    phys_addr,
                    status,
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

pub use create_region_reply::CreateRegionReplyClient;

mod map_region {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "map_region::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            caller_domain_id: u64,
            region_id: u64,
            target_domain_id: u64,
            shm_cap: u64,
            rights: u32,
            cache_mode: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::map_region harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct MapRegionClient {
        cap: u64,
    }

    impl MapRegionClient {
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
            caller_domain_id: u64,
            region_id: u64,
            target_domain_id: u64,
            shm_cap: u64,
            rights: u32,
            cache_mode: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    caller_domain_id,
                    region_id,
                    target_domain_id,
                    shm_cap,
                    rights,
                    cache_mode,
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

pub use map_region::MapRegionClient;

mod map_region_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "map_region_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            region_id: u64,
            mapping_id: u64,
            status: u32,
            reserved: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::map_region_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct MapRegionReplyClient {
        cap: u64,
    }

    impl MapRegionReplyClient {
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
            region_id: u64,
            mapping_id: u64,
            status: u32,
            reserved: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    region_id,
                    mapping_id,
                    status,
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

pub use map_region_reply::MapRegionReplyClient;

mod shmem_read {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "shmem_read::call"]
        fn harness_call(
            cap: u64,
            shm_cap: u64,
            offset: u64,
            len: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::shmem_read harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct ShmemReadClient {
        cap: u64,
    }

    impl ShmemReadClient {
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
            shm_cap: u64,
            offset: u64,
            len: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    shm_cap,
                    offset,
                    len,
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

pub use shmem_read::ShmemReadClient;

mod shmem_read_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "shmem_read_reply::call"]
        fn harness_call(
            cap: u64,
            status: u32,
            bytes_read: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::shmem_read_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct ShmemReadReplyClient {
        cap: u64,
    }

    impl ShmemReadReplyClient {
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
            status: u32,
            bytes_read: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    status,
                    bytes_read,
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

pub use shmem_read_reply::ShmemReadReplyClient;

mod shmem_write {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "shmem_write::call"]
        fn harness_call(
            cap: u64,
            shm_cap: u64,
            offset: u64,
            data_offset: u64,
            len: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::shmem_write harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct ShmemWriteClient {
        cap: u64,
    }

    impl ShmemWriteClient {
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
            shm_cap: u64,
            offset: u64,
            data_offset: u64,
            len: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    shm_cap,
                    offset,
                    data_offset,
                    len,
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

pub use shmem_write::ShmemWriteClient;

mod shmem_write_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "shmem_write_reply::call"]
        fn harness_call(
            cap: u64,
            status: u32,
            bytes_written: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::shmem_write_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct ShmemWriteReplyClient {
        cap: u64,
    }

    impl ShmemWriteReplyClient {
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
            status: u32,
            bytes_written: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    status,
                    bytes_written,
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

pub use shmem_write_reply::ShmemWriteReplyClient;

mod unmap_region {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "unmap_region::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            caller_domain_id: u64,
            mapping_id: u64,
            target_domain_id: u64,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::unmap_region harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct UnmapRegionClient {
        cap: u64,
    }

    impl UnmapRegionClient {
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
            caller_domain_id: u64,
            mapping_id: u64,
            target_domain_id: u64,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    caller_domain_id,
                    mapping_id,
                    target_domain_id,
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

pub use unmap_region::UnmapRegionClient;

mod unmap_region_reply {
    #[link(wasm_import_module = "ramen::shared_memory.control")]
    extern "C" {
        #[link_name = "unmap_region_reply::call"]
        fn harness_call(
            cap: u64,
            request_id: u64,
            mapping_id: u64,
            status: u32,
            reserved: u32,
            out_ptr: *mut u8,
            out_len: *mut i32,
        ) -> i32;
    }

    /// Client for shared_memory.control::unmap_region_reply harness
    ///
    /// Created from a capability handle provided by the runner.
    pub struct UnmapRegionReplyClient {
        cap: u64,
    }

    impl UnmapRegionReplyClient {
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
            mapping_id: u64,
            status: u32,
            reserved: u32,
            out_buf: &mut [u8],
        ) -> (crate::Status, usize) {
            let mut out_len: i32 = out_buf.len() as i32;
            let status = unsafe {
                harness_call(
                    self.cap,
                    request_id,
                    mapping_id,
                    status,
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

pub use unmap_region_reply::UnmapRegionReplyClient;
