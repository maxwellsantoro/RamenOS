# Boot frame ownership v0

**Status:** Pure RUN0.0 admission prerequisite implemented and independently
reviewed; 17 named assertions pass. Firmware adapter, actual ownership transition
and QEMU allocator acceptance remain pending.

The pure prerequisite admits a checked, bounded pool from supplied typed map and
retention records. The successor adapter must obtain the actual map returned by
successful firmware exit and connect admission to the physical-frame allocator.
A real QEMU consumer
must allocate, write, read, release and reuse frames while the existing S8
shared-memory consumer continues working. This establishes one boot ownership
boundary. It does not establish target application execution, user-mode isolation,
native device access, interrupt recovery, or physical-machine qualification.

## Source and prerequisites

At base `f80d017e4ce2d2db472a2658ba0c23b298bdce09`,
`kernel_uefi/src/main.rs::efi_main` enters `boot_main` with boot services active.
Its earlier map converts boot-services code/data to usable memory and precedes
later probes. `BootMemoryMap::add` silently drops entries beyond 64.
`mm::init` selects the largest usable range, rounds its length up to pages, and
logs reservations without excluding them. `BitmapAllocator` accepts at most
131072 frames. These are source observations, not observations of corruption.

The pinned local primary source is `uefi` 0.27.0
`src/table/system.rs::SystemTable<Boot>::exit_boot_services`. It consumes the
boot-time table and returns `(SystemTable<Runtime>, MemoryMap<'static>)`. It
allocates the final map buffer with the caller's memory type, gets a current map
and exits together, tries at most twice, and resets on unrecoverable failure.
The successor adapter must call it with `MemoryType::LOADER_DATA`, without an
outer retry, fake success result, or reuse of the earlier map.

Complete init-image loading and every existing GOP, IOMMU, NVMe, A/B-slot and
boot-nonce probe before consuming `st`. Close scoped firmware protocols and
other boot-service-dependent resources before exit. Immediately after a
successful return, the successor architecture glue must mask interrupts and
observe RFLAGS.IF clear before kernel entry or allocator construction. Own GDT/IDT, timer and
user-mode entry remain successor work. No firmware call is permitted afterward.

The coordinator owns module declarations, boot/architecture glue, map conversion,
allocator installation, gate registration and shared output resources. Workers
implement only subsequently assigned exact paths. This contract and its pure
assertions introduce no native IPC interface or new kernel dependency.

## Pure validation API

The implemented module is `kernel::mm::boot_pool`. It is allocation-free and does
not read registers, dereference physical pointers, initialize an allocator,
change globals, or assert that its caller actually exited firmware.

```rust,ignore
pub const MAX_BOOT_REGIONS: usize = 256;
pub const MAX_RETAINED_RANGES: usize = 256;
pub const MAX_POOL_FRAMES: usize = 131072;
pub const POOL_START: u64 = 4096;
pub const POOL_END_EXCLUSIVE: u64 = 0xc000_0000;
pub const MAX_PHYSICAL_END_EXCLUSIVE: u64 = 1 << 52;
pub const RETENTION_END_EXCLUSIVE: u64 = 1 << 32;

pub enum FirmwareStage {
    BootServicesActive,
    ExitedInterruptsUnverified,
    ExitedInterruptsMasked,
}

pub enum RetainedKind {
    CurrentStack, InitImage, KernelImage, PageTables, Gdt, Idt, FinalMapBuffer,
}

pub struct RetainedRange {
    pub start: u64,
    pub len_bytes: u64,
    pub kind: RetainedKind,
}

pub enum BootPoolError {
    FirmwareNotOwned, InvalidMapCount, InvalidRegion, OverlappingRegions,
    InvalidRetentionCount, InvalidRetainedRange, MissingRetention,
    UnreservedRetention, NoEligiblePool,
}

pub struct BootPool { /* private validated fields */ }
impl BootPool {
    pub fn base(&self) -> PhysFrame;
    pub fn frame_count(&self) -> usize;
    pub fn contains(&self, frame: PhysFrame) -> bool;
}

pub fn select_boot_pool(
    map: &BootMemoryMap,
    stage: FirmwareStage,
    retained: &[RetainedRange],
) -> Result<BootPool, BootPoolError>;
```

These enums and range records support `Copy`, `Clone`, `Debug`, `PartialEq`, and
`Eq`. `BootPool` supports `Debug`; only successful validation constructs it.
Errors are values, not panics. The future allocator installer must accept this validated
pool rather than reselecting a range from unchecked input.

`BootMemoryMap` remains the maintained handoff type; its fixed capacity changes
to 256. Its existing `add` API preserves a private sticky overflow flag when an
insertion exceeds capacity, leaves the count bounded, and never writes outside
the array. `is_complete()` reports whether any descriptor was lost; callers
cannot clear that flag by changing the public count. The pure selector rejects
an incomplete map as `InvalidMapCount` before indexing. All final descriptors count
toward this bound, including non-usable descriptors. The first actual OVMF gate
records descriptor count. This bound is a proposed supported profile, not an
assertion about every firmware. An overflowing real map fails closed and requires
reviewed capacity adjustment; truncation, filtering to hide overflow and silent
compaction are forbidden.

