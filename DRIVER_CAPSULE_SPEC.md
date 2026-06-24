# DRIVER_CAPSULE_SPEC.md
## Driver Capsule v0 — legacy driver/service hosted in a quarantined microVM behind typed harnesses

**Status:** draft
**Slice:** S3.x (post-S3), depends on S2.2 hardening (wire helpers, trace ring contract, negative gates)
**Primary goal:** Achieve day-1 hardware coverage without polluting kernel architecture, while producing protocol traces that enable eventual native rewrites.

---

## 0. Summary

A **Driver Capsule** is a quarantined domain (initially a Linux microVM) that hosts a legacy driver or driver-adjacent service and exposes it to the native RamenOS world **only through typed harness contracts**.

The capsule is treated as **hostile**:
- it is not part of the TCB,
- it can crash without taking the system down,
- it has sharply bounded resource access,
- and it must continuously emit **trace artifacts** usable by Foundry for replay/minimization.

This is not “compat for apps.” This is “compat for the hardware long tail.”

---

## 1. Goals (v0)

1. **Harness-first boundary**
   - Native clients speak only to a typed harness endpoint.
   - Capsule implementation is a replaceable backend.

2. **Quarantined execution**
   - Capsule can fail arbitrarily without kernel compromise.
   - Device access is explicit and minimized (IOMMU / mediated access).

3. **Protocol traces as spec**
   - Every capsule session can emit a **protocol_trace** (typed harness transcript).
   - Traces are stored in the Artifact Store as `trace_artifact_v0`.
   - Foundry can replay traces against the capsule backend (and later against native drivers).

4. **One “hello-world” device class**
   - v0 targets a single harness (recommend `net_harness_v0` or `block_harness_v0`).
   - The initial backend may “cheat” (Linux stack) as long as the harness contract and tracing are real.

---

## 2. Non-goals (v0)

- “Universal Windows XP driver support”
- Raw MMIO/DMA capture as the *primary* trace format
- Passing through arbitrary PCIe devices without a safety story
- High-performance data plane optimization (we define the shape; perf comes later)

---

## 3. Threat model & invariants

### 3.1 Trust model
- Capsule VM + its processes are **untrusted**.
- The **only** trusted components are:
  - the kernel + kernel_api contracts
  - runtime_supervisor + Foundry gate tooling (within their limited roles)
  - Artifact Store integrity checks

### 3.2 Required invariants
- **No ambient authority**: capsule has no implicit access to host FS, host network, or other devices.
- **Explicit device access**: any device access is capability-scoped and mediable.
- **No write-back**: capsule can never write to the artifact mount; any outputs must be explicit artifacts created by the host (or uploaded via a controlled channel).
- **Auditable boundary**: every harness call is traceable (request/response + metadata).

---

## 4. Architecture

### 4.1 Components

1. **runtime_supervisor**
   - Starts/stops the capsule VM (QEMU initially).
   - Provides IPC endpoints for native clients.
   - Records traces crossing the harness boundary.
   - Enforces policy (RO mounts, timeouts, resource limits).

2. **capsule VM**
   - Minimal Linux image (or later other OS) hosting:
     - a **capsule agent** process
     - a legacy driver/service (kernel driver, userspace daemon, vendor blob, etc.)

3. **capsule agent**
   - Speaks a typed “capsule control” protocol to runtime_supervisor.
   - Implements one or more harness backends (proxying to the legacy stack).
   - Provides health + version reporting.

4. **trace recorder**
   - Lives on the host side (runtime_supervisor).
   - Emits `trace_artifact_v0` with `protocol_trace` payload.

### 4.2 Data/control plane split (v0 shape)

- **Control plane**: typed messages (IDL) for:
  - capsule lifecycle (start/stop/health)
  - harness request/response framing
  - trace metadata and session ids

- **Data plane**: may start as:
  - bounded byte payloads inside messages (v0),
  - then evolve to shared-memory queues (v1) for throughput.

---

## 5. Interfaces

