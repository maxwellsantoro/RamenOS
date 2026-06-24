//! Kernel API - Shared types for kernel <-> runtime communication.
//!
//! This crate provides the common types used by both the kernel and
//! user-space runtime components. It is designed to be `no_std` compatible
//! for use in bare-metal kernel builds.
//!
//! # Architecture
//!
//! The kernel API is organized around several key concepts:
//!
//! - **Capabilities** ([`cap`]): Unforgeable tokens that grant access to kernel resources.
//!   Handles include generation counters to prevent stale handle reuse attacks.
//! - **IPC** ([`ipc`]): Inter-process communication via typed message envelopes.
//!   The control plane uses message passing for all operations.
//! - **Shared Memory** ([`ring_buffer`]): Zero-copy data plane transfers via lock-free
//!   single-producer single-consumer ring buffers.
//! - **Tracing** ([`trace`]): Structured event emission for debugging and monitoring.
//!
//! # Modules
//!
//! - [`cap`] - Capability handle types with generation counters
//! - [`ipc`] - IPC envelope and message types
//! - [`trace`] - Trace event types and capability rights
//! - [`wire`] - Wire format serialization helpers
//! - [`ring_buffer`] - Lock-free SPSC ring buffer for data plane
//! - [`generated`] - IDL-generated message types
//!
//! # Example
//!
//! ```
//! use kernel_api::cap::{Handle, HandleKind};
//! use kernel_api::ipc::Envelope;
//!
//! // Create an IPC handle
//! let handle = Handle {
//!     kind: HandleKind::Ipc,
//!     index: 1,
//!     generation: 42,
//! };
//!
//! // Pack for wire transmission
//! let packed = handle.pack();
//! let unpacked = Handle::unpack(packed);
//! assert_eq!(handle, unpacked);
//!
//! // Create an empty envelope
//! let env = Envelope::empty(1, 1);
//! assert_eq!(env.protocol, 1);
//! ```
//!
//! # Safety
//!
//! Types in this crate are used for kernel-user communication. Invalid
//! data could cause kernel misbehavior. All types implement validation
//! where appropriate.
//!
//! # Design Principles
//!
//! - **Rust-first**: Native Rust types, not POSIX compatibility layers
//! - **Capability-based**: All kernel access requires validated handles
//! - **Typed IDL**: Message formats are defined in IDL and code-generated
//! - **Zero-copy**: Data plane uses shared memory, not message copying

#![no_std]

/// Capability handle types for kernel resource access.
///
/// This module provides types for managing capability handles, which are
/// unforgeable tokens that grant access to kernel resources such as IPC
/// endpoints, shared memory regions, and trace buffers.
///
/// # Security Model
///
/// Capability handles in RamenOS implement several security properties:
///
/// - **Unforgeability**: Handles are kernel-issued and cannot be forged by user-space.
/// - **Generation counters**: Each handle includes a generation counter that is
///   incremented when a capability slot is reused, preventing stale handle attacks.
/// - **Kind discrimination**: Handles include a kind field that distinguishes between
///   different resource types, preventing cross-table aliasing attacks.
///
/// # Example
///
/// ```
/// use kernel_api::cap::{Handle, HandleKind};
///
/// // Create an IPC handle
/// let handle = Handle {
///     kind: HandleKind::Ipc,
///     index: 1,
///     generation: 42,
/// };
///
/// // Validate the handle kind
/// assert!(handle.kind == HandleKind::Ipc);
///
/// // Pack for wire transmission
/// let packed = handle.pack();
/// assert_ne!(packed, 0);
/// ```
pub mod cap {
    /// Handle kind discriminator to prevent cross-table aliasing attacks.
    ///
    /// The handle kind distinguishes between different capability types,
    /// ensuring that a handle for one resource type cannot be mistakenly
    /// or maliciously used to access a different resource type.
    ///
    /// # Security
    ///
    /// V-16 (SC-13): This discriminator prevents IPC/shmem handle confusion
    /// attacks where an attacker might try to use an IPC handle to access
    /// shared memory or vice versa.
    ///
    /// V-012: Added `Trace` handle kind for trace buffer access control.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::cap::HandleKind;
    ///
    /// let kind = HandleKind::Ipc;
    /// assert_eq!(kind as u8, 1);
    /// ```
    #[repr(u8)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[doc(alias = "CapabilityKind")]
    #[doc(alias = "HandleType")]
    pub enum HandleKind {
        /// Invalid or unallocated handle.
        ///
        /// An invalid handle has no associated resource and will be rejected
        /// by all kernel operations. This is the default state for unallocated
        /// capability table slots.
        Invalid = 0,

        /// IPC endpoint capability.
        ///
        /// IPC handles grant access to inter-process communication endpoints.
        /// They can be used to send and receive messages via the kernel's
        /// IPC subsystem.
        Ipc = 1,

        /// Shared memory region capability.
        ///
        /// Shmem handles grant access to shared memory regions. They control
        /// the ability to map, unmap, read, and write shared memory pages.
        Shmem = 2,

        /// Trace buffer capability.
        ///
        /// Trace handles grant access to trace buffers for emitting or
        /// consuming trace events. Rights are controlled by the trace
        /// capability rights flags.
        Trace = 3,
    }

