# External boot-profile Oracle v0

**Status:** Independently reviewed proposed preparation contract. No collector,
inspection schema implementation, gate, Reference Vault or capture is claimed
to exist. Coordinator registration and failing assertions precede implementation.

**Preparation base:** `a44993e5880fee83d9ca4400f8a291f309ef813c` plus the
coordinator's assembled source state. The reviewed
[boot frame ownership contract](../BOOT_FRAME_OWNERSHIP_V0.md) has a pure
admission prerequisite integrated at `a2d239736dafd6a92d535821025b49e835bc7dcd`.
Its numeric tests do not depend
on this capture. Firmware exit, retained backing, CPU access and live allocator
installation remain separate work.

## Outcome and observation boundary

Observe one unmodified, pinned RamenOS EFI guest under one pinned QEMU/OVMF
profile using external debug reads. Capture the actual existing `GetMemoryMap`
request and return, raw descriptors, CPU state and bounded paging-structure RAM
frames. An offline validator reconstructs supported translations and permissions
for named byte extents. These observations inform the initial table-access
precondition before adding architecture-specific boot glue.

The current guest does not call `ExitBootServices`. Every accepted baseline
checkpoint is labelled `firmware_active`. A successful debugger read is an
emulator inspection, not an executed guest dereference. The report always has
`guest_access_executed=false`, `post_exit_verified=false`,
`interrupt_masking_verified=false`, `retention_complete=false`,
`allocator_installed=false`, `target_user_execution=false`, and
`physical_hardware=false`. These seven names are the complete required claim
keys, each an exact JSON boolean false. Observed IF is
recorded without turning debugger scheduling controls into a guest CLI witness.
This finite profile does not establish continuity, noninterference, containment,
general firmware behavior or metal qualification.

The [Reference Vault workflow](../../drivers/reference_vaults/README.md) and
[existing Oracle promotion workflow](../../drivers/reference_vaults/virtio-net/traces/README.md)
own provenance principles: retain original bytes, origin and hashes; validate
before promotion; distinguish fixtures from actual captures. This Oracle is
`qemu_ovmf_external_inspection`, not a Linux driver capture. The
[Vault template](../../drivers/reference_vaults/template/README.md) will need an
explicit CPU-inspection context convention: no canonical native CPU harness
exists to copy into `harness.toml`. Do not invent one or silently reuse a device
IDL. A future vault needs primary-source notes, capture contract, trace artifacts,
README and limits, assigned by the coordinator after review.

## Primary sources and source pins