`RegionKind::Usable` means only final EFI `CONVENTIONAL` memory without the
`RUNTIME` attribute. The production converter is responsible for this meaning;
the pure selector cannot attest the origin of a typed map. Known loader code/data
retain their existing corresponding kinds. Known boot-services, runtime, ACPI,
MMIO and other supported non-conventional types become `Reserved`. Unknown raw
EFI types fail conversion; they never become usable. Attribute classification
requires checking the supported mask in the pinned UEFI definitions before
freezing raw converter assertions; an unknown attribute is not by itself evidence
of unsafe backing. Raw conversion fixtures belong to the later adapter gate, because the
four typed `RegionKind` variants cannot represent an unknown raw EFI value.

The collector retains a bounded parallel record of every original EFI type,
attribute, start and page count. `Reserved` includes MMIO and is not a
dereferenceability proof. Before reading a retained object or a paging-structure
frame, require supported raw RAM backing and a valid current mapping for the
complete accessed extent. MMIO, unaccepted/unusable memory, unsupported raw
backing and inaccessible extents fail collection. The supported raw RAM-type and
attribute policy is frozen separately from the pure selector; do not discard the
raw classification after projection into `RegionKind`.

Validation proceeds in this order:

1. Require `ExitedInterruptsMasked`. Production sets this stage only after the
   actual exit result and observed interrupt masking. A test enum value alone
   is synthetic input, not firmware evidence.
2. Require `map.is_complete()` and `1 <= map.count <= 256`, with array capacity exactly 256. Before any
   indexing, reject a count beyond capacity. Each populated descriptor has
   nonzero page-aligned length and page-aligned start, checked end arithmetic,
   and end at most `1 << 52`. Validate every descriptor, even a non-candidate.
   Reject duplicate or overlapping descriptor intervals; adjacency and unsorted
   input are allowed. Ignore unused array slots.
3. Require `1 <= retained.len() <= 256`, with at least one range for every
   `RetainedKind`. Every retained range is nonzero, page-aligned, checked and
   below the 4 GiB retention bound of the selected x86/QEMU identity-access
   profile. It must fit completely inside one
   validated non-usable descriptor. Overlapping retained ranges and repeated
   kinds are allowed: several reasons can retain the same descriptor, and page
   tables can occupy several descriptors. A missing, uncovered or conventionally
   backed retained range fails closed. This v0 does not carve retained ranges
   out of conventional descriptors.
4. Intersect each usable descriptor with `[4096, 0xc0000000)`. The
   upper bound is the existing x86 boot profile's MMIO boundary, not a general
   hardware inventory claim. Empty intersections are ineligible; a descriptor
   crossing either boundary contributes only its in-profile pages. Compare
   complete eligible interval lengths before the capacity limit, choosing the
   largest interval and breaking ties by lowest base. Then clip the selected
   interval to 131072 frames from its start; this explicit capacity policy leaves
   the remainder unused. Do not combine descriptors. Return `NoEligiblePool` if
   no intersection qualifies.

No rounding up, saturating arithmetic, silent invalid-record dropping, firmware
memory reclamation, or allocator reset is permitted. `contains` accepts exactly
the frames in the selected half-open range. The 52-bit bound is structural;
the real adapter must also check the actual CPU's supported physical width and
current translation mode before following page tables.

## Retention evidence and required architecture glue

Retention records are derived from actual objects and the returned final map,
not from a presumed firmware allocation type. Retain the complete containing
non-usable descriptor for each observed object. If an object spans descriptors,
record each containing descriptor and prove complete coverage of its byte range.
Reject gaps, unsupported translations, conventional backing, or capacity excess.
The pure helper validates the records; the adapter proves their completeness.

| Reason | Required source and coverage |
|---|---|
| CurrentStack | Observe current RSP and its actual translated physical address. Retain its complete final descriptor, and constrain remaining boot/kernel stack use to that retained extent. A single sampled address does not prove arbitrary future stack growth; use an explicit owned stack if that extent cannot be established. |
| InitImage | Record the actual `allocate_pages` base and complete allocated page length, separately from bytes read. Check the init byte range is inside that allocation. Existing `InitImage.len` alone is insufficient. |
| KernelImage | Before exit, use the current image handle's `LoadedImage::info()` base/size for the complete UEFI image containing the linked kernel. Translate and cover that extent after exit; do not infer LoaderCode backing. |
| PageTables | Read masked CR3 and enumerate every reachable paging-structure frame under the supported four-level mode, honoring huge-page leaves. Require a bounded complete walk, validated addresses, a visited-frame bound of 256, and no malformed cycles. Retain root and all intermediate table frames. The current MMU assumes identity-accessible physical table pointers; validate that assumption before dereference or provide safe architecture access. |
| Gdt / Idt | Observe SGDT/SIDT bases and inclusive limits, checked `limit + 1`, actual translations and complete physical coverage. Retain backing ranges; this does not establish own descriptors or trap handlers. |
| FinalMapBuffer | Derive physical backing from public returned map-entry references and retain every containing final descriptor. Public `entries().len()` gives the count; the allocation length and slack remain private. Do not transmute private layout or claim the last entry proves the complete allocated length. The pinned API allocates the buffer as LoaderData; preserve every original LoaderData descriptor, including descriptors without an entry reference, so unobserved allocation slack and headers stay outside the pool. Reject unsupported entry backing or incomplete observed-entry coverage. |

