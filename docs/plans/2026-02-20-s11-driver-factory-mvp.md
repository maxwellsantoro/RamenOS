# S11: The Driver Factory MVP Design

**Last Updated:** 2026-06-21
**Status:** Complete (S11 Definition of Done satisfied via `just s11`)
**Related:** CONSTITUTION.md, Reference Vault Spec, The Oracle Rule

---

## 0. Oracle Device Selection (resolved 2026-06-17)

**CHOSEN:** `virtio-net-pci` in QEMU Linux Oracle capsule for S11 MVP.

| Criterion | virtio-net (QEMU) | NVMe (generic) |
|-----------|-------------------|----------------|
| QEMU availability | Built-in `virtio-net-pci`; no passthrough | Needs virtio-blk or PCI passthrough + vendor variance |
| Trace complexity | Moderate (virtqueues, PCI config, MSI-X optional) | High (admin/IO queues, many opcodes, error paths) |
| Harness target | `net_v1` sketched below | Requires `block_harness_v0` (not landed) |
| Foundry reuse | Capsule relay + virtio-serial trace path (S10.5.2) | Separate block stack |
| Downstream metal | Common in cloud/VM bring-up | Boot-critical — deferred to **S13** |

**Deferred:** generic NVMe Oracle for S13 persistent storage; revisit after S11 replay gate is green.

**Gates:**
- `tools/ci/foundry_s11_driver_factory_s11_0.sh` (inventory + S11.1 capture scaffold) — PASS.
- `tools/ci/foundry_s11_replay.sh` (S11.2 replay scoreboard) — PASS.
- `REQUIRE_LIVE_ORACLE_TRACE=1 tools/ci/foundry_s11_reference_vault_s11_3.sh` — PASS.
- `tools/ci/foundry_s11_runtime_net_s11_8.sh` (S11.8 runtime harness I/O) — PASS.
- Fast-path umbrella: `just s11`.

---

## Executive Summary

S11 implements the first vertical slice of the **Driver Foundry**. We will use an AI coding agent to distill a native Rust driver for a simple device (e.g., `virtio-net` or a generic `NVMe` controller) by observing its interactions in a Linux Oracle capsule.

**Key Design Decisions:**
- **Capture-First:** No C code is ported. We capture the **Protocol Trace** (MMIO/PCI/IRQ) of the Linux driver first.
- **Reference Vault as Context:** The AI agent is given a pinned "Reference Vault" containing the trace, the vendor datasheet (Markdown), and the Linux driver source (for semantics only).
- **Harness-First Distillation:** The generated Rust driver must fulfill a pre-defined IDL Harness (e.g., `net_v1`).
- **Deterministic Scoreboard:** The `ReplayGate` uses the Oracle trace as a deterministic pass/fail metric.

---

## 1. Overview & Goals

### Objective

Prove the feasibility of the AI-augmented "distillation" model by delivering a functional, native Rust driver for a Tier-1 device.

### Design Goals

| Goal | Rationale |
|------|-----------|
| **Oracle Conformance** | The native driver must perform the same sequence of MMIO writes as the Linux driver for the same request. |
| **No "Hallucinated" Registers** | All register offsets and bitmasks must be derived from the trace and datasheet. |
| **Zero-Trust Memory** | The native driver must use `volatile` and `SharedMemory` primitives, never raw pointers. |
| **Reproducible Failure** | If the Linux driver fails in the capsule, the native driver must fail identically on replay. |

---

## 2. The Driver Foundry Pipeline

### Step 1: Oracle Capture (Trace Collection)

We run a Linux microVM (Dom1) with a custom `pci_mmio_tracer` kernel module.

```
+-----------------------------------------------------------------------------+
|                           Dom1: Linux Oracle Capsule                         |
+-----------------------------------------------------------------------------+
|                                                                              |
|  +--------------+    +--------------+    +------------------------------+   |
|  | Linux Driver |<-->| MMIO Tracer  |<-->|  Virtio-Serial Transport     |   |
|  | (Virtio-Net) |    | (Hook)       |    |  (Trace Stream)              |   |
|  +------+-------+    +------+-------+    +------------------------------+   |
|         |                   |                                                |
|         v                   v                                                |
|  +-----------------------------------------------------------------------+   |
|  |                        Virtual Hardware Device                        |   |
|  +-----------------------------------------------------------------------+   |
|                                                                              |
+----------------------------------+-------------------------------------------+
                                   |
                                   v
+------------------------------------------------------------------------------+
|                              Host (Foundry)                                  |
+------------------------------------------------------------------------------+
|                                                                               |
|   Capture `driver_protocol_trace.json` (MMIO_WRITE 0x10=0x01, MMIO_READ...)   |
|                                                                               |
+------------------------------------------------------------------------------+
```

