# S10.5: Host-to-Target Integration

**Last Updated:** 2026-06-17
**Status:** Active (S10.5.0–S10.5.2 complete; S10.5.3+ deferred)
**Gate:** `tools/ci/foundry_host_target_s10_5.sh` (in `foundry_ci_extended.sh`)
**Related:** ROADMAP.md §5/§13, `docs/plans/2026-06-17-s10-2-1-subscribe-reactor.md`, CONSTITUTION.md

---

## Executive Summary

S10.0–S10.4 delivered substantial **host-side** contracts: Wasmtime native runner, semantic snapshots/reactor, store service IPC, execution-fabric simulation. QEMU today runs the RamenOS kernel plus **init bytecode** (hello, ping/pong, shmem tests) — not Wasmtime, not `native_runner`, not `semantic_state` WASM.

S10.5 is the **bridge slice**: prove one end-to-end path where a **semantic-state operation executes under QEMU** using real kernel shmem + typed protocol bytes, not host-process scaffolding alone.

**Chosen option: A (Semantic State on QEMU)** — with an explicit inventory-derived implementation split (see §3).

---

## 1. Inventory (concrete, 2026-06-17)

### What runs in QEMU today

| Component | QEMU? | Evidence |
|-----------|-------|----------|
| RamenOS kernel (`kernel_uefi`) | **Yes** | `foundry_s0.sh`, `foundry_shmem_dataplane_s8_phase4_integration.sh` |
| Init bytecode image (`RINI` / `build_init_image.py`) | **Yes** | Profiles: `default`, `alt`, `bad`, `shmem_test` |
| Init ops (kernel-side handlers in `kernel/src/init.rs`) | **Yes** | `OP_HELLO`, `OP_PING_PONG`, `OP_TRACE`, `OP_SHMEM_TEST`, … |
| Linux compat capsule (S2) | **Yes** (separate VM) | `foundry_compat_s2.sh` — **Linux** kernel + initrd, not native RamenOS domains |
| `native_runner` (Wasmtime) | **No** | Host `std` + `wasmtime` crate; `foundry_native_runner_s10_0.sh` is `cargo test` only |
| `semantic_state` WASM cdylib | **No** | Built `wasm32-unknown-unknown`; executed only via host Wasmtime |
| `domain_manager` / broker | **No** | Host process; `SimulatedKernelOps` |
| `store_service` | **No** | Host Unix socket |
| `runtime_supervisor` | **No** | Host orchestrator |
| Kernel Unix socket IPC to host (`/run/ramen/kernel.sock`) | **No** | Designed in native-runner docs; **not implemented in kernel** |
| Userspace native domain loader | **No** | `domain_registry` / MM tables exist; no process loader for WASM domains on target |

### What runs on host Wasmtime today

| Path | Entry | Kernel bridge |
|------|-------|---------------|
| `native_runner` CLI / lib | `services/native_runner` | `KernelBridge` → Unix socket (mock `/dev/null` in tests) |
| `runtime_supervisor` `native_wasm_v0` | `native_wasm_runner.rs` | Stub grants; Wasmtime on host |
| `semantic_state` `_start` | WASM cdylib | Harness imports → host-generated shims → `kernel_bridge.transact()` |
| S10.2 subscribe reactor | `SemanticReactor` (host `rlib`) | In-memory; no QEMU |

**Conclusion:** `native_runner` is **host-only today**. Option A cannot mean “run `semantic_state.wasm` inside QEMU via Wasmtime” in S10.5.0 without first building a target userspace runtime (out of scope).

---

## 2. Option decision

| Option | Description | S10.5.0 verdict |
|--------|-------------|-----------------|
| **A — Semantic state on QEMU** | Typed `get_snapshot` / shmem delivery under QEMU | **CHOSEN** — implement via kernel init harness path first |
| B — Projection query via semantic_store harness | QEMU domain queries store index | **Deferred** — requires store harness service on target; heavier than A |
| C — Broker kernel bridge only | Real grants for native_runner | **S10.5.1 unlock** — smallest bridge after A proves target semantic bytes |

### Why A with an init-bridge sub-phase

Preserves Option A intent (semantic state contract proven on QEMU) while respecting inventory:

