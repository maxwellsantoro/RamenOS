//! Subscribe push-model for S10.2.1.

pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_MASK: u32 = 3;

/// Domain inventory change bit in `subscribe.event_mask`.
pub const EVENT_MASK_DOMAIN_STATE_CHANGED: u32 = 0x1;

/// `state_changed_event.event_type` for domain inventory changes.
pub const EVENT_TYPE_DOMAIN_STATE_CHANGED: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::SemanticReactor;
    use kernel_api::generated::semantic_state_v1::{StateChangedEvent, Subscribe};
    use kernel_api::wire::read_payload;

    #[test]
    fn subscribe_delivery_registers_and_delivers_typed_event() {
        let mut reactor = SemanticReactor::new();
        let reply = reactor.handle_subscribe(Subscribe {
            request_id: 42,
            event_mask: EVENT_MASK_DOMAIN_STATE_CHANGED,
        });
        assert_eq!(reply.status, STATUS_OK);
        assert_ne!(reply.subscription_id, 0);

        reactor.publish_domain_state_changed(0xDEAD_BEEF);
        let deliveries = reactor.reactor_tick();
        assert_eq!(deliveries.len(), 1);

        let event: StateChangedEvent =
            read_payload(&deliveries[0]).expect("state_changed_event payload");
        assert_eq!(event.subscription_id, reply.subscription_id);
        assert_eq!(event.event_type, EVENT_TYPE_DOMAIN_STATE_CHANGED);
        assert_eq!(event.shm_cap, 0xDEAD_BEEF);
    }
}
