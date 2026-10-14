# Strict Linux56 qualification — 2026-10-13

**Outcome:** PASS — all 56 canonical stages, one complete successful invocation.
**Scope:** Scripted Linux host/Docker controls and QEMU; no model/metal or release claim.

The current dirty working tree, including all reliability and build/test changes
and the pre-existing Repomix edits, was copied exactly to an isolated native Linux
checkout on `bigman`. Git base `e36b5ef12d24294021694260177965996cfce175` and 795 source-packet files matched the
local checkout before and after qualification. The complete source fingerprint is
`f2fb768a3309a922be4fad73588f2782636d6736a6b7d79156953dc21caa1f04`. Cargo.lock remained unchanged.

The pinned proof image
`python@sha256:139020233cc412efe4c8135b0efe1c7569dc8b28ddd88bddb109b764f8977e30`
was installed in local Docker and verified on the Linux host. Its x86_64 image ID
is `sha256:0b14a859cdba15104c5f194ef813fcccbf2749d74bc7be4550c06a0fc0d482e6`.
Actual Docker configuration includes builtin seccomp; JSON Schema, both QEMU
architectures, pinned Rust components and all required targets are available.
The S2 compatibility kernel package matches the configured SHA256
`cd62738b1a85029517d7aa64018454e78c0492391ea7b0334d1b254587c358cc`.

## Command and environment

`just preflight` ran with `umask 022`, the manifest-pinned nightly, two Cargo jobs,
serialized Rust tests, strict mode and lint-warning rejection. Acceptance compiler
reuse remained opt-in, with fresh assertions. Native Runner's declared CI subset
and HIL docs/manifest mode retain their existing scopes. The matching OVMF 4M
code/variables templates were selected explicitly. The private Cargo home and
checkout reside under `/tmp/ramenos-linux56-20261013.7mAs0E99`; the existing host
workspace and permission settings were not modified.

The actual 56 command records match the canonical inventory exactly, in order,
with exit 0, child return 0, actual parent reap and no uncertain cleanup. Their
summed observed stage duration was 1188.643 seconds
(~19.8 minutes), including a cold Linux compilation. This is one qualification
run, not a controlled hosted speedup benchmark.

| Largest stage | Elapsed seconds |
| --- | ---: |
| `agent-task` | 300.056 |
| `desktop-editor-task` | 131.225 |
| `workspace-tests` | 92.275 |
| `editor-reliability` | 79.673 |
| `compat-cleanup` | 71.370 |
| `editor-native-read` | 61.170 |

Fresh required behavior includes the complete Linux agent-task sequence
(contracts, RT/LT/LS controls, evaluator sessions, interrupted-commit
reconciliation and finite requestable authority), original 18/Reader93, all editor
consumers/adapters, regression 13, workspace tests, all lint tranches and the
required x86_64/aarch64/QEMU, replay, driver and storage assertions.

## Failures investigated and resolved

Initial preparation correctly denied the group-writable default Cargo home.
A protected private Cargo home was created with its own rustup proxies and
verified downloads; existing host permissions were left unchanged.

The first canonical attempt stopped at tooling because S2 preparation inherited
the SSH account's umask 002 and created group-writable output ancestors. Corrected
only the isolated checkout's owned output permissions, retained the failed log,
and enforced umask 022 for the entire run. Focused tooling 130 Python/seven Rust
controls then passed. The next full invocation passed all 56 stages. No application
code change, assertion removal, timeout expansion, cached PASS reuse or automatic
retry was needed.

## Evidence and review

Local artifacts live under `out/linux56-qualification/`:

- `qualification.json`: exact canonical stage records, source/lock identities,
  flags, per-command exits/reaps and explicit scope limits.
- `preflight-attempt-2.log`: complete successful stdout/stderr.
- `preflight-attempt-1.log`, setup logs and focused tooling log: preserved initial
  failures and their correction evidence.
- `environment.txt` and `source-packet.json`: actual platform/tool/image/kernel
  identities and exact source transport manifest.
- `evidence/`: transferred gate evidence, source manifests, fresh recordings,
  logs, timings and retained binaries. Directories named `target`, `compiler-cache`
  and `development-cache` are excluded; legacy named build directories are retained.
- `review.json`: independent qualification review.

The qualification report SHA256 is
`dddebd92464688507295714cea5e9795d1283edb9e7b6df32c4555fea913e6d7`.

The accepted runtime candidate precedes the subsequent documentation-only
status/queue/history update. Those documents do not alter the runtime source
closures or relabel the tested fingerprint. The isolated Linux checkout remains
available for inspection and warm follow-up; accepted local evidence is retained
separately.

Complete Linux56 is now qualified. Fresh execution of the five-lane GitHub
workflow remains a separate hosted requirement. Provider accounting remains
offline synthetic; no actual provider/model comparison, general containment,
editor child, target persistence, physical actuation or metal graduation follows
from this run. Merge/release authority is unchanged.
