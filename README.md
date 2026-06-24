# RamenOS

[![ci](https://github.com/maxwellsantoro/RamenOS/actions/workflows/ci.yml/badge.svg)](https://github.com/maxwellsantoro/RamenOS/actions/workflows/ci.yml)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](Cargo.toml)

**Status:** active prototype

**Last updated:** 2026-06-24

**Current focus:** S12.4 HIL appliance v0 physical loop, then S13 metal HIL graduation

RamenOS is an experimental, Rust-first operating system built around typed
interfaces, capability-bounded execution, and evidence-driven hardware work.
The long-term bet is simple: AI agents and applications should not have to
drive computers by pretending to be humans at a terminal or a screen. Native OS
interfaces should expose structured state, explicit authority, and auditable
effects.

This repository is not a production OS and does not claim metal graduation,
security readiness, or release readiness without matching evidence. The current
default CI path proves QEMU and Foundry gates; physical hardware claims require
explicit HIL evidence.

## What Is Here

RamenOS is developed as vertical slices across three connected pillars:

- **OS Core:** kernel, IPC, capabilities, trace ring, typed harnesses, runtime
  supervisor, and core services.
- **Driver Foundry:** trace capture, replay, scoreboard, distillation, and
  gates for turning observed device behavior into native Rust components.
- **Store Platform:** artifact contracts, launch plans, signature/access
  policy, and the "run now -> vote/port -> publish" path.

The core design rules are intentionally narrow:

- Native APIs are typed IDL contracts, not ioctl-style escape hatches.
- Control plane is typed messages; data plane is zero-copy shared memory.
- Fast-path capability validation belongs in the kernel.
- Compatibility is allowed, but it is quarantined and never the native API.
- Claims are gated by evidence levels such as `PASS/QEMU`, `PASS/HIL-LOG`,
  `PASS/HIL-APPLIANCE`, and `PASS/METAL`.

## What Runs Today

The repo currently has working QEMU/host gates for:

- x86_64 and aarch64 boot/IPC/trace baseline.
- IDL code generation and wire-contract integrity checks.
- S7 security gates for POSIX runner and store-service fail-closed behavior.
- S10 semantic state, execution fabric, native WASM, and QEMU IPC bridge flows.
- S11 Driver Foundry virtio-net trace replay, live Oracle provenance, and
  runtime `harness.net` packet I/O.
- S12 golden-machine scaffolds, UEFI GOP probe, IOMMU inventory, and HIL
  appliance controller contract.
- S13 persistent-storage contracts, virtio-blk Oracle/replay, runtime
  `harness.block` I/O, and QEMU scaffolds for NVMe boot and atomic rollback.
- G0 RamenOrg governance scaffold and packet validators for bounded agent work.

Current active work is the HIL appliance physical loop:

1. Serial observer.
2. Power/reset actuator.
3. S13 metal graduation through appliance-mediated live capture.

See [CURRENT_STATUS.md](CURRENT_STATUS.md) for landed state and
[NEXT_TASKS.md](NEXT_TASKS.md) for the next executable task. Treat
[ROADMAP.md](ROADMAP.md) as background planning, not operational truth.

## Quick Start

Install:

- Rust nightly with `rust-src`, `rustfmt`, and `clippy`.
- QEMU and OVMF firmware for target gates.
- `just` for the task aliases.

Common commands:

```bash
just build-host
just codegen
just build-targets
just preflight
```

Useful focused gates:

```bash
just s11
just s12
just s13
just hil-appliance
just foundry-org-governance-g0
```

`just preflight` runs format checking, IDL generation, strict lint tranches,
workspace tests, and the Foundry umbrella gate. CI also runs the extended
Foundry gates and the G0 governance gate.

## Store CLI Examples

Emit a launch plan from the catalog:

```bash
cargo run -p store_cli -- emit-plan \
  --catalog store/catalog.json \
  --program-id ramen.demo.hello \
  --out out/store/launch_plan.json
```

Ingest a file into a local installed store:

```bash
cargo run -p store_cli -- ingest \
  --src /path/to/file \
  --installed-root out/installed
```

Validate an execution launch plan:

```bash
cargo run -p store_cli -- validate-execution-launch-plan \
  --src out/store/launch_plan.json
```

## Hardware And Evidence

Default CI is intentionally hardware-free. It proves inventory, schemas,
negative checks, QEMU behavior, and replay determinism. Physical claims require
explicit environment flags and provenance:

```bash
RAMEN_HIL_APPLIANCE=1 just hil-appliance
RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GRADUATION=1 just s13-hil
RAMEN_HIL_APPLIANCE=1 RAMEN_HIL_GOLDEN_MACHINE=1 just s12-hil
```

Important boundary: the HIL appliance is lab infrastructure, not target TCB.
The serial observer can produce `PASS/HIL-LOG` from development replay or
`PASS/HIL-APPLIANCE` from live appliance capture. `PASS/METAL` requires the
matching hardware evidence.

## Operational Knobs

Store service:

- `RAMEN_STORE_TRUSTED_KEYS`: trusted Ed25519 key file, required outside dev.
- `RAMEN_STORE_DEV_MODE`: explicit local-dev opt-in for unsigned artifacts.
- `RAMEN_STORE_ACCESS_POLICY`: `AllowAll`, `RequireCredentials`,
  `RequireKnownService`, or `Whitelist`; default is fail-closed.
- `RAMEN_STORE_SOCKET`, `RAMEN_STORE_ROOT`, `RAMEN_STORE_AUDIT_LOG`: local paths.

POSIX runner:

- `RAMEN_POSIX_RUNNER_ACK_RISK=1`: required kill-switch acknowledgment.
- `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1`: dangerous local-dev bypass.

HIL:

- `RAMEN_HIL_APPLIANCE=1`: enable physical appliance inventory/control paths.
- `RAMEN_HIL_GRADUATION=1`: require live graduation discipline.
- `RAMEN_HIL_SERIAL_DEV` / `RAMEN_HIL_SERIAL_LOG`: live serial device or
  development log input, depending on the gate.

Development modes are explicit, noisy, and should never be treated as release
configuration.

## Repository Map

- [kernel/](kernel/) and [kernel_uefi/](kernel_uefi/): target-side kernel work.
- [kernel_api/](kernel_api/): shared typed contracts and generated bindings.
- [idl/](idl/): IDL source of truth for new interfaces.
- [services/](services/): domain manager, semantic state, execution fabric, and
  supporting services.
- [runtime_supervisor/](runtime_supervisor/): host/runtime launch path.
- [driver_foundry/](driver_foundry/): trace import/replay/assert tooling.
- [drivers/reference_vaults/](drivers/reference_vaults/): captured device
  references and Oracle traces.
- [tools/ci/](tools/ci/): Foundry gates used locally and in GitHub Actions.
- [tools/hil/](tools/hil/): hardware-in-the-loop helper scripts.
- [docs/](docs/): plans, evidence schemas, research, and governance artifacts.
- [hardware/](hardware/): machine, storage, and appliance contracts.

## Key Documents

- [CURRENT_STATUS.md](CURRENT_STATUS.md): what has landed.
- [NEXT_TASKS.md](NEXT_TASKS.md): next executable work.
- [PLATFORM_OVERVIEW.md](PLATFORM_OVERVIEW.md): architecture and design model.
- [CONSTITUTION.md](CONSTITUTION.md): project principles.
- [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md): claim/evidence vocabulary.
- [SECURITY_STATUS.md](SECURITY_STATUS.md): security posture and boundaries.
- [SLICES.md](SLICES.md): completed slice inventory.
- [STORE_SPEC.md](STORE_SPEC.md): store platform contracts.
- [CONTRIBUTING.md](CONTRIBUTING.md): local preflight and lint policy.
- [AGENTS.md](AGENTS.md): coding-agent operating rules.

## License

RamenOS is licensed under either of:

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)

at your option.
