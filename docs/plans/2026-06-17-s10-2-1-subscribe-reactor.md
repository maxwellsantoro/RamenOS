# S10.2.1: Subscribe Reactor (v1)

**Last Updated:** 2026-06-17
**Status:** Active
**Gate:** `tools/ci/foundry_semantic_state_s10_2.sh` (`subscribe_delivery` step)

---

## Goal

Prove a real host-side event path for semantic state before S10.5 host→QEMU integration. S10.5 should bridge an existing reactor, not invent push semantics mid-slice.

## Reactor ownership

| Component | Owner | Role |
|-----------|-------|------|
| `SemanticReactor` | `services/semantic_state` (host `rlib`) | Subscription registry, pending event queue, `reactor_tick()` delivery |
| `subscribe` handler | `semantic_state::subscribe` + reactor | Registers interest; returns `SubscribeReply` (not `NOT_IMPLEMENTED`) |
| State sources (v1) | Callers of `publish_domain_state_changed` | Domain inventory transitions enqueue `state_changed_event` |
| WASM guest loop | `semantic_state` cdylib (future) | Calls `subscribe`, then blocks/polls for pushed events via harness imports |
| Capability filter | Deferred (v1.1+) | v1 matches `event_mask` only; all subscribers see the same snapshot cap |

**Boundary:** The reactor lives in the semantic-state service crate on the host. `domain_manager` does not own the reactor; it may call `SemanticReactor::publish_domain_state_changed` when inventory changes (wired in a later aggregator phase). v1 proves delivery with unit tests and typed wire envelopes.

## Event flow (v1)

```
Agent/subscriber          SemanticReactor              State source
     |                          |                            |
     |-- subscribe(mask) ------>| register subscription        |
     |<-- subscribe_reply ------|                            |
     |                          |<-- publish_domain_changed --|
     |                          | enqueue StateChangedEvent    |
     |                          |                            |
     |<-- reactor_tick() -------| drain typed Envelope bytes   |
```

Delivery uses existing IDL types (`Subscribe`, `SubscribeReply`, `StateChangedEvent`) and `kernel_api::wire` encoding into `Envelope` protocol `10`.

## Event mask (v1)

| Bit | Name | `event_type` |
|-----|------|--------------|
| `0x1` | Domain inventory changed | `1` |

Empty mask is rejected (`STATUS_INVALID_MASK`).

## Out of scope (v1)

- Multi-source aggregation (kernel trace, store, network)
- Full capability-filtered snapshot visibility
- QEMU / target placement
- Rich compute-fabric live emission
- Persistent subscriptions across process restart

## Success

`cargo test -p semantic_state subscribe_delivery` passes: subscribe → publish domain change → `reactor_tick` yields one `state_changed_event` envelope with matching `subscription_id` and `shm_cap`.
