# RamenOS Glossary

This document defines key terms used throughout the RamenOS project documentation and codebase.

## Table of Contents

- [Architecture Terms](#architecture-terms)
- [Security Terms](#security-terms)
- [Development Terms](#development-terms)
- [Component Terms](#component-terms)
- [Memory Terms](#memory-terms)
- [IPC Terms](#ipc-terms)

---

## Architecture Terms

### Post-Unix

Native OS interfaces follow RamenOS's product needs through typed Harnesses,
Portals, and explicit capabilities. POSIX and legacy stacks are compatibility
paths rather than the native API blueprint. Existing techniques remain useful
when they serve the design.

**See Also:** [Vision](../VISION.md), [Constitution](../CONSTITUTION.md)

### Everyday OS for Humans and AI Agents

The product destination: approachable human interaction and structured agent
interaction on one modular OS, with explicit policy, useful compatibility, and
recoverable failures. RamenOS is currently public pre-alpha; this term describes
the goal rather than current desktop or release readiness.

**See Also:** [Vision](../VISION.md), [Current Status](../CURRENT_STATUS.md)

### Kernel

The core of RamenOS providing IPC, capabilities, memory management, and domain isolation. The kernel implements capability validation for fast-path operations and maintains strict separation between control plane (typed messages) and data plane (zero-copy shared memory).

**Related Terms:** Service, Domain, Capability

**See Also:** [Platform Overview](../PLATFORM_OVERVIEW.md)

### Service

A component providing system functionality through typed contracts. The target
model runs services in isolated domains; many current services are host processes
or libraries. The execution environment determines the actual isolation boundary.

**Related Terms:** Kernel, Domain, Harness

**See Also:** [Platform Overview](../PLATFORM_OVERVIEW.md)

### Store

The software discovery, launch, porting, and publication pillar. It uses the
content-addressed Artifact Store for blobs, manifests, and evidence, and the
Store Service for artifact I/O and verification. The complete product UI remains work.

**Related Terms:** Store Service, Capability, Evidence Policy

**See Also:** [Store Specification](../STORE_SPEC.md)

### Harness

A kernel-level service interface providing core OS capabilities. Harnesses include `shmem_control` (shared memory management), `trace_service` (kernel tracing), and `echo_harness` (IPC testing). Harnesses are accessed via IPC with capability validation.

**Related Terms:** Service, Portal, Capability

**See Also:** [IDL Definitions](../idl/)

### Portal

A user-space portal for desktop integration. Portals provide controlled access to system resources like clipboard, file picker, notifications, and screen capture. Portals implement the principle of least privilege by requiring explicit capability grants.

**Related Terms:** Harness, Service, Capability

**See Also:** [Portal IDL Definitions](../idl/portals/)

### Domain

An intended execution and authority boundary with scoped capabilities, memory,
and resource accounting. Current kernel registries and host lifecycle fixtures
implement parts of that model; the general target userspace loader is pending.

**Related Terms:** Capability, Address Space, Domain Manager

**See Also:** [Multi-Domain Documentation](MULTI_DOMAIN.md)

---

## Security Terms

### Capability

Explicit rights to a resource, validated by the authority that owns it. Kernel
handles refer to validated kernel table entries; host service/task grants have
their own registries and lifetimes. A syntactically valid ID does not itself grant
authority or establish unforgeability.

**Related Terms:** Handle, Token, Capability Table

**See Also:** [Security Status](../SECURITY_STATUS.md)

### Handle

A typed reference carrying kind, index, and generation information. The owning
table validates those fields and rights. Kernel handles and scoped host grant
handles must not be treated as interchangeable.

**Related Terms:** Capability, Generation Counter, HandleKind

**See Also:** [Kernel Capability Table](../kernel/src/cap_table.rs)

### HandleKind

A discriminator indicating the type of resource a handle references. HandleKinds include `Ipc` (IPC endpoint), `Shmem` (shared memory region), and `Trace` (trace buffer). The kernel uses HandleKind to route operations correctly.

**Related Terms:** Handle, Capability

**See Also:** [Kernel API](../kernel_api/src/lib.rs)

### Token

A cryptographic credential used for authentication and authorization. Unlike capabilities (which are kernel-managed), tokens are cryptographic constructs that can be verified without kernel involvement. Examples include display capability tokens for GPU access.

**Related Terms:** Capability, Signature

**See Also:** [Artifact Store Schema](../artifact_store_schema/src/signature.rs)

### Generation Counter

A value checked against a resource slot to reject stale references after reuse.
`kernel_api::cap::Handle` stores a u64 generation; its packed wire form carries
the low 32 generation bits and low 16 index bits. That finite encoding needs
its stated lifetime/reuse bounds; it is not an unlimited anti-aliasing guarantee.

**Related Terms:** Handle, Capability

**See Also:** [Kernel Capability Table](../kernel/src/cap_table.rs)

---

## Development Terms

### Slice

A vertical development unit that ships a complete feature end-to-end. Slices cut across all layers (kernel, services, IDL, tests) rather than building horizontal subsystems in isolation. Each slice has defined completion criteria and Foundry gates.

**Related Terms:** Foundry, Gate

**See Also:** [Slices Documentation](../SLICES.md)

### Foundry

The tooling and evidence pillar: host assertions, contract checks, trace/replay,
QEMU integration, and opt-in hardware gates. Each gate states a bounded claim
and its evidence level.

**Related Terms:** Gate, Slice

**See Also:** [Tools Directory](../tools/ci/)

### Gate

A Foundry check for named behavior and negative cases. It may run host tests,
replay, QEMU, inventory, or prepared physical HIL. PASS establishes only those
assertions; missing prerequisites or physical evidence may be INCOMPLETE.

**Related Terms:** Foundry, Slice

**See Also:** [Tools CI Directory](../tools/ci/)

### IDL

Interface Definition Language for defining message formats and service contracts. IDL files (`.toml`) define protocols, message types, and field layouts. The `idl_codegen` tool generates Rust code from IDL definitions.

**Related Terms:** Protocol, Message Type, Wire Format

**See Also:** [IDL Directory](../idl/), [IDL Codegen](../idl_codegen/)

### Envelope

The typed IPC container with protocol, message type, handle, length, and a fixed
64-byte payload. The bridge encoding in `ipc_frame.rs` is 88 bytes before its
length prefix. The payload capacity is not the total message size.

**Related Terms:** IPC, Protocol, Message Type

**See Also:** [Kernel API](../kernel_api/src/lib.rs)

---

## Component Terms

### Capsule

An isolated execution unit that can be either a driver or compatibility domain. Capsules run with restricted capabilities and communicate with the rest of the system via well-defined interfaces. Driver capsules provide hardware abstraction; compat capsules run legacy OS code.

**Related Terms:** Compat Domain, Domain, Capsule Relay

**See Also:** [Driver Capsule Specification](../DRIVER_CAPSULE_SPEC.md)

### Compat Domain

A compatibility environment for legacy software, currently including a Linux VM
launched by `compat_runner`. The separate POSIX runner executes shell scripts
on the host with its documented rlimits-only default; it does not manage that VM.

**Related Terms:** Capsule, Domain, POSIX Runner

**See Also:** [Compat Capsule Documentation](COMPAT_CAPSULE_V0.md)

### Domain Manager

The service managing domain lifecycle and resource allocation. The Domain Manager creates, destroys, and configures domains, allocates memory and capabilities, and facilitates IPC setup between domains.

**Related Terms:** Domain, Service, Capability

**See Also:** [Domain Manager Service](../services/domain_manager/)

### Store Service

The service providing artifact storage and verification. The Store Service implements the Store specification, managing artifact persistence, cryptographic verification, and capability-based access control.

**Related Terms:** Store, Service, Capability

**See Also:** [Store Service](../services/store_service/)

### Capsule Relay

The service bridging capsules to VM backends. The Capsule Relay manages communication between RamenOS domains and capsule execution environments, handling IPC translation and capability mediation.

**Related Terms:** Capsule, Service, VM Backend

**See Also:** [Capsule Relay Service](../services/capsule_relay/)

---

## Memory Terms

### Shared Memory (Shmem)

A zero-copy data plane communication mechanism. Shared memory enables high-throughput data transfer between domains without copying through the kernel. Shmem regions are capability-controlled and require explicit allocation and mapping.

**Related Terms:** Frame, Address Space, Capability

**See Also:** [Kernel Shmem](../kernel/src/shmem.rs)

### Frame

A physical memory page (4KB). Frames are the unit of physical memory allocation. The kernel maintains a frame allocator and tracks frame ownership via capabilities.

**Related Terms:** Shared Memory, Address Space

**See Also:** [Kernel Memory Management](../kernel/src/mm/)

### Address Space

A domain memory context represented by a page-table root. Kernel mapping
primitives implement parts of the intended separation; complete target process
isolation and multi-core qualification require additional integration evidence.

**Related Terms:** Domain, Frame, MMU

**See Also:** [Kernel Address Space](../kernel/src/mm/address_space.rs)

### Ring Buffer

A lock-free single-producer single-consumer (SPSC) queue for kernel-userspace communication. Ring buffers are used for trace events, IPC notifications, and other high-frequency data streams. They enable efficient communication without kernel transitions.

**Related Terms:** Trace, IPC

**See Also:** [Ring Buffer Documentation](RING_BUFFER_V0.md)

---

## IPC Terms

### Protocol

A namespace for related message types. Protocols group operations that serve a common purpose (e.g., `shmem_control_v1`, `domain_manager_v1`). Each protocol has a unique ID used in message routing.

**Related Terms:** Message Type, IDL, Envelope

**See Also:** [IDL Directory](../idl/)

### Message Type

A specific operation within a protocol. Message types define the structure and semantics of individual operations (e.g., `ShmemAllocate`, `ShmemMap` within the `shmem_control_v1` protocol).

**Related Terms:** Protocol, IDL, Wire Format

**See Also:** [Generated IPC Code](../kernel_api/src/generated/)

### Wire Format

The binary encoding for IPC messages. Wire formats define how message fields are serialized into the envelope's bounded 64-byte payload. IDL generates layouts and IDs; the current wire helpers copy native struct
representations, while the outer bridge frame has a separate explicit encoding.

**Related Terms:** Envelope, Protocol, Message Type

**See Also:** [Kernel API Wire Module](../kernel_api/src/wire.rs)

---