    /// A capability handle for kernel resources.
    ///
    /// Handles are unforgeable tokens that grant access to kernel resources.
    /// Each handle includes:
    /// - A kind field identifying the resource type (IPC, Shmem, Trace)
    /// - An index into the kernel's capability table
    /// - A generation counter preventing stale handle reuse
    ///
    /// # Memory Layout
    ///
    /// ```text
    /// | Field      | Type       | Size | Offset |
    /// |------------|------------|------|--------|
    /// | kind       | HandleKind | 1    | 0      |
    /// | (padding)  | [u8; 3]    | 3    | 1      |
    /// | index      | u32        | 4    | 4      |
    /// | generation | u64        | 8    | 8      |
    /// ```
    ///
    /// Total size: 16 bytes.
    ///
    /// # Wire Format
    ///
    /// For transmission over IPC, handles are packed into a 64-bit value:
    /// ```text
    /// | Bits  | Field      |
    /// |-------|------------|
    /// | 56-63 | kind       |
    /// | 48-55 | reserved   |
    /// | 32-47 | index      |
    /// | 0-31  | generation |
    /// ```
    ///
    /// # Security
    ///
    /// Generation counters prevent TOCTOU attacks where a capability is
    /// revoked and a new one allocated at the same slot. The generation
    /// counter changes on each allocation, invalidating old handles.
    ///
    /// - V-05/V-06: Generation counter prevents stale handle reuse attacks.
    /// - V-16 (SC-13): Kind field prevents IPC/shmem handle confusion.
    /// - V-004: Generation counter is u64 to prevent practical wrapping
    ///   (centuries of continuous allocation at 1M ops/sec).
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::cap::{Handle, HandleKind};
    ///
    /// // Create an IPC handle
    /// let handle = Handle {
    ///     kind: HandleKind::Ipc,
    ///     index: 1,
    ///     generation: 42,
    /// };
    ///
    /// // Pack for wire transmission
    /// let packed = handle.pack();
    /// let unpacked = Handle::unpack(packed);
    /// assert_eq!(handle, unpacked);
    /// ```
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[doc(alias = "Capability")]
    #[doc(alias = "Cap")]
    pub struct Handle {
        /// Handle kind discriminator.
        ///
        /// Distinguishes between IPC, shared memory, and trace capabilities.
        /// Unknown kinds are treated as invalid by the kernel.
        pub kind: HandleKind,

        /// Index into the kernel's capability table.
        ///
        /// This index identifies the specific slot in the kernel's capability
        /// table that holds the resource metadata. Valid indices are determined
        /// by the kernel's configuration (e.g., CAP_TABLE_SIZE).
        pub index: u32,

        /// Generation counter for stale handle detection.
        ///
        /// Incremented each time a capability slot is reused. This prevents
        /// an attacker from using a stale handle after a capability has been
        /// revoked and the slot reallocated.
        ///
        /// The generation is stored as u64 internally but truncated to u32
        /// for wire format transmission. This is sufficient for security
        /// purposes as 2^32 generations would require centuries of continuous
        /// allocation at practical rates.
        pub generation: u64,
    }

    impl Handle {
        /// The canonical invalid handle.
        ///
        /// This constant represents a handle that is guaranteed to be rejected
        /// by all kernel operations. It is used as a default value and to
        /// indicate "no handle" in data structures.
        ///
        /// # Properties
        ///
        /// - `kind` is `HandleKind::Invalid`
        /// - `index` is 0
        /// - `generation` is 0
        /// - `pack()` returns 0
        ///
        /// # Example
        ///
        /// ```
        /// use kernel_api::cap::{Handle, HandleKind};
        ///
        /// let invalid = Handle::INVALID;
        /// assert_eq!(invalid.kind, HandleKind::Invalid);
        /// assert_eq!(invalid.pack(), 0);
        /// ```
        pub const INVALID: Handle = Handle {
            kind: HandleKind::Invalid,
            index: 0,
            generation: 0,
        };

        /// Pack handle into a 64-bit value for wire format transmission.
        ///
        /// The wire format encoding is:
        /// ```text
        /// | Bits  | Field      | Size    |
        /// |-------|------------|---------|
        /// | 56-63 | kind       | 8 bits  |
        /// | 48-55 | reserved   | 8 bits  |
        /// | 32-47 | index      | 16 bits |
        /// | 0-31  | generation | 32 bits |
        /// ```
        ///
        /// # Design Notes
        ///
        /// S8 Phase 2: Index is truncated to 16 bits (65536 handles per kind).
        /// - Current static limits (CAP_TABLE_SIZE=64, MAX_REGIONS=16) make overflow impossible.
        /// - Reserved 8 bits available for future expansion if dynamic allocation exceeds 65k.
        /// - Wire format kept at 64 bits for IPC efficiency.
        /// - If indices grow beyond 16 bits, we can expand into reserved field or version the format.
        ///
        /// V-004: Generation is truncated to 32 bits on wire (sufficient for security),
        /// but stored as u64 internally to prevent practical wrapping.
        ///
        /// # Example
        ///
        /// ```
        /// use kernel_api::cap::{Handle, HandleKind};
        ///
        /// let handle = Handle {
        ///     kind: HandleKind::Ipc,
        ///     index: 0x1234,
        ///     generation: 0xDEADBEEF,
        /// };
        /// let packed = handle.pack();
        ///
        /// // Verify encoding
        /// assert_eq!((packed >> 56) & 0xFF, HandleKind::Ipc as u64);
        /// assert_eq!((packed >> 32) & 0xFFFF, 0x1234);
        /// assert_eq!(packed & 0xFFFFFFFF, 0xDEADBEEF);
        /// ```
        pub const fn pack(self) -> u64 {
            let kind_bits = (self.kind as u64) & 0xFF;
            let index_bits = (self.index as u64) & 0xFFFF;
            let gen_bits = self.generation & 0xFFFFFFFF; // Truncate to 32 bits for wire format
            (kind_bits << 56) | (index_bits << 32) | gen_bits
        }

        /// Unpack a 64-bit wire format value into a Handle.
        ///
        /// This is the inverse of [`pack`](Self::pack). Unknown kind values
        /// are treated as `HandleKind::Invalid` to ensure fail-safe behavior.
        ///
        /// # Wire Format
        ///
        /// ```text
        /// | Bits  | Field      | Size    |
        /// |-------|------------|---------|
        /// | 56-63 | kind       | 8 bits  |
        /// | 48-55 | reserved   | 8 bits  |
        /// | 32-47 | index      | 16 bits |
        /// | 0-31  | generation | 32 bits |
        /// ```
        ///
        /// # Security
        ///
        /// - V-004: Generation is zero-extended from 32 bits to 64 bits.
        /// - V-012: Added Trace handle kind support.
        /// - Unknown kind values are treated as `Invalid` (fail-closed).
        ///
        /// # Example
        ///
        /// ```
        /// use kernel_api::cap::{Handle, HandleKind};
        ///
        /// // Pack and unpack round-trip
        /// let original = Handle {
        ///     kind: HandleKind::Shmem,
        ///     index: 42,
        ///     generation: 123,
        /// };
        /// let packed = original.pack();
        /// let unpacked = Handle::unpack(packed);
        /// assert_eq!(original, unpacked);
        ///
        /// // Unknown kinds become Invalid
        /// let unknown = Handle::unpack(0xFF_00_0000_0000_0001);
        /// assert_eq!(unknown.kind, HandleKind::Invalid);
        /// ```
        pub const fn unpack(value: u64) -> Self {
            let kind_raw = ((value >> 56) & 0xFF) as u8;
            let kind = match kind_raw {
                0 => HandleKind::Invalid,
                1 => HandleKind::Ipc,
                2 => HandleKind::Shmem,
                3 => HandleKind::Trace,
                _ => HandleKind::Invalid, // Unknown kinds treated as invalid
            };
            Handle {
                kind,
                index: ((value >> 32) & 0xFFFF) as u32,
                generation: (value & 0xFFFFFFFF), // Zero-extend to 64 bits
            }
        }
    }

