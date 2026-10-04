# RamenOS

![RamenOS circuit-board ramen bowl header](docs/assets/ramenos-header.png)

[![ci](https://github.com/maxwellsantoro/RamenOS/actions/workflows/ci.yml/badge.svg)](https://github.com/maxwellsantoro/RamenOS/actions/workflows/ci.yml)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)

**Last Updated:** 2026-10-04
**Status:** Public pre-alpha, active development

RamenOS is a modern, post-Unix operating system being built for **humans and AI
agents** to use efficiently and safely. Its goal is an everyday OS that combines
fast execution, hardware adaptability, approachable human interactions, and
structured interfaces for agents.

The design separates kernel mechanisms, services, and software distribution.
Drivers and software are intended to evolve independently behind **typed
contracts**, with isolated execution, explicit permissions, and Foundry evidence
to check failures and effects across boundaries. Useful compatibility stays,
while native interfaces can evolve beyond inherited Unix constraints.

Humans remain in control of intent and policy. Agents should discover permitted
state and act through **scoped, revocable capabilities**, without requiring
screen scraping or fragile command-output parsing for core control. OS Core,
Foundry, and the Store are the three pillars supporting this product.

Read the [Vision](VISION.md) for the destination. The project is public pre-alpha:
substantial kernel and host components exist, while the integrated desktop,
target runtime, and broader hardware qualification remain work. The Agent Task
Proof below tests one part of the vision; comparative agent benefits remain
unproved.

Founded by [Maxwell Santoro](https://maxwellsantoro.com).

## Run the useful host task proof

A **scripted, host-only proof works today**: read a scoped configuration, stage an
immutable repair, run the pinned WASM validator, commit the artifact, deny an
unrelated resource read, and recover the original receipt after restart.
Install the pinned Rust toolchain and `just` using [Getting Started](docs/GETTING_STARTED.md), then run:

```bash
git clone https://github.com/maxwellsantoro/RamenOS.git
cd RamenOS
just foundry-agent-task-proof-rt
```

This gate needs neither QEMU nor a model provider. It writes inspectable evidence
to `out/agent-task-proof-rt/`: `report.json` contains the input/candidate/output
identities, validation, denied operation and receipt; `journal.json` retains the
transactions and audit; `manifest.json` records the source and host fingerprints.
The tests verify receipt replay before saving those files. Set
`RAMEN_TASK_EVIDENCE_DIR` to retain a run in another directory.

A [retained host result](docs/examples/agent-task-proof-rt/report.json) and its
[journal](docs/examples/agent-task-proof-rt/journal.json) show this actual repair:

| Observation | Retained result |
|-------------|-----------------|
| Input | `sha256:8c28330c…`, revision 0, `enabled: false` |
| Staged candidate | `{"enabled":true,"label":"keep"}`, `sha256:9ec6d48c…` |
| Pinned validation | `valid`, untruncated diagnostics |
| Committed output | `sha256:9ec6d48c…`, revision 1 |
| Denied operation | `read_input` for resource 999: denied, no returned mapping |
| Receipt and replay | Request 4 binds revisions 0 → 1; restart retry returns the original receipt; independent replay checked |

The linked files retain full hashes. Their [provenance](docs/examples/agent-task-proof-rt/provenance.json)
identifies a local worktree run. These are trusted development fixtures and a
scripted consumer, with **host-service enforcement**. They establish neither
model performance nor task-specific RamenOS kernel enforcement. See the
[service proof contract](docs/AGENT_TASK_SERVICE_PROOF_V1.md) for its scope.

## The planned controlled comparison

The [Agent Task Proof study](docs/plans/2026-09-16-agent-task-proof.md) uses three
arms with equivalent task resources: **Linux scoped shell, Linux typed, and
RamenOS typed**. The typed arms share their agent-visible protocol:

- Linux typed vs Linux shell measures the value of structured interaction.
- RamenOS typed vs Linux typed tests what the implemented host service designs add.
- RamenOS typed vs Linux shell measures the complete task-level proposition.

Implemented controls support this future experiment; the complete controlled
model comparison and target-native task environment remain unfinished. The study
will check completion, effective authority, forbidden probes, context/tool cost,
recovery, and audit/replay, reporting success, authority and cost separately.
No comparative advantage is claimed yet. Host results cannot establish an
advantage arising from the RamenOS kernel.

## What is real today

| Component | Landed behavior | Execution boundary |
|-----------|-----------------|--------------------|
| Kernel | x86_64 and aarch64 boot; typed IPC; capabilities; shared memory; tracing | QEMU target paths; single-threaded capability-table prototype; SMP use is deliberately blocked |
| Typed contracts | IDL/codegen and wire checks for Harnesses and Portals | Native interfaces are IDL-defined; project policy forbids ioctl-style escape hatches |
| Native WASM runner | Wasmtime execution, granted-handle injection, missing-capability rejection | Host runtime, not Wasmtime running on the target |
| Semantic State | Snapshot contracts, subscriptions, capability-filtered host views | Host reactor plus selected QEMU snapshot/IPC bridges; default snapshot metadata still contains placeholders |
| Store and projections | Artifact ingestion, ownership checks, queries, copy-on-write foundations | Host services; full user launch/porting integration remains work |
| Task transaction | Scoped repair, immutable staging, pinned validation, durable commit receipts and replay | Scripted host proof; model comparison and target task enforcement pending |
| Execution fabric | Placement and launch-plan contracts | Simulation-only routing/load; no distributed transport claim |
| Driver Foundry | virtio-net and virtio-blk Oracle/replay loops and harness vector transfers | Host replay, Linux Oracle devices, and QEMU harness fixtures |
| Hardware loop | Golden-machine contract, appliance inventory and serial-capture tooling | First live Pi↔M900 capture and physical graduation remain pending |

The [integration inventory](docs/plans/2026-06-17-s10-5-host-to-target-integration.md)
explains the host/target split. The kernel's capability checks and the host
services' policies are real components; they are not yet one complete
target-native agent environment.

## Run the existing components

Install the pinned Rust toolchain, `just`, QEMU, and OVMF using
[Getting Started](docs/GETTING_STARTED.md), then:

```bash
git clone https://github.com/maxwellsantoro/RamenOS.git
cd RamenOS

# Host: snapshots, subscriptions, filtered views, and runner integration tests
just foundry-semantic-state-s10-2

# Target: dual-architecture QEMU boot, IPC, and tracing
just foundry-s0
```

The first command exercises host component behavior; the second proves the boot
and IPC baseline. The useful task proof is the host entry point above; these
additional gates cover other components and require no autonomous model.

| Evidence to inspect | Command |
|---------------------|---------|
| Canonical protocol IDs and direct IPC wire types | `just idl-lint` |
| Host broker and semantic/shmem proxy | `just foundry-broker-kernel-bridge-s10-5-1` |
| Selected host-to-QEMU IPC paths | `just foundry-qemu-ipc-bridge-s10-5-2` |
| Driver replay and net/block harness vector transfers | `just s11`, `just s13` |
| Golden-machine, GOP, and appliance scaffolds | `just s12` |

Foundry is how claims are checked: host tests, QEMU, replay, live HIL, and metal
observations have different meanings. Default CI is hardware-free.
[`PASS/QEMU` does not imply `PASS/METAL`](EVIDENCE_LEVELS.md).

Full `just preflight` mirrors CI's complete SW0 sequence. It requires a Linux
host, Python `jsonschema`, a Docker engine with its builtin seccomp profile, and
the pinned image from `tools/agent_task/linux_sandbox.py` already installed.
Missing prerequisites produce `INCOMPLETE` before the build; individual host/QEMU
gates remain useful on macOS. S11.8/S13.6 validate embedded-vector harness
transfers and do not establish device-backed native net/block I/O.

## What comes next

Development follows a dependency-driven plan for a coordinator and simultaneous
agents. The next product checkpoint is one complete human task: launch an app,
understand its permissions, use keyboard input, save and reopen an artifact, and
recover from a failed component without an AI model. Contracts and focused gates
let runtime, input, desktop, Store, and SW0 work progress independently before
integration. [Next Tasks](NEXT_TASKS.md) identifies the bounded work ready to assign.

The physical lane remains **live serial capture → AMT power/reset → S12 on SATA
→ S13 NVMe boot and verified reboot/rollback**, awaiting setup. Driver work still
requires its Reference Vault, Oracle traces, and gates. Host/replay/QEMU preparation
does not wait for physical qualification or the opt-in model comparison; each
claim requires evidence from its actual execution environment.

[Current Status](CURRENT_STATUS.md) records landed work and
[Next Tasks](NEXT_TASKS.md) owns execution order.
[Roadmap](ROADMAP.md) describes longer-range direction.

Today's pre-alpha supports development and evaluation of typed OS interfaces,
agent authority, semantic observability, and driver evidence. Everyday use is
the destination; a complete desktop and production security assurance still
require implementation and validation. POSIX remains a compatibility layer.
See [Security Status](SECURITY_STATUS.md) for implementation limits and open risks.

## Explore and contribute

- **Understand the design:** [Platform Overview](PLATFORM_OVERVIEW.md) and
  [Constitution](CONSTITUTION.md), guided by the [Vision](VISION.md), including
  human control and request authority versus observable authority.
- **Run or debug components:** [Getting Started](docs/GETTING_STARTED.md) and
  [Development Reference](docs/DEVELOPMENT_REFERENCE.md) for Store examples,
  operator settings, and the repository map.
- **Help demonstrate the thesis:** [Agent Task Proof plan](docs/plans/2026-09-16-agent-task-proof.md).
- **Help with hardware:** [Next Tasks](NEXT_TASKS.md) and
  [Evidence Levels](EVIDENCE_LEVELS.md); start driver work from Reference Vaults
  and protocol traces.
- **Contribute a slice:** [Contributing](CONTRIBUTING.md), [Agent Instructions](AGENTS.md),
  and [Slices](SLICES.md). Each slice needs a consumer, a bounded contract, and
  a deterministic Foundry gate.
- **Coordinate parallel work:** [Agentic Workflow](docs/AGENTIC_WORKFLOW.md) defines
  assignment, shared-file ownership, review, and integration; the project
  [coordinate-work skill](.agents/skills/coordinate-work/SKILL.md) applies it.
- **Find other docs:** [Documentation Index](docs/INDEX.md), including the
  subordinate RamenOrg governance and research tracks. Those artifacts grant no
  merge, release, hardware, or public-support authority on their own.

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report vulnerabilities
through [Security](SECURITY.md).

## License

RamenOS is licensed under either [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.