| Source | Use and limitation |
|---|---|
| [UEFI 2.10 §2.3.4](https://uefi.org/specs/UEFI/2.10/02_Overview.html#x64-platforms) | x64 ABI and identity-mapped boot environment, with protection-related restrictions or absent mappings. This is a precondition to verify in the selected profile, not unconditional access permission. |
| [UEFI 2.10 §7.2.3](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#getmemorymap) | Public GetMemoryMap prototype, descriptor stride/version, map key and returned buffer size. Map ownership and freshness change with firmware allocations. |
| [Intel SDM Vol. 3A, 253668-084US, June 2024](https://cdrdv2-public.intel.com/825758/253668-sdm-vol-3a.pdf) §§2.4–2.5, 4.1, 4.5, 4.6 | Register meaning, paging mode, physical width, huge leaves and access checks. Freeze supported modifiers before implementing the decoder. |
| [QEMU v8.2.2 GDB documentation](https://raw.githubusercontent.com/qemu/qemu/v8.2.2/docs/system/gdb.rst) | TCG hardware-breakpoint support, relocation, private Unix sockets, and debugger stepping behavior. Debugging changes scheduling. |
| [QEMU v8.2.2 monitor commands](https://raw.githubusercontent.com/qemu/qemu/v8.2.2/hmp-commands.hx) and [information commands](https://raw.githubusercontent.com/qemu/qemu/v8.2.2/hmp-commands-info.hx) | Register, memory-tree and bounded physical-memory inspection commands. Check the installed patched binary's actual capabilities. |
| [QEMU v8.2.2 x86 monitor implementation](https://raw.githubusercontent.com/qemu/qemu/v8.2.2/target/i386/monitor.c) | Its bulk mapping walkers perform physical reads internally. Do not invoke these uncontrolled walkers as the collector's bounded, RAM-only reader. |

The local primary crate sources inspected during preparation are pinned by
`Cargo.lock`. Their package checksums and inspected-file SHA-256 values are:

```text
uefi 0.27.0 package:
89ee9c34c612d45735fef4c478450cd2a1e64e59ad1e5988e5024af53914a854
src/table/system.rs:
0fba7d83f6f4ecc1794e285c5dd0482a5ff45398b16e2bb603dc34a5bc1d2620
src/table/boot.rs:
15378543e55f035447abb24fd51d3b98f7ca27383d84832aadac95f0409cb057
src/proto/loaded_image.rs:
37305fa52b939e28bd83f3232e6f93bec68861c920c4b11af33d4f773031fc5e
uefi-raw 0.5.2 package:
efa8716f52e8cab8bcedfd5052388a0f263b69fe5cc2561548dc6a530678333c
src/table/boot.rs:
c75b7d6e9b22a750fa1d5b646e149fa367a20c317b45df2d2d690bb4c4490559
```

`uefi` 0.27.0 captures descriptor version internally but does not expose it on
`MemoryMap`; the external ABI observation therefore retains the actual output.
Public entry references expose used records, not the allocation's complete size
or slack. `LoadedImage::info()` exposes the complete EFI image base/size but does
not prove physical backing or full kernel retention. The future exit API consumes
the boot table, allocates its map with the requested type, tries twice and resets
on failure; this baseline does not exercise that transition.

## Frozen profile inputs and resources

The observed available environment is bigman with QEMU 8.2.2, package version
`1:8.2.2+ds-0ubuntu1.18`, and GDB installed. Those observations are readiness
inputs, not executable hashes or capture evidence. A standalone `llvm-nm` name
is absent; resolve already installed Rust-bundled LLVM tools through the pinned
toolchain and record their paths, versions and hashes. Missing usable symbol
tools produce `INCOMPLETE`; no package installation or guessed symbol address.

The proposed single profile is x86_64, TCG, `pc-q35-8.2`, `qemu64`, one vCPU,
512 MiB RAM, no network, no device passthrough and no guest input injection.
Record exact expanded CPU properties; do not use `-cpu host` or an implicit
machine-version alias. Required command capabilities and CPU fields must be
checked against the actual binary before capture. Unsupported protection state
is a denial, not a reason to disable a guest feature through the debugger.

Inputs are a source commit plus assembled-patch digest, QEMU/GDB/tool binaries,
OVMF CODE and VARS template, EFI binary, matching symbols, init image, and a
prepared read-only ESP image. Bound EFI to 16 MiB, init to 4 MiB, ESP to 64 MiB
and each firmware image to 16 MiB. Record size and SHA-256 for each. The collector
does not rebuild or patch the guest. The coordinator prepares these artifacts
and reviews their provenance before authorizing capture.

Use a fresh coordinator-reserved directory, a private disposable VARS clone,
owner-only Unix debug/monitor sockets and the read-only ESP. Normal firmware
execution can change its disposable VARS clone; prohibit capture-induced guest
writes. Stop/resume and hardware breakpoint installation are explicit emulator
observation controls, not physical actuation or an unperturbed execution claim.
Record them. Never promote or overwrite an existing vault automatically.

Suggested future argv shape, with all paths resolved from the reviewed profile:

```text
qemu-system-x86_64 -machine pc-q35-8.2,accel=tcg -cpu qemu64
  -smp 1 -m 512M -nic none -S -display none -no-reboot
  -drive if=pflash,format=raw,readonly=on,file=<pinned-code>
  -drive if=pflash,format=raw,file=<private-vars-clone>
  -drive format=raw,readonly=on,file=<pinned-esp>
  -serial file:<run>/serial.log
  -qmp unix:<run>/qmp.sock,server=on,wait=off
  -chardev socket,path=<run>/gdb.sock,server=on,wait=off,id=gdb0
  -gdb chardev:gdb0
```

The profile validator rejects path escapes, symlinks to outside inputs, shared
VARS/output paths and artifact/hash mismatches before launch. Record both VARS
template and disposable clone initial/final hashes. The supervisor owns this VM's
PID and sockets, applies deadlines and cleans up only its own resources.

## Checkpoints and command boundary

Exactly three checkpoint identities form this baseline:

| ID | Binding and required observation |
|---|---|
| C0 `efi_entry` | Actual PE entry point, image relocation and original loaded instruction bytes. Capture RCX image handle, RDX system-table pointer, registers and emulator RAM layout. Resolve the actual BootServices GetMemoryMap function pointer through checked public UEFI structure layouts. |
| C1 `get_memory_map_entry` | Hardware breakpoint on that actual function pointer after C0. Bind the caller return address to the pinned EFI caller's instruction range. Record entry RSP and five ABI argument pointers, input buffer capacity and register state. |
| C2 `get_memory_map_return` | Hardware breakpoint on the return address saved from C1. Match the same call, actual RIP and output pointers; require EFI_SUCCESS for acceptance. Read returned size, key, stride/version and raw bytes while the returned map buffer is live. Capture registers and complete bounded table frames at this stop. |

Entry-address resolution is a remaining input, not solved by a PE entry RVA.
Before collector implementation, the coordinator must freeze a bounded method
that supplies an actual relocated-address candidate soon enough to install C0.
[EDK II's image loader](https://github.com/tianocore/edk2/blob/edk2-stable202402/MdeModulePkg/Core/Dxe/Image/Image.c)
contains a debug load-address/entry-point diagnostic; the installed firmware's
build, enabled diagnostics and delivery timing have not been verified. A
firmware log or prior run may provide a candidate, but cannot certify this run.
Only this run's actual C0 stop, PE relocation relationship and checked instruction
bytes accept it. Missing diagnostics, ambiguous identity or a missed entry
produce `INCOMPLETE`. No full-memory scan, inserted guest spin loop or register
rewrite may replace this prerequisite. If extra discovery stops are necessary,
the coordinator must revise the three-checkpoint contract before implementation.

For this public x64 ABI the first four pointers arrive in RCX/RDX/R8/R9; the
fifth is at entry RSP+40, and the saved return address is at entry RSP. Capture
and validate these using the official ABI, checked arithmetic and offline
translation, not Rust-private field offsets. Reject noncanonical pointers,
unsupported layouts or caller binding failures. Save entry pointers externally;
volatile registers at return need not retain them. The selected existing call
has no observer-created retry: preserve a failed return and mark acceptance
incomplete. Any later capture is a fresh run ID.

On the selected x64 profile, RAX carries the 64-bit EFI status. Returned
MemoryMapSize, MapKey and DescriptorSize are 64-bit native-width scalars;
DescriptorVersion is exactly 32 bits. Preserve those widths in raw ABI reads,
call bindings and decoder schemas; do not read an extra word as part of the
version or substitute a configured version for the observed output.

Only audited host capability/status queries, register reads, hardware breakpoint
control, resume/stop, bounded RAM reads and owned-VM teardown are allowed.
Require GDB remote hardware breakpoint operations (`Z1`/`z1`) and refuse software
breakpoint fallback. No `load`, injected `call`, register assignment, memory-write
packet, I/O-port read, MMIO read, new assembly, reset injection or guest key input.
Record actual remote/monitor traffic; high-level GDB command text alone cannot
show that a software breakpoint fallback did not occur. Unexpected commands
deny dispatch before reaching the transport.

Read registers and the emulator's flattened RAM/device layout before reading
physical memory. Permit a physical read only if its entire byte range belongs
to active ordinary RAM, with ambiguous overlays/ROM/device regions rejected.
Use bounded `xp` reads or equivalently audited physical-debug reads; verify exact
returned bytes. Disable general virtual-memory reads until the decoder proves
their complete translation and RAM backing. Do not call `info mem`, `info tlb`
or unchecked `gva2gpa` as a substitute for the bounded traversal: internal walks
can read physical addresses before this collector's backing checks.

The external physical reader avoids using a guest pointer to validate that same
guest pointer. After capture, cross-check observed table/object frames against
raw EFI RAM classifications. For this conservative v0 check, admissible backing
types are LoaderCode/Data, BootServicesCode/Data, RuntimeServicesCode/Data,
ConventionalMemory, ACPIReclaimMemory and ACPIMemoryNVS, combined with independent
ordinary-RAM layout evidence. Reserved, Unusable, MMIO, MMIOPortSpace, PAL,
persistent, unaccepted or unknown classifications cannot certify table/object
backing. Validate each nonzero descriptor extent with checked page arithmetic
and pairwise nonoverlap; require full extent coverage, without silently bridging
a missing descriptor. Preserve every raw type and attribute bit: this capture
does not decide the later converter's complete attribute policy. Typed `Reserved`
alone is insufficient because it includes MMIO. The initial in-guest reader
precondition remains open until this
profile is independently reviewed and later matched by actual boot glue.

## Distinct CPU inspection trace

Propose a local evidence schema named `cpu_boot_inspection_trace_v0`, version 1.
It is not `driver_protocol_trace_v0`: that existing schema requires PCI identities
and its event kinds describe PCI/MMIO/IRQ/DMA. Do not forge identifiers or call
CPU register observations MMIO events. It is also not a native typed harness
transcript. A scenario trace may reference its content ID after validation.
Registration or changes to shared artifact schemas belong to the coordinator.

The proposed strict top-level fields are:

| Field | Shape |
|---|---|
| `schema_version` / `trace_kind` | Integer 1 / exact string `cpu_boot_inspection_trace_v0`. |
| `run_id` | Fresh lowercase identifier, 1–64 ASCII characters from `[a-z0-9-]`. |
| `origin` | Exact string `qemu_ovmf_external_inspection`. |
| `profile_ref` | Blob reference binding the reviewed profile and every input. |
| `checkpoints` | An ordered prefix of C0/C1/C2, length 0–3. Each observed checkpoint has phase `firmware_active`, RIP, stop/register evidence references and a snapshot identity. Completion requires all three and the same C1/C2 call binding. Never invent a missing stop. |
| `events` | Ordered inspection request/response records, contiguous `seq=1..N`, at most 1024. |
| `termination` | Exact enum `complete`, `error`, `timeout`, or `limit_exceeded`, plus bounded reason code. |
| `claims` | Fixed false guest-access, post-exit, interrupt-masking, retention, allocator and target-execution flags described above. |

Every event has exactly `seq`, `checkpoint`, `operation`, `request_ref`,
`response_ref`, `started_ns`, `finished_ns`, `result`. Operation is one of
`capability_query`, `register_read`, `ram_layout_read`, `ram_read`,
`hardware_breakpoint_set`, `hardware_breakpoint_clear`, `resume`, `stop`, `quit`.
`checkpoint` is null for pre-stop controls or one of C0/C1/C2 for operations
bound to that actual stopped snapshot. Each checkpoint has exactly `id`,
`phase`, `rip`, `snapshot_id`, `stop_ref`, `register_ref`, `call_ref`: ID is
C0/C1/C2; RIP is an unsigned 64-bit integer; snapshot ID is its checkpoint ID;
the evidence references use the blob shape below. C0 has null `call_ref`; C1/C2
reference the same external call-binding record. That record has exactly
`entry_rsp`, `return_rip`, `argument_pointers` (five unsigned 64-bit integers in
ABI order), and `input_capacity` (unsigned 64-bit bytes), bound to C1 evidence.
Reject booleans, out-of-range integers or arithmetic overflow.

Result is `ok`, `error`, `timeout`, or `pending`. Pending has null `response_ref`
and `finished_ns`; other results require a bounded completion timestamp.
An absent response on error/timeout stays null and cannot certify an observation.
An expected teardown EOF is recorded as a supervisor result referencing the
actual raw EOF record, not a fabricated debugger acknowledgement. Preserve
failures and unknowns; no silent drop, duplicate observation or automatic replay.
Timestamps are host-monotonic integer nanoseconds; completed operations require
`0 <= started_ns <= finished_ns <= 2^63-1`, and pending operations require only
`0 <= started_ns <= 2^63-1`. UTC start/end are
separate profile metadata. Boolean-as-integer, unknown fields, invalid enum
values and unknown versions fail closed.

Keep asynchronous stop notifications and transport framing/acknowledgements in
the raw traffic log. Bind a stop notification to the controlling operation and
checkpoint; do not invent a request/response pair for unsolicited traffic. Every
CPU/memory snapshot must be bracketed by one stopped-state witness and its next
resume, or by teardown. Reads from different stops cannot share a snapshot.

A blob reference has exactly `path`, `sha256`, `length`: safe relative path,
64 lowercase hex characters and checked byte length. Event blobs are 1–65536
bytes. Successful RAM-read responses carry the exact requested byte count; do
not zero-fill missing bytes. Keep original command/response bytes and decode
them independently. The raw transcript cannot contain the hash of itself;
`hashes.json` outside it hashes the finalized trace and report.

The profile record binds source identity, binary/symbol/input references, exact
argv and expanded CPU configuration, tool versions/package identity, required
capabilities, resource paths, checkpoint-resolution evidence, observer version,
UTC timestamps and debugger stepping policy. All hashes are actual file-byte
hashes; an available package name is not a binary pin. No observed CPUID is
invented from configuration: distinguish configured/model-derived properties
from actual register observations.

## Bounds and offline checks

| Resource | Hard v0 bound and overflow behavior |
|---|---|
| Checkpoints / guest calls | Three checkpoint identities and one selected GetMemoryMap call; no fallback call injected. |
| Map | At most 256 descriptors, version 1, stride 40–256 bytes and a multiple of 8, returned size an exact multiple of stride and at most 65536 bytes, no larger than entry buffer capacity. Preserve extension bytes without interpreting them. |
| Tables | At most 256 distinct `(checkpoint, physical-address)` 4096-byte frame snapshots across the entire run, each with 512 entries; iterative queue/visited traversal with no recursive expansion. Reusing an address at a later stop requires new bytes and consumes capacity. |
| Other RAM reads | At most 64 KiB total for ABI scalars, public UEFI structure headers and checkpoint code identity; each request at most 4096 bytes. Map reads are separately bounded above. |
| Events / logs | At most 1024 paired operations and 32 MiB total retained evidence excluding pinned input artifacts. Detect exhaustion and preserve an explicit incomplete terminal report; never silently truncate and pass. |
| Time | 60 seconds to each required stop, 180 seconds total VM lifetime, 5 seconds per inspection response. Kill/reap only the owned VM after failure. |

At C0/C1, bounded partial walks resolve only the public structures and ABI words
needed for the next checkpoint. At stopped C2, reconstruct the complete current
four-level paging graph. Bind the physical-address width to the pinned QEMU CPU
model's reported configuration; do not call it an executed CPUID observation.
Missing or unsupported model-width evidence is incomplete, with no guessed
52-bit default. Apply reserved-bit checks, huge-leaf alignment, canonical
addresses and correct depth. Same-frame reuse at the same level is allowed if
consistent; cycles or reuse at incompatible levels reject. Every read is first
bounded by emulator RAM layout; every discovered table is then classified by the
captured raw firmware map. No full-RAM dump or unbounded enumeration.

The v0 access profile requires CPL0, long mode and four-level paging. Require
unsupported modifiers such as LA57, SMAP, PKE, PKS and CET disabled rather than
guessing missing state. Record CR0.WP, EFER.NXE and all ancestor present/RW/US/NX
bits; distinguish a reconstructed data-access result from executable permissions.
Conservatively require RW throughout a proposed writable extent even when WP is
clear. Decode supported 4 KiB/2 MiB/1 GiB translations and require complete
identity coverage for the named table-reader and future-pool extents. Report
holes, nonidentity or restricted mappings as failed preconditions, retaining the
raw evidence. NX does not prohibit data reads. No debugger command is a guest
load/store test, and no snapshot proves cached translations or future stability.
Capture CR0/CR3/CR4, EFER, RFLAGS, CS privilege and GDTR/IDTR base/limit fields;
use explicit numeric widths and distinguish unavailable fields from zero. GDTR
and IDTR are observations here, not full descriptor-table retention witnesses.

Named inspected extents include the current table frames, C1/C2 stack words,
GetMemoryMap parameter/output objects and raw returned descriptor bytes. They
do not establish the complete kernel image, full allocated init extent, future
stack growth or all retained objects. An offline hypothetical pool projection
must say `firmware_active_projection`, not claim firmware ownership; it cannot
be installed or used by the guest during this packet.

## Gate-first case inventory

The successor author implements these 16 named cases before collector handlers.
Pure fixtures are explicitly synthetic. Transport-spy assertions verify denied
commands/reads are rejected before dispatch. The coordinator assigns exact
paths, schema ownership and output resources after independent review.

| Case | Assertion |
|---|---|
| BP-O01 | Valid bounded profile and matching input byte hashes are admitted; actual capture remains required. |
| BP-O02 | C0/C1/C2 and matching public ABI call/return produce a bound raw-map observation; all broader claims stay false. |
| BP-O03 | Complete bounded table fixture accepts 4 KiB, 2 MiB and supported 1 GiB leaves without treating leaves as child tables. |
| BP-O04 | Offline identity/RW checks cover the whole requested extent, including ancestor restrictions and descriptor boundaries. |
| BP-O05 | MMIO/ROM/ambiguous/partially RAM-backed reads and write/injected-call/software-breakpoint commands deny before transport dispatch. |
| BP-O06 | Machine/CPU/accelerator/source/binary/symbol/firmware/init/profile mismatch cannot reuse acceptance from another capture. |
| BP-O07 | Unsupported paging mode, protection modifiers, physical width or invalid register state cannot certify initial access. |
| BP-O08 | Zero/overflowing/nondivisible map size, wrong version/stride, excessive count or exceeded input capacity fail before map reads. |
| BP-O09 | Missing, nonidentity or read-only mapping preserves its observation and makes the relevant access precondition fail. |
| BP-O10 | Missing/wrong caller or return checkpoint, changed argument binding or unsuccessful EFI return cannot become a successful map. |
| BP-O11 | Missing/duplicate/reordered sequence, unmatched response, invalid timestamp or resume during a supposedly stopped snapshot rejects. |
| BP-O12 | Consistent same-level table reuse is allowed; cycles, incompatible-depth reuse, invalid huge flags or malformed table addresses reject. |
| BP-O13 | Frame/event/storage/time limits stop before an extra read and preserve an incomplete result; zero/skipped named cases cannot pass. |
| BP-O14 | Hash/length mismatch, missing blob or path escape fails; raw response bytes remain distinct from decoded summaries. |
| BP-O15 | Boolean integers, unknown fields/opcodes/versions and fabricated broader claim flags fail closed. |
| BP-O16 | Failed/unknown captures remain retained; cleanup reaps the owned VM, and a fresh run uses fresh resources without silent replay. |

## Runnable successor and completion

After review, the coordinator freezes the distinct schema/profile and registers
a proposed `foundry-boot-profile-oracle-p0` gate. No such recipe is claimed here.
Assign new host-only collector, decoder and pure tests under `tools/trace/` with
exact filenames. First record RED for absent implementation; then implement
validation and transport-spy cases before launching QEMU. Capture producer and
decoder need independent review. No kernel or UEFI modifications belong to this
successor, and no shared driver schema needs speculative expansion.

Proposed command interface, to be registered rather than assumed available:

```text
python3 tools/trace/boot_profile_oracle.py collect
  --profile <reviewed-profile.json> --evidence <fresh-directory>
python3 tools/trace/boot_profile_oracle_validate.py
  --profile <reviewed-profile.json> --evidence <fresh-directory>
  --report <fresh-directory>/validation_report.json
```

The decoder report separates `capture_complete`, `profile_preconditions_met`
and per-extent access checks from fixed false broader claims. Every failed or
pending required observation sets complete/preconditions false. Missing QEMU,
GDB, symbols, firmware, artifact pins or capabilities yield `INCOMPLETE` and no
capture acceptance. Parser-fixture PASS is labelled portable synthetic evidence.

Use the evidence layout below, with the actual trace/report hashes outside their
contents. Raw evidence is immutable after capture and reviewed before any future
Vault promotion. Existing shared `out/oracle_capture` scripts must not run
concurrently against these resources.

```text
out/oracle_capture/uefi-boot-profile/<unique-run-id>/
  profile.json
  commands.jsonl
  debugger.log
  serial.log
  checkpoints/C0/registers.txt
  checkpoints/C0/memory-tree.txt
  checkpoints/C0/tables/<physical-address>.bin
  checkpoints/C1/registers.txt
  checkpoints/C1/memory-tree.txt
  checkpoints/C1/tables/<physical-address>.bin
  checkpoints/C2/registers.txt
  checkpoints/C2/memory-tree.txt
  checkpoints/C2/map.bin
  checkpoints/C2/map.json
  checkpoints/C2/tables/<physical-address>.bin
  inspection_trace.json
  validation_report.json
  hashes.json
```

Required remaining inputs are actual tool/firmware/artifact hashes, the reviewed
C0 address-resolution method, verified command/property support, and the frozen
profile/schema with red assertions. The declared environment alone supplies none
of these acceptance witnesses. This file authorizes preparation, not a capture.

Completion is one independently reviewed actual capture of the frozen profile,
all 16 executable validation cases, verified source/input/output hashes and a
report retaining the initial-access evidence boundary. It prepares a selected
profile; it does not authorize an unchecked in-guest reader. Matching actual
post-exit/interrupt-masked collection, complete seven-reason retention, live
pool access and S8 consumer behavior remain later reviewed packets.
