# RamenOS

**Last Updated:** 2026-06-17
**Status:** Active

The first true AI-native, reliability-first, post-Unix operating system. Built around:
- **Agentic Substrate:** Semantic State APIs and Typed IDL interfaces (no screen-scraping or brittle human-emulation required for AI).
- **Quantized Authority:** Capability-backed execution that physically prevents AI agents (and apps) from hallucinating destructive actions.
- **Quarantined Compatibility:** Isolated domains for Linux/Flatpak and GPU blobs (compatibility without surrender).
- **The Driver Foundry:** A unified pipeline (Trace → Replay → Fuzz → Minimize → Gate) allowing AI agents to systematically distill legacy drivers into native Rust components.

This repo is organized as three pillars:
1) OS Core (kernel + services + runtimes)
2) Foundry (tooling + CI gates)
3) Store Platform (Run Now → Vote/Port → Publish)

## Quick Start (Day 0)
Requirements:
- Rust (nightly) + rust-src
- `just` (optional but recommended)

Commands:
- `just build-host`
- `just codegen`
- `just build-targets`
- `just preflight` (format + codegen + strict lint + tests + Foundry umbrella)

Store CLI:
- Emit a launch plan from the catalog:
  - `cargo run -p store_cli -- emit-plan --catalog store/catalog.json --program-id ramen.demo.hello --out out/store/launch_plan.json`
- Ingest a file into the installed store (prints content ID):
  - `cargo run -p store_cli -- ingest --src /path/to/file --installed-root out/installed`

Compat Kernel Mirror (CI):
- Run **Actions → Mirror compat kernel** (workflow `mirror_compat_kernel.yml`) after updating the pinned URL/SHA.
- CI uses the release asset `compat-kernel-v6.6.50/compat-kernel.deb` by default.

Operational current state lives in `CURRENT_STATUS.md`; next work lives in `NEXT_TASKS.md`.
`README.md` summarizes stable user-facing state only.

QEMU boot gates are implemented and exercised by `tools/ci/foundry_s0.sh`
(x86_64 UEFI path + aarch64 direct-kernel path).

Recent additions include:
- S11.2-pre IDL/wire contract integrity gate (`just idl-lint`) for canonical protocol/message IDs and fixed-wire IPC payloads,
- S11.1 Driver Factory Oracle capture scaffold and S11.2 replay-scoreboard red gate,
- S10.2 v1.1 capability-filtered semantic snapshots + `domain_manager` reactor publish,
- S10.5.0/10.5.1 host→target integration (QEMU semantic snapshot + broker/kernel harness bridge),
- S10.3 projection storage (durable index, read-only VFS, CoW commits),
- S10.4 execution fabric (simulation + canonical launch plans),
- `posix_runner_v0` host-shell runner path in `runtime_supervisor`,
- IDL-generated C header flow for capsule control contracts,
- `evidence_policy.toml` redaction/size hook for evidence ingestion,
- `domain_manager_v1` typed lifecycle contract + `domain_manager` service,
- expanded portal suite (`clipboard`, `notifications`, `screen_capture`) with typed traces/evidence,
- V-012 Phase 5 trace client (`services/trace_client`) with domain-manager integration,
- S10.0/S10.1 native WASM runner (`services/native_runner`) with capability broker integration,
- S10.2 semantic state substrate (`services/semantic_state`) with platform snapshot schema + subscribe reactor,
- S0→S7 umbrella gate (`tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh`) plus CI extended gates (`tools/ci/foundry_ci_extended.sh`, includes S10.5.0/10.5.1).

## Environment Variables (S7 Security Hardening)

### Store Service
- `RAMEN_STORE_TRUSTED_KEYS`: Path to file containing trusted Ed25519 public keys (REQUIRED in production)
- `RAMEN_STORE_DEV_MODE`: Runtime dev flag (`1`/`true`/`yes`/`on`) for unsigned artifacts and synthetic capabilities in local Foundry flows (DEVELOPMENT ONLY)
- `store_service --features dev_insecure`: Compile-time dev flag for trusted-key fallback during signing tests (MUST NOT be used in production builds)
- `RAMEN_STORE_ACCESS_POLICY`: Access control policy (AllowAll, RequireCredentials, RequireKnownService, Whitelist)
  - Default: RequireCredentials (fail-closed)
- `RAMEN_STORE_SOCKET`: Path to store service Unix domain socket (default: out/store_service.sock)
- `RAMEN_STORE_ROOT`: Path to store root directory (default: out/installed/artifacts)
- `RAMEN_STORE_AUDIT_LOG`: Path to audit log file (default: out/store_service_audit.log)

### POSIX Runner
- `RAMEN_POSIX_RUNNER_ACK_RISK=1`: Must be set to allow script execution (kill-switch)
- `RAMEN_POSIX_RUNNER_DISABLE_SANDBOX=1`: Disable sandbox (DANGEROUS, dev only)

### Important Security Notes
- All new environment variables default to fail-closed (deny-by-default) behavior
- Development modes require explicit opt-in with prominent warnings
- All security violations are logged with forensic detail

## Key Docs
- PLATFORM_OVERVIEW.md
- ROADMAP.md
- SLICES.md
- STORE_SPEC.md
- CONTRIBUTING.md
- docs/LINT_DEBT.md
- docs/EVIDENCE_POLICY_V0.md
- docs/S7_SECURITY_HARDENING_PHASE2.md
- docs/INDEX.md
- CURRENT_STATUS.md
- NEXT_TASKS.md
- AGENTS.md
