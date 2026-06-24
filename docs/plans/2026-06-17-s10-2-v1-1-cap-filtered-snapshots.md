# S10.2 v1.1: Capability-Filtered Snapshots + Reactor Publish

**Last Updated:** 2026-06-17
**Status:** Complete
**Gate:** `tools/ci/foundry_semantic_state_s10_2.sh` (`capability_filter`, `domain_manager_reactor_publish`)

---

## Goal

Close the privacy gap left by S10.2.1: subscribers no longer all receive identical snapshot capability tokens. `domain_manager` inventory transitions publish filtered views per subscriber grants.

## Filter semantics (v1.1)

| Viewer grant | Visibility |
|--------------|------------|
| `services.semantic_state_v1` | Full platform snapshot (observer role) |
| Other interfaces | Kernel domain (id 0) + domains whose held capabilities intersect viewer grants + harness entries matching viewer grants |

Implementation: `artifact_store_schema::semantic_state::filter_platform_snapshot_for_viewer`.

## Reactor changes

- `handle_subscribe_with_viewer(request, viewer_interfaces)` stores per-subscription filter inputs.
- `publish_domain_inventory_changed(snapshot)` enqueues one `state_changed_event` per subscriber with a deterministic `shm_cap` token derived from the filtered snapshot bytes (host v1.1 stand-in until real shmem attach per subscriber).

## Domain manager aggregator

- `DomainManager` owns `semantic_state::SemanticReactor`.
- `notify_semantic_inventory_changed()` runs after successful `start_domain` / `stop_domain`.
- Builds live inventory snapshot and calls `publish_domain_inventory_changed`.

## Out of scope (deferred)

- WASM guest reactor loop
- Multi-source aggregation (kernel trace, store, network)
- IDL wire field for viewer caps on `subscribe` (host-side extension only in v1.1)

## Success

- `cargo test -p artifact_store_schema filter_`
- `cargo test -p semantic_state cap_filtered`
- `cargo test -p domain_manager inventory_snapshot_publish`
