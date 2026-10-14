# October 14 follow-up review resolution

This local successor is based on merged `f3fa057`. It addresses the supplied
project reviews with executable bootstrap/authority repairs, maintained-document
corrections, and bounded editor-process assertion preparation. This record
describes local acceptance before hosted qualification; it adds no physical
graduation. `CURRENT_STATUS.md` and
`NEXT_TASKS.md` remain authoritative for acceptance and dispatch.

## Findings and dispositions

| Review observation | Resolution and remaining scope |
| --- | --- |
| The largest usable region could expose firmware, loader or reserved frames. | Replace that selection with a held, successful 16 MiB UEFI `AllocatePages(LOADER_DATA)` allocation, a complete supported descriptor projection, checked ownership admission, and one-shot allocator installation. Boot-services/runtime regions remain unavailable. AArch64 explicitly uses a bounded QEMU fixture. |
| Map conversion filtered descriptors and treated active boot-services memory as usable. | Preserve every supported descriptor, reject unknown/overflowing/oversized maps, retain loader kinds, reserve boot-services/runtime memory, and fail closed before allocator publication. Failed capture frees the exact newly allocated pool; uncertain release retains ownership and halts. |
| Default boot images enabled capability-bypassing test protocols. | Both boot dependencies leave `test_protocols` off by default. Explicit Oracle gates can enable providers, but ordinary IPC admission validates live capability kind/generation in either configuration. Boot fixtures obtain real IPC capabilities. |
| Trace CREATE preceded IPC authorization. | Both dispatch entrypoints require a live IPC capability bound to the requested creator domain before allocating/resetting trace state. Read/info/destroy retain the existing Trace bearer contract. Generic IPC handles still lack protocol-specific rights. |
| Root risks, roadmap, slices and PR #38 status were stale. | Update the October 14 maintained documents, five-lane inventory, accepted host repairs, and the exact merged/post-merge evidence. The strict Linux56 wrapper was not repeated for PR #38. |
| The next process checkpoint needs assertions before implementation. | Add two runnable genuine setup controls and three authored consumers of the absent frozen process API. The separate preparation recipe requires actual setup success and narrowly classified missing-API compiler RED. Private native fault controls and shared app Save ownership remain prerequisites; P0's thirty families are not complete. |
| Broad security and readiness conclusions exceeded the evidence. | Clarify the rlimits-only default POSIX runner. Typed contracts and `Lang`/`ObsContract` separation express obligations; they do not prove covert-channel noninterference, full revocation atomicity, target isolation, everyday readiness or release security. |
| Large editor implementation and fixed kernel capacities. | Keep the process ownership split and bounded resource denials visible in the queue and risks. File size alone does not justify a broad refactor alongside these boundary repairs. |

## Local verification

The coordinator serializes gates in a disposable checkout of the same source,
with the pinned Rust toolchain, bounded Cargo jobs and physical actuation disabled.
The boundary gate includes default and explicitly enabled Oracle configurations,
complete-map/allocator denials, successful next operations, trace consumer checks,
and actual boot dependency feature-graph checks. Independent investigation and
fresh candidate review covered the runtime repairs; the review's map-capture
cleanup finding was fixed with release/retry and retained-owner assertions.

Fresh checks cover both target builds and QEMU boots; S8 shared-memory integration;
S11 network, S12 GOP and S13 block Oracle paths; host/target and QEMU IPC bridge;
trace client and the separate seventeen-case RUN0 numeric gate; editor reliability,
original task/recording-reader, NativeRead and NativePreview; CI tooling, IDL and
organization governance. Final results belong to the tested local source manifest,
not to the earlier hosted run.

Preserve failed attempts: an initial source-registry pin mismatch and setup
checkpoint declaration failure were corrected before rerunning; a long temporary
path exceeded macOS Unix-socket limits, so the same checkout was accessed through
a short alias. The bridge's initial transaction timeout and unchanged existing
retry remain evidence. No assertion, runtime deadline or canonical stage was
removed to obtain acceptance.

## Open boundaries

This bootstrap allocation is separate from RUN0's firmware exit, complete retained
ranges, interrupt/access prerequisites and target userspace. Oracle vectors and
QEMU do not establish live hardware behavior. The editor remains an in-process
host task; authored missing-API assertions have not executed in a child. Full
preflight requires Linux and its pinned Docker/seccomp prerequisites and was not
run on this macOS successor. The canonical 56-stage inventory and five hosted
lanes remain unchanged; the process preparation recipe is explicitly optional
preparation, not a substitute process acceptance gate.
