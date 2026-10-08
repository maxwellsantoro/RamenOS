# Getting Started with RamenOS

**Last Updated:** 2026-10-08
**Status:** Active contributor guide

RamenOS is an everyday OS being built for humans and AI agents. It is public
pre-alpha: the commands below exercise host components and selected target
paths. Read [Current Status](../CURRENT_STATUS.md) and
[Next Tasks](../NEXT_TASKS.md) for evidence and current priorities.

## Prerequisites

Use Rust via rustup, `just`, Python 3, Git, and QEMU with both
`qemu-system-x86_64` and `qemu-system-aarch64`. x86_64 UEFI gates also need
OVMF firmware. The repository pins the Rust nightly, components, and cross-targets
in [rust-toolchain.toml](../rust-toolchain.toml); Cargo uses that toolchain when
run from this checkout.

On macOS, the basic Homebrew tools are:

```bash
brew install qemu just
```

Provide OVMF code and, when available, a matching variables template. Gates
search the paths in [foundry_s0.sh](../tools/ci/foundry_s0.sh), including Homebrew
QEMU/edk2 and Linux OVMF locations. For another installation, set `OVMF_CODE`
and `OVMF_VARS` to the actual firmware files. Gates copy the variables template
into `out/`; do not use an installed firmware template as writable VM storage.

The Linux CI dependencies are recorded in
[ci.yml](../.github/workflows/ci.yml). On a Debian/Ubuntu host the boot/tooling
packages include:

```bash
sudo apt-get install qemu-system-x86 qemu-system-arm ovmf \
  python3 python3-jsonschema gcc cpio gzip e2fsprogs curl ripgrep
```

Install rustup and `just` separately if unavailable. Full preflight additionally
requires Linux evaluator facilities and the configured Docker image. Use focused
gates on other hosts; a missing prerequisite is not a successful evaluation.
Use `umask 022` for complete Linux preflight: compiler outputs must not be
writable by another identity, while SW0 exports must remain readable by the
configured Docker identity. A stricter mask is supported by the focused compiler
cache but can make SW0 fixture inputs unreadable.

Verify the toolchain from the repository root:

```bash
cargo --version
just --version
python3 --version
qemu-system-x86_64 --version
qemu-system-aarch64 --version
```

## Choose a first run

| Goal | Command | Evidence |
|------|---------|----------|
| Run the scripted human editor task | `just codegen` then `just foundry-desktop-editor-task-ui1-1c` | Injected keyboard/offscreen frames and actual host Store Save/reopen; trusted in-process, review repairs pending |
| Build typed host components | `just build-host` | Host build; runs codegen first |
| Inspect structured OS state | `just foundry-semantic-state-s10-2` | Host snapshots, subscriptions, filtered views, runner checks |
| Run a Store demo | `just foundry-store-s0` | Self-contained host service and launch plan |
| Boot both kernel architectures | `just foundry-s0` | QEMU boot, IPC, tracing, memory initialization |
| Inspect the selected target bridge | `just foundry-qemu-ipc-bridge-s10-5-2` | Framed host/QEMU IPC for the selected contract |
| Check hardware/storage foundations | `just s11`, `just s12`, `just s13` | Replay, inventory, embedded vectors and QEMU assertions |

For SW0's implemented task controls, use the command/scope table in
[Current Status](../CURRENT_STATUS.md#sw0-runnable-evidence-not-a-completed-experiment).
The physical lane and software lane proceed independently. Default hardware
gates do not establish a physical run or metal graduation.

For a team of agents, the coordinator selects ready tasks from
[Next Tasks](../NEXT_TASKS.md) and uses [Agentic Workflow](AGENTIC_WORKFLOW.md)
to assign file ownership, contract dependencies and acceptance gates. Reserve
shared output directories, QEMU images/sockets and physical equipment before
parallel runs; separate source files alone do not isolate those resources.

## Build and regenerate

Run commands from the repository root. [justfile](../justfile) owns recipes;
[run_codegen.sh](../tools/ci/run_codegen.sh) owns the complete binding-output list.

```bash
just codegen
just build-host
just build-targets
just build-uefi
```

`build-targets` cross-compiles the no_std kernel/API and aarch64 boot path.
`build-uefi` builds UEFI images. These builds alone do not assert boot behavior.
`codegen` updates Rust kernel bindings, the C capsule header, SDK WASM imports,
and native-runner host bindings. Never hand-edit generated content. See
[IDL Tools](../idl/tools/README.md) before introducing a native interface.

## QEMU boot

Use `just foundry-s0` for a repeatable boot. It builds the init image, stages
`BOOTX64.EFI` and `init.img` together under `out/uefi/x86_64/EFI/BOOT`, prepares
firmware variables, boots both architectures, and checks the serial assertions.
Inspect `out/logs/` on failure.

For interactive debugging, adapt that gate's QEMU commands and keep its init
loading behavior. The aarch64 path loads the init image at `0x44000000` using
QEMU's loader device; specifying only `-kernel` omits the required init payload.
The x86_64 path loads `init.img` beside the EFI executable, not at the FAT root.
Use one serial stdio owner, for example `-display none -monitor none -serial
stdio`; avoid conflicting monitor/serial assignments.

Stop debugging VMs before rerunning a gate that reuses its output images or
sockets. Boot logs are QEMU evidence; physical observation requires the
[HIL protocol](plans/2026-06-22-hil-appliance-controller.md).

## Validate a change

Choose the gate that asserts the changed behavior, then the required integration
checks. For docs/org planning, keep these green:

```bash
just s11
just s12
just s13
just foundry-org-governance-g0
```

For a focused edit, use the [warm developer loop](FOUNDRY_CI_OPTIMIZATION_V0.md).
For code changes, the complete acceptance entry point is `just preflight`. It checks
prerequisites before regeneration, formatting, lint/build/test tranches, and the
Foundry suites. Its agent-task suite requires Linux, JSON-schema support, and
the configured container runtime/image. `INCOMPLETE` reports missing prerequisites
or evidence; it must not be reported as PASS. Read the specific gate log rather
than repeating unrelated checks.

## Troubleshooting and next steps

- **Toolchain/target missing:** let rustup install the pinned toolchain and targets
  from `rust-toolchain.toml`; confirm the command runs in this checkout.
- **OVMF missing:** check `OVMF_CODE`/`OVMF_VARS` and the gate's searched paths.
  Use a matching firmware pair and a private writable variables copy.
- **Boot stops before init:** confirm the staged init image or aarch64 loader
  argument. Keep the serial log to identify the last completed assertion.
- **Store client cannot connect:** start the service and pass its exact socket;
  service and client defaults currently differ.
- **Linux evaluator reports INCOMPLETE:** inspect the prerequisite report; do not
  substitute macOS or an uncontained shell run for a required Linux control.

Use [Development Reference](DEVELOPMENT_REFERENCE.md) for Store examples and
operator settings, [Contributing](../CONTRIBUTING.md) for review requirements,
and [Documentation Index](INDEX.md) for architecture and contracts.