The complete physical mapped extents used by this collector must be below 4 GiB
for this selected x86/QEMU profile; this is not a general hardware claim.
The architecture collector must finish without allocating from the future pool.
All coverage and physical-address dereferences remain fallible. Preserve the
runtime table and final map as live retained values without calling runtime
services. Existing firmware table pointer retention needs explicit accounting if
later kernel consumers dereference them; probe summaries that copied values do
not by themselves retain arbitrary ACPI tables. These glue changes return to the
coordinator for exact ownership assignment.

The existing x86 implementation casts page-table physical addresses directly to
pointers under an identity-mapping assumption. A descriptor's type and an address
bound do not validate that assumption. The coordinator must establish a supported
initial table-access contract before the first physical-table dereference; a
walk cannot bootstrap its own unchecked physical reader. The complete bounded
walk must then validate current translations for every accessed object. Before
the actual QEMU allocator consumer dereferences any selected pool page, establish
identity translation and effective write permission across the complete page,
including ancestor permissions and huge-page mappings. Reject missing,
non-identity or read-only mappings before read/write, rather than relying on an
unhandled fault. Pure `BootPool` construction and address bounds establish none
of these CPU mapping observations.

Page-table discovery uses a bounded iterative work list and visited-frame set,
at most 256 frames with 512 entries per frame, without recursive expansion. Check
the actual CPU paging mode and physical width, distinguish 1 GiB/2 MiB huge leaves
from intermediate tables, and reject malformed flags, addresses, cycles,
inconsistent depth reuse or capacity excess before the next dereference. Scratch
storage is borrowed from retained image memory or an explicit pre-exit LoaderData
allocation, avoiding an unbounded demand on the firmware stack. The pinned raw
EFI attribute definitions describe capabilities; they do not replace observation
of active page permissions. This collector and its initial-access precondition
require independent review before real boot integration.

## Assertion inventory and evidence

`kernel/src/mm/boot_pool_contract_tests.rs` contains the 17 named pure assertions.
Independent contract review preceded registration; the initial absent-module run
failed before implementation. Overflow and malformed-public-count insertion
assertions were frozen before the scoped worker implemented the selector and
`BootMemoryMap` changes. `just foundry-boot-frame-pool-run0-0` passes, binds source,
test executable and log digests, and leaves actual firmware/collector/allocator/
physical-access claims false. Kernel unit tests (262), both architecture and UEFI
builds, the existing S8 shared-memory integration and affected S11/S12/S13 gates
pass on the integrated host/QEMU state. The selector does not touch shared
allocator globals; legacy UEFI conversion and `mm::init` are not yet connected
to it. These checks do not prove a complete RUN0.0 ownership transition.

Pure cases cover valid conventional selection, stage denials, map capacity,
all-descriptor alignment/arithmetic/overlap rejection, retention completeness and
backing, reserved-only failure, pool boundaries, explicit capacity clipping,
stable tie-breaking and exact containment. Converter/collector tests separately
must deny unknown raw kinds, invalid attributes under the independently checked
pinned definition, incomplete page-table walks, unvalidated
identity assumptions and missing actual retention sources. Test fixture metadata
never substitutes for those observations.

The later QEMU gate requires actual successful firmware exit, observed IF=0,
the complete returned descriptor count and seven retention reasons, selected pool
bounds, and named successful allocate/write/read/free/reuse/exhaustion cases.
Use a bounded temporary allocator to exercise exhaustion without consuming or
reinitializing the live kernel allocator. Verify a subsequent unrelated consumer
still allocates correctly and preserve the init-image bytes. Run the existing S8
data-plane integration against the same handoff implementation. Missing, skipped,
duplicate or zero named cases fail; missing QEMU/firmware prerequisites produce
`INCOMPLETE`, never acceptance. Record source, EFI binary, init-image and firmware
digests plus actual logs in coordinator-reserved evidence paths.

The UEFI API resets on actual exit failure; a synthetic failure fixture may prove
that no allocator is installed on that path, but cannot claim a real firmware
exit failure was injected. Fault injection and current-map-key retry evidence
must state their actual scope. This packet has no model, paid execution, release,
merge or physical actuation authority.