### Step 2: Reference Vault Assembly

A Reference Vault is a structured directory for the AI agent:
- `traces/`: Raw protocol traces (JSON).
- `docs/`: Markdown-formatted datasheets/specs.
- `oracle/`: Corresponding Linux driver source file (C).
- `harness.toml`: The target IDL contract (Rust).

### Step 3: Distillation (The Agentic Workflow)

The agent follows the `SKILL-CREATOR` or `RAMEN-CONVENTIONS` skill to:
1. **Analyze:** Correlate MMIO offsets in the trace with names in the datasheet.
2. **Model:** Create a `RegisterMap` struct using `volatile-register`.
3. **Implement:** Write the `net_v1` harness implementation.
4. **Assert:** Add internal assertions for register state transitions.

### Step 4: Replay Gate (The Scoreboard)

The `foundry_s11_replay.sh` gate:
1. Compiles the native Rust driver.
2. Injects the `driver_protocol_trace.json`.
3. Runs the driver in a "Mock Hardware Environment".
4. **FAIL:** If the MMIO sequence diverges from the Oracle.
5. **PASS:** If the sequence matches and the Harness `send_packet` returns OK.

---

## 3. Harness Specification (`net_v1`)

```toml
namespace = "harness.net"
version = "1"

[message.send_packet]
fields = ["data:bytes"]
reply = ["status:u32"]

[message.receive_packet]
fields = ["buffer_cap:u32"]
reply = ["data:bytes", "status:u32"]
```

---

## 4. Implementation Phases

### Phase 1: Capture Tooling (S11.1)
- ✅ Implement `tools/trace/pci_mmio_tracer.c` for Linux guests.
- ✅ Update `capsule_relay` to support hardware trace types.
- ✅ Define `DriverProtocolTraceV0` schema.

### Phase 2: Mock Hardware Environment (S11.2)
- ✅ Create a `kernel_api::mock::pci_device` helper.
- ✅ Implement the `ReplayScoreboard` (compares live MMIO ops against the trace).
- ✅ Gate: `tools/ci/foundry_s11_replay.sh`.

### Phase 3: The Distillation (S11.3–S11.7)
- ✅ Assemble the scaffold Reference Vault for `virtio-net`.
- ✅ Translate `DriverProtocolTraceV0` events into `PciReplayEvent` arrays.
- ✅ Replace scaffold trace fixtures with live Linux Oracle capture artifacts.
- ✅ Distill init driver (`virtio_net_init`) and packet I/O (`virtio_net_packet`).
- ✅ Live hardware packet RX via kernel netdev Oracle capture.

### Phase 4: Runtime harness I/O (S11.8)
- ✅ `kernel_api::net_packet_oracle_vector` baked Oracle payloads.
- ✅ Kernel NET_V1 harness provider + `OP_NET_PACKET_IO` QEMU init profile.
- ✅ `foundry_s11_runtime_net_s11_8.sh` asserts serial `harness.net: packet_io ok`.

## 4b. S11 Definition of Done

S11 is complete when all four gates pass via `just s11`:

1. **Init replay** — `foundry_s11_replay.sh` replays live `oracle_init_trace.json` through `MockPciDevice`.
2. **Packet replay** — `foundry_s11_replay.sh` replays live `oracle_packet_trace.json` through `MockPacketHarness`.
3. **Live Oracle provenance** — `REQUIRE_LIVE_ORACLE_TRACE=1 foundry_s11_reference_vault_s11_3.sh` (init + packet + hardware RX).
4. **Runtime harness I/O** — `foundry_s11_runtime_net_s11_8.sh` boots QEMU and asserts typed `harness.net` send/receive over shared memory.

---

## 5. Success Metrics

- **Trace Fidelity:** 100% match on initialization MMIO sequence.
- **Safety:** Zero use of `unsafe` outside of the register mapping block.
- **Performance:** Native driver latency within 10% of the Linux Oracle (excluding VM overhead).