### 5.1 IDL namespaces (suggested)
- `capsule.control/v0`
  - `hello { capsule_id, backend_caps, versions }`
  - `health { status, last_error, stats }`
  - `shutdown { reason }`

- `capsule.harness_relay/v0`
  - `open_endpoint { harness_name, version } -> { endpoint_handle }`
  - `call { endpoint_handle, request_bytes } -> { response_bytes }`
  - `close_endpoint { endpoint_handle }`

> Note: the relay protocol is deliberately generic so one capsule can host multiple harnesses later.

### 5.2 Wire format constraints (depends on S2.2)
- All messages are versioned.
- Payloads are length-prefixed and validated.
- Unsafe reads/writes are centralized in one helper module (no ad-hoc casts).

---

## 6. Trace artifacts

### 6.1 `trace_artifact_v0`
Two trace types are supported; Driver Capsule v0 primarily uses **protocol_trace**.

#### A) protocol_trace (required for Driver Capsule v0)
A transcript of typed harness traffic at the relay boundary:

- session metadata:
  - `trace_id` (content address)
  - `timestamp_start/end`
  - `capsule_id` + capsule image hash
  - `harness_name` + version
  - `policy_bundle_id` (if applicable)

- events (ordered):
  - `t` (monotonic timestamp or sequence index)
  - `dir` (request/response)
  - `op` (optional string op name if known)
  - `bytes` (opaque request/response payload bytes)
  - `result` (ok/error code)
  - `notes` (optional)

This is the **spec-by-example** artifact. It must be replayable.

#### B) scenario_trace (optional in S3; used by apps/portals)
Not required for capsule v0 but allowed under same artifact umbrella.

### 6.2 Redaction & size policy (required)
- protocol_trace must support:
  - field-level redaction hooks (if harness schema is known)
  - size caps (truncate with marker)
  - opt-in export controls
- default: **local-only** unless user/system policy allows upload.

---

## 7. Replay model

### 7.1 Deterministic replay contract (v0)
Foundry can replay a protocol_trace by:
- launching the same capsule backend (same capsule image hash),
- sending the recorded request sequence,
- asserting response equivalence under a defined comparator:
  - exact byte match (v0 default),
  - or schema-aware comparison (later).

### 7.2 Minimize contract
If replay fails:
- Foundry attempts delta-debugging on event subsequences to find a minimal failing trace.
- The minimized trace becomes a new regression seed.

---

## 8. Policy & resource controls (v0)

- RO artifact mount only (enforced by runtime_supervisor + gate assertions).
- Fixed memory/CPU budget for capsule.
- No direct host networking unless explicitly bridged for the device class.
- Timeouts on all relay calls.
- Crash containment:
  - capsule exit triggers clean failure path
  - trace finalized with failure metadata

---

## 9. Slice deliverables & gates

### 9.1 S3.x Driver Capsule v0 deliverables
1. `DRIVER_CAPSULE_SPEC.md` (this doc)
2. `capsule.control/v0` IDL + codegen
3. `capsule.harness_relay/v0` IDL + codegen
4. runtime_supervisor:
   - launch capsule VM
   - handshake with capsule agent
   - relay calls + record protocol_trace
   - store trace_artifact_v0 in Artifact Store
5. Foundry gate:
   - boot capsule
   - run a small harness test sequence
   - assert protocol_trace emitted and stored
   - replay the trace (fresh capsule) and assert equivalence
   - negative tests (bad lengths, unknown version, RO mount violation attempts)

### 9.2 Success criteria
- A native test program can call a harness endpoint whose backend is hosted in the capsule.
- A trace is recorded, stored, replayed, and minimized on failure.
- Capsule compromise cannot yield host FS/network access beyond granted capabilities.

---

## 10. Evolution notes (post-v0)

- v1: shared-memory queues for data plane; schema-aware tracing
- v2: mediated PCIe passthrough for selected devices; richer evidence traces
- v3: native driver substitution + hot-swap guided by trace corpus equivalence
