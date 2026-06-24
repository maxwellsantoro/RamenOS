//! virtio-net packet I/O driver distilled from the harness.net Oracle trace.
//!
//! Replays shared-memory `send_packet` / `receive_packet` exchanges against
//! `MockPacketHarness`.

use kernel_api::generated::{ReceivePacket, ReceivePacketReply, SendPacket, SendPacketReply};
use kernel_api::mock::packet_harness::{
    MockPacketHarness, PacketReplayEvent, PacketReplayMismatch, PacketReplayOp, PacketReplayResult,
};

pub const RECV_BUFFER_LEN: u32 = 1500;

/// Run the packet exchange sequence described by the Oracle trace events.
pub fn exchange_packets_from_events(
    harness: &mut MockPacketHarness<'_>,
    events: &[PacketReplayEvent<'_>],
) -> PacketReplayResult<()> {
    let mut send_event = None;
    let mut recv_event = None;

    for event in events {
        match event.op {
            PacketReplayOp::SendPacket => send_event = Some(*event),
            PacketReplayOp::ReceivePacket => recv_event = Some(*event),
        }
    }

    let send = send_event.ok_or(PacketReplayMismatch::Incomplete {
        expected_total: events.len(),
        observed_total: 0,
    })?;
    let recv = recv_event.ok_or(PacketReplayMismatch::Incomplete {
        expected_total: events.len(),
        observed_total: 1,
    })?;

    send_packet_from_event(harness, send)?;
    receive_packet_from_event(harness, recv)?;
    harness.finish()
}

/// Stage and transmit one Oracle send event through `harness.net`.
pub fn send_packet_from_event(
    harness: &mut MockPacketHarness<'_>,
    event: PacketReplayEvent<'_>,
) -> PacketReplayResult<SendPacketReply> {
    harness
        .shmem()
        .write(event.shm_cap, event.offset, event.payload)?;

    let reply = harness.send_packet(SendPacket {
        request_id: event.request_id,
        data_shm_cap: event.shm_cap,
        data_offset: event.offset,
        data_len: event.len,
    })?;
    if reply.status != event.status {
        return Err(PacketReplayMismatch::Status {
            expected: event.status,
            observed: reply.status,
        });
    }
    if reply.bytes_sent != event.bytes {
        return Err(PacketReplayMismatch::Bytes {
            expected: event.bytes,
            observed: reply.bytes_sent,
        });
    }
    Ok(reply)
}

/// Receive one Oracle receive event into shared memory.
pub fn receive_packet_from_event(
    harness: &mut MockPacketHarness<'_>,
    event: PacketReplayEvent<'_>,
) -> PacketReplayResult<ReceivePacketReply> {
    let reply = harness.receive_packet(ReceivePacket {
        request_id: event.request_id,
        buffer_shm_cap: event.shm_cap,
        buffer_offset: event.offset,
        buffer_len: RECV_BUFFER_LEN.max(event.len),
    })?;
    if reply.status != event.status {
        return Err(PacketReplayMismatch::Status {
            expected: event.status,
            observed: reply.status,
        });
    }
    if reply.bytes_received != event.bytes {
        return Err(PacketReplayMismatch::Bytes {
            expected: event.bytes,
            observed: reply.bytes_received,
        });
    }

    let observed = harness
        .shmem()
        .read(event.shm_cap, event.offset, event.len)?;
    if observed != event.payload {
        return Err(PacketReplayMismatch::Payload);
    }

    Ok(reply)
}

/// Replay the vault oracle packet trace through the distilled driver.
pub fn replay_vault_packet_trace() -> Result<(), crate::DriverReplayError> {
    let trace = crate::load_packet_trace(vault_packet_fixture_path())?;
    replay_packet_trace(&trace)
}

/// Replay a validated harness packet trace through the distilled driver.
pub fn replay_packet_trace(
    trace: &artifact_store_schema::net_packet_trace::NetPacketTraceV0,
) -> Result<(), crate::DriverReplayError> {
    let bundle = crate::packet_trace_to_replay_bundle(trace)?;
    bundle.with_events(|events| {
        let mut harness = MockPacketHarness::new(events, bundle.shm_cap());
        exchange_packets_from_events(&mut harness, events)
            .map_err(|err| crate::DriverReplayError::PacketReplay(format!("{err:?}")))
    })
}

fn vault_packet_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../drivers/reference_vaults/virtio-net/traces/oracle_packet_trace.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEND_PAYLOAD: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    const RECV_PAYLOAD: [u8; 4] = [0xca, 0xfe, 0xba, 0xbe];

    #[test]
    fn send_packet_from_event_stages_oracle_payload_before_send() {
        let trace = [PacketReplayEvent {
            op: PacketReplayOp::SendPacket,
            request_id: 1,
            shm_cap: 0x1000,
            offset: 0,
            len: 4,
            payload: &SEND_PAYLOAD,
            status: 0,
            bytes: 4,
        }];
        let mut harness = MockPacketHarness::new(&trace, 0x1000);
        send_packet_from_event(&mut harness, trace[0]).unwrap();
        assert_eq!(harness.shmem().read(0x1000, 0, 4).unwrap(), &SEND_PAYLOAD);
    }

    #[test]
    fn exchange_packets_from_events_replays_send_then_receive() {
        let trace = [
            PacketReplayEvent {
                op: PacketReplayOp::SendPacket,
                request_id: 1,
                shm_cap: 0x1000,
                offset: 0,
                len: 4,
                payload: &SEND_PAYLOAD,
                status: 0,
                bytes: 4,
            },
            PacketReplayEvent {
                op: PacketReplayOp::ReceivePacket,
                request_id: 2,
                shm_cap: 0x1000,
                offset: 256,
                len: 4,
                payload: &RECV_PAYLOAD,
                status: 0,
                bytes: 4,
            },
        ];
        let mut harness = MockPacketHarness::new(&trace, 0x1000);
        exchange_packets_from_events(&mut harness, &trace).unwrap();
    }

    #[test]
    fn replay_vault_packet_trace_matches_oracle_fixture() {
        replay_vault_packet_trace().expect("vault packet trace must replay");
    }
}
