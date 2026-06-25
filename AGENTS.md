# RamenOS — Agent Instructions

Reliability-first, post-Unix operating system. Three pillars: **OS Core** (kernel + services + runtimes), **Foundry** (tooling + CI gates), **Store** (Run Now → Vote/Port → Publish).

> **Operational truth lives in `CURRENT_STATUS.md` (landed state) + `NEXT_TASKS.md` (next work).** `ROADMAP.md` is directional, not operational. This file is the *stable agent contract*; slice-level status and history live in the status docs and `SLICES.md`, so it should change rarely.

## Mission
Implement the OS via vertical slices — do not build large subsystems in isolation. **Every change must do at least one of:**
- improve boot/run behavior, **or**
- implement a defined IDL contract, **or**
- add a Foundry gate/test, **or**
- implement a Store feature that consumes an OS capability.

If blocked, pick the simplest viable default, record it in `DECISIONS.md`, and move on — do not stop for "perfect design." Prefer small diffs with tests over big refactors; update `CURRENT_STATUS.md` + `CHANGELOG.md` per milestone.

## Non-negotiables (Constitution)
1. Rust-first kernel and core services.
2. Native interfaces are typed **Harnesses/Portals** — no ioctl escape hatches.
3. **POSIX is compatibility-only** — never design native APIs around POSIX.
4. Capability validation for fast-path ops is **kernel-side**; user-space brokers decide grants.
5. **Control plane = typed messages; data plane = zero-copy shared memory.**
6. Preserve boundaries: **kernel ≠ services ≠ store**.

`CONSTITUTION.md` holds the full invariants; do not modify it without a `DECISIONS.md` entry. No "temporary hacks" that violate it.

## Active track
- **Now:** S12.4 HIL appliance v0 physical loop (serial observer first, then power/reset actuation), feeding S13 metal HIL graduation. S14 USB xHCI + HID is deferred to a design pass.
- **Authoritative pair:** `CURRENT_STATUS.md` + `NEXT_TASKS.md` (deferred decisions in `ROADMAP.md` §13). `SLICES.md` has slice history.
- **Keep green:** `just s11`, `just s12`, `just s13`, and `just foundry-org-governance-g0` when touching org/research planning.

## Workspace crates
Key crates (full workspace in `Cargo.toml`):

| Crate | Purpose | Targets | Ext deps? |
|-------|---------|---------|-----------|
| `kernel/` | Core kernel library (`#![no_std]`) | `x86_64-unknown-none`, `aarch64-unknown-none` | **None** |
| `kernel_api/` | Shared types for kernel↔runtime (`#![no_std]`) | same bare-metal | **None** |
| `kernel_uefi/`, `kernel_aarch64/` | UEFI / aarch64 boot | uefi / aarch64-none | No |
| `idl_codegen/` | Code generator for IDL TOML specs | Host | Yes |
| `runtime_supervisor/` | Process lifecycle + compat/posix/gpu runners | Host | Yes |
| `store_cli/` | Store catalog + launch-plan tool | Host | Yes |
| `artifact_store_core/`, `artifact_store_schema/` | Content-addressed artifact storage + schemas | Host | Yes |
| `services/*` | Portals, store_service, domain_manager, native_runner, semantic_state, … | Host | Yes |
| `driver_foundry/` | Driver Factory host replay tooling | Host | Yes |

## Build
```sh
just fmt              # cargo fmt --all
just clippy           # clippy (excludes kernel_uefi, kernel_aarch64)
just codegen          # generate Rust bindings from IDL TOML specs
just build-host       # build host crates (runs codegen first)
just build-targets    # cross-compile kernel/kernel_api for bare-metal
just build-uefi       # build UEFI boot images
just preflight        # fmt + codegen + strict lint + tests + Foundry umbrella
```
Rust nightly is pinned in `rust-toolchain.toml`.

## Foundry gates
Gates are shell scripts in `tools/ci/`, run via `just`. Representative set (full list in the `justfile`):

| Gate | Command | Tests |
|------|---------|-------|
| S0 | `just foundry-s0` | QEMU boot x86_64+aarch64, IPC ping, trace ring |
| S1 | `just foundry-artifact-s1` | Artifact store lifecycle (CAS, install, rollback) |
| S2 | `just foundry-compat-s2` | Compat capsule boot, read-only artifact mount |
| S3 | `just foundry-trace-s3` / `foundry-portal-file-ro-s3` | Trace replay / portal file picker RO |
| S11 | `just s11` | Driver Factory replay + reference vault + net harness |
| S13 | `just s13` | Persistent storage / block Oracle (QEMU) |
| Org governance | `just foundry-org-governance-g0` | RamenOrg packets, drift, merge gate |
| Umbrella | `just foundry-all-s0-s1-s2-s3` | Full S0–S8 suite (used in CI) |
| CI extended | `just foundry-ci-extended` | S7 security + S9/S10/S11 subset |

