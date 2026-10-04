# Multi-Domain Memory Architecture

**Last Updated:** 2026-10-03
**Status:** Kernel primitives and bounded tests landed; complete target isolation pending

Domains are intended execution and authority boundaries. The kernel has domain
registration, address-space roots, shared-memory accounting, and architecture
MMU operations. These foundations do not yet provide a general target userspace
loader or qualify isolation of arbitrary untrusted programs.

## Ownership and limits

| Component | Role |
|-----------|------|
| [Domain registry](../kernel/src/domain_registry.rs) | Bounded domain IDs/state; `MAX_DOMAINS = 16` |
| [Address-space table](../kernel/src/mm/address_space.rs) | Per-domain page-table roots |
| [MMU interface](../kernel/src/arch/mmu.rs) | Architecture mapping/unmapping contract |
| [x86_64 MMU](../kernel/src/arch/x86_64/mmu.rs) | Page-table allocation/traversal and leaf mappings |
| [aarch64 MMU](../kernel/src/arch/aarch64/mmu.rs) | Translation-table allocation/traversal and leaf mappings |
| [Shared memory](../kernel/src/shmem.rs) | Region capability validation, frames, mapping reservations, refcounts |
| [Host Domain Manager](../services/domain_manager/) | Lifecycle/broker policy and host integration |

Domain 0 is reserved for the kernel context. Address-space roots and bounded
backing structures are explicit resources; missing roots, exhaustion, or invalid
IDs cannot be interpreted as successful domain creation. Architecture code can
allocate missing intermediate tables during mapping; the old claim that mapping
only works in domain 0 is superseded.

## Shared-memory mapping lifecycle

A region has a typed handle, owner/accounting state, backing frames, and allowed
rights. Mapping validates the capability and requested domain/range, reserves
mapping state, and invokes the architecture MMU. Failure paths must unwind
reservations and frame/reference accounting. Unmapping updates that state only
according to the corresponding MMU result.

Shared mappings deliberately expose the same physical bytes to authorized
participants. Capability rights and page permissions limit intended access;
consumers must also preserve object lifetime and typed bounds. A mapped pointer
cannot remain valid after its backing mapping is revoked or released.

[Ring Buffer V0](RING_BUFFER_V0.md) is one consumer. Its SPSC data operations
have additional role, pointer-validity, and cooperating-peer requirements.
A capability-checked setup does not validate every subsequent byte or index.

## Concurrency and evidence boundary

Frame and address-space backing use protected state, while the shared-memory
control table assumes a single control thread. Locks around allocation do not
qualify page-table mutation, TLB invalidation, context switching, or DMA isolation
for multi-core workloads. Architecture-specific barriers and target behavior
need their own assertions.

Host unit tests exercise registries, allocation/accounting, handle checks, and
mapping failure bookkeeping. QEMU shared-memory gates exercise selected target
paths. Host MMU scaffolding cannot prove hardware page permissions, and selected
QEMU paths cannot prove a complete process-isolation boundary on metal.

Run the shared-memory recipes in [justfile](../justfile), including
`just foundry-shmem-dataplane-s8-phase4-integration`, for their named assertions.
Complete domain loading, multi-core MMU qualification, teardown/lifetime checks,
DMA boundaries, and physical fault containment remain separately gated work.
[Current Status](../CURRENT_STATUS.md), [Next Tasks](../NEXT_TASKS.md), and
[Security Status](../SECURITY_STATUS.md) own the current evidence and priorities.