    /// Errors returned by capability table operations.
    ///
    /// These errors indicate failures in capability management operations
    /// such as allocation, validation, and deallocation.
    ///
    /// # Security
    ///
    /// V-05/V-06: These error types support the generation counter security
    /// model by distinguishing between invalid handles (wrong kind or index)
    /// and stale handles (correct slot but outdated generation).
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[doc(alias = "CapabilityError")]
    #[doc(alias = "CapError")]
    pub enum CapTableError {
        /// The capability table has no available slots.
        ///
        /// This error is returned when attempting to allocate a new capability
        /// but all slots in the table are occupied. The caller may need to
        /// deallocate unused capabilities or increase the table size.
        TableFull,

        /// The handle is invalid (wrong kind or out-of-range index).
        ///
        /// This error indicates that the handle does not correspond to any
        /// valid capability slot. This could be due to:
        /// - Wrong handle kind for the operation
        /// - Index beyond table bounds
        /// - Handle with `HandleKind::Invalid`
        InvalidHandle,

        /// The handle is stale (generation mismatch).
        ///
        /// This error indicates that the handle once referred to a valid
        /// capability, but the slot has been reused and the generation
        /// counter has changed. This is a security condition that prevents
        /// stale handle reuse attacks.
        ///
        /// V-05/V-06: This error is critical for preventing TOCTOU attacks.
        StaleHandle,
    }

    /// Trait for kernel capability table implementations.
    ///
    /// The capability table manages the allocation, validation, and deallocation
    /// of capability handles. Implementations must enforce the generation counter
    /// security model to prevent stale handle reuse attacks.
    ///
    /// # Security Requirements
    ///
    /// Implementations must:
    /// - Reject handles with mismatched generation counters (`StaleHandle` error)
    /// - Reject handles with invalid indices or kinds (`InvalidHandle` error)
    /// - Increment the generation counter on each deallocation
    /// - Ensure generation counters never wrap in practice (use u64)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use kernel_api::cap::{CapTable, CapTableError, Handle};
    ///
    /// // Allocate a new capability
    /// let handle = table.allocate()?;
    ///
    /// // Validate the handle
    /// assert!(table.validate(handle));
    ///
    /// // Deallocate (increments generation)
    /// table.deallocate(handle)?;
    ///
    /// // Old handle is now stale
    /// assert!(!table.validate(handle));
    /// ```
    pub trait CapTable {
        /// Allocate a new capability slot and return its handle.
        ///
        /// The returned handle will have:
        /// - A unique index into the capability table
        /// - The current generation counter for that slot
        /// - The appropriate handle kind for the table
        ///
        /// # Errors
        ///
        /// Returns [`CapTableError::TableFull`] if all slots are occupied.
        ///
        /// # Example
        ///
        /// ```ignore
        /// let handle = table.allocate().expect("table has space");
        /// assert_ne!(handle, Handle::INVALID);
        /// ```
        fn allocate(&mut self) -> Result<Handle, CapTableError>;

        /// Validate a handle and return true if it's current (not stale).
        ///
        /// This checks that:
        /// - The handle's index is within bounds
        /// - The handle's kind matches the table's kind
        /// - The handle's generation matches the slot's current generation
        ///
        /// # Security
        ///
        /// This is the primary defense against stale handle reuse attacks.
        /// A handle that was valid but has been deallocated will fail this
        /// check because the generation counter was incremented.
        ///
        /// # Example
        ///
        /// ```ignore
        /// let handle = table.allocate().unwrap();
        /// assert!(table.validate(handle));
        ///
        /// table.deallocate(handle).unwrap();
        /// assert!(!table.validate(handle)); // Stale after deallocation
        /// ```
        fn validate(&self, handle: Handle) -> bool;

        /// Deallocate a capability slot, incrementing its generation counter.
        ///
        /// After deallocation:
        /// - The slot becomes available for reallocation
        /// - The generation counter is incremented
        /// - Any existing handles to this slot become stale
        ///
        /// # Errors
        ///
        /// - [`CapTableError::InvalidHandle`] - Wrong kind or out-of-range index
        /// - [`CapTableError::StaleHandle`] - Generation mismatch (already deallocated)
        ///
        /// # Security
        ///
        /// The generation counter increment ensures that any cached handles
        /// become invalid, preventing use-after-free style attacks.
        ///
        /// # Example
        ///
        /// ```ignore
        /// let handle = table.allocate().unwrap();
        /// table.deallocate(handle).expect("valid handle");
        ///
        /// // Attempting to deallocate again fails
        /// assert_eq!(table.deallocate(handle), Err(CapTableError::StaleHandle));
        /// ```
        fn deallocate(&mut self, handle: Handle) -> Result<(), CapTableError>;
    }
}

/// Inter-process communication types for kernel-user message passing.
///
/// This module provides the IPC envelope type and protocol constants used
/// for all control-plane communication in RamenOS. The control plane uses
/// typed message passing, while the data plane uses zero-copy shared memory.
///
/// # Design Principles
///
/// - **Typed messages**: All message formats are defined in IDL and code-generated
/// - **Fail-closed**: Unknown protocol/msg_type combinations are rejected
/// - **Deterministic**: Little-endian encoding for cross-architecture consistency
/// - **Fixed size**: Envelopes are fixed-size for kernel simplicity
///
/// # Example
///
/// ```
/// use kernel_api::ipc::{Envelope, PROTOCOL_PING, MSG_PING};
///
/// // Create an empty envelope for a specific protocol
/// let env = Envelope::empty(PROTOCOL_PING, MSG_PING);
/// assert_eq!(env.protocol, PROTOCOL_PING);
/// assert_eq!(env.msg_type, MSG_PING);
/// assert_eq!(env.payload_len, 0);
/// ```
pub mod ipc {
    use super::cap::Handle;