1. **S10.5.0 (this slice):** Kernel init op exercises `services.semantic_state` protocol + shmem data plane on QEMU serial — **non-host semantic bytes**.
2. **S10.5.1:** Broker/kernel bridge for the **one harness path** (`shmem_control` + `semantic_state`) so host `native_runner` uses broker-provided caps and a host proxy.
3. **S10.5.2:** Host Wasmtime `native_runner` against QEMU-exposed IPC **or** target userspace loader — only after 5.0/5.1.

**Out of scope (scope guard):** moving `store_service` into QEMU; full broker migration; S10.2 capability-filtered snapshots; multi-source aggregation.

---

## 3. S10.5.0 design — `get_snapshot` on QEMU via init profile

### Init profile

Add to `tools/init/build_init_image.py`:

| Profile | content_id | Ops |
|---------|------------|-----|
| `semantic_snapshot` | `init-semantic-snapshot` | `[OP_SEMANTIC_SNAPSHOT]` (new op `7`) |

Pattern matches `shmem_test` (S8 Phase 4 integration gate).

### Kernel handler (`kernel/src/init.rs`)

New `OP_SEMANTIC_SNAPSHOT` handler (behind `test_protocols` or dedicated `semantic_state_init` feature):

1. Create shmem region via in-kernel `ShmemRegionTable` (same primitives as `OP_SHMEM_TEST`).
2. Write a **deterministic** `PlatformSnapshotV0` Markdown payload (fixed timestamp/arch/boot_id — same discipline as `semantic_state::build_default_snapshot` stub).
3. Compute **SHA256 prefix** of snapshot bytes (first 16 hex chars).
4. Emit serial markers (Foundry PASS strings):

```
semantic_state: get_snapshot ok
semantic_state: snapshot_format=markdown
semantic_state: snapshot_bytes=<N>
semantic_state: snapshot_sha256=<16-hex-prefix>
semantic_state: shm_cap=<u64>
```

Optional trace ring event with same sha256 prefix (not required for S10.5.0 PASS).

This is **not** running WASM; it proves the **typed semantic snapshot bytes** and **shmem delivery primitive** on the QEMU target path — the same contract `get_snapshot` / `get_snapshot_reply` + shmem payload depend on.

### Capability / shmem path (v0)

| Layer | S10.5.0 behavior |
|-------|------------------|
| Cap grants | Init runs in kernel/init context; uses `ShmemRegionTable` directly (no broker) |
| shmem | Kernel `shmem_control` data plane (create region, map, write payload) |
| Wire types | `kernel_api::generated::semantic_state_v1::{GetSnapshot, GetSnapshotReply}` encoded in handler self-test log (optional envelope roundtrip unit test in kernel) |
| Broker | **Unchanged** — `SimulatedKernelOps` remains on host |

S10.5.1 will grant real `RAMEN_CAP_SHMEM_CONTROL` + `RAMEN_CAP_SEMANTIC_STATE` handles for host `native_runner` against a real kernel IPC socket.

### Catalog / store (host path — not S10.5.0 gate)

For later host Wasmtime + real broker (S10.5.1+):

| Artifact | kind | Notes |
|----------|------|-------|
| `semantic_state.wasm` | `native_wasm_v0` blob | `wasm32-unknown-unknown` release build |
| Manifest | `NativeWasmManifestV0` | `entrypoint: _start`; caps: `harness.shmem_control_v1`, `services.semantic_state_v1` |
| Launch plan | `native_wasm_v0` runner | `domain_id != 0`; `kernel_ipc` → real socket |

**S10.5.0 does not require store catalog entries** — init image is built by `build_init_image.py`, not ingested via `store_service`.

---

## 4. Foundry gate — `foundry_host_target_s10_5.sh`

### Phase 0 — Inventory (must PASS now)

Static assertions documenting host-vs-target split:

```
FOUNDRY_HOST_TARGET_S10_5: INVENTORY native_runner_host_wasmtime=true
FOUNDRY_HOST_TARGET_S10_5: INVENTORY semantic_state_wasm32_cdylib=true
FOUNDRY_HOST_TARGET_S10_5: INVENTORY qemu_runs_kernel_init_only=true
FOUNDRY_HOST_TARGET_S10_5: INVENTORY broker_simulated_kernel_ops=true
```

