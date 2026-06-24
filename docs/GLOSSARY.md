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

### Kernel

The core of RamenOS providing IPC, capabilities, memory management, and domain isolation. The kernel implements capability validation for fast-path operations and maintains strict separation between control plane (typed messages) and data plane (zero-copy shared memory).

**Related Terms:** Service, Domain, Capability

**See Also:** [Platform Overview](../PLATFORM_OVERVIEW.md)

### Service

A user-space process providing system functionality. Services run in isolated domains and communicate via IPC. Examples include the Domain Manager, Store Service, and Capsule Relay.

**Related Terms:** Kernel, Domain, Harness

**See Also:** [Platform Overview](../PLATFORM_OVERVIEW.md)

### Store

The artifact storage and verification system. The Store manages persistent artifacts (traces, claims, crash contexts) with cryptographic verification and capability-based access control.

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

An isolated execution context with its own capabilities, address space, and resource limits. Domains are the fundamental isolation boundary in RamenOS. Each domain has a capability table that determines what resources it can access.

**Related Terms:** Capability, Address Space, Domain Manager

**See Also:** [Multi-Domain Documentation](MULTI_DOMAIN.md)

---

## Security Terms

### Capability

An unforgeable token granting specific rights to a resource. Capabilities are the foundation of RamenOS security. They are stored in kernel-managed capability tables and referenced via Handles. Capabilities cannot be forged or modified by user-space code.

**Related Terms:** Handle, Token, Capability Table

**See Also:** [Security Status](../SECURITY_STATUS.md)

### Handle

A kernel-managed reference to a capability, consisting of an index and generation counter. Handles are the user-space representation of capabilities. The generation counter prevents handle reuse attacks when capabilities are revoked and the slot is reallocated.

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

A 32-bit counter embedded in handles that prevents handle reuse attacks. When a capability is revoked and its slot in the capability table is reallocated, the generation counter increments. This ensures stale handles from one domain cannot accidentally reference new capabilities allocated to different domains.

**Related Terms:** Handle, Capability

**See Also:** [Kernel Capability Table](../kernel/src/cap_table.rs)

---

## Development Terms

### Slice

A vertical development unit that ships a complete feature end-to-end. Slices cut across all layers (kernel, services, IDL, tests) rather than building horizontal subsystems in isolation. Each slice has defined completion criteria and Foundry gates.

**Related Terms:** Foundry, Gate

**See Also:** [Slices Documentation](../SLICES.md)

### Foundry

The test framework for running QEMU-based integration tests. Foundry scripts boot RamenOS in QEMU and validate specific functionality by checking output logs and behavior. Foundry tests are called "Gates."

**Related Terms:** Gate, Slice

**See Also:** [Tools Directory](../tools/ci/)

### Gate

An individual Foundry test script that validates specific functionality. Gates run QEMU with specific configurations and assert expected behavior. Gates are named for the slice they test (e.g., `foundry_s0.sh`, `foundry_shmem_control_s8_phase2.sh`).

**Related Terms:** Foundry, Slice

**See Also:** [Tools CI Directory](../tools/ci/)

### IDL

Interface Definition Language for defining message formats and service contracts. IDL files (`.toml`) define protocols, message types, and field layouts. The `idl_codegen` tool generates Rust code from IDL definitions.

**Related Terms:** Protocol, Message Type, Wire Format

**See Also:** [IDL Directory](../idl/), [IDL Codegen](../idl_codegen/)

### Envelope

A 64-byte IPC message container. Envelopes contain a header (protocol ID, message type, flags) and payload. All IPC messages in RamenOS use the envelope format for uniformity.

**Related Terms:** IPC, Protocol, Message Type

**See Also:** [Kernel API](../kernel_api/src/lib.rs)

---

## Component Terms

### Capsule

An isolated execution unit that can be either a driver or compatibility domain. Capsules run with restricted capabilities and communicate with the rest of the system via well-defined interfaces. Driver capsules provide hardware abstraction; compat capsules run legacy OS code.

**Related Terms:** Compat Domain, Domain, Capsule Relay

**See Also:** [Driver Capsule Specification](../DRIVER_CAPSULE_SPEC.md)

### Compat Domain

A domain running a legacy OS (typically Linux) with restricted capabilities. Compat domains enable running existing applications while maintaining RamenOS security boundaries. The POSIX runner manages compat domain execution.

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

The virtual memory context for a domain. Each domain has its own address space, providing isolation from other domains. Address spaces are managed by the kernel's memory management subsystem.

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

The binary encoding for IPC messages. Wire formats define how message fields are serialized into the 64-byte envelope. The IDL codegen generates serialization and deserialization code for each message type.

**Related Terms:** Envelope, Protocol, Message Type

**See Also:** [Kernel API Wire Module](../kernel_api/src/wire.rs)

---

## Cross-Reference Index

| Term | Category | Primary Related Terms |
|------|----------|----------------------|
| Kernel | Architecture | Service, Domain, Capability |
| Service | Architecture | Kernel, Domain, Harness |
| Store | Architecture | Store Service, Capability |
| Harness | Architecture | Service, Portal, Capability |
| Portal | Architecture | Harness, Service, Capability |
| Domain | Architecture | Capability, Address Space |
| Capability | Security | Handle, Token |
| Handle | Security | Capability, Generation Counter |
| HandleKind | Security | Handle, Capability |
| Token | Security | Capability, Signature |
| Generation Counter | Security | Handle, Capability |
| Slice | Development | Foundry, Gate |
| Foundry | Development | Gate, Slice |
| Gate | Development | Foundry, Slice |
| IDL | Development | Protocol, Message Type |
| Envelope | Development | IPC, Protocol |
| Capsule | Component | Compat Domain, Domain |
| Compat Domain | Component | Capsule, Domain |
| Domain Manager | Component | Domain, Service |
| Store Service | Component | Store, Service |
| Capsule Relay | Component | Capsule, Service |
| Shared Memory | Memory | Frame, Address Space |
| Frame | Memory | Shared Memory, Address Space |
| Address Space | Memory | Domain, Frame |
| Ring Buffer | Memory | Trace, IPC |
| Protocol | IPC | Message Type, IDL |
| Message Type | IPC | Protocol, Wire Format |
| Wire Format | IPC | Envelope, Protocol |