    /// IPC message envelope for kernel-user communication.
    ///
    /// The envelope is the fundamental unit of communication in RamenOS IPC.
    /// It wraps typed message payloads with routing and metadata fields.
    ///
    /// # Memory Layout
    ///
    /// ```text
    /// | Offset | Size | Field       | Description              |
    /// |--------|------|-------------|--------------------------|
    /// | 0      | 4    | protocol    | Protocol identifier      |
    /// | 4      | 4    | msg_type    | Message type within proto|
    /// | 8      | 8    | handle      | Target capability handle |
    /// | 16     | 4    | payload_len | Actual payload bytes     |
    /// | 20     | 64   | payload     | Message payload data     |
    /// ```
    ///
    /// Total size: 84 bytes.
    ///
    /// # Wire Format
    ///
    /// Envelopes use little-endian encoding for cross-architecture determinism.
    /// The handle field is packed using [`Handle::pack`](super::cap::Handle::pack).
    ///
    /// # Security
    ///
    /// - Payload length is validated against maximum size (64 bytes)
    /// - Unknown protocol/msg_type combinations are rejected (fail-closed)
    /// - Payload parsing validates field types and ranges
    /// - Handle is validated against the caller's capability table
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::ipc::Envelope;
    /// use kernel_api::cap::Handle;
    ///
    /// // Create an empty envelope
    /// let env = Envelope::empty(1, 1);
    /// assert_eq!(env.protocol, 1);
    /// assert_eq!(env.msg_type, 1);
    /// assert_eq!(env.handle, Handle::INVALID);
    /// assert_eq!(env.payload_len, 0);
    /// ```
    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    #[doc(alias = "Message")]
    #[doc(alias = "IpcMessage")]
    pub struct Envelope {
        /// Protocol identifier.
        ///
        /// Identifies the service or harness that will handle this message.
        /// Protocol IDs are assigned centrally and generated from IDL files.
        ///
        /// Well-known protocols:
        /// - `PROTOCOL_PING` (1): Ping harness for IPC testing
        /// - See [`generated`](crate::generated) for IDL-generated protocols
        pub protocol: u32,

        /// Message type within the protocol.
        ///
        /// Identifies the specific operation (request type or reply type)
        /// within the protocol. Message types are defined in IDL files.
        ///
        /// Example: For a shmem control protocol, types might include
        /// CreateRegion, MapRegion, UnmapRegion, etc.
        pub msg_type: u32,

        /// Target capability handle.
        ///
        /// The handle identifying the target resource for this message.
        /// For requests, this is typically the server's IPC endpoint.
        /// For replies, this is the client's reply endpoint.
        pub handle: Handle,

        /// Actual payload length in bytes.
        ///
        /// Must be <= 64. The payload array is always 64 bytes, but only
        /// the first `payload_len` bytes contain valid data. The remaining
        /// bytes should be zeroed.
        pub payload_len: u32,

        /// Message payload data.
        ///
        /// Fixed-size buffer for message data. Only the first `payload_len`
        /// bytes are valid; the rest should be zeroed for security.
        ///
        /// Payloads are serialized using the [`wire`](crate::wire) module's
        /// helpers, which ensure proper encoding and zeroing.
        pub payload: [u8; 64],
    }

    impl Envelope {
        /// Create an empty envelope with the specified protocol and message type.
        ///
        /// The handle is set to [`Handle::INVALID`](super::cap::Handle::INVALID) and
        /// the payload is zeroed with length 0.
        ///
        /// # Example
        ///
        /// ```
        /// use kernel_api::ipc::Envelope;
        ///
        /// let env = Envelope::empty(1, 2);
        /// assert_eq!(env.protocol, 1);
        /// assert_eq!(env.msg_type, 2);
        /// assert_eq!(env.payload_len, 0);
        /// ```
        pub const fn empty(protocol: u32, msg_type: u32) -> Self {
            Self {
                protocol,
                msg_type,
                handle: Handle::INVALID,
                payload_len: 0,
                payload: [0u8; 64],
            }
        }
    }

    /// Protocol ID for the ping harness.
    ///
    /// The ping harness is a simple IPC testing service that responds to
    /// ping messages with pong replies. It is used for IPC validation
    /// and latency measurement.
    ///
    /// # Message Types
    ///
    /// - [`MSG_PING`]: Request a pong reply
    /// - [`MSG_PONG`]: Reply to a ping request
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::ipc::{Envelope, PROTOCOL_PING, MSG_PING};
    ///
    /// let ping = Envelope::empty(PROTOCOL_PING, MSG_PING);
    /// assert_eq!(ping.protocol, PROTOCOL_PING);
    /// ```
    pub const PROTOCOL_PING: u32 = 1;

    /// Message type for ping requests.
    ///
    /// Sent to the ping harness to request a pong reply.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::ipc::{Envelope, PROTOCOL_PING, MSG_PING};
    ///
    /// let request = Envelope::empty(PROTOCOL_PING, MSG_PING);
    /// ```
    pub const MSG_PING: u32 = 1;

    /// Message type for pong replies.
    ///
    /// Sent by the ping harness in response to a ping request.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::ipc::{Envelope, PROTOCOL_PING, MSG_PONG};
    ///
    /// let reply = Envelope::empty(PROTOCOL_PING, MSG_PONG);
    /// ```
    pub const MSG_PONG: u32 = 2;
}

