# Host editor reliability review resolution

**Date:** 2026-10-13

**Base:** `e36b5ef12d24294021694260177965996cfce175`

**Scope:** R1–R4, default-off trusted host editor and Store owners

The supplied comprehensive review identified four bounded repair/investigation
packets and several subsequent product milestones. This batch implements the
repair packets with separate Store and Desktop writers, independent assertion and
implementation review, and coordinator-owned source registries, gate registration
and integration. Existing Repomix workspace changes are unrelated and preserved.
Original task assertions and their support files remain unchanged.

| Review item | Resolution and required witness |
|---|---|
| R1, Store reply ownership | Actual request-owned data survives the descriptor-to-lease gap. Two concurrent approved objects retain correct selected bytes and receipt ownership; held leases survive reclamation, dropped owners release bounded capacity, and mutation counts prove no replay. Disabling only the pending owner makes both concurrency regressions fail. |
| R2, acquisition and focus | Trusted approved handoff assigns focus once. Rendering cannot acquire focus. Cleanup retires the exact abandoned mapping and matching unpublished frozen frame, preserving another buffer. Pre-Present and post-Present focus switches are followed by explicit focus restoration and successful render/composition; launcher keyboard routing survives background redraw. |
| R3, Save before admission | Expired queued Ctrl+S creates no intent. Authentic never-admitted intent expiry closes its owner state. Sticky entry history preserves admitted uncertainty. The real app discards only a definitive pre-entry timeout chain, then saves the unchanged draft exactly once. |
| R4, JournalSync staging | Reproduced a pre-rename stage blocking joined-owner reopen. The actual held file descriptor and canonical journal establish cleanup ownership. Foreign, corrupt, symlink and same-byte replacement stages deny. Pre-rename noncommit and post-rename committed recovery preserve original outcomes; another consumer progresses and a distinct subsequent Save succeeds. |
| P0–P3, editor process | Remain explicit follow-on milestones. These repairs establish no child, authenticated process carrier, sealed publication or Save ownership after child death. Complete P0 assertions and required qualification before dependent runtime integration. |
| RUN0, target execution | Remains the separate relocated-entry, retained-range, firmware-exit, allocator and target-application sequence. No target-memory or loader implementation is supplied by host editor repair evidence. |
| H0–H3, physical qualification | Remain deferred to real appliance/target setup and authorized physical execution. No host or QEMU result supplies physical capture or actuation evidence. |

The new `just foundry-editor-reliability-ui1-1` gate executes seven Desktop
integration cases, five Store cases and one complete app-private unit case. Its
independently reviewed registry has 224 paths and eight assertion/support pins;
the original task, NativeRead and Preview registries preserve every old path and
add exactly the three regression sources. The unchanged transitive collector
still checks source membership. The additive host CI stage preserves the previous
55/39-stage inventory and gives 56/40 stages; no existing stage is removed.

Workers retained behavioral REDs, the R1 pin-removal negative control, focused
GREENs and strict all-targets Clippy under `out/review-desktop-*` and
`out/review-store-evidence/`. R4 diagnostic exports preserve actual original reply
bytes, selection/staging files, IO events, joined-owner fences and reopened
selection bytes. These diagnostic files do not supply authority or replace the
original Reader93 semantic acceptance.

Coordinator validation retains these fresh results:

| Check | Result and retained evidence |
|---|---|
| macOS reliability, Desktop7/Store5/app-private1 and strict Clippy3 | PASS: `out/foundry/editor-reliability/run.pyuuotj9/result.json`; outer log `out/review-regression-serial.log` |
| Linux reliability, same complete inventories and strict Clippy3 | PASS: `out/review-linux/foundry/editor-reliability/run.0uzoef61/result.json`; outer log `out/review-linux-regression-serial.log` |
| Original macOS task18, complete Reader93, strict Clippy3 and default-off rejection | PASS: `out/desktop/native-task-source.d1kN31ey/run/process/result.json`; outer log `out/review-original-task-serial.log` |
| Affected macOS consumers: volatile13, Store7, Preview codec7, Save codec8, NativeRead9, NativePreview5, adapter4 | PASS with strict/feature checks: `out/review-consumers-serial.log`; actual NativeRead/Preview evidence in `out/desktop/native-read-2f1814a0e93dc195b55b1ccc21513d31/evidence` and `out/desktop/native-preview-read-08bb1356c4c917c3ad9b5c343a51c82b/evidence` |

Both regression results bind source-manifest SHA-256
`ffc472cf7f9cacd72b3c5da809780cc55ec7d4d2d4c7d049bbefbe194f20cafd`.
The original task binds
`37a69d1a1379f80d074d15b4a6a1c7cda29c80e808c750cd29cb521d98bdf0b5`.
Independent review checked the actual complete inventories, captured log hashes,
owned exits/reaps, common source rows and R4 joined-owner diagnostic fences.
The regression registry SHA-256 is
`0585d32bcd51caa265d863aca5e1d9e2c031c9a4d136225f1ab460a1429c5fe8`;
all 224 source rows remain unchanged during acceptance. Existing task/support
assertions and the shared collector/runner helpers are unchanged.

The CI tooling gate passes all 71 Python controls and seven Rust content-stable
code-generation controls. S11, S12, S13, appliance docs/manifest mode and
org-governance pass on this assembled revision. Evidence is retained in
`out/review-tooling-planning-final.log`, `out/review-planning-retry.log` and
`out/review-s13-governance-retry.log`. The first S12 GOP and S13 block QEMU runs
missed their boot markers; each unchanged retry passed with the existing deadline.
Failures remain retained, including the initial S12 serial/parameter traces in
`out/review-s12-failed/`. No gate or deadline was changed to obtain acceptance.
`cargo fmt --all --check`, `git diff --check` and a final hash comparison of all
224 regression source rows pass with no source changes. Final scope belongs
in [Current Status](../../CURRENT_STATUS.md). Complete Linux56 preflight and
hosted merge qualification remain pending: full preflight requires a preinstalled
pinned proof image, which is unavailable locally. Focused Linux regression
uses the pinned Rust toolchain in an existing Rust image and cannot substitute
for that preflight. No crash-orphan cleanup, CAS temporary-file recovery, target
durability, physical qualification or release claim follows.

Earlier over-parallelized build attempts were deliberately cancelled before
assertion acceptance and retained under `out/review-*-*.log` and the initial
regression run directories. Owned runners exited/reaped; no cancelled result is
reused as acceptance. Final checks run serially with two Cargo build jobs and one
Rust test thread, preserving the existing real assertion deadlines.

## Qualification successor — 2026-10-13

The missing pinned image is now installed. The complete strict Linux56 preflight
passes on the exact assembled runtime candidate, including original18/Reader93,
all consumers, regression13 and actual Linux evaluator/reconciliation controls.
[The qualification](linux56-qualification-2026-10-13.md) records source and evidence.
Earlier INCOMPLETE reports and failed setup attempts remain historical evidence;
fresh five-lane hosted qualification and process/target/physical work remain
separate. This documentation-only update does not relabel old tested snapshots.
