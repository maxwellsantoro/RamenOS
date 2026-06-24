//! Host-side subscribe reactor for semantic-state push delivery (S10.2.1+).

use artifact_store_schema::semantic_state::{
    PlatformSnapshotV0, filter_platform_snapshot_for_viewer,
};
use kernel_api::generated::semantic_state_v1::{StateChangedEvent, Subscribe, SubscribeReply};
use kernel_api::ipc::Envelope;
use kernel_api::wire::write_payload;

use crate::subscribe::{
    EVENT_MASK_DOMAIN_STATE_CHANGED, EVENT_TYPE_DOMAIN_STATE_CHANGED, STATUS_INVALID_MASK,
    STATUS_OK,
};

pub const PROTOCOL_SEMANTIC_STATE_V1: u32 = 10;
pub const MSG_STATE_CHANGED_EVENT: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Subscription {
    id: u64,
    event_mask: u32,
    viewer_interfaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingEvent {
    subscription_id: u64,
    event_type: u32,
    shm_cap: u64,
}

/// Owns subscription registry and pending push events for semantic-state v1.
#[derive(Debug, Default)]
pub struct SemanticReactor {
    next_subscription_id: u64,
    subscriptions: Vec<Subscription>,
    pending: Vec<PendingEvent>,
}

impl SemanticReactor {
    pub fn new() -> Self {
        Self {
            next_subscription_id: 1,
            ..Self::default()
        }
    }

    /// Handle `subscribe` — registers interest when the mask is non-zero.
    pub fn handle_subscribe(&mut self, request: Subscribe) -> SubscribeReply {
        self.handle_subscribe_with_viewer(request, Vec::new())
    }

    /// Handle `subscribe` with viewer capability interfaces for snapshot filtering (S10.2 v1.1).
    pub fn handle_subscribe_with_viewer(
        &mut self,
        request: Subscribe,
        viewer_interfaces: Vec<String>,
    ) -> SubscribeReply {
        if request.event_mask == 0 {
            return SubscribeReply {
                request_id: request.request_id,
                status: STATUS_INVALID_MASK,
                subscription_id: 0,
            };
        }

        let subscription_id = self.next_subscription_id;
        self.next_subscription_id = self.next_subscription_id.saturating_add(1);
        self.subscriptions.push(Subscription {
            id: subscription_id,
            event_mask: request.event_mask,
            viewer_interfaces,
        });

        SubscribeReply {
            request_id: request.request_id,
            status: STATUS_OK,
            subscription_id,
        }
    }

    /// Enqueue domain inventory change notifications for matching subscribers.
    pub fn publish_domain_state_changed(&mut self, shm_cap: u64) {
        for sub in &self.subscriptions {
            if sub.event_mask & EVENT_MASK_DOMAIN_STATE_CHANGED != 0 {
                self.pending.push(PendingEvent {
                    subscription_id: sub.id,
                    event_type: EVENT_TYPE_DOMAIN_STATE_CHANGED,
                    shm_cap,
                });
            }
        }
    }

    /// Publish per-subscriber filtered snapshot capability tokens (S10.2 v1.1).
    pub fn publish_domain_inventory_changed(&mut self, snapshot: &PlatformSnapshotV0) {
        for sub in &self.subscriptions {
            if sub.event_mask & EVENT_MASK_DOMAIN_STATE_CHANGED == 0 {
                continue;
            }
            let viewer: Vec<&str> = sub.viewer_interfaces.iter().map(String::as_str).collect();
            let filtered = filter_platform_snapshot_for_viewer(snapshot, &viewer);
            let shm_cap = snapshot_shm_cap_token(&filtered);
            self.pending.push(PendingEvent {
                subscription_id: sub.id,
                event_type: EVENT_TYPE_DOMAIN_STATE_CHANGED,
                shm_cap,
            });
        }
    }

    /// Drain pending events into typed IPC envelopes (one reactor tick).
    pub fn reactor_tick(&mut self) -> Vec<Envelope> {
        let pending = std::mem::take(&mut self.pending);
        pending
            .into_iter()
            .filter_map(|event| encode_state_changed_event(event).ok())
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }
}

fn encode_state_changed_event(event: PendingEvent) -> Result<Envelope, ()> {
    let mut env = Envelope::empty(PROTOCOL_SEMANTIC_STATE_V1, MSG_STATE_CHANGED_EVENT);
    let payload = StateChangedEvent {
        subscription_id: event.subscription_id,
        event_type: event.event_type,
        shm_cap: event.shm_cap,
    };
    write_payload(&mut env, &payload).map_err(|_| ())?;
    Ok(env)
}

fn snapshot_shm_cap_token(snapshot: &PlatformSnapshotV0) -> u64 {
    let bytes = serde_json::to_vec(snapshot).unwrap_or_default();
    let mut hash: u64 = 0;
    for byte in bytes {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    0xA000_0000_0000_0000 | (hash & 0x0000_FFFF_FFFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::wire::read_payload;

    #[test]
    fn subscribe_delivery_delivers_state_changed_event() {
        let mut reactor = SemanticReactor::new();
        let reply = reactor.handle_subscribe(Subscribe {
            request_id: 7,
            event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
        });
        assert_eq!(reply.status, STATUS_OK);
        assert_eq!(reply.subscription_id, 1);

        reactor.publish_domain_state_changed(0xA000);
        assert_eq!(reactor.pending_count(), 1);

        let deliveries = reactor.reactor_tick();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].protocol, PROTOCOL_SEMANTIC_STATE_V1);
        assert_eq!(deliveries[0].msg_type, MSG_STATE_CHANGED_EVENT);

        let event: StateChangedEvent = read_payload(&deliveries[0]).expect("decode event");
        assert_eq!(event.subscription_id, 1);
        assert_eq!(event.event_type, EVENT_TYPE_DOMAIN_STATE_CHANGED);
        assert_eq!(event.shm_cap, 0xA000);
    }

    #[test]
    fn subscribe_rejects_empty_event_mask() {
        let mut reactor = SemanticReactor::new();
        let reply = reactor.handle_subscribe(Subscribe {
            request_id: 1,
            event_mask: 0,
        });
        assert_eq!(reply.status, STATUS_INVALID_MASK);
        assert_eq!(reply.subscription_id, 0);
        assert_eq!(reactor.subscription_count(), 0);
    }

    #[test]
    fn event_mask_filters_non_matching_subscribers() {
        let mut reactor = SemanticReactor::new();
        reactor.handle_subscribe(Subscribe {
            request_id: 1,
            event_mask: 0x2, // not domain-state bit
        });
        reactor.publish_domain_state_changed(0x100);
        assert_eq!(reactor.pending_count(), 0);
        assert!(reactor.reactor_tick().is_empty());
    }

    #[test]
    fn multiple_subscribers_receive_independent_events() {
        let mut reactor = SemanticReactor::new();
        let first = reactor.handle_subscribe(Subscribe {
            request_id: 1,
            event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
        });
        let second = reactor.handle_subscribe(Subscribe {
            request_id: 2,
            event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
        });

        reactor.publish_domain_state_changed(0x200);
        let deliveries = reactor.reactor_tick();
        assert_eq!(deliveries.len(), 2);

        let mut ids: Vec<u64> = deliveries
            .iter()
            .map(|env| {
                read_payload::<StateChangedEvent>(env)
                    .unwrap()
                    .subscription_id
            })
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![first.subscription_id, second.subscription_id]);
    }

    #[test]
    fn cap_filtered_publish_assigns_distinct_shm_caps() {
        use artifact_store_schema::semantic_state::{
            CapInfoV0, DomainInventoryEntry, DomainStatusV0, HarnessInfoV0, OBSERVER_INTERFACE,
            PlatformSnapshotV0, SystemInfoV0,
        };

        let mut reactor = SemanticReactor::new();
        let observer = reactor.handle_subscribe_with_viewer(
            Subscribe {
                request_id: 1,
                event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
            },
            vec![OBSERVER_INTERFACE.into()],
        );
        let echo_only = reactor.handle_subscribe_with_viewer(
            Subscribe {
                request_id: 2,
                event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
            },
            vec!["harness.echo".into()],
        );

        let snapshot = PlatformSnapshotV0::from_inventory(
            "2026-06-17T00:00:00Z",
            SystemInfoV0 {
                arch: "x86_64".into(),
                boot_id: "boot".into(),
                uptime_seconds: 1,
            },
            vec![
                DomainInventoryEntry {
                    id: 0,
                    name: "kernel".into(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![],
                    metrics: None,
                },
                DomainInventoryEntry {
                    id: 1,
                    name: "echo".into(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![CapInfoV0 {
                        handle: 1,
                        interface: "harness.echo".into(),
                        rights: vec!["read".into()],
                    }],
                    metrics: None,
                },
                DomainInventoryEntry {
                    id: 2,
                    name: "secret".into(),
                    status: DomainStatusV0::Active,
                    capabilities: vec![CapInfoV0 {
                        handle: 2,
                        interface: "harness.secret".into(),
                        rights: vec!["read".into()],
                    }],
                    metrics: None,
                },
            ],
            vec![
                HarnessInfoV0 {
                    interface: "harness.echo".into(),
                    providers: vec![1],
                },
                HarnessInfoV0 {
                    interface: "harness.secret".into(),
                    providers: vec![2],
                },
            ],
        );

        reactor.publish_domain_inventory_changed(&snapshot);
        let deliveries = reactor.reactor_tick();
        assert_eq!(deliveries.len(), 2);

        let observer_cap = deliveries
            .iter()
            .find(|env| {
                read_payload::<StateChangedEvent>(env)
                    .map(|event| event.subscription_id == observer.subscription_id)
                    .unwrap_or(false)
            })
            .and_then(|env| read_payload::<StateChangedEvent>(env).ok())
            .map(|event| event.shm_cap)
            .expect("observer delivery");
        let echo_cap = deliveries
            .iter()
            .find(|env| {
                read_payload::<StateChangedEvent>(env)
                    .map(|event| event.subscription_id == echo_only.subscription_id)
                    .unwrap_or(false)
            })
            .and_then(|env| read_payload::<StateChangedEvent>(env).ok())
            .map(|event| event.shm_cap)
            .expect("echo delivery");

        assert_ne!(observer_cap, echo_cap);
        assert_ne!(observer_cap, 0);
        assert_ne!(echo_cap, 0);
    }
}