/// Trace event types and capability rights for kernel observability.
///
/// This module provides types for structured trace event emission and
/// consumption. Trace events are written to a ring buffer and can be
/// read by authorized components for debugging and monitoring.
///
/// # Capability Model
///
/// V-012: Trace access is controlled by capability rights:
/// - `TRACE_RIGHT_READ`: Read trace events from the buffer
/// - `TRACE_RIGHT_WRITE`: Emit trace events to the buffer
/// - `TRACE_RIGHT_ADMIN`: Manage trace buffer configuration
///
/// # Example
///
/// ```
/// use kernel_api::trace::{Event, TAG_BOOT, TRACE_RIGHT_READ};
///
/// // Create a boot event
/// let event = Event {
///     tag: TAG_BOOT,
///     arg0: 0,
///     arg1: 0,
/// };
///
/// // Check trace rights
/// let can_read = (TRACE_RIGHT_READ & TRACE_RIGHT_READ) != 0;
/// assert!(can_read);
/// ```
pub mod trace {
    /// Minimal trace event for kernel observability.
    ///
    /// Trace events are emitted by the kernel and user-space components
    /// to a ring buffer for later analysis. Each event contains:
    /// - A tag identifying the event type
    /// - Two 64-bit arguments for event-specific data
    ///
    /// # Memory Layout
    ///
    /// ```text
    /// | Offset | Size | Field |
    /// |--------|------|-------|
    /// | 0      | 4    | tag   |
    /// | 4      | 4    | (pad) |
    /// | 8      | 8    | arg0  |
    /// | 16     | 8    | arg1  |
    /// ```
    ///
    /// Total size: 24 bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::{Event, TAG_BOOT, TAG_IPC};
    ///
    /// // Boot event
    /// let boot = Event {
    ///     tag: TAG_BOOT,
    ///     arg0: 0x1234, // architecture ID
    ///     arg1: 0,
    /// };
    ///
    /// // IPC event
    /// let ipc = Event {
    ///     tag: TAG_IPC,
    ///     arg0: 1, // sender domain
    ///     arg1: 2, // receiver domain
    /// };
    /// ```
    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    #[doc(alias = "TraceEvent")]
    #[doc(alias = "TraceRecord")]
    pub struct Event {
        /// Event type tag.
        ///
        /// Identifies the kind of event. Well-known tags include:
        /// - [`TAG_BOOT`]: Kernel boot event
        /// - [`TAG_IPC`]: IPC operation event
        pub tag: u32,

        /// First event-specific argument.
        ///
        /// Interpretation depends on the event tag. For example:
        /// - `TAG_BOOT`: Architecture identifier
        /// - `TAG_IPC`: Sender domain ID
        pub arg0: u64,

        /// Second event-specific argument.
        ///
        /// Interpretation depends on the event tag. For example:
        /// - `TAG_BOOT`: Reserved (0)
        /// - `TAG_IPC`: Receiver domain ID
        pub arg1: u64,
    }

    /// Event tag for kernel boot events.
    ///
    /// Emitted early in the kernel initialization process to mark
    /// the start of boot. The `arg0` field typically contains an
    /// architecture identifier.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::{Event, TAG_BOOT};
    ///
    /// let boot = Event {
    ///     tag: TAG_BOOT,
    ///     arg0: 1, // x86_64
    ///     arg1: 0,
    /// };
    /// ```
    pub const TAG_BOOT: u32 = 1;

    /// Event tag for IPC operation events.
    ///
    /// Emitted when IPC messages are sent or received. The arguments
    /// typically identify the communicating domains.
    ///
    /// # Arguments
    ///
    /// - `arg0`: Sender domain ID
    /// - `arg1`: Receiver domain ID
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::{Event, TAG_IPC};
    ///
    /// let ipc = Event {
    ///     tag: TAG_IPC,
    ///     arg0: 1, // sender
    ///     arg1: 2, // receiver
    /// };
    /// ```
    pub const TAG_IPC: u32 = 2;

    // ---- Trace Capability Rights (V-012 Phase 3) ----

    /// Right to read trace events from the trace buffer.
    ///
    /// Components with this right can consume trace events from the
    /// kernel's trace ring buffer. This is typically granted to
    /// diagnostic and monitoring services.
    ///
    /// V-012 Phase 3: Fine-grained trace capability access control.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::TRACE_RIGHT_READ;
    ///
    /// // Check if read right is present
    /// let rights = TRACE_RIGHT_READ;
    /// assert_eq!(rights & TRACE_RIGHT_READ, TRACE_RIGHT_READ);
    /// ```
    pub const TRACE_RIGHT_READ: u8 = 0x01;

    /// Right to emit trace events to the trace buffer.
    ///
    /// Components with this right can write trace events to the
    /// kernel's trace ring buffer. This is typically granted to
    /// kernel services and trusted components.
    ///
    /// V-012 Phase 3: Fine-grained trace capability access control.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::TRACE_RIGHT_WRITE;
    ///
    /// // Check if write right is present
    /// let rights = TRACE_RIGHT_WRITE;
    /// assert_eq!(rights & TRACE_RIGHT_WRITE, TRACE_RIGHT_WRITE);
    /// ```
    pub const TRACE_RIGHT_WRITE: u8 = 0x02;

    /// Right to manage trace buffer configuration.
    ///
    /// Components with this right can configure the trace buffer,
    /// including setting buffer size, enabling/disabling tracing,
    /// and managing writer registrations.
    ///
    /// V-012 Phase 3: Fine-grained trace capability access control.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::TRACE_RIGHT_ADMIN;
    ///
    /// // Check if admin right is present
    /// let rights = TRACE_RIGHT_ADMIN;
    /// assert_eq!(rights & TRACE_RIGHT_ADMIN, TRACE_RIGHT_ADMIN);
    /// ```
    pub const TRACE_RIGHT_ADMIN: u8 = 0x04;

    /// All trace capability rights combined.
    ///
    /// Combines `TRACE_RIGHT_READ | TRACE_RIGHT_WRITE | TRACE_RIGHT_ADMIN`.
    /// This is typically granted to the trace service component.
    ///
    /// V-012 Phase 3: Fine-grained trace capability access control.
    ///
    /// # Example
    ///
    /// ```
    /// use kernel_api::trace::{TRACE_RIGHT_ALL, TRACE_RIGHT_READ, TRACE_RIGHT_WRITE, TRACE_RIGHT_ADMIN};
    ///
    /// // All rights includes all individual rights
    /// assert_eq!(TRACE_RIGHT_ALL, TRACE_RIGHT_READ | TRACE_RIGHT_WRITE | TRACE_RIGHT_ADMIN);
    /// ```
    pub const TRACE_RIGHT_ALL: u8 = 0x07;
}

