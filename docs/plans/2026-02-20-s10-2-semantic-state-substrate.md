# S10.2: Semantic State Substrate Design

**Last Updated:** 2026-06-17
**Status:** Active (scaffold + v1 subscribe complete)
**Related:** PLATFORM_OVERVIEW.md, CONSTITUTION.md, AI-Native OS Vision

---

## Executive Summary

This document defines the architecture for the **Semantic State Service (SSS)**, the primary interface for AI agents to introspect and interact with RamenOS. Unlike legacy systems that require parsing text from `/proc` or scraping pixels, SSS exposes the entire OS topology and status as structured, deterministic, and semantically rich data.

**Key Design Decisions:**
- **JSON/Markdown Dual Representation:** Snapshots optimized for both machine ingestion (JSON) and LLM context-window reading (Markdown).
- **Capability-Gated Introspection:** Visibility into system state is restricted by the agent's held capabilities.
- **Hierarchical Topology:** State is represented as a graph of Domains, Services, Capabilities, and Resources.
- **Subscription Model:** Agents can subscribe to "Semantic Events" (e.g., "new capability granted", "domain crash") to avoid polling.

---

## 1. Overview & Goals

### Objective

Formalize the "Agentic Substrate" by implementing a dedicated service that provides a live, structured view of the operating system's internal state.

### Design Goals

| Goal | Rationale |
|------|-----------|
| **Structured Discovery** | Agents must be able to discover available harnesses and portals without prior knowledge. |
| **Deterministic Snapshots** | Identical system states must produce identical semantic snapshots. |
| **Context-Window Friendly** | State summaries must be dense and readable by LLMs (avoiding deep nesting or redundant metadata). |
| **Real-time Observability** | Agents should react to system changes (e.g., hardware hotplug) via a native notification stream. |

---

## 2. Architecture

### Component Overview

```
+-----------------------------------------------------------------------------+
|                         Semantic State Service (SSS)                        |
+-----------------------------------------------------------------------------+
|                                                                              |
|  +--------------+    +--------------+    +------------------------------+   |
|  |   Snapshot   |    |  Capability  |    |      Subscription            |   |
|  |   Engine     |<-->|   Filter     |<-->|      Manager                 |   |
|  | (JSON/MD)    |    |              |    |      (Push/SSE)              |   |
|  +------+-------+    +------+-------+    +------------------------------+   |
|         |                   |                                                |
|         v                   v                                                |
|  +-----------------------------------------------------------------------+   |
|  |                        System State Aggregator                        |   |
|  +-----------------------------------------------------------------------+   |
|         |                   |                   |                   |        |
|         v                   v                   v                   v        |
|  +-------------+    +--------------+    +--------------+    +--------------+ |
|  |   Domain    |    |   Kernel     |    |   Artifact   |    |   Network    | |
|  |   Manager   |    |   Trace      |    |   Store      |    |   Topology   | |
|  +-------------+    +--------------+    +--------------+    +--------------+ |
|                                                                              |
+-----------------------------------------------------------------------------+
```

### Data Model (The Semantic Graph)

The system state is exposed as a root-level `PlatformSnapshot`:

```json
{
  "timestamp": "2026-02-20T14:30:00Z",
  "system": {
    "arch": "x86_64",
    "boot_id": "uuid-...",
    "uptime_seconds": 3600
  },
  "topology": {
    "domains": [
      {
        "id": 1,
        "name": "native_shell",
        "status": "active",
        "capabilities": ["cap:harness:echo_v1", "cap:portal:file_picker"],
        "metrics": { "cpu_pct": 2.5, "mem_mb": 128 }
      },
      {
        "id": 2,
        "name": "linux_compat",
        "status": "quarantined",
        "sandboxed": true
      }
    ],
    "harnesses": [
      { "interface": "harness.echo_v1", "providers": [10, 11] },
      { "interface": "harness.net_v1", "providers": [5] }
    ]
  },
  "active_grants": [
    { "subject": "dom:1", "resource": "file:sha256:...", "rights": ["RO"] }
  ]
}
```

---

## 3. The Agentic Interface

### Semantic Snapshot API

Agents call `sss_get_snapshot(mask: u32)` to receive a tailored view.

**Markdown Example (for LLM context):**
```markdown
# RamenOS State Snapshot
**Uptime:** 1h 2m | **Status:** Nominal

## Domains
- [1] **Native Shell** (Active): Has echo_v1, file_picker.
- [2] **Linux Compat** (Quarantined): Running Flatpak.

## Warnings
- Domain [5] (GPU Manager) restarted 2 times in last 10m.
- Trace buffer [kernel] is at 85% capacity.
```

### Semantic Events (Push)

Instead of polling, agents subscribe to a `SemanticEvent` stream:

| Event Type | Payload |
|------------|---------|
| `DOMAIN_CRASH` | `{ "domain_id": 5, "crash_context_id": "sha256:..." }` |
| `CAP_GRANTED` | `{ "subject": 1, "interface": "harness.echo_v1" }` |
| `HIL_READY` | `{ "device": "nvme_controller", "status": "unclaimed" }` |

---

## 4. Scaffold vs v1

### Scaffold complete (S10.2.0) — DONE
- `semantic_state_v1` IDL + codegen (kernel_api, SDK, native_runner)
- `PlatformSnapshotV0` schema + `DomainInventoryEntry` builder
- WASM shmem snapshot delivery (`get_snapshot` / `get_snapshot_reply`)
- Live snapshot from `domain_manager` (`--emit-semantic-snapshot`, `--ingest-semantic-snapshot`)
- `store_cli ingest-platform-snapshot`
- Subscribe stub (`STATUS_NOT_IMPLEMENTED`) + gate assertion
- Gate: `foundry_semantic_state_s10_2.sh`

### v1 subscribe (S10.2.1) — DONE
- `SemanticReactor` host-side registry + `reactor_tick()` typed `state_changed_event` delivery
- `subscribe` registers interest; event mask bit `0x1` = domain inventory changed
- Minimal capability filtering (mask only); full privacy model deferred
- Design: `docs/plans/2026-06-17-s10-2-1-subscribe-reactor.md`
- Gate: `subscribe_delivery` in `foundry_semantic_state_s10_2.sh`

### v1.1+ — PLANNED
- Capability-filtered snapshots (visibility matches agent caps)
- Multi-source aggregator: domain_manager + kernel trace + store + (later) network
- Deterministic snapshot ordering for replay gates
- WASM guest reactor loop; `domain_manager` live IPC → reactor publish

---

## 5. Implementation Phases

### Phase 1: Aggregation (S10.2.1)
- Create `services/semantic_state` crate.
- Implement polling integration with `DomainManager` (for domain lists) and `KernelTraceRing` (for system logs).
- Define `SnapshotV0` Rust structs.

### Phase 2: Translation (S10.2.2)
- Implement JSON and Markdown serializers.
- Add `ramen::harness::semantic_state_v1` IDL contract.
- Create a test "State Observer" agent (simple WASM module) that prints the snapshot to the console.

### Phase 3: Notifications (S10.2.3)
- Implement the Event Stream (Push) mechanism.
- Add subscription filters (e.g., "only notify me about crashes in Domain 2").

---

## 6. Success Metrics

- **Semantic Density:** A 20-domain system state fits within 4KB of Markdown context.
- **Update Latency:** System events are reflected in the semantic stream in < 50ms.
- **Agent Zero-Knowledge Discovery:** A fresh agent can identify and call a new harness using *only* information from the Semantic State snapshot.
