# RamenOS

Reliability-first, post-Unix operating system. Three pillars: OS Core, Foundry, Store.

Current execution truth lives in `CURRENT_STATUS.md` + `NEXT_TASKS.md`. The
active OS track is S12.4 HIL appliance serial observation, followed by
power/reset actuation and S13 metal graduation.

## Constitutional Non-Negotiables

1. Native interfaces are typed Harnesses/Portals — no ioctl escape hatches
2. POSIX is compatibility-only — never design native APIs around POSIX
3. Capability validation is kernel-side for fast-path ops; brokers decide grants
4. Control plane = typed messages; data plane = zero-copy shared memory
5. Preserve boundaries: kernel ≠ services ≠ store

## Workspace Crates

| Crate | Purpose | Targets | External deps? |
|-------|---------|---------|----------------|
| `kernel/` | Core kernel library | `x86_64-unknown-none`, `aarch64-unknown-none` | **None** |
| `kernel_api/` | Shared types for kernel↔runtime | `x86_64-unknown-none`, `aarch64-unknown-none` | **None** |
| `kernel_aarch64/` | aarch64-specific boot | `aarch64-unknown-none` | No |
| `kernel_uefi/` | UEFI boot path | `x86_64-unknown-uefi`, `aarch64-unknown-uefi` | No |
| `idl_codegen/` | Code generator for IDL TOML specs | Host | Yes |
| `runtime_supervisor/` | Process lifecycle + compat runner | Host | Yes |
| `store_cli/` | Store catalog + launch plan tool | Host | Yes |
| `artifact_store_core/` | Content-addressed artifact storage | Host | Yes |
| `services/portals/` | Portal service (file picker, etc.) | Host | Yes |

## Build Commands

```sh
just fmt              # cargo fmt --all
just clippy           # clippy (excludes kernel_uefi, kernel_aarch64)
just codegen          # generate Rust bindings from IDL TOML specs
just build-host       # build host-target crates (runs codegen first)
just build-targets    # cross-compile kernel/kernel_api for bare-metal targets
just build-uefi       # build UEFI boot images
```

## Foundry Gates

Gates are shell scripts in `tools/ci/`. Run via `just`:

| Gate | Command | Tests |
|------|---------|-------|
| S0 | `just foundry-s0` | QEMU boot x86_64+aarch64, IPC ping, trace ring |
| Store S0 | `just foundry-store-s0` | Store catalog + launch plan + supervisor |
| S1 | `just foundry-artifact-s1` | Artifact store lifecycle (CAS, install, rollback) |
| S2 | `just foundry-compat-s2` | Compat capsule boot, read-only artifact mount |
| S2.2 | `just foundry-init-s2-2` | Init swap/malformed init assertions |
| S3 trace | `just foundry-trace-s3` | Trace artifact schema + replay |
| S3 portal | `just foundry-portal-file-ro-s3` | Portal file picker RO + token validation |
| S9.0 POSIX runner | `just foundry-posix-runner-s9-0-mitigation` | POSIX runner feature flag gating and warnings |
| S9.0 Boundary | `just foundry-boundary-s9-0-cleanup` | Services dependency cleanup (schema types only) |
| S9.0 Trace isolation | `just foundry-trace-isolation-s9-0-per-domain` | Per-domain trace ring buffers |
| S9.1 Store IPC | `just foundry-v007-phase2-store-service-ipc` | Store service IPC (Unix domain sockets) |
| S9.1 Store hardening | `just foundry-v007-phase3-store-hardening` | Store service hardening (signatures, audit) |
| S10.0 Native runner | `just foundry-native-runner-s10-0` | WASM executor + harness host functions |
| S10.1 Native runner prod | `just foundry-native-runner-s10-1` | Manifest schema, broker, supervisor integration |
| S10.2 Semantic state | `just foundry-semantic-state-s10-2` | Platform snapshot schema + IDL contract |
| S10.5.2 IPC bridge | `just foundry-qemu-ipc-bridge-s10-5-2` | QEMU COM2 framed `get_snapshot` roundtrip |
| CI extended | `just foundry-ci-extended` | S7 security + S9/S10 gates (CI subset) |
| Umbrella | `just foundry-all-s0-s1-s2-s3` | Full S0+S1+S2+S3 suite (used in CI) |

S2 gates require local env vars: `S2_COMPAT_KERNEL`, `S2_COMPAT_INITRD`, `S2_COMPAT_ARTIFACT`.

## IDL Workflow

1. Define interface in `/idl/harness/*.toml` (harness) or `/idl/portals/*.toml` (portal)
2. Run `just codegen` — generates `kernel_api/src/generated/*.generated.rs`
3. Never hand-edit `*.generated.rs` files

## Key Conventions

- No heap allocation in `kernel/` (no `alloc`, `Vec`, `String`, `Box`)
- Arch-specific code belongs in `kernel/src/arch/`
- Gate-first: write the Foundry assertion before the implementation
- Every change must: improve boot/run, implement an IDL contract, add a gate, or implement a Store feature
- Rust nightly toolchain (see `rust-toolchain.toml`)

## Current Status

S10.2 Semantic State Substrate scaffold complete. Active focus: S10.3 Projection Storage. See `CURRENT_STATUS.md` for details.

## Security Lessons Learned (S9.0)

1. **Architectural Boundaries Matter:** The "kernel ≠ services ≠ store" boundary is not just aesthetic—it prevents privilege escalation and data tampering. Services must depend on schema types only, never IO functions.

2. **Feature Flags as Safety Mechanisms:** Host-side scaffolding (like `posix_runner_v0_dev`) requires explicit feature flag gating with prominent warnings to prevent accidental production use.

3. **Per-Domain Isolation is Foundational:** Trace isolation, capability validation, and resource accounting all require per-domain data structures. Global singletons are a security liability.

4. **Fail-Closed Defaults:** Code generation, wire format parsing, and capability validation must fail-closed. Default-deny is safer than default-allow.

5. **Defense in Depth:** Multiple layers (capability validation, schema validation, seccomp filters, namespace isolation) provide better security than any single mechanism.

## Key Documents

- `CONSTITUTION.md` — system invariants (do not modify without DECISIONS.md entry)
- `CURRENT_STATUS.md` + `NEXT_TASKS.md` — authoritative landed state and next work
- `SLICES.md` — slice definitions and status summary
- `ROADMAP.md` — execution sequencing
- `docs/INDEX.md` — maintained documentation map
- `PLATFORM_OVERVIEW.md` — full architecture
- `DRIVER_CAPSULE_SPEC.md` — quarantined driver design
- `docs/plans/security_remediation_v006_v007_v012.md` — detailed security remediation plans
- `docs/plans/v007_phase2_store_service_ipc_design.md` — store service IPC design