pub mod ipc_frame;
pub mod mock;
/// Wire format serialization helpers for IPC payloads.
///
/// This module provides functions for serializing and deserializing
/// message payloads to/from IPC envelopes. It ensures proper encoding,
/// length validation, and tail zeroing for security.
///
/// # Functions
///
/// - `write_payload`: Serialize a value into an envelope's payload
/// - `read_payload`: Deserialize a value from an envelope's payload
///
/// # Safety
///
/// The wire module validates payload lengths and ensures that:
/// - Payload length matches the serialized type size
/// - Unused payload bytes are zeroed
/// - Buffer overflows are prevented
///
/// # Example
///
/// ```ignore
/// use kernel_api::ipc::Envelope;
/// use kernel_api::wire;
///
/// let value: u32 = 0x12345678;
/// let mut env = Envelope::empty(1, 1);
///
/// // Write to payload
/// wire::write_payload(&mut env, &value).expect("write");
///
/// // Read back
/// let decoded: u32 = wire::read_payload(&env).expect("read");
/// assert_eq!(decoded, value);
/// ```
pub mod wire;

/// Lock-free single-producer single-consumer ring buffer for zero-copy data transfer.
///
/// This module provides the data plane for high-throughput communication
/// between domains. Unlike the control plane (IPC envelopes), the data plane
/// uses shared memory ring buffers for zero-copy transfers.
///
/// # Design
///
/// - **Lock-free**: Uses atomic indices for producer/consumer coordination
/// - **SPSC**: Single-producer, single-consumer for simplicity and performance
/// - **Power-of-2 sizes**: Buffer sizes are powers of 2 for efficient modulo
///
/// # Security
///
/// - Producer and consumer are in different trust domains
/// - Buffer access is mediated by capability handles
/// - Bounds are validated to prevent memory corruption
///
/// # Example
///
/// ```ignore
/// use kernel_api::ring_buffer::RingBuffer;
///
/// // Create a ring buffer (typically done by kernel)
/// let mut rb = RingBuffer::new(4096);
///
/// // Producer writes
/// let data = &[1, 2, 3, 4];
/// rb.write(data);
///
/// // Consumer reads
/// let mut buf = [0u8; 4];
/// rb.read(&mut buf);
/// ```
pub mod ring_buffer;

/// IDL-generated message types for typed IPC communication.
///
/// This module contains types generated from IDL (Interface Definition Language)
/// files by the `just codegen` command. Each protocol has its own generated
/// file with request and reply message types.
///
/// # Included Protocols
///
/// ## Harness Protocols
/// - `ping_harness`: Simple ping/pong for IPC testing
/// - `echo_harness_v0`: Echo service for message testing (legacy, no cap_handle)
/// - `echo_harness_v1`: Capability-based echo service (includes cap_handle)
/// - `capsule_control_v0`: Capsule lifecycle control
/// - `gpu_quarantine_v1`: GPU device quarantine interface
/// - `shmem_control_v1`: Shared memory region management
/// - `trace_service_v1`: Trace buffer service interface (legacy, request_id-based)
/// - `trace_service_v2`: Capability-based trace service (includes cap_handle)
///
/// ## Service Protocols
/// - `domain_manager_v1`: Domain creation and management
/// - `store_service_v1`: Artifact store service interface
///
/// ## Portal Protocols
/// - `portal_file_picker`: File selection portal
/// - `portal_clipboard`: Clipboard access portal
/// - `portal_notifications`: Notification portal
/// - `portal_screen_capture`: Screen capture portal
///
/// # Code Generation
///
/// These types are generated by `idl_codegen` from `.toml` IDL files in
/// the `/idl` directory. The generated code includes:
/// - Request and reply structs with `#[repr(C)]`
/// - Protocol and message type constants
/// - Serialization support via the `wire` module
///
/// # Example
///
/// ```ignore
/// use kernel_api::generated::CreateRegion;
/// use kernel_api::ipc::Envelope;
/// use kernel_api::wire;
///
/// let req = CreateRegion {
///     request_id: 1,
///     owner_domain_id: 100,
///     size_bytes: 4096,
///     flags: 0,
///     page_size: 4096,
/// };
///
/// let mut env = Envelope::empty(PROTOCOL_SHMEM_CONTROL, MSG_CREATE_REGION);
/// wire::write_payload(&mut env, &req).expect("write");
/// ```
/// S11.8 harness.net Oracle packet bytes shared by QEMU runtime I/O and Foundry gates.
pub mod net_packet_oracle_vector;

/// S13.6 harness.block Oracle sector bytes shared by QEMU runtime I/O and Foundry gates.
pub mod block_oracle_vector;

/// S10.5 semantic snapshot test vector shared by QEMU init and the host proxy.
pub mod semantic_snapshot_vector {
    /// Snapshot bytes whose SHA-256 prefix is `9c0de4419f03f426`.
    pub const S10_5_SEMANTIC_SNAPSHOT_BYTES: &[u8] = br#"{"schema_version":1,"system":{"arch":"x86_64","boot_id":"qemu-s10-5","uptime_seconds":0},"domains":[{"id":0,"name":"kernel","status":"Active"}],"harnesses":[{"interface":"services.semantic_state","providers":[0]},{"interface":"harness.shmem_control","providers":[0]}]}"#;

    /// Hex prefix of the SHA-256 digest used by S10.5 Foundry gates.
    pub const S10_5_SEMANTIC_SNAPSHOT_SHA256_PREFIX: &str = "9c0de4419f03f426";
}

pub mod generated {
    // This file is generated by `just codegen`.
    // Keep it in source control for Day 0 simplicity; later generate to OUT_DIR.
    include!("generated/ping_harness.generated.rs");
    include!("generated/capsule_control_v0.generated.rs");
    include!("generated/echo_harness_v0.generated.rs");
    include!("generated/portal_file_picker.generated.rs");
    include!("generated/portal_clipboard.generated.rs");
    include!("generated/portal_notifications.generated.rs");
    include!("generated/portal_screen_capture.generated.rs");
    include!("generated/domain_manager_v1.generated.rs");
    include!("generated/gpu_quarantine_v1.generated.rs");
    include!("generated/net_v1.generated.rs");
    include!("generated/block_v1.generated.rs");
    include!("generated/shmem_control_v1.generated.rs");
    include!("generated/store_service_v1.generated.rs");
    include!("generated/trace_service_v1.generated.rs");