S2 needs `S2_COMPAT_KERNEL`/`S2_COMPAT_INITRD`/`S2_COMPAT_ARTIFACT` (or `S2_COMPAT_KERNEL_URL` to fetch).

## Merge policy (path-scoped gate)
The branch rule requires the **`merge-gate`** check, which is path-scoped:
- **Docs/org-only PRs** (no `.rs`/`Cargo`/`idl`/`rust-toolchain`) — the heavy `foundry` job is **skipped**; the PR merges on `org-governance` + `merge-gate` in seconds.
- **OS-code PRs** — `foundry` **runs** and `merge-gate` refuses to pass unless it succeeds. OS-code changes are forced through the full Foundry suite.

### PR flow: open as the bot, approve as a different identity
Every PR is opened by the `ramen-implementer` bot (A2) and approved + merged by a **different** identity (A3) — GitHub blocks self-approval, which enforces the separation of duties.
- **Open (as the bot):** `export GH_TOKEN=$(python3 tools/org/mint_app_token.py --app-id 4129163 --key ~/.config/ramenos/ramen-implementer.private-key.pem)` → `git push -u origin <branch>` → `gh pr create …`. The PR author is `ramen-implementer[bot]`.
- **Approve + merge (as a different identity):** **`unset GH_TOKEN` first** — otherwise the approve runs as the bot and GitHub rejects the self-approval — then `gh pr review <N> --approve` + `gh pr merge <N> --squash --delete-branch` as the human (A3). (Stage 2: a second bot, `ramen-reviewer`, approves as A3.)
- Full details — identity, key, token mint, bot verification, separation of duties: see `docs/org/RAMEN_IMPLEMENTER_BOT.md`.

## IDL workflow
1. Define the interface in `idl/harness/*.toml` (harness) or `idl/portals/*.toml` (portal).
2. `just codegen` → `kernel_api/src/generated/*.generated.rs` (+ sdk/native_runner host bindings + native_runner `generated/mod.rs`).
3. **Never hand-edit `*.generated.rs`** or any `generated/` content.

## Conventions & guardrails
- **No heap allocation in `kernel/`** (no `alloc`/`Vec`/`String`/`Box`); arch-specific code in `kernel/src/arch/`.
- IPC message formats are typed + versionable via `kernel_api`.
- **Gate-first:** write the Foundry assertion *before* the implementation.
- Any new native interface goes through `/idl` and code generation.

## AI & Foundry workflow
- **Building a driver:** do not write hardware interactions from pre-training. Request the **Reference Vault** and the `protocol_trace` artifacts first; the goal is Rust code that reproduces the Oracle trace.
- **Porting applications:** use the `observed_caps_v0` artifact to generate the exact minimal capability manifest. Measure, don't guess.

## RamenOrg & research-backed work
- RamenOS is a **research-backed OS, not a research OS.** Research is first-class only when tied to a product risk, claim boundary, evidence plan, and landing path.
- Org/governance work uses bounded artifacts (`WorkOrderV0`, `HandoffPacketV0`, `BoardVoteV0`) and grants **no merge/release/hardware/public-support authority** without an explicit later decision.
- **Separation of duties:** never let one agent write, approve, merge, *and* announce a change. The `ramen-implementer` bot (A2) opens PRs; a human (A3) approves + merges — see `docs/org/RAMEN_IMPLEMENTER_BOT.md`.
- For agent-facing/cross-domain service boundaries, separate request authority (`Lang`: what a holder may ask) from observable authority (`ObsContract`: what a holder may learn).
- Do not claim hidden-affordance noninterference, metal graduation, security, or release readiness without the matching evidence level + claim boundary (`EVIDENCE_LEVELS.md`).

## Security posture
- **Boundaries prevent escalation:** services depend on schema types only, never IO functions.
- Feature-flag host scaffolding (e.g. `posix_runner_v0_dev`) with prominent warnings; never on by default.
- **Per-domain isolation** (trace, capabilities, accounting); global singletons are a liability.
- **Fail-closed defaults** (codegen, wire parsing, capability validation).
- **Defense in depth** (capabilities + schema + seccomp + namespaces).

## Key documents
- `CONSTITUTION.md` — invariants (modify only with a `DECISIONS.md` entry)
- `CURRENT_STATUS.md` + `NEXT_TASKS.md` — landed state + next work
- `SLICES.md` — slice definitions/status · `ROADMAP.md` — sequencing
- `docs/INDEX.md` — documentation map · `PLATFORM_OVERVIEW.md` — architecture · `STORE_SPEC.md` — store model
- `docs/org/` — RamenOrg governance (authority levels, merge gate, implementer bot, claim safety)
- `docs/research/` — research program (RQ-0001 offer boundaries, RQ-0002 org kernel)
- `EVIDENCE_LEVELS.md` — `PASS/QEMU` … `PASS/METAL` claim levels
