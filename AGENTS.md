# RamenOS — Agent Instructions

Build a reliability-first, Rust-first, post-Unix OS for **humans and AI agents**,
with everyday use as the destination. The three pillars are **OS Core** (kernel,
services, runtimes), **Foundry** (tooling and evidence gates), and **Store**
(discovery, permission previews, execution, and native ports).

This is the stable agent contract. Read [VISION.md](VISION.md) for product
direction, [CURRENT_STATUS.md](CURRENT_STATUS.md) for landed behavior, and
[NEXT_TASKS.md](NEXT_TASKS.md) for executable work. [ROADMAP.md](ROADMAP.md)
is directional; [SLICES.md](SLICES.md) defines slices; [CHANGELOG.md](CHANGELOG.md)
and [DECISIONS.md](DECISIONS.md) preserve history and rationale. Do not copy the
task queue or milestone history into this file. `CLAUDE.md` links to this file.

## Product and evidence

- Keep human interaction, hardware support, and developer workflows visible
  alongside agent interfaces. SW0 Agent Task Proof evaluates one part of the OS.
- Core human interactions must work without an AI model. Models may translate
  intent; explicit policy and enforcement decide authority.
- Describe speed, safety, hardware adaptability, isolation, and everyday
  readiness as goals until matching evidence exists. The project is public
  pre-alpha; host services, replay, simulation, QEMU, and metal have different scopes.
- Isolation contains faults and typed contracts reduce coupling. Test affected
  consumers and shared-resource behavior when changing a component.
- Use [EVIDENCE_LEVELS.md](EVIDENCE_LEVELS.md) for hardware claims and
  [SECURITY_STATUS.md](SECURITY_STATUS.md) for current security limits. Never infer
  metal graduation, hidden-affordance noninterference, or release readiness.

## Non-negotiables

1. Rust-first kernel and core services.
2. Native interfaces are typed **Harnesses/Portals** defined in IDL; no ioctl escape hatches.
3. **POSIX is compatibility-only** and must not define native APIs.
4. The kernel validates capabilities for fast-path operations; user-space brokers decide grants.
5. **Control plane: typed messages. Data plane: zero-copy shared memory.**
6. Preserve **kernel ≠ services ≠ store**.

[CONSTITUTION.md](CONSTITUTION.md) holds the full invariants. Changing it requires
a `DECISIONS.md` entry. No temporary exception may silently violate it.

## Slice workflow

Implement vertical slices with a bounded behavior or contract, a consumer, and a
Foundry gate. Each implementation change must improve boot/run behavior, implement
an IDL contract, add a Foundry assertion, or implement a Store feature consuming an
OS capability. Documentation and governance changes must support those outcomes
and their evidence.

1. Read the authoritative status/task pair and the relevant maintained contract.
2. **Gate-first:** write behavior, denial, and failure assertions before implementation.
3. Define new native interfaces in `idl/harness/`, `idl/portals/`, or `idl/services/`.
4. Run `just codegen`; never hand-edit `*.generated.rs` or any `generated/` content.
5. Implement the smallest path across the intended boundary and run affected gates.
6. Update `CURRENT_STATUS.md` and `CHANGELOG.md` per milestone; update `NEXT_TASKS.md`
   when work is completed or its order changes. Record design choices in `DECISIONS.md`.

If design is blocked, choose the simplest viable default within the Constitution,
record it, and continue. Missing authority or evidence cannot be replaced by a default.

## Code guardrails

- No heap allocation in `kernel/`: no `alloc`, `Vec`, `String`, or `Box`.
- Architecture-specific code belongs in `kernel/src/arch/`.
- `kernel_api/` is `#![no_std]` with no external dependencies. `kernel/` permits
  the recorded `spin` dependency; new dependencies require an explicit decision.
- Services may use `kernel_api` contracts, never kernel internals. Service-facing
  artifact types come from `artifact_store_schema`; keep Store IO ownership in
  `artifact_store_core` / `store_service`, not schema consumers.
- Keep IPC formats typed, versioned, bounded, and fail closed on invalid wire data.
- Keep trace, capability, and accounting state scoped by domain. Avoid ambient globals.
- Host development scaffolds require explicit, default-off features and visible
  limits; helper sandbox controls do not establish default runtime containment.
- For agent-facing or cross-domain boundaries, define request authority (`Lang`)
  separately from observable authority (`ObsContract`).

## Driver and application work

- Before hardware interactions, obtain the **Reference Vault** and Oracle
  `protocol_trace` artifacts. Derive registers and behavior from that evidence,
  not pre-training. See [Reference Vaults](drivers/reference_vaults/README.md).
- For application ports, derive the capability manifest from `observed_caps_v0`
  and validate it against scenarios. Observation of one run is not all possible authority.

## Active track and validation

- **Now:** S12.4 HIL appliance v0 physical loop; live serial observation precedes actuation. Physical runs await setup; see `NEXT_TASKS.md`.
- **Independent software lane:** SW0 continues without H0–H3 or NVMe graduation.
  The status/task pair owns its landed scope and remaining controls, including S14 prerequisites.
- **Keep green for org/research planning:** `just s11`, `just s12`, `just s13`,
  and `just foundry-org-governance-g0`. Run `just hil-appliance` for appliance docs/contracts.

Use the pinned toolchain in `rust-toolchain.toml` and commands in [justfile](justfile):

| Work | Commands |
|------|----------|
| Formatting and host lint | `just fmt`, `just clippy` |
| Generated bindings and IDL checks | `just codegen`, `just idl-lint` |
| Host / bare-metal / UEFI builds | `just build-host`, `just build-targets`, `just build-uefi` |
| Boot baseline | `just foundry-s0` |
| Complete local CI sequence | `just preflight` |

Full preflight requires Linux, Python `jsonschema`, Docker with builtin seccomp,
and the already installed pinned image. Missing prerequisites produce `INCOMPLETE`.
Use focused host/QEMU gates on macOS and report limitations. Setup, gate coverage,
and S2 inputs are in [Getting Started](docs/GETTING_STARTED.md) and
[Development Reference](docs/DEVELOPMENT_REFERENCE.md).

## PR and authority boundaries

- The path-scoped **`merge-gate`** requires `org-governance` for docs/org-only PRs
  and successful `foundry` for OS-code PRs. Markdown outside `.github/` and
  JSON/YAML packets under `docs/` qualify for the docs/org path; tooling, workflows,
  and `justfile` require Foundry. Classification errors fail closed.
- Open PRs as **`ramen-implementer[bot]` (A2)**. A **different identity (A3)**
  approves and merges. Drop `GH_TOKEN` before switching to the human reviewer.
  Follow [the implementer-bot guide](docs/org/RAMEN_IMPLEMENTER_BOT.md) for credentials
  and exact commands; this file grants no approval, merge, or release authority.
- RamenOrg work uses bounded `WorkOrderV0`, `HandoffPacketV0`, and `BoardVoteV0`
  artifacts. A2-local grants no merge, release, self-approval, HIL actuation, or
  public-support authority. Preserve separation of duties.
- Research must connect a product risk, claim boundary, evidence plan, and landing
  path. Use [the research index](docs/research/INDEX.md); RamenOS is research-backed.

Use [docs/INDEX.md](docs/INDEX.md) to find maintained references and archive policy.