    /// Semantic State Service (v1)
    /// Provides structured introspection of the operating system state.
    pub mod semantic_state_v1 {
        include!("generated/semantic_state_v1.generated.rs");
    }

    /// Capability-based echo harness (v1)
    /// Native WASM runners should use this version which includes cap_handle.
    pub mod echo_harness_v1 {
        include!("generated/echo_harness_v1.generated.rs");
    }

    /// Capability-based trace service (v2)
    /// Native WASM runners should use this version which includes cap_handle.
    pub mod trace_service_v2 {
        include!("generated/trace_service_v2.generated.rs");
    }

    /// Semantic store harness (v1) for projection storage queries.
    pub mod semantic_store_v1 {
        include!("generated/semantic_store_v1.generated.rs");
    }

    /// Execution fabric service (v1).
    pub mod execution_fabric_v1 {
        include!("generated/execution_fabric_v1.generated.rs");
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::cap::{Handle, HandleKind};
    use super::generated::{
        CloseRegion, CloseRegionReply, CreateRegion, CreateRegionReply, MapRegion, MapRegionReply,
        UnmapRegion, UnmapRegionReply,
    };
    use super::ipc::Envelope;
    use super::wire::{self, WireError};

    #[test]
    fn envelope_empty_sets_defaults() {
        let env = Envelope::empty(1, 2);
        assert_eq!(env.protocol, 1);
        assert_eq!(env.msg_type, 2);
        assert_eq!(env.handle, Handle::INVALID);
        assert_eq!(env.payload_len, 0);
    }

    #[test]
    fn semantic_state_v1_contract_roundtrip() {
        use super::generated::semantic_state_v1::{GetSnapshot, GetSnapshotReply};

        let req = GetSnapshot {
            cap_handle: 0x5310_0000_0000_0002,
            request_id: 42,
            format: 1,
        };
        let mut env = Envelope::empty(10, 1);
        wire::write_payload(&mut env, &req).expect("write payload");
        let decoded: GetSnapshot = wire::read_payload(&env).expect("read payload");
        assert_eq!(decoded.cap_handle, 0x5310_0000_0000_0002);
        assert_eq!(decoded.request_id, 42);
        assert_eq!(decoded.format, 1);

        let reply = GetSnapshotReply {
            request_id: 42,
            status: 0,
            shm_cap: 0x100,
            shm_size: 4096,
        };
        let mut reply_env = Envelope::empty(10, 2);
        wire::write_payload(&mut reply_env, &reply).expect("write reply");
        let decoded_reply: GetSnapshotReply = wire::read_payload(&reply_env).expect("read reply");
        assert_eq!(decoded_reply.shm_cap, 0x100);
        assert_eq!(decoded_reply.shm_size, 4096);
    }

    #[test]
    fn semantic_state_v1_payload_sizes_fit_ipc_envelope() {
        use super::generated::semantic_state_v1::{
            GetSnapshot, GetSnapshotReply, StateChangedEvent, Subscribe, SubscribeReply,
        };

        assert!(size_of::<GetSnapshot>() <= 64);
        assert!(size_of::<GetSnapshotReply>() <= 64);
        assert!(size_of::<Subscribe>() <= 64);
        assert!(size_of::<SubscribeReply>() <= 64);
        assert!(size_of::<StateChangedEvent>() <= 64);
    }

    #[test]
    fn semantic_store_v1_payload_sizes_fit_ipc_envelope() {
        use super::generated::semantic_store_v1::{
            QueryByPath, QueryByPathReply, QueryByTag, QueryByTagReply,
        };

        assert!(size_of::<QueryByPath>() <= 64);
        assert!(size_of::<QueryByPathReply>() <= 64);
        assert!(size_of::<QueryByTag>() <= 64);
        assert!(size_of::<QueryByTagReply>() <= 64);
    }

    #[test]
    fn semantic_store_v1_numeric_fields_roundtrip() {
        use super::generated::semantic_store_v1::{QueryByPath, QueryByPathReply};

        let req = QueryByPath {
            cap_handle: 0x10,
            request_id: 7,
            path_shm_cap: 0x200,
            path_len: 12,
        };
        let mut env = Envelope::empty(12, 1);
        wire::write_payload(&mut env, &req).expect("write payload");
        let decoded: QueryByPath = wire::read_payload(&env).expect("read payload");
        assert_eq!(decoded.cap_handle, 0x10);
        assert_eq!(decoded.request_id, 7);
        assert_eq!(decoded.path_shm_cap, 0x200);
        assert_eq!(decoded.path_len, 12);

        let reply = QueryByPathReply {
            request_id: 7,
            status: 0,
            content_id_hash: [0xabu8; 32],
        };
        let mut reply_env = Envelope::empty(12, 2);
        wire::write_payload(&mut reply_env, &reply).expect("write reply");
        let decoded_reply: QueryByPathReply = wire::read_payload(&reply_env).expect("read reply");
        assert_eq!(decoded_reply.status, 0);
        assert_eq!(decoded_reply.content_id_hash, [0xabu8; 32]);
    }

    #[test]
    fn execution_fabric_v1_payload_sizes_fit_ipc_envelope() {
        use super::generated::execution_fabric_v1::{
            AttachExecution, AttachExecutionReply, CancelExecution, CancelExecutionReply,
            RegisterNode, RegisterNodeReply, RequestLease, RequestLeaseReply, SubmitExecution,
            SubmitExecutionReply,
        };

        assert!(size_of::<RegisterNode>() <= 64);
        assert!(size_of::<RegisterNodeReply>() <= 64);
        assert!(size_of::<RequestLease>() <= 64);
        assert!(size_of::<RequestLeaseReply>() <= 64);
        assert!(size_of::<SubmitExecution>() <= 64);
        assert!(size_of::<SubmitExecutionReply>() <= 64);
        assert!(size_of::<AttachExecution>() <= 64);
        assert!(size_of::<AttachExecutionReply>() <= 64);
        assert!(size_of::<CancelExecution>() <= 64);
        assert!(size_of::<CancelExecutionReply>() <= 64);
    }