Checks: `wasmtime` in `native_runner/Cargo.toml`; no QEMU invocation in `foundry_native_runner_s10_*.sh`; `semantic_snapshot` profile documented.

### Phase 1 — QEMU semantic snapshot

1. Build `kernel_uefi` with required feature (`test_protocols` or `semantic_state_init`).
2. Build init image: `--profile semantic_snapshot`.
3. Boot QEMU x86_64 UEFI (same harness as `foundry_s0.sh` / shmem integration).
4. Grep serial log for PASS markers:

```
semantic_state: get_snapshot ok
semantic_state: snapshot_sha256=<prefix>
```

### Phase 2 — S10.5.1 broker/kernel bridge (separate gate)

See `docs/plans/2026-06-17-s10-5-1-broker-kernel-bridge.md` and `foundry_broker_kernel_bridge_s10_5_1.sh`.

```
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: METRIC snapshot_sha256_prefix=9c0de4419f03f426
FOUNDRY_BROKER_KERNEL_BRIDGE_S10_5_1: PASS
```

Host-only: broker allowlist grants → `KernelHarnessProxy` transact → `get_snapshot` with shmem. Not QEMU.

### Gate PASS line

```
FOUNDRY_HOST_TARGET_S10_5: PASS
FOUNDRY_HOST_TARGET_S10_5: ok
```

### CI policy

- S10.5.0: `just foundry-host-target-s10-5` (requires QEMU). Not yet in `foundry_ci_extended`.
- S10.5.1: `just foundry-broker-kernel-bridge-s10-5-1` (host-only) — PASS.

---

## 5. Fallback / blocker documentation

**Blocker (confirmed):** `native_runner` cannot execute inside QEMU today.

| Blocker | Unlock (smallest step) | Slice |
|---------|------------------------|-------|
| No Wasmtime on target | S10.5.0 init-bridge proves semantic bytes on QEMU without WASM | S10.5.0 |
| No kernel Unix socket IPC | `KernelBridge` + broker `grant()` → real kernel fast-path for `shmem_control` | S10.5.1 |
| `request_capability_grants` stubbed | Wire `native_wasm_runner` to `domain_manager` IPC | S10.5.1 |
| No userspace domain loader | Host Wasmtime + QEMU IPC proxy **or** future target loader | S10.5.2+ |

If Phase 1 stalls, **do not** pivot to Option B or move `store_service`; expand S10.5.1 broker bridge instead.

---

## 6. What remains host-only after S10.5.0

| Component | After S10.5.0 |
|-----------|---------------|
| `native_runner` / Wasmtime | Host-only |
| `semantic_state` WASM execution | Host-only |
| `SemanticReactor` subscribe push | Host-only |
| `domain_manager` + `SimulatedKernelOps` | Host-only |
| `store_service` | Host-only |
| `runtime_supervisor` | Host-only |
| **QEMU** | Kernel + init semantic snapshot op (NEW) |

---

## 7. Success metrics

- [ ] `foundry_host_target_s10_5.sh` Phase 0 inventory PASS (now)
- [x] Phase 1 QEMU serial contains `semantic_state: get_snapshot ok` + verifiable `snapshot_sha256` (prefix `9c0de4419f03f426`)
- [x] S10.5.1 host broker/proxy bridge green (`foundry_broker_kernel_bridge_s10_5_1.sh`)
- [ ] No regression in `foundry_ci_extended.sh`
- [ ] `CURRENT_STATUS.md` updated when Phase 1 green

---

## 8. Implementation checklist

1. ✅ Add `OP_SEMANTIC_SNAPSHOT` + `semantic_snapshot` init profile
2. ✅ Implement kernel init handler with deterministic snapshot + sha256 serial markers
3. 📋 Green Phase 1 in `foundry_host_target_s10_5.sh` on a QEMU-capable host
4. ✅ Wire into `justfile`; add to CI extended when green
5. ✅ S10.5.1 broker/proxy bridge: real broker grant fetch for `shmem_control` + `semantic_state` only
