# RamenOS

**Last Updated:** 2026-06-24
**Status:** Public pre-alpha, active development

RamenOS is an experimental, Rust-first operating system project for an AI-native
computing era.

The goal is not to wrap Unix in friendlier automation. The goal is to build an
operating system where agents, applications, and services interact through typed
interfaces, explicit capability grants, observable state, and evidence-backed
hardware support.

Founded by [Maxwell Santoro](https://maxwellsantoro.com).

## What Makes It Different

- **Typed native interfaces:** OS services communicate through IDL-defined
  contracts instead of ioctl-like escape hatches or screen-scraped human UI.
- **Capability-backed authority:** Components receive explicit, minimal handles;
  fast-path capability validation belongs in the kernel.
- **Control/data plane split:** Typed messages handle coordination; shared memory
  handles move bulk data.
- **Quarantined compatibility:** POSIX and Linux compatibility are treated as
  compatibility layers, not the native application model.
- **Driver Foundry:** Hardware support is developed through an evidence loop:
  reference vaults, protocol traces, replay scoreboards, minimization, fuzzing,
  and Foundry gates.
- **Research-backed, product-bound:** Research informs the OS where it reduces a
  product or safety risk, with explicit claim boundaries and landing paths.

## Project Shape

The repository is organized around three pillars:

1. **OS Core**: kernel, boot paths, IPC, capabilities, shmem, tracing, services,
   and runtimes.
2. **Foundry**: trace capture, replay, hardware-in-the-loop gates, evidence
   policy, and CI-style validation.
3. **Store Platform**: artifact ingestion, launch plans, native runtime paths,
   compatibility runners, and the early porting ladder.

Development happens through vertical slices. A change should improve boot/run
behavior, implement an IDL contract, add a Foundry gate, or build a Store feature
that consumes an OS capability.

## Current State

RamenOS is not a daily-driver operating system yet. It is a working pre-alpha
codebase with boot gates, typed contracts, host services, QEMU harnesses, and
hardware evidence scaffolding.

Recently landed work includes:

- S0 boot/IPC/tracing baseline in QEMU.
- IDL and wire-contract integrity gates.
- Semantic state snapshots and capability-filtered projections.
- Native runner, Store CLI paths, and compatibility runner scaffolding.
- Driver Foundry loops for virtio-net and virtio-blk using reference vaults,
  live Oracle capture, replay scoreboards, and runtime QEMU harnesses.
- S12 first-metal scaffolding: UEFI GOP probe, physical HIL boot gate, IOMMU
  inventory, and HIL appliance controller scaffold.
- S13 persistent-storage scaffolding: virtio-blk Oracle capture, sector replay,
  block I/O harness, NVMe boot detection scaffold, and atomic-update metadata
  probe.
- G0 RamenOrg governance scaffolding for bounded agent work, research-backed
  planning, and claim-safety gates.

The active execution track is S12.4: a HIL appliance v0 physical loop, starting
with serial observation and then power/reset actuation. S13 metal graduation is
expected to run through that appliance loop once it is stable.

See [CURRENT_STATUS.md](CURRENT_STATUS.md) for landed state and
[NEXT_TASKS.md](NEXT_TASKS.md) for the next executable tasks.

## Evidence Levels

Foundry gates are intentionally precise about what has been proven. A `PASS/QEMU`
gate is not the same thing as `PASS/METAL`.

Common levels include:

- `PASS/QEMU`: build, inventory, and QEMU smoke validation.
- `PASS/HIL-LOG`: replay of an operator-provided serial log.
- `PASS/HIL-LIVE`: live serial captured during the gate run.
- `PASS/HIL-APPLIANCE`: live serial plus appliance controller evidence.
- `PASS/METAL`: graduation mode on Tier-1 hardware with provenance markers.

See [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md) before interpreting hardware claims.

## Quick Start

Requirements:

- Rust toolchain pinned by [rust-toolchain.toml](rust-toolchain.toml)
- `rust-src`
- `just`
- QEMU/OVMF tooling for the boot and Foundry gates that require emulation

Useful commands:

```bash
just build-host
just codegen
just build-targets
just preflight
```

Slice gates:

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

Hardware-in-the-loop gates are opt-in and require the relevant environment
variables. Default gates avoid claiming metal success without live evidence.

## Store CLI Example

Emit a launch plan from the catalog:

```bash
cargo run -p store_cli -- emit-plan \
  --catalog store/catalog.json \
  --program-id ramen.demo.hello \
  --out out/store/launch_plan.json
```

Ingest a file into the installed store:

```bash
cargo run -p store_cli -- ingest \
  --src /path/to/file \
  --installed-root out/installed
```

## Contributing

RamenOS favors small, evidence-bearing slices over large subsystem drops.

Before proposing a change:

- Read [CONTRIBUTING.md](CONTRIBUTING.md).
- Read [AGENTS.md](AGENTS.md) if you are working with an AI coding agent.
- Add new native interfaces under [idl](idl/) and regenerate bindings.
- Keep kernel, services, and Store boundaries separate.
- Run `just preflight` before pushing when practical.

For driver work, start from the Reference Vault and protocol traces. The goal is
to produce code whose observed behavior matches the Oracle, then gate it.

## Key Docs

- [PLATFORM_OVERVIEW.md](PLATFORM_OVERVIEW.md)
- [CURRENT_STATUS.md](CURRENT_STATUS.md)
- [NEXT_TASKS.md](NEXT_TASKS.md)
- [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md)
- [ROADMAP.md](ROADMAP.md)
- [SLICES.md](SLICES.md)
- [STORE_SPEC.md](STORE_SPEC.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/INDEX.md](docs/INDEX.md)
- [AGENTS.md](AGENTS.md)

## License

RamenOS is dual-licensed under MIT or Apache-2.0. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