    #[test]
    fn execution_fabric_v1_numeric_fields_roundtrip() {
        use super::generated::execution_fabric_v1::{AttachExecutionReply, CancelExecution};

        let req = CancelExecution {
            request_id: 99,
            execution_id: 42,
            capability_token: 0xfeed,
        };
        let mut env = Envelope::empty(11, 11);
        wire::write_payload(&mut env, &req).expect("write payload");
        let decoded: CancelExecution = wire::read_payload(&env).expect("read payload");
        assert_eq!(decoded.request_id, 99);
        assert_eq!(decoded.execution_id, 42);
        assert_eq!(decoded.capability_token, 0xfeed);

        let reply = AttachExecutionReply {
            request_id: 99,
            status: 0,
            stream_id: 5,
        };
        let mut reply_env = Envelope::empty(11, 10);
        wire::write_payload(&mut reply_env, &reply).expect("write reply");
        let decoded_reply: AttachExecutionReply =
            wire::read_payload(&reply_env).expect("read reply");
        assert_eq!(decoded_reply.stream_id, 5);
    }

    #[test]
    fn shmem_control_contract_roundtrip() {
        let req = CreateRegion {
            request_id: 11,
            owner_domain_id: 700,
            size_bytes: 4096,
            flags: 1,
            page_size: 4096,
        };
        let mut env = Envelope::empty(8, 1);
        wire::write_payload(&mut env, &req).expect("write payload");
        let decoded: CreateRegion = wire::read_payload(&env).expect("read payload");

        assert_eq!(decoded.request_id, req.request_id);
        assert_eq!(decoded.owner_domain_id, req.owner_domain_id);
        assert_eq!(decoded.size_bytes, req.size_bytes);
        assert_eq!(decoded.flags, req.flags);
        assert_eq!(decoded.page_size, req.page_size);
    }

    #[test]
    fn shmem_control_contract_payload_sizes_fit_ipc_envelope() {
        assert!(size_of::<CreateRegion>() <= 64);
        assert!(size_of::<CreateRegionReply>() <= 64);
        assert!(size_of::<MapRegion>() <= 64);
        assert!(size_of::<MapRegionReply>() <= 64);
        assert!(size_of::<UnmapRegion>() <= 64);
        assert!(size_of::<UnmapRegionReply>() <= 64);
        assert!(size_of::<CloseRegion>() <= 64);
        assert!(size_of::<CloseRegionReply>() <= 64);
    }

    #[test]
    fn wire_read_payload_rejects_len_mismatch_larger_than_type() {
        let req = CreateRegion {
            request_id: 22,
            owner_domain_id: 701,
            size_bytes: 8192,
            flags: 1,
            page_size: 4096,
        };
        let mut env = Envelope::empty(8, 1);
        wire::write_payload(&mut env, &req).expect("write payload");
        env.payload_len += 1;

        let err = wire::read_payload::<CreateRegion>(&env).unwrap_err();
        assert_eq!(err, WireError::PayloadLenMismatch);
    }

    #[test]
    fn wire_read_payload_rejects_len_too_small() {
        let req = CreateRegion {
            request_id: 23,
            owner_domain_id: 702,
            size_bytes: 16384,
            flags: 2,
            page_size: 4096,
        };
        let mut env = Envelope::empty(8, 1);
        wire::write_payload(&mut env, &req).expect("write payload");
        env.payload_len -= 1;

        let err = wire::read_payload::<CreateRegion>(&env).unwrap_err();
        assert_eq!(err, WireError::PayloadTooSmall);
    }

    #[test]
    fn wire_write_payload_sets_strict_len_and_zeroes_tail() {
        let value: u32 = 0xA5A5_5A5A;
        let mut env = Envelope::empty(9, 9);
        env.payload.fill(0xFF);

        wire::write_payload(&mut env, &value).expect("write payload");

        assert_eq!(env.payload_len as usize, size_of::<u32>());
        assert_eq!(
            &env.payload[size_of::<u32>()..],
            &[0u8; 64 - size_of::<u32>()]
        );

        let decoded: u32 = wire::read_payload(&env).expect("read payload");
        assert_eq!(decoded, value);
    }

    // V-05/V-06: Handle pack/unpack tests
    #[test]
    fn handle_pack_unpack_roundtrip() {
        let handle = Handle {
            kind: HandleKind::Ipc,
            index: 42,
            generation: 123,
        };
        let packed = handle.pack();
        let unpacked = Handle::unpack(packed);
        assert_eq!(handle, unpacked);
    }

    #[test]
    fn handle_invalid_is_zero() {
        assert_eq!(Handle::INVALID.index, 0);
        assert_eq!(Handle::INVALID.generation, 0);
        assert_eq!(Handle::INVALID.pack(), 0);
    }

    #[test]
    fn handle_pack_preserves_index_and_generation() {
        // V-004: Test with valid wire format values
        // Wire format: [kind: 8bits | reserved: 8bits | index: 16bits | generation: 32bits]
        let handle = Handle {
            kind: HandleKind::Ipc,
            index: 0xBEEF, // Only lower 16 bits are preserved
            generation: 0xCAFEBABE,
        };
        let packed = handle.pack();
        // Upper bits: kind (1) << 56 | index (0xBEEF) << 32
        assert_eq!((packed >> 56) & 0xFF, 1); // kind
        assert_eq!((packed >> 32) & 0xFFFF, 0xBEEF); // index
        assert_eq!((packed & 0xFFFFFFFF) as u32, 0xCAFEBABE); // generation
    }

    #[test]
    fn handle_unpack_reconstructs_original() {
        // V-004: Test with valid wire format
        // Packed: [kind=1 (Ipc) | reserved=0 | index=0xBEEF | generation=0xCAFEBABE]
        let packed: u64 = (1u64 << 56) | (0xBEEFu64 << 32) | 0xCAFEBABEu64;
        let handle = Handle::unpack(packed);
        assert_eq!(handle.kind, HandleKind::Ipc);
        assert_eq!(handle.index, 0xBEEF);
        assert_eq!(handle.generation, 0xCAFEBABE);
    }
}
